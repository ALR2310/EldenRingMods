//! Scales the player's equip load (both the displayed number AND the actual
//! gameplay roll/run behavior) by a configurable factor, ported from
//! WeightEngine.cpp/CodePatch.cpp - same single hook, patched into the exact
//! instruction (`movaps xmm0,xmm6`, confirmed against a working third-party
//! "NoWeight" DLL - see the original README's "Lịch sử tìm offset" for the
//! full story of how two earlier hook points were tried and ruled out) right
//! after the game finishes summing every equipped item's weight, so the
//! multiply runs exactly once per recalculation instead of compounding
//! across the summing loop's 5 iterations.
//!
//! Unlike `autoregen`/`sometweaks`/`runemultiplier`, the patched instruction
//! here is only 7 bytes long - not enough room for the `mov reg, imm64; jmp
//! reg` absolute redirect those use (needs ~12 bytes). This keeps the
//! original technique instead: a 5-byte relative `E9 rel32` JMP, via
//! [`common::codepatch`] (ported from the C++ `CodePatch.cpp`/`.h`), whose
//! stub has to be allocated within reach of a 32-bit signed displacement.
//!
//! `Mode=1` (`FixedValue`) reuses this exact same hook point instead of
//! patching mid-loop the way a third-party "NoWeight" DLL does (see the
//! original README's "Lịch sử tìm offset") - since this hook already runs
//! exactly once, after the loop has finished summing, overwriting `xmm6`
//! here works just as well as multiplying it, without needing a second
//! patch site.
//!
//! `ReloadKey` is watched by [`common::reload`] (the shared watcher every mod
//! uses). Changing `Multiplier`/`FixedValue` on reload is a plain value write
//! (the stub already reads it from a fixed address every time it runs);
//! changing `Mode` additionally re-patches the stub's `mulss`/`movss`
//! instruction in place via [`common::codepatch::overwrite_bytes`], since
//! which of the two runs is baked into the executing code itself, not read
//! from memory - same inherent risk as the original `install()` (rewriting
//! code the CPU could be mid-execution of), just repeated on demand instead of
//! only once.

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use common::codepatch;
use common::config;
use common::conflict;
use common::diag;
use common::logger;
use common::memscan;

// Cross-checked against a third-party "NoWeight" DLL confirmed to work
// correctly in-game (WeightValue=0 actually changes the roll weight class,
// not just the displayed number) - its own log printed the exact address it
// patches, which matched this same pattern's match position exactly:
//
//   addss xmm6, dword ptr [rax + 0xc]   ; add this slot's item weight
//   inc ebx                             ; <-- the third-party DLL hooks here (sets a fixed value)
//   cmp ebx, 5
//   jl <loop start>
//   lea r11, [rsp+0x70]
//   movaps xmm0, xmm6                   ; <-- this hook's target: copy the finished total out
//   mov rbx, [r11+0x10]
//
// Hooking the third-party DLL's spot instead would compound a percentage
// multiply across all 5 loop iterations (items summed earlier end up scaled
// down far more than later ones) - fine for "set to a fixed value", wrong
// for a clean percentage. `movaps xmm0,xmm6` runs exactly once, after the
// loop has already finished, so scaling xmm6 right there multiplies the true
// final total by a single clean factor.
const ANCHOR_PATTERN: &str = "FF C3 83 FB ?? 7C ?? 4C 8D 5C 24 ??";
const ANCHOR_TO_TARGET: usize = 12; // skip inc+cmp+jl (7 bytes) + lea r11,[rsp+0x70] (5 bytes)
const TARGET_INSTRUCTION_LEN: usize = 7; // "movaps xmm0,xmm6" (3) + "mov rbx,[r11+0x10]" (4)

const MODE_MULTIPLIER: i32 = 0;
const MODE_FIXED_VALUE: i32 = 1;

// The mode currently baked into the installed stub's mulss/movss instruction
// - compared against on reload to decide whether that instruction needs
// re-patching (see [reload_weight_value]).
static WEIGHT_MODE: AtomicI32 = AtomicI32::new(MODE_MULTIPLIER);

