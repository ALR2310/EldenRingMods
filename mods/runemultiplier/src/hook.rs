//! Multiplies rune gains from every source by a configurable factor: one hook
//! on `AddSoul_Call`, the game's lowest-level "add this many runes" function
//! (enemy kills and rune items alike). Its first 12 bytes are replaced by an
//! absolute jump into a generated stub that scales `EDX` (the amount), replays
//! those bytes and jumps back. The disassembly, the anchor, why spends are
//! never scaled and the history: `docs/addsoul_hook.md`.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use eldenring::cs::CSTaskGroupIndex;

use common::{codepatch, config, logger, memscan};

/// Only used to LOCATE `AddSoul_Call` (not patched): the enemy-kill reward
/// scaling step (`mulss xmm0,xmm1`) from the "Rune Multiplier" CE table entry,
/// since `AddSoul_Call` has no distinctive byte pattern of its own.
const ANCHOR_PATTERN: &str = "F3 0F 59 C1 F3 0F 2C F8 48 8B 8B";

/// `call AddSoul_Call` sits at anchor + 0x11 (`mulss` 4 + `cvttss2si` 4 +
/// `mov rcx,[rbx+0x570]` 7 + `mov edx,edi` 2).
const ADDSOUL_CALL_SITE_OFFSET: usize = 0x11;

/// The first 3 instructions of `AddSoul_Call` (`mov r9d,[rcx+0x6c]`,
/// `xor r11d,r11d`, `mov [rsp+0x10],r11d`), overwritten. None reads `RDX`, so
/// the multiply can run first and the copied originals replay after it.
const ADDSOUL_PREFIX_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

/// The multiplier as a Q20 fixed-point integer (`round(multiplier * 2^20)`),
/// read by the stub through a pointer baked in at install time. A reload
/// updates it in place; the stub always re-reads it, so nothing is re-patched.
static FIXED_Q20: AtomicI64 = AtomicI64::new(1 << 20);

fn to_q20(multiplier: f64) -> i64 {
    (multiplier * (1i64 << 20) as f64 + 0.5) as i64
}

fn init_multiplier() {
    let multiplier = config::get_double("Multiplier", 1.0);
    set_multiplier(multiplier);
    logger::log(&format!("Multiplier={multiplier:.3}"));
}

/// Applies a multiplier live (also called from the in-game menu number row).
pub fn set_multiplier(multiplier: f64) {
    FIXED_Q20.store(to_q20(multiplier), Ordering::Relaxed);
}

/// The absolute target of the 5-byte `E8 rel32` call at `call_site`, or `None`
/// if the byte there isn't `E8` (the layout differs: fail safe rather than
/// jump somewhere wrong).
fn resolve_call_target(call_site: *const u8) -> Option<*mut u8> {
    unsafe {
        if *call_site != 0xE8 {
            return None;
        }
        let rel = i32::from_le_bytes(*(call_site.add(1) as *const [u8; 4]));
        Some(call_site.add(5).offset(rel as isize) as *mut u8)
    }
}

/// Length of the multiply block: `mov r10,imm64` + `mov r8,[r10]` + `imul` +
/// `sar` + `mov edx,eax`. The `jle` before it skips exactly this much.
const MULTIPLY_BLOCK_LEN: u8 = 10 + 3 + 4 + 4 + 2;

/// The stub's machine code: scale `EDX` by the Q20 value at `fixed_q20_addr`
/// with integer math only (no XMM register is assumed safe to clobber:
/// `AddSoul_Call` is called from far more places than this hook), replay
/// `original_prefix`, then jump to `return_addr` (`AddSoul_Call` + the prefix).
///
/// `AddSoul_Call` serves every rune change, gains and spends alike (a signed
/// `EDX`); only gains are scaled, so the multiply is skipped when the amount
/// is `<= 0` and level-up costs and shop prices stay untouched.
fn build_stub(original_prefix: &[u8; ADDSOUL_PREFIX_LEN], fixed_q20_addr: u64, return_addr: u64) -> Vec<u8> {
    let mut body = Vec::with_capacity(55);

    body.extend_from_slice(&[0x48, 0x63, 0xC2]); // movsxd rax, edx
    body.extend_from_slice(&[0x85, 0xC0]); // test eax, eax
    body.extend_from_slice(&[0x7E, MULTIPLY_BLOCK_LEN]); // jle skip_multiply

    body.extend_from_slice(&[0x49, 0xBA]); // mov r10, &FIXED_Q20
    body.extend_from_slice(&fixed_q20_addr.to_le_bytes());
    body.extend_from_slice(&[0x4D, 0x8B, 0x02]); // mov r8, [r10]
    body.extend_from_slice(&[0x49, 0x0F, 0xAF, 0xC0]); // imul rax, r8
    body.extend_from_slice(&[0x48, 0xC1, 0xF8, 0x14]); // sar rax, 20 (undo the Q20 scale)
    body.extend_from_slice(&[0x89, 0xC2]); // mov edx, eax

    // skip_multiply: the overwritten instructions.
    body.extend_from_slice(original_prefix);

    // Absolute jump back, so the stub can live anywhere in the address space.
    body.extend_from_slice(&[0x49, 0xBB]); // mov r11, return_addr
    body.extend_from_slice(&return_addr.to_le_bytes());
    body.extend_from_slice(&[0x41, 0xFF, 0xE3]); // jmp r11

    body
}

