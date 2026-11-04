//! Debug-only (`EffectProbe`, off by default) diagnostics: logs every change of
//! the local player's SpEffect set, and once the non-zero fields of the
//! SpEffectParam rows `503045` and `2147300000`, the ones involved when a spirit cannot be
//! summoned again under Seamless Co-op (`docs/seamless_conflict.md`).

use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository, SpEffectParam, WorldChrMan};
use fromsoftware_shared::FromStatic;

use shared::logger;

use crate::config;

/// SpEffects of the Seamless re-summon investigation: `503045` is the row the
/// old "infinite summoning" fix rewrote inside Seamless, `2147300000` the one
/// that makes re-summoning possible when applied.
const WATCHED_ROWS: [u32; 2] = [503045, 2147300000];

/// Logs the non-zero 4-byte fields of the SpEffectParam rows in
/// [WATCHED_ROWS] (or that they do not exist).
fn log_watched_rows(repo: &SoloParamRepository) {
    let Some(ids) = shared::params::row_ids::<SpEffectParam>(repo) else {
        logger::log("SpEffectParam: row ids unreadable.");
        return;
    };
    for id in WATCHED_ROWS {
        let Some(index) = ids.iter().position(|&row_id| row_id == id) else {
            logger::log(&format!("SpEffectParam {id}: no such row."));
            continue;
        };
        let Some(row) = repo.get_row_by_index::<SpEffectParam>(index) else {
            continue;
        };
        // SAFETY: the row is a whole param row of the live table.
        let bytes = unsafe {
            std::slice::from_raw_parts(row as *const _ as *const u8, std::mem::size_of_val(row))
        };
        let fields: Vec<String> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let v = u32::from_le_bytes(*c);
                (v != 0).then(|| format!("+{:#x}={:#x}", i * 4, v))
            })
            .collect();
        logger::log(&format!(
            "SpEffectParam {id}: {} non-zero dword(s): {}",
            fields.len(),
            fields.join(" ")
        ));
    }
}

pub fn run() {
    if !config::get().logging.effect_probe {
        logger::log("EffectProbe=false - probe disabled.");
        return;
    }

    let cs_task = shared::task::wait_for_cs_task();

    let mut last_effects: Vec<i32> = Vec::new();
    let mut logged_rows = false;

    let _handle = shared::task::run_recurring_safe(
        cs_task,
        "EffectProbe",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            // Every frame, so a short-lived effect is not missed.
            if shared::player::main_player_chr_ins_ptr().is_some() {
                // SAFETY: the accessor returns `Err` while the singleton does not exist.
                if let Ok(world_chr_man) = unsafe { WorldChrMan::instance() }
                    && let Some(player) = world_chr_man.main_player.as_ref()
                {
                    let mut effects: Vec<i32> =
                        player.chr_ins.special_effect.entries().map(|e| e.param_id).collect();
                    effects.sort_unstable();
                    effects.dedup();
                    if effects != last_effects {
                        let added: Vec<i32> = effects
                            .iter()
                            .filter(|id| !last_effects.contains(id))
                            .copied()
                            .collect();
                        let removed: Vec<i32> = last_effects
                            .iter()
                            .filter(|id| !effects.contains(id))
                            .copied()
                            .collect();
                        logger::log(&format!("Player SpEffects: +{added:?} -{removed:?}"));
                        last_effects = effects;
                    }
                }
                // SAFETY: the accessor returns `Err` while the singleton does not exist.
                if !logged_rows && let Ok(repo) = unsafe { SoloParamRepository::instance() } {
                    logged_rows = true;
                    log_watched_rows(repo);
                }
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