// The value applied to the finished equip-load total - a multiplier factor
// when WEIGHT_MODE is MODE_MULTIPLIER, or the fixed total itself when
// MODE_FIXED_VALUE. Stored as raw f32 bits (no AtomicF32 in std) - read by
// the injected stub via an absolute address baked into it at hook-install
// time, every time it runs, so updating this alone (no re-patch needed) is
// enough to hot-reload a new Multiplier/FixedValue number.
static WEIGHT_VALUE: AtomicU32 = AtomicU32::new(0x3F80_0000); // 1.0f32 bit pattern

// Address of the mulss/movss instruction inside the installed stub (see
// [mode_instruction] + STUB_MODE_INSTR_OFFSET) - 0 until [install] succeeds.
// Only [reload_weight_value] writes to this address again, if Mode changes.
static STUB_MODE_INSTR_ADDR: AtomicU64 = AtomicU64::new(0);

// Offset of the mulss/movss instruction within the stub built by
// [build_stub]: right after the 10-byte "mov rcx, imm64" that loads
// &WEIGHT_VALUE.
const STUB_MODE_INSTR_OFFSET: usize = 10;

/// The 4-byte instruction [build_stub] injects for `mode`: `mulss xmm6,[rcx]`
/// (MODE_MULTIPLIER) or `movss xmm6,[rcx]` (MODE_FIXED_VALUE, overwrites the
/// finished total outright instead of scaling it).
fn mode_instruction(mode: i32) -> [u8; 4] {
    if mode == MODE_FIXED_VALUE {
        [0xF3, 0x0F, 0x10, 0x31]
    } else {
        [0xF3, 0x0F, 0x59, 0x31]
    }
}

/// The configured `Multiplier` (1.0 = vanilla when the key is missing); the
/// in-game menu shows this same value.
pub fn configured_multiplier() -> f64 {
    config::get_double("Multiplier", 1.0)
}

fn read_weight_value(mode: i32) -> f32 {
    (if mode == MODE_FIXED_VALUE {
        config::get_double("FixedValue", 0.0)
    } else {
        configured_multiplier()
    }) as f32
}

// Serialises config reloads: the reload key's task and the menu's saver thread
// (via [reload_from_config]) both end up in [reload_weight_value_locked].
static RELOAD_LOCK: Mutex<()> = Mutex::new(());

fn lock_reload() -> MutexGuard<'static, ()> {
    RELOAD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn init_weight_value() {
    let mode = config::get_int("Mode", MODE_MULTIPLIER);
    let value = read_weight_value(mode);
    WEIGHT_MODE.store(mode, Ordering::Relaxed);
    WEIGHT_VALUE.store(value.to_bits(), Ordering::Relaxed);
    if config::get_bool("LogFile", false) {
        logger::log(&format!("Mode={mode}, value={value:.3}"));
    }
}

/// Re-reads `Mode`/`Multiplier`/`FixedValue` from the just-`config::load`ed
/// ini and applies them to the already-installed hook: always updates
/// `WEIGHT_VALUE` (the stub re-reads it every time it runs, no patch
/// needed); if `Mode` itself changed, also re-patches the stub's
/// mulss/movss instruction in place via `codepatch::overwrite_bytes` - see
/// the module doc comment for why that's riskier than the value-only path.
/// Takes `RELOAD_LOCK`, so it never interleaves with another reload.
fn reload_weight_value() {
    let _guard = lock_reload();
    reload_weight_value_locked();
}

/// Body of [reload_weight_value]; the caller holds `RELOAD_LOCK`.
fn reload_weight_value_locked() {
    let new_mode = config::get_int("Mode", MODE_MULTIPLIER);
    let old_mode = WEIGHT_MODE.load(Ordering::Relaxed);
    let mut mode = new_mode;
    let mut value = read_weight_value(new_mode);

    if new_mode != old_mode {
        let addr = STUB_MODE_INSTR_ADDR.load(Ordering::Relaxed);
        if addr != 0 {
            // Patch the instruction first and publish the new value only
            // afterwards, so the stub never runs with the new mode's value
            // under the old mode's instruction.
            let patched = unsafe { codepatch::overwrite_bytes(addr as *mut u8, &mode_instruction(new_mode)) };
            if !patched {
                logger::warn("Reload: Mode re-patch failed, kept previous Mode");
                // The instruction is unchanged: keep the old mode and its value.
                mode = old_mode;
                value = read_weight_value(old_mode);
            }
        }
    }

    WEIGHT_VALUE.store(value.to_bits(), Ordering::Relaxed);
    WEIGHT_MODE.store(mode, Ordering::Relaxed);

    if config::get_bool("LogFile", false) {
        logger::log(&format!("Mode={mode}, value={value:.3}"));
    }
}

