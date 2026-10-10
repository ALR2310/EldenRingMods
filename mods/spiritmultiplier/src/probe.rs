//! Debug-only (`SlotProbe`, off by default) diagnostics that located the
//! 10-spirit cap (see `band.rs`) - no gameplay
//! change at all, only logging.
//!
//! `sometweaks`'s `spirit::summon_count` (and `.docs/x10_summon/er10x.dll`
//! before it) can lengthen an Ash's SpEffect -> BuddyParam chain freely, but
//! the engine only ever spawns 10: per `er10x.ini`, each player gets a fixed
//! "band" of 10 slots inside `WorldChrMan.summon_buddy_chr_set`, whose start
//! and length are engine constants. Before deciding between widening that
//! band inside the ChrSet's existing capacity or reallocating the ChrSet's
//! `entries` array outright, we need two facts this module logs:
//!
//! 1. The ChrSet's real capacity (both the `capacity` field and the vtable's
//!    own `get_capacity()`, in case they disagree) - logged once, the first
//!    time the player is in the world.
//! 2. Which slot indices summoned spirits (and Torrent) actually land in -
//!    logged every time the occupied-slot layout changes, so summoning a
//!    multi-creature Ash shows where the local player's band starts.
//!
//! Also logs the SpEffect -> BuddyParam chain-length histogram once (how
//! many vanilla Ashes summon 1, 2, 3... creatures), plus a few
//! `SummonBuddyManager` fields that look slot-related (`last_buddy_slot`,
//! `active_summon_speffect_id`) alongside each layout change.
//!
//! Seamless Co-op re-summon investigation (2026-10-10): logs every change of
//! the local player's SpEffect set (a Seamless user reported spirits cannot
//! be summoned again unless SpEffect 2147300000 is applied), and once the
//! non-zero fields of the SpEffectParam rows `503045` and `2147300000`.

use std::collections::BTreeMap;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository, SpEffectParam, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::{config, logger};

const TICK_INTERVAL_MS: f64 = 500.0;

/// SpEffects of the Seamless re-summon investigation: `503045` is the row the
/// old "infinite summoning" fix rewrote inside Seamless, `2147300000` the one
/// that makes re-summoning possible when applied.
const WATCHED_ROWS: [u32; 2] = [503045, 2147300000];

/// Logs the non-zero 4-byte fields of the SpEffectParam rows in
/// [WATCHED_ROWS] (or that they do not exist).
fn log_watched_rows(repo: &SoloParamRepository) {
    let Some(ids) = common::params::row_ids::<SpEffectParam>(repo) else {
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
        let bytes = unsafe {
            std::slice::from_raw_parts(
                row as *const _ as *const u8,
                std::mem::size_of_val(row),
            )
        };
        let fields: Vec<String> = bytes
            .chunks_exact(4)
            .enumerate()
            .filter_map(|(i, c)| {
                let v = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                (v != 0).then(|| format!("+{:#x}={:#x}", i * 4, v))
            })
            .collect();
        logger::log(&format!("SpEffectParam {id}: {} non-zero dword(s): {}", fields.len(), fields.join(" ")));
    }
}

/// One occupied slot: (index, npc_param_id, load status).
type SlotInfo = (u32, i32, String);

pub fn run() {
    if !config::get_bool("SlotProbe", false) {
        logger::log("SlotProbe=false - probe disabled.");
        return;
    }

    let cs_task = common::task::wait_for_cs_task();

    let mut elapsed_ms: f64 = 0.0;
    let mut logged_static_info = false;
    let mut last_layout: Option<Vec<SlotInfo>> = None;
    let mut last_stones: Option<(u32, u32, bool, bool)> = None;
    let mut last_effects: Vec<i32> = Vec::new();
    let mut logged_rows = false;

    let _handle = common::task::run_recurring_safe(
        cs_task,
        "SlotProbe",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            // Every frame, so a short-lived effect is not missed.
            if common::player::main_player_chr_ins_ptr().is_some() {
                if let Ok(world_chr_man) = unsafe { WorldChrMan::instance() } {
                    if let Some(player) = world_chr_man.main_player.as_ref() {
                        let mut effects: Vec<i32> =
                            player.chr_ins.special_effect.entries().map(|e| e.param_id).collect();
                        effects.sort_unstable();
                        effects.dedup();
                        if effects != last_effects {
                            let added: Vec<i32> =
                                effects.iter().filter(|id| !last_effects.contains(id)).copied().collect();
                            let removed: Vec<i32> =
                                last_effects.iter().filter(|id| !effects.contains(id)).copied().collect();
                            logger::log(&format!("Player SpEffects: +{added:?} -{removed:?}"));
                            last_effects = effects;
                        }
                    }
                }
                if !logged_rows {
                    if let Ok(repo) = unsafe { SoloParamRepository::instance() } {
                        logged_rows = true;
                        log_watched_rows(repo);
                    }
                }
            }

            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < TICK_INTERVAL_MS {
                return;
            }
            elapsed_ms = 0.0;

            // Same "actually in the world" gate as sometweaks' summon_count:
            // `summon_buddy_manager`'s storage isn't populated before this.
            if common::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
                return;
            };

            let chr_set = &world_chr_man.summon_buddy_chr_set;
            let manager = &world_chr_man.summon_buddy_manager;

            if !logged_static_info {
                logged_static_info = true;
                logger::log(&format!(
                    "summon_buddy_chr_set: index={} capacity(field)={} capacity(vtable)={}",
                    chr_set.index,
                    chr_set.capacity,
                    chr_set.get_capacity(),
                ));

                let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
                for (_, head) in manager.trigger_speffect_to_buddy_map.iter_chains() {
                    *histogram.entry(head.chain_len()).or_default() += 1;
                }
                logger::log(&format!(
                    "trigger_speffect_to_buddy_map: {} chain(s), length histogram (len: count) = {:?}",
                    histogram.values().sum::<usize>(),
                    histogram,
                ));
            }

            let mut layout: Vec<SlotInfo> = Vec::new();
            for i in 0..chr_set.capacity {
                let entry = unsafe { chr_set.entries.add(i as usize).as_ref() };
                if let Some(chr_ins) = entry.chr_ins {
                    let npc_param_id = unsafe { chr_ins.as_ref() }.npc_param_id;
                    layout.push((i, npc_param_id, format!("{:?}", entry.chr_load_status)));
                }
            }

            // Buddy stone ids + range flags (MultiSpirit investigation,
            // 2026-09-29): logged, and a change in them also logs a line.
            let stones = (
                manager.buddy_stone_entity_id,
                manager.active_summmon_buddy_stone_entity_id,
                manager.is_within_activation_range,
                manager.is_within_warn_range,
            );
            if last_stones != Some(stones) {
                logger::log(&format!(
                    "stones: current={} active={} in_activation_range={} in_warn_range={}",
                    stones.0, stones.1, stones.2, stones.3
                ));
                last_stones = Some(stones);
            }

            if last_layout.as_ref() != Some(&layout) {
                let slots = layout
                    .iter()
                    .map(|(i, npc, status)| format!("#{i}(npc={npc},{status})"))
                    .collect::<Vec<_>>()
                    .join(" ");
                logger::log(&format!(
                    "slots changed: {} occupied [{}] | active_summon_speffect_id={} last_buddy_slot={} player_has_alive_summon={}",
                    layout.len(),
                    slots,
                    manager.active_summon_speffect_id,
                    manager.last_buddy_slot,
                    manager.player_has_alive_summon,
                ));
                last_layout = Some(layout);
            }
        },
    );

    logger::log("SlotProbe: tick registered on CSTaskGroupIndex::FrameBegin.");

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
