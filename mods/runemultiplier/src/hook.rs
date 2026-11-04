//! Multiplies rune gains from every source by one factor: a hook on `AddSoul_Call`, the game's
//! lowest-level "add this many runes" function (enemy kills and rune items alike). Its first 12
//! bytes become an absolute jump into a generated stub that scales `EDX` (the amount), replays
//! those bytes and jumps back. The disassembly, the anchor and why spends are never scaled:
//! `docs/addsoul_hook.md`.

use std::sync::atomic::{AtomicI64, Ordering};

use shared::{codepatch, logger, memscan};

use crate::config;

/// Only used to locate `AddSoul_Call`, which has no distinctive pattern of its own: the enemy-kill
/// reward scaling step (`mulss xmm0,xmm1`).
const ANCHOR_PATTERN: &str = "F3 0F 59 C1 F3 0F 2C F8 48 8B 8B";

/// `call AddSoul_Call` sits at anchor + 0x11: `mulss` 4, `cvttss2si` 4, `mov rcx,[rbx+0x570]` 7 and
/// `mov edx,edi` 2.
const ADDSOUL_CALL_SITE_OFFSET: usize = 0x11;

/// The first 3 instructions of `AddSoul_Call` (`mov r9d,[rcx+0x6c]`, `xor r11d,r11d`,
/// `mov [rsp+0x10],r11d`), overwritten. None reads `RDX`, so the multiply can run first and the
/// copied originals replay after it.
const ADDSOUL_PREFIX_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

/// The multiplier as a Q20 fixed-point integer (`round(multiplier * 2^20)`), read by the stub
/// through a pointer baked in at install time. A reload updates it in place, so nothing is
/// re-patched.
static FIXED_Q20: AtomicI64 = AtomicI64::new(1 << 20);

fn to_q20(multiplier: f64) -> i64 {
    (multiplier * (1i64 << 20) as f64 + 0.5) as i64
}

/// Applies a multiplier live; also called from the in-game menu row.
pub fn set_multiplier(multiplier: f64) {
    FIXED_Q20.store(to_q20(multiplier), Ordering::Relaxed);
}

/// Applies `[Settings] Multiplier` of the config in use.
pub fn apply_config() {
    let multiplier = config::get().settings.multiplier;
    set_multiplier(multiplier);
    logger::log(&format!("Rune: multiplier {multiplier:.3}"));
}

/// The target of the `call` at `call_site`; `None` if the byte there is not `E8`, which means the
/// layout differs and jumping on would go somewhere wrong.
fn resolve_call_target(call_site: *const u8) -> Option<*mut u8> {
    // SAFETY: `call_site` is inside the pattern the scan matched, in the game image.
    if unsafe { call_site.read() } != 0xE8 {
        return None;
    }
    // SAFETY: as above, and the opcode is a 5-byte call.
    unsafe { codepatch::rel32_target(call_site) }.map(|target| target as *mut u8)
}

/// Length of the multiply block: `mov r10,imm64`, `mov r8,[r10]`, `imul`, `sar` and `mov edx,eax`.
/// The `jle` before it skips exactly this much.
const MULTIPLY_BLOCK_LEN: u8 = 10 + 3 + 4 + 4 + 2;

/// The stub: scale `EDX` by the Q20 value at `fixed_q20_addr` with integer math only (no XMM
/// register is safe to clobber, since `AddSoul_Call` has many callers), replay `original_prefix`,
/// then jump to `return_addr`.
///
/// `AddSoul_Call` serves every rune change, gains and spends alike (a signed `EDX`). Only gains are
/// scaled: the multiply is skipped when the amount is `<= 0`, so level-up costs and shop prices
/// stay as they are.
fn build_stub(
    original_prefix: &[u8; ADDSOUL_PREFIX_LEN],
    fixed_q20_addr: u64,
    return_addr: u64,
) -> Vec<u8> {
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

    body.extend_from_slice(&[0x49, 0xBB]); // mov r11, return_addr
    body.extend_from_slice(&return_addr.to_le_bytes());
    body.extend_from_slice(&[0x41, 0xFF, 0xE3]); // jmp r11

    body
}

/// Finds `AddSoul_Call` from the anchor and redirects its first 12 bytes into a new stub. `false`,
/// with an error logged and the original bytes untouched, on any failure.
pub fn install() -> bool {
    let Some(anchor) = memscan::find_pattern_in_module(ANCHOR_PATTERN) else {
        logger::error("Hook: anchor not found, the game may need a mod update");
        return false;
    };

    // SAFETY: the offset is inside the matched pattern's neighborhood, in the game image.
    let call_site = unsafe { anchor.add(ADDSOUL_CALL_SITE_OFFSET) };
    let Some(addsoul_entry) = resolve_call_target(call_site) else {
        logger::error("Hook: AddSoul_Call not resolved, the byte layout differs");
        return false;
    };

    // SAFETY: `addsoul_entry` is the start of a game function, longer than the prefix.
    let original_prefix: [u8; ADDSOUL_PREFIX_LEN] =
        unsafe { std::slice::from_raw_parts(addsoul_entry, ADDSOUL_PREFIX_LEN) }
            .try_into()
            .unwrap();
    // SAFETY: as above, one past the prefix is still inside the function.
    let return_addr = unsafe { addsoul_entry.add(ADDSOUL_PREFIX_LEN) } as u64;
    let stub_body =
        build_stub(&original_prefix, &FIXED_Q20 as *const AtomicI64 as u64, return_addr);

    let Some(stub) = codepatch::alloc_executable(stub_body.len()) else {
        logger::error("Hook: VirtualAlloc failed");
        return false;
    };
    // SAFETY: `stub` is a fresh allocation of `stub_body.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(stub_body.as_ptr(), stub, stub_body.len()) };

    // SAFETY: the jump fills the whole 12-byte prefix exactly, so no half instruction is left.
    if !unsafe { codepatch::overwrite_bytes(addsoul_entry, &codepatch::absolute_jmp(stub as u64)) }
    {
        logger::error("Hook: VirtualProtect failed");
        return false;
    }

    logger::log("Hook: installed");
    true
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
