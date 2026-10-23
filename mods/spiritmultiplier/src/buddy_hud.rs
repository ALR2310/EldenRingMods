//! Spirit HP bars on the HUD. `HubHpBars` (5-30, startup only) lifts the vanilla
//! limit of 5. Analysis and offsets: `docs/buddy_hud.md`.
//!
//! Each frame the game fills 5 slots (`sub_140771ED0`, handle + HP per spirit)
//! and a view object (`CS::BuddyViewParts`) binds them to the movie's
//! `GaugeList` rows `Item_0`...`Item_4`. The view keeps its rows in a fixed
//! 5-entry vector, so further views are built on the same `Buddy` clip, each
//! keeping the next five rows, and are fed the slots past the fifth:
//! - the movie gets more rows ([common::gfx], through [common::gfx_hook]);
//! - [hud_blob] records every spirit the list builder offers a slot to;
//! - [hud_build] turns the ones past the fifth into extra slots;
//! - [hud_ctor] builds the extra views next to the game's, [hud_item] gives
//!   each view its own rows;
//! - [hud_view] updates the extra views right after the first, and moves the
//!   panel up by the height of the rows in use, so it sits where the game puts
//!   it with 5 bars or fewer and only grows into free space above.
//!
//! The same `sub_1404B7A10` redirect also leaves the Mimic Blob out of the
//! list while `InstantMimic` is on (`docs/mimic_tear.md`).

use std::alloc::{Layout, alloc_zeroed};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Duration;

use eldenring::cs::ChrIns;

use common::{codepatch, config, gfx, gfx_hook, logger, memscan};

/// Rows one view holds, and the bars the game shows.
const VANILLA_BARS: usize = 5;
const MAX_BARS: usize = 30;
const DEFAULT_BARS: usize = VANILLA_BARS;
const MAX_EXTRA_VIEWS: usize = MAX_BARS / VANILLA_BARS - 1;

/// One slot: handle (8), HP (4), padding (4), max HP lost (4), max HP (4).
const SLOT_LEN: usize = 24;
const EXTRA_SLOTS: usize = MAX_BARS - VANILLA_BARS;
/// The block a view reads: 136 bytes of header, then its 5 slots.
const BLOCK_HEADER: usize = 136;
const BLOCK_LEN: usize = BLOCK_HEADER + SLOT_LEN * VANILLA_BARS;
/// A view object (`CS::BuddyViewParts`) is 0x430 bytes in the HUD; the margin
/// covers a build that grew it.
const VIEW_LEN: usize = 0x1000;

/// Movie to extend and the instance path of its list of rows.
const MOVIE: &str = "01_000_fe.gfx";
const MOVIE_PATH: [&str; 3] = ["Buddy", "Anim", "GaugeList"];
const MOVIE_ROW: &str = "Item_";
/// Gap between two rows in pixels (300 twips in the movie). `Anim`, the clip
/// holding the whole panel, sits at (0, 0) inside `Buddy`.
const ROW_PX: f32 = 15.0;
/// The view's component bound to `Anim` (`CSMenuStaticAnimComponent`, built by
/// `sub_1408C1D20` at view+776), and the offset of its clip proxy inside it.
const ANIM_COMPONENT: usize = 776;
const COMPONENT_PROXY: usize = 8;

/// `sub_1404B7A10(group, chr)`'s call in the list builder, then `cmp edi,5`.
/// Unique on 2.6.2 - 2.7.2. The `call` is at +10.
const BLOB_AOB: &str = "84 C0 74 ?? 48 8B D6 49 8B CE E8 ?? ?? ?? ?? 84 C0 75 ?? 83 FF 05";
const BLOB_CALL: usize = 10;
/// The list builder's call in the HUD update (`sub_140772F00`), between two
/// neighbours that pin it. The `call` is at +11.
const BUILD_AOB: &str = "E8 ?? ?? ?? ?? 0F 28 CE 48 8B CB E8 ?? ?? ?? ?? 0F 28 CE 48 8B CB E8 ?? ?? ?? ?? 4C 8B C7";
const BUILD_CALL: usize = 11;
/// The view update's call (`sub_1408C2600`) with its block argument. The `call` is at +40.
const VIEW_AOB: &str = "48 81 C2 ?? ?? 00 00 48 8D 8E ?? ?? ?? ?? E8 ?? ?? ?? ?? 4C 8B 46 60 49 81 C0 ?? ?? 00 00 \
     48 8D 8E ?? ?? ?? ?? 0F 28 CE E8 ?? ?? ?? ??";
