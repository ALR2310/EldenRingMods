//! The passive-rune tick: a task on the game's own `FrameBegin` group that
//! adds runes to the main player's `rune_count` over time, plus milestone
//! bonuses. The amounts and the clocks are in `math.rs`; why this reaches the
//! player through `WorldChrMan` and not `GameDataMan` (no RVA table), and the
//! rest of the design: `docs/rune_math.md`, `docs/startup_and_game_updates.md`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, PlayerGameData, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::{config, logger};

use crate::math::{self, Clock, InterestState, MAX_HELD_RUNES};

/// Gives `f` mutable access to the main player's `PlayerGameData`. `None` (a
/// no-op) until a live player character exists: `WorldChrMan.main_player` is
/// the in-world gate, `GameDataMan` alone resolves already at the title screen.
fn with_player_game_data<R>(f: impl FnOnce(&mut PlayerGameData) -> R) -> Option<R> {
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let main_player = world_chr_man.main_player.as_mut()?;
    let data = unsafe { main_player.player_game_data.as_mut() };
    Some(f(data))
}

/// The main player's state right after a grant, for the log.
struct Granted {
    held: u32,
    level: u32,
}

/// Adds `delta` runes (negative takes runes away), kept within the game's
/// limits. `None` if the player isn't in the world yet.
fn apply_runes(delta: i64) -> Option<Granted> {
    with_player_game_data(|data| {
        data.rune_count = math::add_runes(data.rune_count, delta);
        Granted { held: data.rune_count, level: data.level }
    })
}

/// The per-interval settings, read fresh every tick so a reload is picked up
/// for free, clamped to the ranges documented in `PassiveRunes.ini`.
struct TickSettings {
    fixed: i64,
    percent: f64,
    interest: f64,
}

impl TickSettings {
    fn from_config() -> Self {
        Self {
            fixed: (config::get_int("Rune.Passive.Amount", 50) as i64)
                .clamp(-(MAX_HELD_RUNES as i64), MAX_HELD_RUNES as i64),
            percent: config::get_double("Rune.Passive.Percent", 0.25).clamp(-100.0, 100.0),
            interest: config::get_double("Rune.Passive.Interest", 0.0).clamp(-100.0, 100.0),
        }
    }
}

/// Registers the tick and blocks the calling thread forever. Run it on its
/// own worker thread; `common::reload` owns the reload key, so this only
/// re-reads its keys every tick.
pub fn run() {
    let cs_task = common::task::wait_for_cs_task();

    // Parsed once: re-parsing would mean deciding what a config change does to
    // the progress through the list. A milestone already granted stays
    // granted across a reload; only future ones see new amounts.
    let milestones = math::parse_milestones(&config::get_string("Rune.Milestone", math::DEFAULT_MILESTONES));
    let mut clock = Clock::default();
    let mut interest = InterestState::default();
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    let installed = common::task::run_recurring_safe(
        cs_task,
        "Rune",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation != last_seen_generation {
                last_seen_generation = generation;
                crate::menu::sync_from_config();
            }

            // `Enabled=false` turns off both the interval grant and the milestones.
            if !config::get_bool("Rune.Passive.Enabled", true) {
                return;
            }
            // Out of the world both clocks pause, so time at the title screen
            // or a loading screen doesn't count toward milestones.
            if common::player::main_player_chr_ins_ptr().is_none() {
                return;
            }

            let dt_ms = (data.delta_time.time as f64) * 1000.0;
            clock.advance_session(dt_ms);

            while let Some(milestone) = clock.due_milestone(&milestones) {
                if milestone.bonus != 0 {
                    // Not in the world after all: leave it due and retry next
                    // tick instead of skipping the bonus forever.
                    let Some(granted) = apply_runes(milestone.bonus) else {
                        break;
                    };
                    logger::log(&format!(
                        "Milestone: {:+} at {}s -> held {}, level {}",
                        milestone.bonus,
                        milestone.at_ms / 1000.0,
                        granted.held,
                        granted.level
                    ));
                }
                clock.milestone_done();
            }

            if !clock.interval_due(dt_ms, config::get_int("Rune.Passive.Interval", 10000).max(0) as f64) {
                return;
            }

            // Fixed amount, % of the next level-up cost and % of the held
            // runes all stack; each is off at 0 and takes runes away when negative.
            let settings = TickSettings::from_config();
            let Some((level, held)) = with_player_game_data(|data| (data.level, data.rune_count)) else {
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
                "Tick: {amount:+} (fixed {:+}, percent {percent:+}, interest {interest_amount:+}) -> held {}, level {}",
                settings.fixed, granted.held, granted.level
            ));
        },
    );

    if !installed {
        logger::error("Rune tick: install failed, game may need a mod update, check Nexus.");
    }

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
