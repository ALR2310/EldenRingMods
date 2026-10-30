//! Ties the pieces together: installs the hook, applies the config (which
//! groups are on, their `Range`) and follows reloads, and every frame tells
//! the hook whether the player is in combat. `common::reload` owns the reload
//! key; this only follows `RELOAD_GENERATION`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use common::logger;

use crate::config::{self, Config};
use crate::groups::Group;
use crate::{animation, combat, hook, probe, range};

fn group_config(config: &Config, group: Group) -> config::Group {
    match group {
        Group::Materials => config.materials,
        Group::Items => config.items,
        Group::CorpseLoot => config.corpse_loot,
        Group::LostRunes => config.lost_runes,
        Group::Chests => config.chests,
    }
}

/// Bit mask of the groups switched on.
fn enabled_mask(config: &Config) -> u8 {
    Group::ALL.into_iter().filter(|&group| group_config(config, group).enabled).fold(0, |mask, group| mask | group.bit())
}

/// The scale to write per group: the configured `Range` while the group is on,
/// 1.0 (the game's own distance) while it is off.
fn ranges(config: &Config) -> Vec<(Group, f32)> {
    Group::ALL
        .into_iter()
        .map(|group| {
            let settings = group_config(config, group);
            (group, if settings.enabled { range::clamp_range(settings.range) } else { 1.0 })
        })
        .collect()
}

fn apply_ranges(repo: &mut SoloParamRepository, config: &Config, suffix: &str) {
    match range::apply(repo, &ranges(config)) {
        Ok(rows) => logger::log(&format!("Range: set on {rows} row(s){suffix}")),
        Err(reason) => logger::warn(&format!("Range: failed ({reason})")),
    }
}

fn apply_animation(repo: &mut SoloParamRepository, config: &Config) {
    if let Err(reason) = animation::apply(repo, config.general.no_pickup_animation) {
        logger::warn(&format!("Animation: failed ({reason})"));
    } else if config.logging.pickup_probe
        && let Some(ids) = animation::current(repo)
    {
        logger::log(&format!("Probe: animation {ids}"));
    }
}

/// Never returns: run it on its own worker thread.
pub fn run() {
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    let config = config::get();
    hook::set_enabled(enabled_mask(&config));
    hook::install();

    let repo = common::player::wait_for_solo_param_repository();
    apply_ranges(repo, &config, "");
    apply_animation(repo, &config);

    let mut waiting_logged = false;
    let mut ranges_pending = false;
    let mut probe_drainer = probe::Drainer::default();
    let cs_task = common::task::wait_for_cs_task();
    common::task::run_recurring_safe(
        cs_task,
        "AutoPickup",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let config = config::get();

            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation != last_seen_generation {
                last_seen_generation = generation;
                hook::set_enabled(enabled_mask(&config));
                ranges_pending = true;
                crate::menu::sync_from_config();
            }
            // Not in the world (a change made from the title screen's menu):
            // keep the request and apply it once the world is loaded.
            if ranges_pending {
                if common::player::main_player_chr_ins_ptr().is_none() {
                    if !waiting_logged {
                        logger::warn("Reload: deferred, not in world");
                        waiting_logged = true;
                    }
                } else if let Ok(repo) = unsafe { SoloParamRepository::instance_mut() } {
                    waiting_logged = false;
                    ranges_pending = false;
                    apply_ranges(repo, &config, " (reload)");
                    apply_animation(repo, &config);
                }
            }

            hook::set_paused(config.general.pause_in_combat && combat::in_combat().unwrap_or(false));
            probe::set_enabled(config.logging.pickup_probe);
            probe_drainer.tick();
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
    fn the_template_switches_every_group_on_but_chests() {
        assert_eq!(enabled_mask(&config::template()), 0b01111);
    }

    #[test]
    fn a_disabled_group_gets_the_vanilla_range() {
        let mut config = config::template();
        config.items.enabled = false;
        config.items.range = 3.0;
        config.materials.range = 2.0;
        let ranges = ranges(&config);
        assert_eq!(ranges.iter().find(|(group, _)| *group == Group::Items).unwrap().1, 1.0);
        assert_eq!(ranges.iter().find(|(group, _)| *group == Group::Materials).unwrap().1, 2.0);
        assert_eq!(enabled_mask(&config) & Group::Items.bit(), 0);
    }
}
