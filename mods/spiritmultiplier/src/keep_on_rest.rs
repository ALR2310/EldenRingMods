//! `KeepSpiritsOnRest`: spirits stay when you rest at a Site of Grace. What
//! the game does and the research: `docs/persist_spirits.md`.
//!
//! Resting gives the player SpEffect 202; the spirit manager's per-frame
//! update checks for it (`sub_1403F21B0(player, 202)`) and calls `DisappearAll`.
//! The `call` of that check is redirected to [has_effect_hook], which answers
//! "no" while the option is on and otherwise calls the game's function.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use eldenring::cs::CSTaskGroupIndex;

use common::{codepatch, config, logger, memscan};

/// `mov rcx,[rcx+1E508h]; mov edx,0CAh; call sub_1403F21B0; test al,al; jz
/// +8; mov rcx,r15; call DisappearAll` in the spirit manager update
/// (0x1404B9640 in 2.7.2.0, unique in 2.6.2.0 - 2.7.2.0). The `call` is at +12.
const AOB: &str = "48 8B 89 08 E5 01 00 BA CA 00 00 00 E8 ?? ?? ?? ?? 84 C0 74 08 49 8B CF E8";
const CALL_OFFSET: usize = 12;

/// `mov rcx,[rsi+1E538h]; test rcx,rcx; jz +5; call sub_1404B83B0; xor ebx,ebx;
/// cmp [rsi+10ED8h],ebx` in `sub_14050E0E0` (0x14050E134 in 2.7.2.0, unique in
/// 2.6.2.0 - 2.7.2.0), the WorldChrMan "reset every chr" routine run by the
/// world reset of a grace rest. `sub_1404B83B0(manager)` sets the removal flag
/// (`group+0x20`) of every spirit group of every owner, and the manager update
/// then destroys them (log 2026-11-03: only the flagged-group destroy site ran).
/// The `call` is at +12.
const RESET_AOB: &str = "48 8B 8E 38 E5 01 00 48 85 C9 74 05 E8 ?? ?? ?? ?? 33 DB 39 9E";
const RESET_CALL_OFFSET: usize = 12;

const TICK_INTERVAL_MS: f64 = 1000.0;

static KEEP: AtomicBool = AtomicBool::new(true);
/// `sub_1403F21B0(player, speffect_id) -> bool`; 0 until installed.
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// Stands in for `sub_1403F21B0` at that one call site (the id is always 202).
unsafe extern "system" fn has_effect_hook(player: usize, id: i32) -> u8 {
    if KEEP.load(Ordering::Relaxed) {
        return 0;
    }
    match ORIGINAL.load(Ordering::Relaxed) {
        0 => 0,
        original => unsafe {
            std::mem::transmute::<usize, unsafe extern "system" fn(usize, i32) -> u8>(original)(player, id)
        },
    }
}

/// `sub_1404B83B0(manager)`: the original.
static FLAG_ALL_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// Stands in for `sub_1404B83B0` at that one call site: does nothing while the
/// option is on, so the world reset leaves the spirit groups alone.
unsafe extern "system" fn flag_all_hook(manager: usize) {
    if KEEP.load(Ordering::Relaxed) {
        return;
    }
    match FLAG_ALL_ORIGINAL.load(Ordering::Relaxed) {
        0 => {}
        original => unsafe { std::mem::transmute::<usize, unsafe extern "system" fn(usize)>(original)(manager) },
    }
}

fn install() -> bool {
    let Some(anchor) = memscan::wait_for_pattern_in_module(AOB, Duration::from_millis(500), Duration::from_secs(30))
    else {
        logger::error("KeepSpiritsOnRest: pattern not found (game updated?)");
        return false;
    };
    let site = unsafe { anchor.add(CALL_OFFSET) };
    // SAFETY: reads the 5 bytes of a call found in the game image.
    let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
        if !unsafe { crate::conflicts::check(site, "KeepSpiritsOnRest") } {
            logger::error("KeepSpiritsOnRest: site is not a call");
        }
        return false;
    };
    ORIGINAL.store(original as usize, Ordering::Relaxed);
    let stub = codepatch::absolute_jmp(has_effect_hook as *const () as usize as u64);
    // SAFETY: `site` is the scanned call; the stub takes the same arguments and returns like the original.
    if (unsafe { codepatch::redirect_rel32(site, &stub) }).is_none() {
        logger::error("KeepSpiritsOnRest: redirect failed");
        return false;
    }
    // The world reset flags every spirit group for removal: the second part.
    let Some(reset) = memscan::wait_for_pattern_in_module(RESET_AOB, Duration::from_millis(500), Duration::from_secs(30))
    else {
        logger::error("KeepSpiritsOnRest: reset pattern not found");
        return true;
    };
    let site = unsafe { reset.add(RESET_CALL_OFFSET) };
    if let Some(original) = unsafe { codepatch::rel32_target(site) } {
        FLAG_ALL_ORIGINAL.store(original as usize, Ordering::Relaxed);
        let stub = codepatch::absolute_jmp(flag_all_hook as *const () as usize as u64);
        if (unsafe { codepatch::redirect_rel32(site, &stub) }).is_none() {
            logger::error("KeepSpiritsOnRest: reset redirect failed");
        }
    } else if !unsafe { crate::conflicts::check(site, "KeepSpiritsOnRest (reset)") } {
        logger::error("KeepSpiritsOnRest: reset site is not a call");
    }
    true
}

pub fn run() {
    KEEP.store(config::get_bool("KeepSpiritsOnRest", true), Ordering::Relaxed);
    if !install() {
        return;
    }
    logger::log("KeepSpiritsOnRest: patched");
    let cs_task = common::task::wait_for_cs_task();
    let mut elapsed_ms: f64 = 0.0;
    let _handle = common::task::run_recurring_safe(
        cs_task,
        "KeepSpiritsOnRest",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < TICK_INTERVAL_MS {
                return;
            }
            elapsed_ms = 0.0;
            let keep = config::get_bool("KeepSpiritsOnRest", true);
            if KEEP.swap(keep, Ordering::Relaxed) != keep {
                logger::log(&format!("KeepSpiritsOnRest={keep}."));
            }
        },
    );
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
