//! The game's row of status icons (the diamonds under the player's bars): where its data is.
//! The game updates the row each frame with `sub_1408D58F0(view, dt, model)`; a redirect of that
//! call keeps `model`, a block of 42 slots of 64 bytes (the image of each icon) followed by one
//! flicker flag per slot. A second redirect hides the game's buildup gauge of the player.
//! Details: `docs/effects.md`.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
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
/// seven statuses, `model` 104 bytes with the flag "shown" at +65 (`sub_1408D2AE0`, which passes the
/// flag to the gauge's `FadeIn`/`FadeOut`). Unique on 2.6.2.0 to 2.7.2.0.
const GAUGE_AOB: &str = "4C 8B 46 60 49 81 C0 ?? ?? 00 00 4C 03 C1 48 69 CA ?? ?? 00 00 48 03 CF 49 03 CE 0F 28 CE E8 ?? ?? ?? ?? FF C3 48 63 D3";
const GAUGE_CALL_AT: usize = 30;
const GAUGE_MODEL_LEN: usize = 104;
const GAUGE_SHOWN_AT: usize = 65;

type GaugeFn = unsafe extern "system" fn(*mut u8, i64, *const u8);

static GAUGE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Bit k is set while the player is afflicted with status k (an index into `target::BUILDUP`), the
/// status whose gauge is hidden: it shows as an effect with its time left instead.
static AFFLICTED: AtomicU32 = AtomicU32::new(0);

/// The last model each gauge was given, as 13 words of 8 bytes each (research: which gauge is
/// which status, and what its flags say; see `describe_gauges`).
const GAUGES: usize = 7;
const GAUGE_WORDS: usize = GAUGE_MODEL_LEN / 8;
static GAUGE_SEEN: [AtomicU64; GAUGES * GAUGE_WORDS] = [const { AtomicU64::new(0) }; GAUGES * GAUGE_WORDS];

/// The gauges that are shown, as `index: icon frame, state flag`, for the log.
pub fn describe_gauges() -> String {
    let mut shown = Vec::new();
    for i in 0..GAUGES {
        let words: Vec<u64> = (0..GAUGE_WORDS).map(|w| GAUGE_SEEN[i * GAUGE_WORDS + w].load(Ordering::Relaxed)).collect();
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        if bytes[GAUGE_SHOWN_AT] == 0 {
            continue;
        }
        let frame = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        shown.push(format!("[{i}] frame {frame} state {} shown {}", bytes[64], bytes[GAUGE_SHOWN_AT]));
    }
    if shown.is_empty() { "none".to_string() } else { shown.join(" | ") }
}

/// Keeps which statuses the player is afflicted with.
pub fn set_afflicted(mask: u32) {
    AFFLICTED.store(mask, Ordering::Relaxed);
}

/// Runs the game's gauge update with the "shown" flag cleared for a status the player is afflicted
/// with (while the player's effects are on), so the gauge fades out: the status shows as an effect
/// with its time left instead. While the status is only building up the gauge stays.
unsafe extern "system" fn gauge_update(view: *mut u8, index: i64, model: *const u8) {
    // SAFETY: `GAUGE_ORIGINAL` is the call's destination, stored before the redirect.
    let original: GaugeFn = unsafe { std::mem::transmute(GAUGE_ORIGINAL.load(Ordering::Acquire)) };
    if (0..GAUGES as i64).contains(&index) {
        if let Some(seen) = memscan::read_safe::<[u64; GAUGE_WORDS]>(model as usize) {
            for (w, word) in seen.iter().enumerate() {
                GAUGE_SEEN[index as usize * GAUGE_WORDS + w].store(*word, Ordering::Relaxed);
            }
        }
    }
    // The first word of the model is the frame of the gauge's `Icon` clip, 1 for poison, 2 for
    // scarlet rot... (the order of `target::BUILDUP`): the game gives the clips to the statuses
    // that are active in turn, so the index of the clip says nothing about the status.
    let kind = memscan::read_safe::<i32>(model as usize).map_or(-1, |frame| frame - 1);
    let afflicted = (0..32).contains(&kind) && AFFLICTED.load(Ordering::Relaxed) & (1 << kind) != 0;
    if afflicted && crate::settings::effects_player() {
        if let Some(mut copy) = memscan::read_safe::<[u8; GAUGE_MODEL_LEN]>(model as usize) {
            copy[GAUGE_SHOWN_AT] = 0;
            unsafe { original(view, index, copy.as_ptr()) };
            return;
        }
    }
    unsafe { original(view, index, model) };
}

/// Redirects the gauge update call, on a thread of its own.
pub fn install_gauge() {
    std::thread::spawn(|| {
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
