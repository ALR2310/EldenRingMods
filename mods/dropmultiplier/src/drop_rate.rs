//! Adjusts enemy drop chances in `ItemLotParam_enemy`, in one of two ways
//! picked by `Mode`: `Multiplier` scales every real item's weight, or
//! `Percentage` forces a row's combined "any real item" chance to a fixed
//! percent. The lottery model, the algebra and the edge cases:
//! `docs/drop_rate.md`.

use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, ItemLotParam_enemy, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use common::{config, logger};

use crate::lot::{self, ItemIds, Points};

/// The game's own original weights by row index (see `common::params` for why
/// not by ID), captured once before any edit, so every reload rescales from the
/// true baseline instead of compounding.
static ORIGINAL_BASE_POINTS: Mutex<Option<Vec<Points>>> = Mutex::new(None);

fn clamp_u16(value: f64) -> u16 {
    value.round().clamp(0.0, u16::MAX as f64) as u16
}

/// The two mutually exclusive adjustments, selected by `Mode`.
enum Mode {
    /// `Mode=0`: scale every real item's weight by this factor.
    Multiplier(f64),
    /// `Mode=1`: force each row's combined real-item chance to this percent
    /// (`0..=100`), keeping each item's share against its row-mates.
    FixedPercent(f64),
}

fn build_mode() -> Mode {
    match config::get_int("Mode", 0) {
        1 => Mode::FixedPercent(config::get_double("Percentage", 50.0).max(0.0)),
        _ => Mode::Multiplier(config::get_double("Multiplier", 2.0).max(0.0)),
    }
}

/// `original` with every real item's weight (slot id != 0) scaled by `factor`.
fn scale_items(original: &Points, item_ids: &ItemIds, factor: f64) -> Points {
    let mut scaled = *original;
    for i in 0..8 {
        if item_ids[i] != 0 {
            scaled[i] = clamp_u16(original[i] as f64 * factor);
        }
    }
    scaled
}

/// One row's new weights for `mode`, always derived from `original` (never
/// from an already-scaled value), so reloading with another mode is safe.
fn scale_row(original: &Points, item_ids: &ItemIds, mode: &Mode) -> Points {
    match *mode {
        Mode::Multiplier(factor) => scale_items(original, item_ids, factor),
        Mode::FixedPercent(percent) => {
            let target = (percent / 100.0).min(1.0);
            let sum = |real: bool| -> u32 {
                (0..8).filter(|&i| (item_ids[i] != 0) == real).map(|i| original[i] as u32).sum()
            };
            let (item_sum, other_sum) = (sum(true), sum(false));
            if item_sum == 0 {
                return *original; // no real item: nothing to force a chance onto
            }
            if target >= 1.0 {
                // 100%: zero the "nothing" slots instead of dividing by zero.
                let mut scaled = *original;
                for i in 0..8 {
                    if item_ids[i] == 0 {
                        scaled[i] = 0;
                    }
                }
                return scaled;
            }
            if other_sum == 0 {
                return *original; // no "nothing" slot to shrink: already drops every time
            }
            scale_items(original, item_ids, target * other_sum as f64 / ((1.0 - target) * item_sum as f64))
        }
    }
}

/// Applies `mode` to every `ItemLotParam_enemy` row from the snapshot of the
/// original weights (taken on the first call). Returns the number of rows
/// touched, or `None` if the param slot isn't what we expect (nothing written).
fn apply(repo: &mut SoloParamRepository, mode: &Mode) -> Option<usize> {
    if let Err(reason) = common::params::check::<ItemLotParam_enemy>(repo) {
        logger::error(&format!("{reason} - drop rates left unchanged"));
        return None;
    }

    // A poisoned lock is recovered: one panic must not break every reload.
    let mut snapshot_guard = ORIGINAL_BASE_POINTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if snapshot_guard.is_none() {
        logger::log(&format!("Snapshot: {}", common::params::describe::<ItemLotParam_enemy>(repo)));
        let mut points = Vec::new();
        common::params::for_each_row_mut::<ItemLotParam_enemy>(repo, |_, row| points.push(lot::base_points(row)));
        *snapshot_guard = Some(points);
    }
    let snapshot = snapshot_guard.as_ref()?;

    let mut changed = 0;
    common::params::for_each_row_mut::<ItemLotParam_enemy>(repo, |index, row| {
        let Some(original) = snapshot.get(index) else {
            return;
        };
        lot::set_base_points(row, scale_row(original, &lot::item_ids(row), mode));
        changed += 1;
    });
    Some(changed)
}

