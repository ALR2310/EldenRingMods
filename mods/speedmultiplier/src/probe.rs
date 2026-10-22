//! Debug-only probes, all off by default and read every frame, so a reload
//! turns them on or off. They only write to the log, to find which animation
//! ids belong to which group (`docs/action_groups.md`) and how to tell enemies
//! apart:
//! - `SpeedProbe`: the player's and Torrent's animation id, group and the
//!   speed set (on change); `weight_type` and `max_equip_load` (the equip-load
//!   class for a `Load` condition in `[[Override]]`); every character's
//!   `throw_state` change; the live spirits.
//! - `EffectProbe`: the player's SpEffect ids as they are added and removed
//!   (for testing `[[Override]]`).
//! - `EnemyProbe`: the character nearest the player's lock-on target, as
//!   ground truth for enemies vs bosses vs NPCs.

use std::collections::HashMap;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::anim::{current_anim_id, format_anim_id, group_of};
use common::logger;

use crate::config;
use crate::speed::{throw_state, torrent, torrent_group_of};

/// `(added, removed)` going from the sorted list `old` to the sorted list `new`.
fn diff_sorted(old: &[i32], new: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let added = new.iter().filter(|id| !old.contains(id)).copied().collect();
    let removed = old.iter().filter(|id| !new.contains(id)).copied().collect();
    (added, removed)
}

/// What each probe saw last frame, so a line is logged only on change.
#[derive(Default)]
struct Probes {
    player_anim: Option<i32>,
    torrent_anim: Option<i32>,
    weight: Option<(u32, f32)>,
    sp_effects: Vec<i32>,
    buddies: Vec<i32>,
    throws: HashMap<usize, u32>,
    lock: usize,
}

impl Probes {
    fn effects(&mut self, chr: &ChrIns) {
        let mut now: Vec<i32> = chr.special_effect.entries().map(|e| e.param_id).collect();
        now.sort_unstable();
        now.dedup();
        if now != self.sp_effects {
            let (added, removed) = diff_sorted(&self.sp_effects, &now);
            logger::log(&format!("P SpEffect +{added:?} -{removed:?} now {now:?}"));
            self.sp_effects = now;
        }
    }

    fn weight(&mut self, chr: &ChrIns, max_equip_load: f32) {
        let now = (chr.chr_ctrl.weight_type, max_equip_load);
        if self.weight != Some(now) {
            self.weight = Some(now);
            logger::log(&format!("P weight_type={} max_equip_load={:.1}", now.0, now.1));
        }
    }

    fn player_anim(&mut self, chr: &ChrIns) {
        let anim = current_anim_id(chr);
        if self.player_anim != Some(anim) {
            self.player_anim = Some(anim);
            logger::log(&format!(
                "P {} ({anim}) {:?} speed={:.2} fp={}/{}",
                format_anim_id(anim),
                group_of(anim),
                chr.modules.behavior.animation_speed,
                chr.modules.data.fp,
                chr.modules.data.max_fp,
            ));
        }
    }

    fn throw(&mut self, who: &str, chr: &ChrIns) {
        let Some(state) = throw_state(chr) else {
            return;
        };
        let key = chr as *const ChrIns as usize;
        // First sight of a character: remember it, only log when it isn't idle.
        let before = self.throws.insert(key, state).unwrap_or(0);
        if state == before {
            return;
        }
        logger::log(&format!(
            "THROW {who} {key:#x} npc={} state {before}->{state} flags={:#x} {} speed={:.2}",
            chr.npc_param_id,
            chr.modules.throw.flags.0,
            format_anim_id(current_anim_id(chr)),
            chr.modules.behavior.animation_speed,
        ));
    }

    fn buddies(&mut self, world_chr_man: &WorldChrMan) {
        let mut now = Vec::new();
        common::player::for_each_active_chr(&world_chr_man.summon_buddy_chr_set, |chr| now.push(chr.npc_param_id));
        if now != self.buddies {
            logger::log(&format!("BUDDY npc {now:?}"));
            self.buddies = now;
        }
    }

