//! The game's row of status icons (the diamonds under the player's bars): where its data is.
//! The game updates the row each frame with `sub_1408D58F0(view, dt, model)`; a redirect of that
//! call keeps `model`, a block of 42 slots of 64 bytes (the image of each icon) followed by one
//! flicker flag per slot. Details: `docs/effects.md`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{codepatch, logger, memscan};

/// The call of the update in the HUD's update function, found by the code around it (unique on
/// 2.6.2.0 to 2.7.2.0): `mov r8,[rsi+60h]; add r8,<model>; lea rcx,[rsi+<view>]; movaps xmm1,xmm6; call`.
const CALL_AOB: &str = "4C 8B 46 60 49 81 C0 ?? ?? 00 00 48 8D 8E ?? ?? ?? ?? 0F 28 CE E8 ?? ?? ?? ?? 41 8B DC 66 66 0F 1F 84";
const CALL_AT: usize = 21;

/// Slots in the model and bytes of one slot; the icon id is the last word of a slot (`docs/effects.md`).
pub const SLOTS: usize = 42;
const SLOT_LEN: usize = 64;
const ICON_AT: usize = 60;
/// The byte after the flicker flags: whether the row is shown. The update passes it to the row's
/// fade (`FadeIn` when set, `FadeOut` when not), which is how the game hides the row out of combat.
const SHOWN_AT: usize = SLOTS * SLOT_LEN + SLOTS;

type UpdateFn = unsafe extern "system" fn(*mut u8, f32, *const u8);

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static MODEL: AtomicUsize = AtomicUsize::new(0);

/// Stands in for the update: runs the game's, and keeps the model it was given.
unsafe extern "system" fn update(view: *mut u8, dt: f32, model: *const u8) {
    // SAFETY: `ORIGINAL` is the call's destination, stored before the redirect.
    let original: UpdateFn = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Acquire)) };
    unsafe { original(view, dt, model) };
    MODEL.store(model as usize, Ordering::Release);
}

/// Redirects the call, on a thread of its own (the scan waits for the game's code to be there).
pub fn install() {
    std::thread::spawn(|| {
        let Some(anchor) = memscan::wait_for_pattern_in_module(CALL_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
            logger::error("Icons: pattern not found");
            return;
        };
        let site = unsafe { anchor.add(CALL_AT) };
        // SAFETY: reads the 5 bytes of a call found in the game image.
        let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
            logger::error("Icons: site is not a call");
            return;
        };
        ORIGINAL.store(original as usize, Ordering::Release);
        // SAFETY: `site` is the scanned call; the stub jumps to a function with the callee's signature.
        let ok = unsafe { codepatch::redirect_rel32(site, &codepatch::absolute_jmp(update as UpdateFn as usize as u64)) }.is_some();
        logger::log(if ok { "Icons: hooked" } else { "Icons: redirect failed" });
    });
}

/// The icon id (`SpEffectParam.iconId`) the game shows in slot `i`; `None` for an empty slot or
/// before the game has updated the row once. An empty slot holds -1 there. The slots fill from 0
/// without gaps, one per icon (effects that share an icon share a slot).
pub fn icon(i: usize) -> Option<i32> {
    let model = MODEL.load(Ordering::Acquire);
    if model == 0 || i >= SLOTS {
        return None;
    }
    let id = memscan::read_safe::<u32>(model + i * SLOT_LEN + ICON_AT)?;
    (1..100_000).contains(&id).then_some(id as i32)
}

/// Whether the game shows its row of icons right now (`false` while it fades the row out, and
/// before the row was updated once).
pub fn shown() -> bool {
    let model = MODEL.load(Ordering::Acquire);
    model != 0 && memscan::read_safe::<u8>(model + SHOWN_AT).is_some_and(|flag| flag != 0)
}
