//! The game's row of status icons (the diamonds under the player's bars): where its data is.
//! The game updates the row each frame with `sub_1408D58F0(view, dt, model)`; a redirect of that
//! call keeps `model`, a block of 42 slots of 64 bytes (the image of each icon) followed by one
//! flicker flag per slot. A second redirect moves the game's buildup gauge of the player.
//! Details: `docs/effects.md`.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
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

/// The call that updates one status buildup gauge of the player (the bar with the icon at the
/// bottom of the screen), in the same HUD function: `update(view, index, model)` for each of the
/// seven clips (`sub_1408D2AE0`; the game gives the clips to the active statuses in turn). After
/// the update the mod sets the position of the clip, so the gauge sits under the target's panel.
/// Unique on 2.6.2.0 to 2.7.2.0.
const GAUGE_AOB: &str = "4C 8B 46 60 49 81 C0 ?? ?? 00 00 4C 03 C1 48 69 CA ?? ?? 00 00 48 03 CF 49 03 CE 0F 28 CE E8 ?? ?? ?? ?? FF C3 48 63 D3";
const GAUGE_CALL_AT: usize = 30;
const GAUGES: usize = 7;
/// Where the component that holds the gauge's root clip is in the view (`sub_14076B7C0(view + 1032,
/// root)`); its vtable's second entry returns the clip's value, as for `HubHpBars`.
const GAUGE_ROOT: usize = 1032;

/// `sub_140D872C0(value, x, y)`: sets `_x` and `_y` of a Scaleform clip (found by the same pattern as
/// in `HubHpBars`; unique on 2.6.2 to 2.7.2).
const SET_POS_AOB: &str = "48 8B C4 48 89 58 10 55 48 8D 68 C8 48 81 EC 30 01 00 00 0F 29 70 E8 0F 29 78 D8 48 8B 05 ?? ?? ?? ?? \
     48 33 C4 48 89 45 00 F6 41 20 8F 0F 28 F2 0F 28 F9 48 8B D9 0F 84 ?? ?? ?? ??";

/// `sub_140D87530(value, [x, y])`: sets `_xscale` and `_yscale` of a clip to `x` and `y` times 100
/// percent (unique on 2.6.2 to 2.7.2).
const SET_SCALE_AOB: &str = "48 89 5C 24 10 48 89 7C 24 18 55 48 8D 6C 24 F0 48 81 EC 10 01 00 00 48 8B 05 ?? ?? ?? ?? \
     48 33 C4 48 89 45 00 F6 41 20 8F 48 8B FA 48 8B D9 0F 84 ?? ?? ?? ??";

/// Where the game puts the gauges: the root of `ConditionGaugeList` is at (960, 520) of the 1080p
/// picture, the first gauge at (-150, 197.25) of it and the next ones 48.85 px higher; the icon is
/// 42 px, and the bar ends 294.5 px right of the origin (measured on screenshots). The mod shrinks them and lines them up downwards instead, from a corner it is told.
const LIST_AT: [f32; 2] = [960.0, 520.0];
const ORIGINAL_AT: [f32; 2] = [-150.0, 197.25];
const STEP: f32 = 48.85;
/// The bar ends this far right of the origin (294.5; the first measure, 308, left the gauge 10 px
/// short of the panel's edge once scaled).
const GAUGE_RIGHT: f32 = 294.5;
const GAUGE_HALF_HEIGHT: f32 = 21.0;
/// The width of a gauge at full size: the icon (42), the bar (292).
pub const GAUGE_WIDTH: f32 = 334.0;

type GaugeFn = unsafe extern "system" fn(*mut u8, i64, *const u8);
type ValueOfFn = unsafe extern "system" fn(*mut u8) -> *mut u8;
type SetPosFn = unsafe extern "system" fn(*mut u8, f32, f32);
type SetScaleFn = unsafe extern "system" fn(*mut u8, *const f32);

static GAUGE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static SET_POS: AtomicUsize = AtomicUsize::new(0);
static SET_SCALE: AtomicUsize = AtomicUsize::new(0);
/// The first gauge's origin in the list, as `f32` bits (x, y), once the panel has told it.
static ANCHOR_X: AtomicU32 = AtomicU32::new(0);
static ANCHOR_Y: AtomicU32 = AtomicU32::new(0);
/// The size of the gauges as a fraction of their full size, as `f32` bits.
static ANCHOR_SCALE: AtomicU32 = AtomicU32::new(0);
static ANCHORED: AtomicBool = AtomicBool::new(false);
/// What was last set on each gauge: the view and the position bits, so a clip is only set when it
/// changes (or when the game rebuilt the view).
static APPLIED_VIEW: [AtomicUsize; GAUGES] = [const { AtomicUsize::new(0) }; GAUGES];
static APPLIED_X: [AtomicU32; GAUGES] = [const { AtomicU32::new(0) }; GAUGES];
static APPLIED_Y: [AtomicU32; GAUGES] = [const { AtomicU32::new(0) }; GAUGES];
static APPLIED_SCALE: [AtomicU32; GAUGES] = [const { AtomicU32::new(0) }; GAUGES];
/// Calls per gauge, to set the clip again now and then even when nothing changed: the game may
/// rebuild its HUD (another save loaded) and a new view at the same address has the default place.
static CALLS: [AtomicU32; GAUGES] = [const { AtomicU32::new(0) }; GAUGES];
/// About every 5 seconds at 60 frames per second.
const REFRESH_EVERY: u32 = 300;