    fn torrent(&mut self, chr: &ChrIns) {
        let anim = current_anim_id(chr);
        if self.torrent_anim != Some(anim) {
            self.torrent_anim = Some(anim);
            logger::log(&format!(
                "T {} ({anim}) {:?} speed={:.2}",
                format_anim_id(anim),
                torrent_group_of(anim),
                chr.modules.behavior.animation_speed,
            ));
        }
    }

    /// The target of a lock-on isn't flagged itself (only the player has
    /// `is_locked_on`), so it is the character whose position is nearest the
    /// player's `lock_on_target_position`.
    fn lock_on(&mut self, world_chr_man: &WorldChrMan) {
        let target = world_chr_man.main_player.as_ref().and_then(|player| {
            let chr = &player.chr_ins;
            chr.is_locked_on.then_some((chr as *const ChrIns as usize, chr.lock_on_target_position))
        });
        let mut best: Option<(f32, usize, usize)> = None;
        if let Some((player_ptr, tp)) = target {
            for (index, set) in world_chr_man.chr_sets.iter().enumerate() {
                let Some(set) = set else { continue };
                for chr in set.characters() {
                    let key = chr as *const ChrIns as usize;
                    if key == player_ptr || index == 111 {
                        continue;
                    }
                    let p = &chr.chunk_position;
                    let d = (p.0 - tp.0).powi(2) + (p.1 - tp.1).powi(2) + (p.2 - tp.2).powi(2);
                    if best.is_none_or(|(best_d, _, _)| d < best_d) {
                        best = Some((d, key, index));
                    }
                }
            }
        }
        match (best, target) {
            (Some((d, key, index)), Some((_, tp))) if self.lock != key => {
                self.lock = key;
                let chr = unsafe { &*(key as *const ChrIns) };
                logger::log(&format!(
                    "LOCKON set={index} dist={:.2} npc_param={} npc_id={} chr_type={:?} team={} hp={}/{} anim={} chr_pos=({:.1},{:.1},{:.1}) target_pos=({:.1},{:.1},{:.1})",
                    d.sqrt(),
                    chr.npc_param_id,
                    chr.npc_id,
                    chr.chr_type,
                    chr.team_type,
                    chr.modules.data.hp,
                    chr.modules.data.max_hp,
                    format_anim_id(current_anim_id(chr)),
                    chr.chunk_position.0, chr.chunk_position.1, chr.chunk_position.2,
                    tp.0, tp.1, tp.2,
                ));
            }
            (None, _) => self.lock = 0,
            _ => {}
        }
    }
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    let mut probes = Probes::default();
    // PostPhysics: after `speed.rs` (PreBehavior) has set this frame's value.
    common::task::run_recurring_safe(
        cs_task,
        "Probe",
        CSTaskGroupIndex::ChrIns_PostPhysics,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let (speed_probe, effect_probe, enemy_probe) = {
                let logging = &config::get().logging;
                (logging.speed_probe, logging.effect_probe, logging.enemy_probe)
            };
            if !(speed_probe || effect_probe || enemy_probe) || common::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
                return;
            };

            if let Some(player) = world_chr_man.main_player.as_ref() {
                if effect_probe {
                    probes.effects(&player.chr_ins);
                }
                if speed_probe {
                    probes.weight(&player.chr_ins, unsafe { player.player_game_data.as_ref() }.max_equip_load);
                    probes.player_anim(&player.chr_ins);
                    probes.throw("player", &player.chr_ins);
                }
            }
            if enemy_probe {
                probes.lock_on(world_chr_man);
            }
            if speed_probe {
                for set in world_chr_man.chr_sets.iter().flatten() {
                    for chr in set.characters() {
                        probes.throw("chr", chr);
                    }
                }
                probes.buddies(world_chr_man);
                if let Some(chr) = torrent(world_chr_man) {
                    probes.torrent(chr);
                }
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_changes_are_reported_as_added_and_removed() {
        assert_eq!(diff_sorted(&[], &[5, 7]), (vec![5, 7], vec![]));
        assert_eq!(diff_sorted(&[5, 7], &[7, 9]), (vec![9], vec![5]));
        assert_eq!(diff_sorted(&[5, 7], &[5, 7]), (vec![], vec![]));
        assert_eq!(diff_sorted(&[5], &[]), (vec![], vec![5]));
    }
}