/// Finds `AddSoul_Call` from the anchor and redirects its first 12 bytes into
/// a freshly allocated stub. `false` (logged) on any failure, with the
/// original bytes left untouched.
fn install() -> bool {
    let Some(anchor) = memscan::find_pattern_in_module(ANCHOR_PATTERN) else {
        logger::error("Hook: anchor not found, game may need a mod update, check Nexus.");
        return false;
    };

    let call_site = unsafe { anchor.add(ADDSOUL_CALL_SITE_OFFSET) };
    let Some(addsoul_entry) = resolve_call_target(call_site) else {
        logger::error("Hook: AddSoul_Call not resolved (byte layout differs)");
        return false;
    };

    // Copied live from the game, never hardcoded.
    let original_prefix: [u8; ADDSOUL_PREFIX_LEN] =
        unsafe { std::slice::from_raw_parts(addsoul_entry, ADDSOUL_PREFIX_LEN) }
            .try_into()
            .unwrap();
    let return_addr = unsafe { addsoul_entry.add(ADDSOUL_PREFIX_LEN) } as u64;
    let stub_body = build_stub(&original_prefix, &FIXED_Q20 as *const AtomicI64 as u64, return_addr);

    let Some(stub) = codepatch::alloc_executable(stub_body.len()) else {
        logger::error("Hook: stub alloc failed");
        return false;
    };
    unsafe { std::ptr::copy_nonoverlapping(stub_body.as_ptr(), stub, stub_body.len()) };

    // The jump fills the whole overwritten region exactly, no NOP padding.
    if !unsafe { codepatch::overwrite_bytes(addsoul_entry, &codepatch::absolute_jmp(stub as u64)) } {
        logger::error("Hook: patch failed (VirtualProtect)");
        return false;
    }

    logger::log("Hook: installed");
    true
}

/// Installs the hook, then re-reads `Multiplier` on every reload (the reload
/// key and banner belong to `common::reload`). Run it on its own worker thread;
/// never returns, except early when the hook fails to install.
pub fn run(ini_path: String, hmodule: u64) {
    init_multiplier();
    // Right away: ERNativeUI only accepts registrations during its startup window.
    crate::menu::start(hmodule, ini_path.clone());

    if !install() {
        logger::error("Hook: install failed, disabled");
        // Right away, not after CSTaskImp: this path never gets there.
        common::diag::log_environment();
        return;
    }

    // `common::reload` must stay the only watcher of the reload key, so this
    // just follows `RELOAD_GENERATION`.
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
    std::thread::spawn(move || common::reload::run(ini_path));

    let cs_task = common::task::wait_for_cs_task();
    common::diag::log_environment();

    let registered = common::task::run_recurring_safe(
        cs_task,
        "RuneMultiplier",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation != last_seen_generation {
                last_seen_generation = generation;
                init_multiplier();
                crate::menu::sync_from_config();
            }
        },
    );
    if !registered {
        logger::error("Reload: task registration failed, hot reload off");
    }

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q20_conversion_rounds_to_the_nearest_step() {
        assert_eq!(to_q20(1.0), 1 << 20);
        assert_eq!(to_q20(2.0), 2 << 20);
        assert_eq!(to_q20(0.5), 1 << 19);
        assert_eq!(to_q20(0.0), 0);
        assert_eq!(to_q20(1.5), 3 << 19);
    }

    #[test]
    fn the_stub_has_the_expected_shape() {
        let prefix: [u8; ADDSOUL_PREFIX_LEN] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let stub = build_stub(&prefix, 0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444);

        // movsxd rax, edx; test eax, eax; jle +MULTIPLY_BLOCK_LEN
        assert_eq!(stub[..7], [0x48, 0x63, 0xC2, 0x85, 0xC0, 0x7E, MULTIPLY_BLOCK_LEN]);
        // The multiply block loads the address of the multiplier...
        assert_eq!(stub[7..9], [0x49, 0xBA]);
        assert_eq!(stub[9..17], 0xAAAA_BBBB_CCCC_DDDDu64.to_le_bytes());
        // ...and the jle lands exactly on the replayed prefix.
        let skip_target = 7 + MULTIPLY_BLOCK_LEN as usize;
        assert_eq!(stub[skip_target..skip_target + ADDSOUL_PREFIX_LEN], prefix);
        // After it: mov r11, return_addr; jmp r11, and nothing else.
        let tail = &stub[skip_target + ADDSOUL_PREFIX_LEN..];
        assert_eq!(tail[..2], [0x49, 0xBB]);
        assert_eq!(tail[2..10], 0x1111_2222_3333_4444u64.to_le_bytes());
        assert_eq!(tail[10..], [0x41, 0xFF, 0xE3]);
        assert_eq!(stub.len(), skip_target + ADDSOUL_PREFIX_LEN + 13);
    }
}