const VIEW_CALL: usize = 40;
/// The view update's own slot window: a 136-byte header (`lea rdi,[r15+88h]`)
/// and 5 slots of 24 bytes (`lea rsi,[rdi+78h]`). Only checked, not patched:
/// it is what the block layout above relies on.
const LAYOUT_AOB: &str = "49 8D BF 88 00 00 00 41 BC FF FF FF FF 48 8D 77 78 48 3B FE";
/// `sub_140D872C0(value, x, y)`: sets `_x` and `_y` of a Scaleform clip. From
/// its entry; unique on 2.6.2 - 2.7.2.
const SET_POS_AOB: &str = "48 8B C4 48 89 58 10 55 48 8D 68 C8 48 81 EC 30 01 00 00 0F 29 70 E8 0F 29 78 D8 48 8B 05 ?? ?? ?? ?? \
     48 33 C4 48 89 45 00 F6 41 20 8F 0F 28 F2 0F 28 F9 48 8B D9 0F 84 ?? ?? ?? ??";
/// The view constructor's call in the HUD constructor (`sub_1408C6C80`), after
/// the lookup of the `Buddy` clip. The `call` is at +31.
const CTOR_AOB: &str = "4C 8D 05 ?? ?? ?? ?? 48 8D 54 24 ?? 49 8B CE E8 ?? ?? ?? ?? 90 49 8D 8E ?? ?? ?? ?? 48 8B D0 E8 ?? ?? ?? ??";
const CTOR_CALL: usize = 31;
/// Entry of the function that adds one row to a view's vector
/// (`sub_1408C24C0`): it stops at 5 rows. Wrapped by a tiny `add rcx,8; jmp`.
const ITEM_AOB: &str = "40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF 48 89 5C 24 48 48 89 74 24 50 48 8B DA 48 8B F1 \
     48 8B 01 48 83 B8 B0 03 00 00 05 0F 83";
/// `add rcx,8` right before that wrapper's `jmp`.
const ITEM_WRAPPER_PREFIX: [u8; 4] = [0x48, 0x83, 0xC1, 0x08];

/// Bars shown, 5 until [install_extended] raises it.
static BARS: AtomicUsize = AtomicUsize::new(VANILLA_BARS);
static ORIG_BLOB: AtomicUsize = AtomicUsize::new(0);
static ORIG_BUILD: AtomicUsize = AtomicUsize::new(0);
static ORIG_VIEW: AtomicUsize = AtomicUsize::new(0);
static ORIG_CTOR: AtomicUsize = AtomicUsize::new(0);
static ORIG_ITEM: AtomicUsize = AtomicUsize::new(0);
/// `sub_140D872C0`, found by [install_extended].
static SET_POS: AtomicUsize = AtomicUsize::new(0);
/// Rows in use (5 to the bar count) and the panel offset last applied, as `f32` bits.
static ROWS: AtomicUsize = AtomicUsize::new(VANILLA_BARS);
static APPLIED_OFFSET: AtomicU32 = AtomicU32::new(0);

/// The game's view, and the ones built next to it (0 until the HUD is built).
static VIEW_FIRST: AtomicUsize = AtomicUsize::new(0);
const NO_VIEW: AtomicUsize = AtomicUsize::new(0);
static VIEW_EXTRA: [AtomicUsize; MAX_EXTRA_VIEWS] = [NO_VIEW; MAX_EXTRA_VIEWS];
/// Which view is being built: 0 the game's, 1... the extra ones. Its rows are
/// the ones numbered `5 * view` to `5 * view + 4`.
static BUILDING_VIEW: AtomicUsize = AtomicUsize::new(0);

/// Characters the list builder offered a slot to this frame, in slot order.
static OFFERED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// The slots past the fifth, rebuilt every frame.
static EXTRA: Mutex<[u8; SLOT_LEN * EXTRA_SLOTS]> = Mutex::new([0xFF; SLOT_LEN * EXTRA_SLOTS]);

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `HubHpBars` from the ini, kept in 5..=30.
fn configured_bars() -> usize {
    (config::get_int("HubHpBars", DEFAULT_BARS as i32).max(0) as usize).clamp(VANILLA_BARS, MAX_BARS)
}

