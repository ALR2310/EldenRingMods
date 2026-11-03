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

/// One redirected `call`: where it is, our stub, the cell holding the
/// destination it had before (the game's function, or another mod's stub).
struct Patch {
    site: *mut u8,
    stub: usize,
    hook: usize,
    original: &'static AtomicUsize,
    name: &'static str,
}

// SAFETY: `site` is an address in the game image, only read/written through codepatch.
unsafe impl Send for Patch {}

impl Patch {
    /// Redirects the `call` at `site` to `hook`, remembering its destination.
    fn apply(site: *mut u8, hook: usize, original: &'static AtomicUsize, name: &'static str) -> Option<Patch> {
        // SAFETY: reads the 5 bytes of a call found in the game image.
        let Some(target) = (unsafe { codepatch::rel32_target(site) }) else {
            if !unsafe { crate::conflicts::check(site, name) } {
                logger::error(&format!("{name}: site is not a call"));
            }
            return None;
        };
        original.store(target as usize, Ordering::Relaxed);
        let jump = codepatch::absolute_jmp(hook as u64);
        // SAFETY: `site` is the scanned call; the stub takes the same arguments and returns like its destination.
        let stub = unsafe { codepatch::redirect_rel32(site, &jump) }?;
        Some(Patch { site, stub: stub as usize, hook, original, name })
    }

    /// Seamless Co-op redirects the same `call` of the manager update (its
    /// diff lists the 202 check) after we did, which silently drops our stub
    /// (log 2026-11-03: the 202 branch ran with the option on). Patches again,
    /// chained to whatever is there now, when the destination is not ours.
    fn keep(&mut self) {
        // SAFETY: reads the 5 bytes of the call at `site`.
        let now = unsafe { codepatch::rel32_target(self.site) };
        if now == Some(self.stub as u64) || now.is_none() {
            return;
        }
        if let Some(again) = Patch::apply(self.site, self.hook, self.original, self.name) {
            logger::log(&format!("{}: patch was replaced, applied again", self.name));
            *self = again;
        }
    }
}

fn find_call(pattern: &str, offset: usize, name: &str) -> Option<*mut u8> {
    let anchor = memscan::wait_for_pattern_in_module(pattern, Duration::from_millis(500), Duration::from_secs(30));
    if anchor.is_none() {
        logger::error(&format!("{name}: pattern not found (game updated?)"));
    }
    // SAFETY: `offset` is inside the pattern that matched.
    anchor.map(|a| unsafe { a.add(offset) })
}

fn install() -> Vec<Patch> {
    let mut patches = Vec::new();
    if let Some(site) = find_call(AOB, CALL_OFFSET, "KeepSpiritsOnRest")
        && let Some(patch) = Patch::apply(site, has_effect_hook as *const () as usize, &ORIGINAL, "KeepSpiritsOnRest")
    {
        patches.push(patch);
    }
    // The world reset flags every spirit group for removal: the second part.
    if let Some(site) = find_call(RESET_AOB, RESET_CALL_OFFSET, "KeepSpiritsOnRest (reset)")
        && let Some(patch) =
            Patch::apply(site, flag_all_hook as *const () as usize, &FLAG_ALL_ORIGINAL, "KeepSpiritsOnRest (reset)")
    {
        patches.push(patch);
    }
    patches
}

pub fn run() {
    KEEP.store(config::get_bool("KeepSpiritsOnRest", true), Ordering::Relaxed);
    let mut patches = install();
    if patches.is_empty() {
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
            for patch in patches.iter_mut() {
                patch.keep();
            }
        },
    );
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
