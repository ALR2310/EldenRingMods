//! The passive-rune tick: a task on the game's `FrameBegin` group that adds runes to the main
//! player's `rune_count` over time, plus milestone bonuses. The amounts and the clocks are in
//! `math.rs`; the design: `docs/rune_math.md`.

use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, PlayerGameData, WorldChrMan};
use fromsoftware_shared::FromStatic;

use shared::logger;

use crate::config;
use crate::math::{self, Clock, InterestState, MAX_HELD_RUNES};

/// Gives `f` mutable access to the main player's `PlayerGameData`. `None`, a no-op, until a live
/// player character exists: `WorldChrMan.main_player` is the in-world gate, while `GameDataMan`
/// alone resolves already at the title screen.
fn with_player_game_data<R>(f: impl FnOnce(&mut PlayerGameData) -> R) -> Option<R> {
    // SAFETY: called from a game task; `instance_mut` returns `Err` while the singleton does not exist.
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let main_player = world_chr_man.main_player.as_mut()?;
    // SAFETY: the game data of the live main player.
    let data = unsafe { main_player.player_game_data.as_mut() };
    Some(f(data))
}

/// The main player's state right after a grant, for the log.
struct Granted {
    held: u32,
    level: u32,
}

/// Adds `delta` runes (negative takes runes away), kept within the game's limits. `None` if the
/// player is not in the world yet.
fn apply_runes(delta: i64) -> Option<Granted> {
    with_player_game_data(|data| {
        data.rune_count = math::add_runes(data.rune_count, delta);
        Granted { held: data.rune_count, level: data.level }
    })
}

/// The per-interval settings, read on every tick so a reload is picked up for free, clamped to the
/// ranges the template documents.
struct TickSettings {
    fixed: i64,
    percent: f64,
    interest: f64,
}

impl TickSettings {
    fn from_config() -> Self {
        let passive = &config::get().passive;
        Self {
            fixed: passive.amount.clamp(-i64::from(MAX_HELD_RUNES), i64::from(MAX_HELD_RUNES)),
            percent: passive.percent.clamp(-100.0, 100.0),
            interest: passive.interest.clamp(-100.0, 100.0),
        }
    }
}

/// Registers the tick and blocks the calling thread. Every setting is read again on each tick, so
/// a reload needs nothing else.
pub fn run() {
    let cs_task = shared::task::wait_for_cs_task();

    // Parsed once: re-parsing would mean deciding what a change does to the progress through the
    // list. A milestone already granted stays granted across a reload; only future ones see new
    // amounts.
    let milestones = math::parse_milestones(&config::get().passive.milestones);
    let mut clock = Clock::default();
    let mut interest = InterestState::default();

    let installed = shared::task::run_recurring_safe(
        cs_task,
        "Rune",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            let passive = config::get().passive.clone();

            // `Enabled = false` turns off both the interval grant and the milestones.
            if !passive.enabled {
                return;
            }
            // Out of the world both clocks pause, so time at the title screen or a loading screen
            // does not count toward milestones.
            if shared::player::main_player_chr_ins_ptr().is_none() {
                return;
            }

            let dt_ms = (data.delta_time.time as f64) * 1000.0;
            clock.advance_session(dt_ms);

            while let Some(milestone) = clock.due_milestone(&milestones) {
                if milestone.bonus != 0 {
                    // Not in the world after all: leave it due and retry next tick instead of
                    // skipping the bonus forever.
                    let Some(granted) = apply_runes(milestone.bonus) else {
                        break;
                    };
                    logger::log(&format!(
                        "Milestone: {:+} at {}s, held {}, level {}",
                        milestone.bonus,
                        milestone.at_ms / 1000.0,
                        granted.held,
                        granted.level
                    ));
                }
                clock.milestone_done();
            }

            if !clock.interval_due(dt_ms, passive.interval.max(0) as f64) {
                return;
            }

            // The fixed amount, the percent of the next level-up cost and the percent of the
            // held runes all stack; each is off at 0 and takes runes away when negative.
            let settings = TickSettings::from_config();
            let Some((level, held)) = with_player_game_data(|data| (data.level, data.rune_count))
            else {
                return;
            };
            let percent = math::percent_amount(math::level_up_cost(level), settings.percent);
            let interest_amount = interest.amount(held, settings.interest);
            let amount = settings.fixed + percent + interest_amount;
            if amount == 0 {
                return;
            }
            let Some(granted) = apply_runes(amount) else {
                return;
            };
            interest.last_held = Some(granted.held);
            logger::log(&format!(
                "Tick: {amount:+} (fixed {:+}, percent {percent:+}, interest {interest_amount:+}), held {}, level {}",
                settings.fixed, granted.held, granted.level
            ));
        },
    );

    if !installed {
        logger::error("Rune: tick not registered, the game may need a mod update");
    }

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
