//! The game's rune counter (`CS::SoulViewParts`, movie `01_001_FE_Soul`), moved to the top right with
//! `HUD.RuneCounter.Enabled` (`docs/rune_counter.md`).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::time::Duration;

use shared::{codepatch, logger, memscan};

use crate::settings;
use crate::state_icons::{clip_at, display_of, set_clip_position, set_clip_scale};

/// The call that updates the counter, `sub_1408D6080(hud + 0x7048, dt, model)`, in the player HUD's
/// update (`sub_1408D1F50` on 2.7.2.0): the instructions that set the arguments and the ones after
/// the call. Unique on 2.6.2.0 to 2.7.2.0.
const CALL_AOB: &str = "4C 8D 87 A8 10 00 00 0F 28 CE 48 8D 8E 48 70 00 00 E8 ?? ?? ?? ?? 48 8D 97 B8 10 00 00 48 8D 8E 60 73 00 00 E8";
const CALL_AT: usize = 17;
/// Where the component that holds the counter's root clip is in the view (`sub_14076B7C0(view + 560,
/// root, 0)` in the view's constructor `sub_1408D5D80`).
const ROOT: usize = 560;

/// The counter where the game puts it, in pixels of the 1080p picture (measured on screenshots): the
/// box (200 x 36 px) ends at x 1873.5 and starts at y 1019; the clip's origin is 109 px left of the
/// box's right edge and 20 px below its top (found from where the scaled box ended up). The mod moves
/// the clip by the difference between the origin it wants and this one, and scales it about the
/// origin, so the clip's own coordinates never have to be known.
const ORIGIN: [f32; 2] = [1873.5 - BOX_RIGHT_OF_ORIGIN, 1039.0];
const BOX_RIGHT_OF_ORIGIN: f32 = 109.0;
const BOX_ABOVE_ORIGIN: f32 = 20.0;
/// Calls between two sets of the position even when nothing changed: the game may rebuild its HUD
/// (another save loaded) and a new view has the default place. About 5 seconds.
const REFRESH_EVERY: u32 = 300;
/// The clip's x on the game's own HUD (2.7.2.0, any resolution).
const VANILLA_X: f32 = 1762.0;

type UpdateFn = unsafe extern "system" fn(*mut u8, f32, *const u8);

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicU32 = AtomicU32::new(0);
/// The view the position was last set on, and the bits of that position.
static MOVED_VIEW: AtomicUsize = AtomicUsize::new(0);
static MOVED_X: AtomicU32 = AtomicU32::new(0);
static MOVED_Y: AtomicU32 = AtomicU32::new(0);
static MOVED_SCALE: AtomicU32 = AtomicU32::new(0);
/// The clip's place before the mod moved it, as `f32` bits, to put it back.
static BEFORE_X: AtomicU32 = AtomicU32::new(0);
static BEFORE_Y: AtomicU32 = AtomicU32::new(0);
static BEFORE_KNOWN: AtomicBool = AtomicBool::new(false);

/// How far the game has moved its HUD out to the edges of a wide screen, in pixels of the 1080p
/// picture, read off the counter's own place (0 on the game's own layout); at most `max`, what the
/// screen leaves beyond the 16:9 picture on each side. The right group moves out by this much, the
/// left group by the same.
pub fn edge_shift(max: f32) -> f32 {
    if !BEFORE_KNOWN.load(Ordering::Acquire) {
        return 0.0;
    }
    (f32::from_bits(BEFORE_X.load(Ordering::Relaxed)) - VANILLA_X).clamp(0.0, max.max(0.0))
}

/// Runs the game's update, then puts the clip where the mod wants it.
unsafe extern "system" fn update(view: *mut u8, dt: f32, model: *const u8) {
    // SAFETY: `ORIGINAL` is the call's destination, stored before the redirect.
    let original: UpdateFn = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Acquire)) };
    // SAFETY: the original function of the game, called with the arguments the game passed.
    unsafe { original(view, dt, model) };
    // A panic must not unwind into the game.
    // SAFETY: `view` is the counter's view the game passed to this call.
    let _ = std::panic::catch_unwind(|| unsafe { place(view) });
}