/// Views needed besides the game's for `bars` bars.
fn extra_views(bars: usize) -> usize {
    bars.saturating_sub(1) / VANILLA_BARS
}

/// A slot in the game's layout. A free slot is all `0xFF`.
fn slot_bytes(handle: u64, hp: i32, max_hp: i32, max_uncapped: i32) -> [u8; SLOT_LEN] {
    let mut slot = [0u8; SLOT_LEN];
    slot[0..8].copy_from_slice(&handle.to_le_bytes());
    slot[8..12].copy_from_slice(&hp.to_le_bytes());
    slot[16..20].copy_from_slice(&(max_uncapped - max_hp).to_le_bytes());
    slot[20..24].copy_from_slice(&max_uncapped.to_le_bytes());
    slot
}

/// The slot the game would have written for `chr` (`sub_140771ED0`).
///
/// # Safety
/// `chr` must point to a live `ChrIns`.
unsafe fn slot_of(chr: usize) -> [u8; SLOT_LEN] {
    // SAFETY: the game's list builder handed this `ChrIns` out this frame.
    let (handle, chr) = unsafe { ((chr as *const u8).add(8).cast::<u64>().read_unaligned(), &*(chr as *const ChrIns)) };
    let data = &chr.modules.data;
    slot_bytes(handle, data.hp, data.max_hp, data.max_uncapped_hp)
}

/// The slots after the fifth: one per offered character past the vanilla 5, free otherwise.
fn extra_slots(offered: &[usize], bars: usize) -> [u8; SLOT_LEN * EXTRA_SLOTS] {
    let mut extra = [0xFF; SLOT_LEN * EXTRA_SLOTS];
    for (k, slot) in extra.chunks_exact_mut(SLOT_LEN).enumerate() {
        if let Some(&chr) = offered.get(VANILLA_BARS + k).filter(|_| VANILLA_BARS + k < bars) {
            // SAFETY: recorded from the list builder this frame.
            slot.copy_from_slice(&unsafe { slot_of(chr) });
        }
    }
    extra
}

type BlobFn = unsafe extern "system" fn(*mut u8, *mut u8) -> u8;
type BuildFn = unsafe extern "system" fn(*mut u8, f32);
type ViewFn = unsafe extern "system" fn(*mut u8, f32, *const u8);
type CtorFn = unsafe extern "system" fn(*mut u8, *mut u8) -> *mut u8;
type ItemFn = unsafe extern "system" fn(*mut u8, *mut u8, u32) -> u64;
type ValueOfFn = unsafe extern "system" fn(*mut u8) -> *mut u8;
type SetPosFn = unsafe extern "system" fn(*mut u8, f32, f32);

/// How far the panel moves up (negative y, pixels) with `rows` rows in use:
/// nothing up to the game's 5.
fn panel_offset(rows: usize) -> f32 {
    -(rows.saturating_sub(VANILLA_BARS) as f32) * ROW_PX
}

/// Moves the panel (the `Anim` clip of the game's view) to the offset for the
/// rows in use, when that changed. The same call the game's own menu code makes
/// to place a clip: the clip proxy's value, then `sub_140D872C0`.
///
/// # Safety
/// `view` must be the game's constructed view.
unsafe fn place_panel(view: *mut u8) {
    let set_pos = SET_POS.load(Ordering::Acquire);
    let offset = panel_offset(ROWS.load(Ordering::Relaxed));
    if set_pos == 0 || offset.to_bits() == APPLIED_OFFSET.load(Ordering::Relaxed) {
        return;
    }
    // SAFETY: the proxy sits at a fixed offset of the view; its vtable's second entry returns the clip's value.
    unsafe {
        let proxy = view.add(ANIM_COMPONENT + COMPONENT_PROXY);
        let value_of: ValueOfFn = std::mem::transmute(*(*(proxy as *const *const usize)).add(1));
        let value = value_of(proxy);
        if value.is_null() {
            return;
        }
        std::mem::transmute::<usize, SetPosFn>(set_pos)(value, 0.0, offset);
    }
    APPLIED_OFFSET.store(offset.to_bits(), Ordering::Relaxed);
}