/// Tells where the gauges go: the right edge and the top of the first one, in pixels of the 1080p
/// picture (not of the screen), and their size as a fraction of the full one.
pub fn place_gauges(right: f32, top: f32, scale: f32) {
    ANCHOR_X.store((right - GAUGE_RIGHT * scale - LIST_AT[0]).to_bits(), Ordering::Relaxed);
    ANCHOR_Y.store((top + GAUGE_HALF_HEIGHT * scale - LIST_AT[1]).to_bits(), Ordering::Relaxed);
    ANCHOR_SCALE.store(scale.to_bits(), Ordering::Relaxed);
    ANCHORED.store(true, Ordering::Release);
}

/// Runs the game's gauge update, then puts the clip where the mod wants it (or back where the
/// game has it while the player's effects are off).
unsafe extern "system" fn gauge_update(view: *mut u8, index: i64, model: *const u8) {
    // SAFETY: `GAUGE_ORIGINAL` is the call's destination, stored before the redirect.
    let original: GaugeFn = unsafe { std::mem::transmute(GAUGE_ORIGINAL.load(Ordering::Acquire)) };
    unsafe { original(view, index, model) };
    if (0..GAUGES as i64).contains(&index) && ANCHORED.load(Ordering::Acquire) {
        // A panic must not unwind into the game.
        let _ = std::panic::catch_unwind(|| unsafe { place_gauge(view, index as usize) });
    }
}

/// # Safety
/// `view` must be the game's gauge view number `index`.
unsafe fn place_gauge(view: *mut u8, index: usize) {
    let set_pos = SET_POS.load(Ordering::Acquire);
    if set_pos == 0 {
        return;
    }
    let (x, y, scale) = if crate::settings::effects_player() {
        let scale = f32::from_bits(ANCHOR_SCALE.load(Ordering::Relaxed));
        (
            f32::from_bits(ANCHOR_X.load(Ordering::Relaxed)),
            f32::from_bits(ANCHOR_Y.load(Ordering::Relaxed)) + STEP * scale * index as f32,
            scale,
        )
    } else {
        (ORIGINAL_AT[0], ORIGINAL_AT[1] - STEP * index as f32, 1.0)
    };
    let refresh = CALLS[index].fetch_add(1, Ordering::Relaxed) % REFRESH_EVERY == 0;
    if !refresh
        && APPLIED_VIEW[index].load(Ordering::Relaxed) == view as usize
        && APPLIED_X[index].load(Ordering::Relaxed) == x.to_bits()
        && APPLIED_Y[index].load(Ordering::Relaxed) == y.to_bits()
        && APPLIED_SCALE[index].load(Ordering::Relaxed) == scale.to_bits()
    {
        return;
    }
    // SAFETY: the component sits at a fixed offset of the view; its vtable's second entry returns the clip's value.
    unsafe {
        let component = view.add(GAUGE_ROOT);
        let value_of: ValueOfFn = std::mem::transmute(*(*(component as *const *const usize)).add(1));
        let value = value_of(component);
        if value.is_null() {
            return;
        }
        std::mem::transmute::<usize, SetPosFn>(set_pos)(value, x, y);
        let set_scale = SET_SCALE.load(Ordering::Acquire);
        if set_scale != 0 && scale.is_finite() && scale > 0.0 {
            let factors = [scale, scale];
            std::mem::transmute::<usize, SetScaleFn>(set_scale)(value, factors.as_ptr());
        }
    }
    APPLIED_SCALE[index].store(scale.to_bits(), Ordering::Relaxed);
    APPLIED_VIEW[index].store(view as usize, Ordering::Relaxed);
    APPLIED_X[index].store(x.to_bits(), Ordering::Relaxed);
    APPLIED_Y[index].store(y.to_bits(), Ordering::Relaxed);
}

/// Redirects the gauge update call, on a thread of its own.
pub fn install_gauge() {
    std::thread::spawn(|| {
        let Some(set_pos) = memscan::wait_for_pattern_in_module(SET_POS_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
            logger::error("Gauge: position function not found");
            return;
        };
        SET_POS.store(set_pos as usize, Ordering::Release);
        match memscan::wait_for_pattern_in_module(SET_SCALE_AOB, Duration::from_millis(500), Duration::from_secs(60)) {
            Some(set_scale) => SET_SCALE.store(set_scale as usize, Ordering::Release),
            None => logger::warn("Gauge: scale function not found, the gauges keep their size"),
        }
        let Some(anchor) = memscan::wait_for_pattern_in_module(GAUGE_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
            logger::error("Gauge: pattern not found");
            return;
        };
        let site = unsafe { anchor.add(GAUGE_CALL_AT) };
        // SAFETY: reads the 5 bytes of a call found in the game image.
        let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
            logger::error("Gauge: site is not a call");
            return;
        };
        GAUGE_ORIGINAL.store(original as usize, Ordering::Release);
        // SAFETY: `site` is the scanned call; the stub jumps to a function with the callee's signature.
        let ok = unsafe { codepatch::redirect_rel32(site, &codepatch::absolute_jmp(gauge_update as GaugeFn as usize as u64)) }.is_some();
        logger::log(if ok { "Gauge: hooked" } else { "Gauge: redirect failed" });
    });
}
