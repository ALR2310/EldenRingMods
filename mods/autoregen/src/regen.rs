//! The `[PerTick]` loop: a task on the game's `FrameBegin` group, the only place the main player is
//! safe to change. Each frame it feeds [Triggers], keeps the hit hook in step with the config and
//! heals when the interval and the trigger allow. Heal math: `heal.rs`; trigger state: `trigger.rs`.

use std::time::Duration;

use eldenring::cs::CSTaskGroupIndex;
use shared::logger;

use crate::config;
use crate::heal::{HealField, heal_main_player_tick};
use crate::hit_hook;
use crate::trigger::{self, Triggers};

/// Registers the tick and blocks the calling thread (the task is cancelled if its handle drops).
pub fn run() {
    let cs_task = shared::task::wait_for_cs_task();
    let mut triggers = Triggers::default();
    let mut elapsed_ms: f64 = 0.0;
    let mut hook_installed = false;
    let mut last_logged: Option<(i32, bool)> = None;

    shared::task::run_recurring_safe(
        cs_task,
        "Regen",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            if let Some((hp, fp, sp)) = hit_hook::take_restored() {
                logger::log(&format!("Hit: +{hp} HP, +{fp} FP, +{sp} SP"));
            }

            let config = config::get();
            let (per_tick, per_hit) = (&config.per_tick, &config.per_hit);

            let now = trigger::now_ms();
            if let Some(input) = shared::player::main_player_action_snapshot() {
                triggers.update(now, &input, trigger::main_player_anim_id(), |id| {
                    per_tick.gesture_id.contains(&id)
                });
            }

            // The hook also tracks combat for the combat triggers, so it is wanted even with
            // `PerHit` off. It is installed once in game, so mods scanning the same code finish
            // first, and re-synced every frame for hot reload.
            let hit_params = hit_hook::OnHitParams::from_config(per_hit);
            let needs_combat_tracking = per_tick.enabled && matches!(per_tick.trigger, 1 | 2);
            let hook_wanted = hit_params.wants_heal() || needs_combat_tracking;
            if hook_wanted && !hook_installed && shared::player::main_player_chr_ins_ptr().is_some()
            {
                hook_installed = hit_hook::try_install(hit_params);
            } else if hook_installed {
                hit_hook::update_params(hit_params);
            }

            let interval_ms = per_tick.interval.max(0) as f64;
            if !per_tick.enabled || interval_ms <= 0.0 {
                return;
            }
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < interval_ms {
                return;
            }
            elapsed_ms = 0.0;

            let met = triggers.condition_met(per_tick.trigger, now);
            // Only a change of outcome is logged: idle and combat flip on their own all the time.
            if per_tick.trigger != 0 && last_logged != Some((per_tick.trigger, met)) {
                last_logged = Some((per_tick.trigger, met));
                logger::log(&format!(
                    "Tick: trigger={} met={met} (combat={}, idle={}, gesture={})",
                    per_tick.trigger,
                    trigger::is_in_combat(),
                    triggers.is_idle(now),
                    triggers.is_gesture_active()
                ));
            }
            if !met {
                return;
            }

            for (field, value) in [
                (HealField::Hp, per_tick.hp),
                (HealField::Fp, per_tick.fp),
                (HealField::Stamina, per_tick.sp),
            ] {
                heal_main_player_tick(field, per_tick.value_type, value, per_tick.cap);
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