/// Stands in for `sub_1404B7A10(group, chr)` ("is this the Mimic Blob"), called
/// by the list builder for every spirit that passed its other checks: answers
/// "no" while `InstantMimic` turned the Blob into a real Mimic, and notes each
/// spirit that gets a slot.
unsafe extern "system" fn hud_blob(group: *mut u8, chr: *mut u8) -> u8 {
    // SAFETY: `ORIG_BLOB` is the call's destination, stored before the redirect.
    let original: BlobFn = unsafe { std::mem::transmute(ORIG_BLOB.load(Ordering::Acquire)) };
    let blob = if crate::mimic::hud_flag() { 0 } else { unsafe { original(group, chr) } };
    if blob == 0 && BARS.load(Ordering::Relaxed) > VANILLA_BARS {
        locked(&OFFERED).push(chr as usize);
    }
    blob
}

/// Stands in for the list builder: runs it, then writes the extra slots.
unsafe extern "system" fn hud_build(hud: *mut u8, dt: f32) {
    locked(&OFFERED).clear();
    // SAFETY: `ORIG_BUILD` is the call's destination, stored before the redirect.
    let original: BuildFn = unsafe { std::mem::transmute(ORIG_BUILD.load(Ordering::Acquire)) };
    unsafe { original(hud, dt) };
    // A panic must not unwind into the game; the old slots stay.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let offered = locked(&OFFERED);
        let bars = BARS.load(Ordering::Relaxed);
        *locked(&EXTRA) = extra_slots(&offered, bars);
        ROWS.store(offered.len().clamp(VANILLA_BARS, bars), Ordering::Relaxed);
    }));
}

/// Stands in for the view constructor in the HUD constructor: builds the
/// game's view, then the extra ones on the same `Buddy` clip for the rows the
/// first cannot hold.
unsafe extern "system" fn hud_ctor(this: *mut u8, clip: *mut u8) -> *mut u8 {
    // SAFETY: `ORIG_CTOR` is the call's destination, stored before the redirect.
    let original: CtorFn = unsafe { std::mem::transmute(ORIG_CTOR.load(Ordering::Acquire)) };
    let first = unsafe { original(this, clip) };
    VIEW_EXTRA.iter().for_each(|view| view.store(0, Ordering::Release));
    VIEW_FIRST.store(this as usize, Ordering::Release);
    // a new movie instance: the panel is at its authored place again
    APPLIED_OFFSET.store(0, Ordering::Relaxed);
    for (k, slot) in VIEW_EXTRA.iter().enumerate().take(extra_views(BARS.load(Ordering::Relaxed))) {
        let Ok(layout) = Layout::from_size_align(VIEW_LEN, 16) else { break };
        // SAFETY: non-zero size; never freed, like the game's view it lives as long as the HUD.
        let view = unsafe { alloc_zeroed(layout) };
        if view.is_null() {
            break;
        }
        BUILDING_VIEW.store(k + 1, Ordering::Release);
        unsafe { original(view, clip) };
        slot.store(view as usize, Ordering::Release);
    }
    BUILDING_VIEW.store(0, Ordering::Release);
    first
}

/// Stands in for the function that adds one `Item_N` row to a view (a `void`
/// callback): each view keeps its own five rows, the game's rows 0-4 and the
/// extra ones the following fives. A row a view refuses is skipped here, not
/// passed to the game's "vector full" early-out: that hides the row's clip,
/// which another view shows.
unsafe extern "system" fn hud_item(capture: *mut u8, row: *mut u8, index: u32) -> u64 {
    if index as usize / VANILLA_BARS != BUILDING_VIEW.load(Ordering::Acquire) {
        return 0;
    }
    // SAFETY: `ORIG_ITEM` is the jump's destination, stored before the redirect.
    let original: ItemFn = unsafe { std::mem::transmute(ORIG_ITEM.load(Ordering::Acquire)) };
    unsafe { original(capture, row, index) }
}

