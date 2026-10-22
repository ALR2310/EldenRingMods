//! Scales the player's equip load (the displayed number AND the roll / run
//! behavior) by a factor, or sets it to a fixed value. One hook, on the
//! `movaps xmm0,xmm6` that copies the finished total out right after the game
//! sums every equipped item's weight, so the change runs exactly once per
//! recalculation. The instruction is only 7 bytes, too short for an absolute
//! jump, so it is redirected with a `E9 rel32` jump through
//! [`common::codepatch`]. Why this spot, the anchor, the stub and the
//! compatibility check: `docs/hook_point.md`.

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use common::{codepatch, config, conflict, diag, logger, memscan};

/// `inc ebx; cmp ebx,5; jl <loop>; lea r11,[rsp+0x70]`, the end of the loop
/// that sums the 5 slots' weights.
const ANCHOR_PATTERN: &str = "FF C3 83 FB ?? 7C ?? 4C 8D 5C 24 ??";
/// Skips `inc`+`cmp`+`jl` (7 bytes) and `lea r11,[rsp+0x70]` (5 bytes).
const ANCHOR_TO_TARGET: usize = 12;
/// `movaps xmm0,xmm6` (3) + `mov rbx,[r11+0x10]` (4).
const TARGET_INSTRUCTION_LEN: usize = 7;

const MODE_MULTIPLIER: i32 = 0;
const MODE_FIXED_VALUE: i32 = 1;

/// The mode baked into the installed stub's `mulss`/`movss`; compared on
/// reload to decide whether that instruction needs re-patching.
static WEIGHT_MODE: AtomicI32 = AtomicI32::new(MODE_MULTIPLIER);

/// The factor (multiplier mode) or the fixed total, as raw `f32` bits (std has
/// no `AtomicF32`). The stub reads it through an address baked in at install
/// time every time it runs, so updating it alone hot-reloads a new number.
static WEIGHT_VALUE: AtomicU32 = AtomicU32::new(0x3F80_0000); // 1.0f32

/// Address of the `mulss`/`movss` inside the installed stub; 0 until
/// [install] succeeds. Only [reload_weight_value_locked] writes to it again.
static STUB_MODE_INSTR_ADDR: AtomicU64 = AtomicU64::new(0);

/// Offset of the `mulss`/`movss` in the stub: right after the 10-byte
/// `mov rcx, imm64` that loads `&WEIGHT_VALUE`.
const STUB_MODE_INSTR_OFFSET: usize = 10;

/// `mulss xmm6,[rcx]` (multiplier mode) or `movss xmm6,[rcx]` (fixed value:
/// overwrites the finished total outright).
fn mode_instruction(mode: i32) -> [u8; 4] {
    if mode == MODE_FIXED_VALUE {
        [0xF3, 0x0F, 0x10, 0x31]
    } else {
        [0xF3, 0x0F, 0x59, 0x31]
    }
}

/// The configured `Multiplier` (1.0 = vanilla when the key is missing); the
/// in-game menu shows the same value.
pub fn configured_multiplier() -> f64 {
    config::get_double("Multiplier", 1.0)
}

fn read_weight_value(mode: i32) -> f32 {
    (if mode == MODE_FIXED_VALUE { config::get_double("FixedValue", 0.0) } else { configured_multiplier() }) as f32
}

/// Serialises reloads: the reload key's task and the menu's saver thread (via
/// [reload_from_config]) both end up in [reload_weight_value_locked].
static RELOAD_LOCK: Mutex<()> = Mutex::new(());

fn lock_reload() -> MutexGuard<'static, ()> {
    RELOAD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn init_weight_value() {
    let mode = config::get_int("Mode", MODE_MULTIPLIER);
    let value = read_weight_value(mode);
    WEIGHT_MODE.store(mode, Ordering::Relaxed);
    WEIGHT_VALUE.store(value.to_bits(), Ordering::Relaxed);
    logger::log(&format!("Weight: mode={mode} value={value:.3}"));
}

/// Applies the freshly loaded config to the running hook. The value is always
/// updated; if `Mode` changed, the stub's `mulss`/`movss` is also re-patched
/// in place (riskier: the CPU may be executing it). The caller holds
/// `RELOAD_LOCK`.
fn reload_weight_value_locked() {
    let new_mode = config::get_int("Mode", MODE_MULTIPLIER);
    let old_mode = WEIGHT_MODE.load(Ordering::Relaxed);
    let mut mode = new_mode;
    let mut value = read_weight_value(new_mode);

    if new_mode != old_mode {
        let addr = STUB_MODE_INSTR_ADDR.load(Ordering::Relaxed);
        if addr != 0 {
            // Patch the instruction first and publish the value after, so the
            // stub never runs the new mode's value under the old instruction.
            let patched = unsafe { codepatch::overwrite_bytes(addr as *mut u8, &mode_instruction(new_mode)) };
            if !patched {
                logger::warn("Reload: Mode re-patch failed, kept previous Mode");
                mode = old_mode;
                value = read_weight_value(old_mode);
            }
        }
    }

    WEIGHT_VALUE.store(value.to_bits(), Ordering::Relaxed);
    WEIGHT_MODE.store(mode, Ordering::Relaxed);
    logger::log(&format!("Weight: mode={mode} value={value:.3}"));
}

/// Applies the current config to the running hook; used by the in-game menu
/// after it has saved a change (the shared config is already updated, so it
/// is not re-read from the file).
pub fn reload_from_config() {
    let result = std::panic::catch_unwind(|| {
        let _guard = lock_reload();
        reload_weight_value_locked();
    });
    if let Err(panic) = result {
        logger::error(&format!("Reload: menu reload panicked ({panic:?})"));
    }
}