/// Applies the current config to the running hook; used by the in-game menu
/// after it has saved a change (same effect as the reload key, without
/// re-reading the file: the menu's saver already updated the shared config).
pub fn reload_from_config() {
    if let Err(panic) = std::panic::catch_unwind(reload_weight_value) {
        logger::error(&format!("Reload: menu reload panicked ({panic:?})"));
    }
}

fn build_stub(mode: i32, weight_value_addr: u64) -> Vec<u8> {
    let mut body = Vec::with_capacity(21);

    // mov rcx, &WEIGHT_VALUE
    body.extend_from_slice(&[0x48, 0xB9]);
    body.extend_from_slice(&weight_value_addr.to_le_bytes());
    debug_assert_eq!(body.len(), STUB_MODE_INSTR_OFFSET);

    // mulss/movss xmm6, dword ptr [rcx] (scale or overwrite the finished
    // total in place, once, before it's copied out)
    body.extend_from_slice(&mode_instruction(mode));

    // movaps xmm0, xmm6 (original instruction, re-executed with the now-scaled/overwritten xmm6)
    body.extend_from_slice(&[0x0F, 0x28, 0xC6]);

    // mov rbx, qword ptr [r11+0x10] (original next instruction, re-executed)
    body.extend_from_slice(&[0x49, 0x8B, 0x5B, 0x10]);

    body
}

// Same anchor, but with `inc ebx; cmp ebx,5` (exactly 5 bytes) replaced by
// a `jmp rel32` - the shape another weight mod (e.g. a "NoWeight" DLL, which
// hooks `inc ebx` - see the comment above ANCHOR_PATTERN) leaves behind.
const FOREIGN_PATCH_PATTERN: &str = "E9 ?? ?? ?? ?? 7C ?? 4C 8D 5C 24 ??";

const FEATURE: &str = "Equip load hook";

fn report_conflict(hook: &conflict::ForeignHook) {
    conflict::report("WeightMultiplier", FEATURE, "WeightMultiplier.log", hook);
}

/// Called only when ANCHOR_PATTERN isn't found: tells "another mod already
/// patched this spot" apart from "the game code changed" (the jmp often lands
/// in a stub allocated outside any module, then only the address is known).
/// Returns whether a foreign hook was found and reported.
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
    // SAFETY: `target` is inside the matched game code, well within one readable page run.
    if let Some(hook) = unsafe { conflict::foreign_hook_at(target) } {
        report_conflict(&hook);
        return false;
    }
    if config::get_bool("LogFile", false) {
        logger::log("Hook: anchor found");
    }

    let value_addr = &WEIGHT_VALUE as *const AtomicU32 as u64;
    let stub_body = build_stub(WEIGHT_MODE.load(Ordering::Relaxed), value_addr);

    // SAFETY: `target` is the scanned instruction in the game image; the length/offset is the one its pattern matched.

    let Some(stub) = (unsafe { codepatch::install_jmp_hook(target, TARGET_INSTRUCTION_LEN, &stub_body) }) else {
        logger::error("Hook: install failed (stub allocation or patch)");
        return false;
    };
    STUB_MODE_INSTR_ADDR.store(unsafe { stub.add(STUB_MODE_INSTR_OFFSET) } as u64, Ordering::Relaxed);

    if config::get_bool("LogFile", false) {
        logger::log("Hook: installed");
    }
    true
}

/// Waits `LoadDelay`, installs the hook, then hands `ReloadKey` to
/// `common::reload` forever. Meant to run on its own worker thread spawned from
/// `DllMain`; never returns once the hook is active.
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