/// Stands in for the view update: updates the game's view, then each extra one
/// with the game's header and its five of the extra slots.
unsafe extern "system" fn hud_view(view: *mut u8, dt: f32, data: *const u8) {
    // SAFETY: `ORIG_VIEW` is the call's destination, stored before the redirect.
    let original: ViewFn = unsafe { std::mem::transmute(ORIG_VIEW.load(Ordering::Acquire)) };
    unsafe { original(view, dt, data) };
    if view as usize != VIEW_FIRST.load(Ordering::Acquire) {
        return;
    }
    // A panic must not unwind into the game.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        unsafe { place_panel(view) };
        let extra = *locked(&EXTRA);
        let mut block = [0u8; BLOCK_LEN];
        // SAFETY: `data` is the game's block, at least header + 5 slots long.
        block[..BLOCK_HEADER].copy_from_slice(unsafe { std::slice::from_raw_parts(data, BLOCK_HEADER) });
        for (k, extra_view) in VIEW_EXTRA.iter().enumerate() {
            let extra_view = extra_view.load(Ordering::Acquire);
            if extra_view == 0 {
                break;
            }
            let slots = &extra[k * SLOT_LEN * VANILLA_BARS..(k + 1) * SLOT_LEN * VANILLA_BARS];
            block[BLOCK_HEADER..].copy_from_slice(slots);
            unsafe { original(extra_view as *mut u8, dt, block.as_ptr()) };
        }
    }));
}

/// Finds `aob`, checks the `call` at `offset` and returns it with its destination.
fn find_call(aob: &str, offset: usize, feature: &str) -> Option<(*mut u8, u64)> {
    let Some(anchor) = memscan::wait_for_pattern_in_module(aob, Duration::from_millis(500), Duration::from_secs(60)) else {
        logger::error(&format!("{feature}: pattern not found"));
        return None;
    };
    let site = unsafe { anchor.add(offset) };
    // SAFETY: reads the 5 bytes of a call found in the game image.
    let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
        if !unsafe { crate::conflicts::check(site, feature) } {
            logger::error(&format!("{feature}: site is not a call"));
        }
        return None;
    };
    Some((site, original))
}

/// The tail `jmp` of the wrapper that calls the row-adding function, found
/// from that function's entry: the `jmp rel32` landing on it, right after `add rcx,8`.
fn find_item_jump() -> Option<(*mut u8, u64)> {
    let Some(entry) = memscan::wait_for_pattern_in_module(ITEM_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
        logger::error("HubHpBars rows: pattern not found");
        return None;
    };
    let sites: Vec<*mut u8> = memscan::find_jumps_to(entry as usize)
        .into_iter()
        // SAFETY: the 4 bytes before a `jmp` found in the game's code are readable code.
        .filter(|&site| unsafe { std::slice::from_raw_parts(site.sub(ITEM_WRAPPER_PREFIX.len()), ITEM_WRAPPER_PREFIX.len()) } == ITEM_WRAPPER_PREFIX)
        .collect();
    let [site] = sites.as_slice() else {
        logger::error(&format!("HubHpBars rows: {} wrapper(s) found, expected 1", sites.len()));
        return None;
    };
    Some((*site, entry as u64))
}

/// Points the `call`/`jmp` at `site` to `detour` and stores the old destination in `slot`.
fn redirect(site: *mut u8, original: u64, slot: &AtomicUsize, detour: usize, feature: &str) -> bool {
    slot.store(original as usize, Ordering::Release);
    // SAFETY: `site` is a scanned `call`/`jmp`; the stub jumps to a function with the callee's signature.
    let ok = unsafe { codepatch::redirect_rel32(site, &codepatch::absolute_jmp(detour as u64)) }.is_some();
    if !ok {
        logger::error(&format!("{feature}: redirect failed"));
    }
    ok
}

