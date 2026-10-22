//! The `Regen.PerTick` loop: a task on the game's own `FrameBegin` group (the
//! main player is only safe to mutate from the game's main thread). Each frame
//! it feeds [Triggers], keeps the hit hook in sync with the config, and heals
//! when the interval and the trigger allow. Heal math: `heal.rs`; trigger
//! state: `trigger.rs`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use common::{config, logger};
use eldenring::cs::CSTaskGroupIndex;

use crate::heal::{HealField, heal_main_player_tick};
use crate::hit_hook;
use crate::trigger::{self, Triggers};

/// Registers the tick and blocks the calling thread forever (the task is
/// cancelled if its handle drops). Run it on its own worker thread.
pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
    let mut triggers = Triggers::default();
    let mut elapsed_ms: f64 = 0.0;
    let mut hook_installed = false;
    let mut last_logged: Option<(i32, bool)> = None;

    common::task::run_recurring_safe(
        cs_task,
        "Regen",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            hit_hook::flush_pending_logs();

            let now = trigger::now_ms();
            if let Some(input) = common::player::main_player_action_snapshot() {
                triggers.update(now, &input, trigger::main_player_anim_id(), |id| {
                    trigger::gesture_trigger_ids().contains(&id)
                });
            }

            // `common::reload` owns the reload key; just refresh the menu rows.
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation != last_seen_generation {
                last_seen_generation = generation;
                crate::menu::sync_from_config();
            }

            let per_tick_enabled = config::get_bool("Regen.PerTick.Enabled", true);
            let trigger_kind = config::get_int("Regen.PerTick.Trigger", 0);

            // The hit hook also tracks combat activity, so it is wanted for
            // the combat triggers even when Regen.PerHit itself is off. It is
            // installed once in game (so other mods scanning the same code
            // finish first) and re-synced every frame for hot reload.
            let hit_params = hit_hook::OnHitParams::from_config();
            let needs_combat_tracking = per_tick_enabled && matches!(trigger_kind, 1 | 2);
            let hook_wanted = hit_params.wants_heal() || needs_combat_tracking;
            if hook_wanted && !hook_installed && common::player::main_player_chr_ins_ptr().is_some() {
                hook_installed = hit_hook::try_install(hit_params);
            } else if hook_installed {
                hit_hook::update_params(hit_params);
            }

            let interval_ms = config::get_int("Regen.PerTick.Interval", 1000).max(0) as f64;
            if !per_tick_enabled || interval_ms <= 0.0 {
                return;
            }
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < interval_ms {
                return;
            }
            elapsed_ms = 0.0;

            let met = triggers.condition_met(trigger_kind, now);
            // Log only when the outcome changes, not every time idle/combat
            // flip on their own (that flooded the log over a long session).
            if trigger_kind != 0 && last_logged != Some((trigger_kind, met)) {
                last_logged = Some((trigger_kind, met));
                logger::log(&format!(
                    "Tick: trigger={trigger_kind} met={met} (combat={}, idle={}, gesture={})",
                    trigger::is_in_combat(),
                    triggers.is_idle(now),
                    triggers.is_gesture_active()
                ));
            }
            if !met {
                return;
            }

            // ValueType: 0 flat points, 1 % of max, 2 % of the missing amount.
            // Cap (percent of max) limits how far a tick restores any stat.
            let value_type = config::get_int("Regen.PerTick.ValueType", 0);
            let cap_pct = config::get_double("Regen.PerTick.Cap", 100.0);
            for (field, key) in [
                (HealField::Hp, "Regen.PerTick.HP"),
                (HealField::Fp, "Regen.PerTick.FP"),
                (HealField::Stamina, "Regen.PerTick.SP"),
            ] {
                heal_main_player_tick(field, value_type, config::get_double(key, 0.0), cap_pct);
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