/// The stub: scale or overwrite `xmm6` by `[weight_value_addr]`, then replay
/// the two overwritten instructions.
fn build_stub(mode: i32, weight_value_addr: u64) -> Vec<u8> {
    let mut body = Vec::with_capacity(21);

    body.extend_from_slice(&[0x48, 0xB9]); // mov rcx, &WEIGHT_VALUE
    body.extend_from_slice(&weight_value_addr.to_le_bytes());
    debug_assert_eq!(body.len(), STUB_MODE_INSTR_OFFSET);

    body.extend_from_slice(&mode_instruction(mode)); // mulss/movss xmm6, [rcx]
    body.extend_from_slice(&[0x0F, 0x28, 0xC6]); // movaps xmm0, xmm6 (original)
    body.extend_from_slice(&[0x49, 0x8B, 0x5B, 0x10]); // mov rbx, [r11+0x10] (original)

    body
}

/// The anchor with `inc ebx; cmp ebx,5` (exactly 5 bytes) replaced by a
/// `jmp rel32`: the shape another weight mod (a "NoWeight" DLL hooking
/// `inc ebx`) leaves behind.
const FOREIGN_PATCH_PATTERN: &str = "E9 ?? ?? ?? ?? 7C ?? 4C 8D 5C 24 ??";

const FEATURE: &str = "Equip load hook";

fn report_conflict(hook: &conflict::ForeignHook) {
    conflict::report("WeightMultiplier", FEATURE, "WeightMultiplier.log", hook);
}

/// Called only when [ANCHOR_PATTERN] isn't found: tells "another mod already
/// patched this spot" apart from "the game code changed". Returns whether a
/// foreign hook was found and reported.
fn report_foreign_patch() -> bool {
    let Some(site) = memscan::find_pattern_in_module(FOREIGN_PATCH_PATTERN) else {
        return false;
    };
    let rel = unsafe { (site.add(1) as *const i32).read_unaligned() };
    let dest = (site as usize + 5).wrapping_add_signed(rel as isize);
    let Some(hook) = conflict::foreign_hook(site as usize, dest) else {
        return false;
    };
    report_conflict(&hook);
    true
}

fn install() -> bool {
    let Some(anchor) = memscan::find_pattern_in_module(ANCHOR_PATTERN) else {
        // A foreign hook explains the miss; only otherwise blame the game version.
        if !report_foreign_patch() {
            logger::error("Hook: anchor pattern not found, game may need a mod update");
        }
        return false;
    };
    let target = unsafe { anchor.add(ANCHOR_TO_TARGET) };
    // SAFETY: `target` is inside the matched game code, within one readable page run.
    if let Some(hook) = unsafe { conflict::foreign_hook_at(target) } {
        report_conflict(&hook);
        return false;
    }

    let value_addr = &WEIGHT_VALUE as *const AtomicU32 as u64;
    let stub_body = build_stub(WEIGHT_MODE.load(Ordering::Relaxed), value_addr);

    // SAFETY: `target` is the scanned instruction in the game image, with the length its pattern matched.
    let Some(stub) = (unsafe { codepatch::install_jmp_hook(target, TARGET_INSTRUCTION_LEN, &stub_body) }) else {
        logger::error("Hook: install failed (stub allocation or patch)");
        return false;
    };
    STUB_MODE_INSTR_ADDR.store(unsafe { stub.add(STUB_MODE_INSTR_OFFSET) } as u64, Ordering::Relaxed);

    logger::log("Hook: installed");
    true
}

/// Waits `LoadDelay`, installs the hook, then hands the reload key to
/// `common::reload` forever. Run it on its own worker thread; never returns
/// once the hook is active.
pub fn run(ini_path: String) {
    let load_delay_ms = config::get_int("LoadDelay", 5000).max(0) as u64;
    std::thread::sleep(Duration::from_millis(load_delay_ms));

    // After LoadDelay, so the other mods' DLLs are loaded by now too.
    diag::log_environment();

    init_weight_value();

    if !install() {
        return;
    }

    common::reload::run_with(
        || config::get_string("ReloadKey", "F5"),
        || config::get_bool("ReloadBanner", false),
        move || {
            let _guard = lock_reload();
            config::load(&ini_path);
            reload_weight_value_locked();
            crate::menu::sync_from_config();
            Ok(())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_decides_between_scaling_and_overwriting() {
        assert_eq!(mode_instruction(MODE_MULTIPLIER), [0xF3, 0x0F, 0x59, 0x31]); // mulss xmm6,[rcx]
        assert_eq!(mode_instruction(MODE_FIXED_VALUE), [0xF3, 0x0F, 0x10, 0x31]); // movss xmm6,[rcx]
        assert_eq!(mode_instruction(7), mode_instruction(MODE_MULTIPLIER)); // unknown modes scale
    }

    #[test]
    fn the_stub_loads_the_value_address_then_replays_the_original_instructions() {
        let stub = build_stub(MODE_FIXED_VALUE, 0x1122_3344_5566_7788);
        assert_eq!(stub[..2], [0x48, 0xB9]);
        assert_eq!(stub[2..10], 0x1122_3344_5566_7788u64.to_le_bytes());
        // The re-patchable instruction sits exactly where `STUB_MODE_INSTR_OFFSET` says.
        assert_eq!(stub[STUB_MODE_INSTR_OFFSET..STUB_MODE_INSTR_OFFSET + 4], mode_instruction(MODE_FIXED_VALUE));
        // The two replayed instructions add up to the 7 bytes the hook overwrote.
        let tail = &stub[STUB_MODE_INSTR_OFFSET + 4..];
        assert_eq!(tail, [0x0F, 0x28, 0xC6, 0x49, 0x8B, 0x5B, 0x10]);
        assert_eq!(tail.len(), TARGET_INSTRUCTION_LEN);
    }
}