/// The patches `HubHpBars` needs beyond the Blob check. Every site is found
/// first, and the layout it relies on checked; the redirects go in an order
/// where stopping part-way leaves 5 bars: the view update and row function
/// pass through until the constructor is redirected, and the list builder only
/// fills slots nothing reads before then.
fn install_extended(bars: usize) -> bool {
    BARS.store(bars, Ordering::Relaxed);
    if memscan::find_pattern_in_module(LAYOUT_AOB).is_none() {
        logger::error("HubHpBars: slot layout differs from the one this is written for");
        return false;
    }
    let Some((view_site, view_original)) = find_call(VIEW_AOB, VIEW_CALL, "HubHpBars view") else {
        return false;
    };
    let Some((item_site, item_original)) = find_item_jump() else {
        return false;
    };
    let Some((ctor_site, ctor_original)) = find_call(CTOR_AOB, CTOR_CALL, "HubHpBars constructor") else {
        return false;
    };
    let Some((build_site, build_original)) = find_call(BUILD_AOB, BUILD_CALL, "HubHpBars list") else {
        return false;
    };
    let Some(set_pos) = memscan::wait_for_pattern_in_module(SET_POS_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
        logger::error("HubHpBars position: pattern not found");
        return false;
    };
    SET_POS.store(set_pos as usize, Ordering::Release);

    redirect(view_site, view_original, &ORIG_VIEW, hud_view as ViewFn as usize, "HubHpBars view")
        && redirect(item_site, item_original, &ORIG_ITEM, hud_item as ItemFn as usize, "HubHpBars rows")
        && redirect(build_site, build_original, &ORIG_BUILD, hud_build as BuildFn as usize, "HubHpBars list")
        && redirect(ctor_site, ctor_original, &ORIG_CTOR, hud_ctor as CtorFn as usize, "HubHpBars constructor")
}

/// The movie with `Item_5`... added, `None` when it can't be (left as the game has it).
fn edit_movie(movie: &[u8]) -> Option<Vec<u8>> {
    match gfx::extend_numbered_placements(movie, &MOVIE_PATH, MOVIE_ROW, BARS.load(Ordering::Relaxed) as u32) {
        Ok(edited) => edited,
        Err(err) => {
            logger::warn(&format!("HubHpBars: movie edit failed ({err:?}), extra bars will not show"));
            None
        }
    }
}

pub fn run() {
    // The Blob check is always redirected: `InstantMimic` needs it too.
    if let Some((site, original)) = find_call(BLOB_AOB, BLOB_CALL, "HUD Blob check")
        && redirect(site, original, &ORIG_BLOB, hud_blob as BlobFn as usize, "HUD Blob check")
    {
        logger::log("HUD: Blob check patched");
    }

    let bars = configured_bars();
    if bars == VANILLA_BARS {
        return;
    }
    if install_extended(bars) && gfx_hook::install(MOVIE, edit_movie) {
        logger::log(&format!("HubHpBars: {bars}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_layout_matches_the_games() {
        let slot = slot_bytes(0x0102_0304_0506_0708, 900, 1000, 1200);
        assert_eq!(slot[0..8], 0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(i32::from_le_bytes(slot[8..12].try_into().unwrap()), 900);
        assert_eq!(slot[12..16], [0; 4]);
        assert_eq!(i32::from_le_bytes(slot[16..20].try_into().unwrap()), 200);
        assert_eq!(i32::from_le_bytes(slot[20..24].try_into().unwrap()), 1200);
    }

    #[test]
    fn extra_slots_are_free_without_offered_characters() {
        assert!(extra_slots(&[], MAX_BARS).iter().all(|&b| b == 0xFF));
        // the first five offered characters keep the game's own slots
        assert!(extra_slots(&[1, 2, 3, 4, 5], MAX_BARS).iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn the_panel_moves_up_one_row_per_bar_past_five() {
        assert_eq!(panel_offset(0), 0.0);
        assert_eq!(panel_offset(5), 0.0);
        assert_eq!(panel_offset(6), -15.0);
        assert_eq!(panel_offset(30), -375.0);
    }

    #[test]
    fn one_extra_view_per_five_bars_past_the_first_five() {
        let views: Vec<usize> = (5..=30).map(extra_views).collect();
        let expected: Vec<usize> = std::iter::once(0).chain((1..=5).flat_map(|n| [n; 5])).collect();
        assert_eq!(views, expected);
        // the extra views and the game's own hold exactly MAX_BARS rows
        assert_eq!((MAX_EXTRA_VIEWS + 1) * VANILLA_BARS, MAX_BARS);
        assert_eq!(EXTRA_SLOTS, MAX_EXTRA_VIEWS * VANILLA_BARS);
        assert_eq!(BLOCK_LEN, 136 + 24 * 5);
    }
}