/// The clip's place and size that put the counter's box where `HUD.RuneCounter.OffsetX/Y` say (from
/// the top-right corner), from the place the game has the clip at.
fn target() -> (f32, f32, f32) {
    let scale = settings::rune_counter_scale();
    let [offset_x, offset_y] = settings::rune_offset();
    let origin =
        [1920.0 - offset_x - BOX_RIGHT_OF_ORIGIN * scale, offset_y + BOX_ABOVE_ORIGIN * scale];
    let (x, y) = (
        f32::from_bits(BEFORE_X.load(Ordering::Relaxed)),
        f32::from_bits(BEFORE_Y.load(Ordering::Relaxed)),
    );
    (x + origin[0] - ORIGIN[0], y + origin[1] - ORIGIN[1], scale)
}

/// # Safety
/// `view` must be the game's `SoulViewParts`.
unsafe fn place(view: *mut u8) {
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    // SAFETY: the component sits at a fixed offset of the view.
    let value = unsafe { clip_at(view, ROOT) };
    if value.is_null() {
        return;
    }
    let fresh = MOVED_VIEW.load(Ordering::Relaxed) != view as usize;
    if !BEFORE_KNOWN.load(Ordering::Acquire) {
        // SAFETY: `value` is the live clip of the view.
        let Some(shown) = (unsafe { display_of(value) }) else { return };
        BEFORE_X.store((shown[0] as f32).to_bits(), Ordering::Relaxed);
        BEFORE_Y.store((shown[1] as f32).to_bits(), Ordering::Relaxed);
        BEFORE_KNOWN.store(true, Ordering::Release);
        logger::log(&format!(
            "Runes: clip at ({:.1}, {:.1}) scale {:.0}",
            shown[0], shown[1], shown[3]
        ));
    }
    if settings::rune_counter() {
        let (x, y, scale) = target();
        let unchanged = !fresh
            && MOVED_X.load(Ordering::Relaxed) == x.to_bits()
            && MOVED_Y.load(Ordering::Relaxed) == y.to_bits()
            && MOVED_SCALE.load(Ordering::Relaxed) == scale.to_bits();
        if unchanged && !calls.is_multiple_of(REFRESH_EVERY) {
            return;
        }
        // SAFETY: `value` is the live clip of the view.
        let set = unsafe { set_clip_position(value, x, y) && set_clip_scale(value, scale) };
        if set {
            MOVED_VIEW.store(view as usize, Ordering::Relaxed);
            MOVED_X.store(x.to_bits(), Ordering::Relaxed);
            MOVED_Y.store(y.to_bits(), Ordering::Relaxed);
            MOVED_SCALE.store(scale.to_bits(), Ordering::Relaxed);
        }
        if calls < 3 || calls == REFRESH_EVERY {
            // SAFETY: `value` is the live clip of the view.
            let now = unsafe { display_of(value) };
            logger::log(&format!(
                "Runes: call {calls}, set ({x:.1}, {y:.1}) {set}, clip now {now:?}"
            ));
        }
    } else if MOVED_VIEW.load(Ordering::Relaxed) == view as usize {
        // Off again (the file was reloaded): back where the game has it. A view the mod did not move is left alone.
        let (x, y) = (
            f32::from_bits(BEFORE_X.load(Ordering::Relaxed)),
            f32::from_bits(BEFORE_Y.load(Ordering::Relaxed)),
        );
        // SAFETY: `value` is the live clip of the view.
        if unsafe { set_clip_position(value, x, y) && set_clip_scale(value, 1.0) } {
            MOVED_VIEW.store(0, Ordering::Relaxed);
        }
    }
}

/// Redirects the call, on a thread of its own (the scan waits for the game's code to be there).
pub fn install() {
    std::thread::spawn(|| {
        let Some(anchor) = memscan::wait_for_pattern_in_module(
            CALL_AOB,
            Duration::from_millis(500),
            Duration::from_secs(60),
        ) else {
            logger::error("Runes: pattern not found");
            return;
        };
        // SAFETY: the offset stays inside the matched game code.
        let site = unsafe { anchor.add(CALL_AT) };
        // SAFETY: reads the 5 bytes of a call found in the game image.
        let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
            logger::error("Runes: site is not a call");
            return;
        };
        ORIGINAL.store(original as usize, Ordering::Release);
        // SAFETY: `site` is the scanned call; the stub jumps to a function with the callee's signature.
        let ok = unsafe {
            codepatch::redirect_rel32(
                site,
                &codepatch::absolute_jmp(update as UpdateFn as usize as u64),
            )
        }
        .is_some();
        logger::log(if ok { "Runes: hooked" } else { "Runes: redirect failed" });
    });
}