fn log_mode(mode: &Mode, changed: usize, suffix: &str) {
    let setting = match mode {
        Mode::Multiplier(factor) => format!("Multiplier={factor:.3}"),
        Mode::FixedPercent(percent) => format!("Percentage={percent:.3}"),
    };
    logger::log(&format!("Drops: {setting} -> {changed} row(s){suffix}"));
}

/// Applies the config once the world is loaded, then follows
/// `RELOAD_GENERATION` for the rest of the DLL's life (`common::reload` owns
/// the reload key). Run it on its own worker thread; never returns.
pub fn run() {
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    let repo = common::player::wait_for_solo_param_repository();
    let mode = build_mode();
    if let Some(changed) = apply(repo, &mode) {
        log_mode(&mode, changed, "");
    }
    crate::materials::apply(repo, "");

    let mut waiting_logged = false;
    let cs_task = common::task::wait_for_cs_task();
    common::task::run_recurring_safe(
        cs_task,
        "DropMultiplier",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation == last_seen_generation {
                return;
            }
            // Not in the world (a change made from the title screen's menu):
            // keep the request and apply it once the world is loaded.
            if common::player::main_player_chr_ins_ptr().is_none() {
                if !waiting_logged {
                    logger::warn("Reload: deferred, not in world");
                    waiting_logged = true;
                }
                return;
            }
            waiting_logged = false;
            last_seen_generation = generation;
            crate::menu::sync_from_config();
            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                logger::warn("Reload: SoloParamRepository unavailable, skipped");
                return;
            };
            let mode = build_mode();
            if let Some(changed) = apply(repo, &mode) {
                log_mode(&mode, changed, " (reload)");
            }
            crate::materials::apply(repo, " (reload)");
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slot 0 holds a real item, slot 1 is the "nothing" slot.
    const IDS: ItemIds = [1, 0, 0, 0, 0, 0, 0, 0];

    fn points(a: u16, b: u16) -> Points {
        [a, b, 0, 0, 0, 0, 0, 0]
    }

    #[test]
    fn multiplier_scales_real_items_and_leaves_the_nothing_slot() {
        let scaled = scale_row(&points(100, 900), &IDS, &Mode::Multiplier(3.0));
        assert_eq!(scaled, points(300, 900));
    }

    #[test]
    fn multiplier_saturates_at_u16() {
        assert_eq!(scale_row(&points(60_000, 0), &IDS, &Mode::Multiplier(10.0))[0], u16::MAX);
        assert_eq!(scale_row(&points(100, 0), &IDS, &Mode::Multiplier(0.0))[0], 0);
    }

    #[test]
    fn fixed_percent_lands_on_the_target_chance() {
        // 100 real / 900 nothing is a 10% drop; forcing 50% needs 900 real.
        let scaled = scale_row(&points(100, 900), &IDS, &Mode::FixedPercent(50.0));
        assert_eq!(scaled, points(900, 900));
        let chance = scaled[0] as f64 / (scaled[0] + scaled[1]) as f64;
        assert!((chance - 0.5).abs() < 1e-9);
    }

    #[test]
    fn fixed_percent_keeps_each_items_share_of_the_row() {
        let ids: ItemIds = [1, 2, 0, 0, 0, 0, 0, 0];
        let scaled = scale_row(&[100, 300, 600, 0, 0, 0, 0, 0], &ids, &Mode::FixedPercent(50.0));
        // Real items' combined weight equals the nothing slot's (50%), 1:3 as before.
        assert_eq!(scaled[0] + scaled[1], 600);
        assert_eq!((scaled[0], scaled[1]), (150, 450));
    }

    #[test]
    fn fixed_hundred_percent_zeroes_the_nothing_slots() {
        let scaled = scale_row(&points(100, 900), &IDS, &Mode::FixedPercent(100.0));
        assert_eq!(scaled, points(100, 0));
        assert_eq!(scale_row(&points(100, 900), &IDS, &Mode::FixedPercent(250.0)), points(100, 0));
    }

    #[test]
    fn fixed_percent_edges_change_nothing() {
        // A row without a real item, or without a "nothing" slot (already
        // drops every time), is left as it is below 100%.
        assert_eq!(scale_row(&points(0, 900), &IDS, &Mode::FixedPercent(50.0)), points(0, 900));
        assert_eq!(scale_row(&points(100, 0), &IDS, &Mode::FixedPercent(50.0)), points(100, 0));
    }

    #[test]
    fn rescaling_from_the_original_never_compounds() {
        let original = points(100, 900);
        let once = scale_row(&original, &IDS, &Mode::Multiplier(2.0));
        let again = scale_row(&original, &IDS, &Mode::Multiplier(2.0));
        assert_eq!(once, again);
        assert_eq!(once[0], 200);
    }
}
