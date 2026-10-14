//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the TOML
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Speed Multiplier           -> opens a page with:
//!   All Player Actions       number 0.1..10  (overrides the rows below)
//!   Player Movement          -> Walk, Run, Sneak, Jump, Roll, Ladder
//!   Player Actions           -> Attack, Critical, Skill, Cast, Item, Other
//!   Torrent                  -> All, Walk, Run, Jump, Other
//!   Spirit & Enemy           -> Spirit, Enemy
//!   Log file                 on/off
//! ```
//!
//! Every speed row is a text row where the multiplier is typed (`1.5` or
//! `1,5`); the bounds 0.1..10 are the ones the mod clamps to when it applies a
//! speed. Typed numbers are saved as TOML floats (`2` becomes `2.0`).
//! Changes are written to `SpeedMultiplier.toml` (comments kept) once the
//! player stops for a second; the mod reads its config snapshot every frame,
//! so they apply at once. `[[Override]]` blocks and the probes stay file-only.

use std::sync::OnceLock;
use std::time::Duration;

use common::erui::{self, NumberSetting, PageHandle, Registration, RowHandle, Setting};
use common::logger;

use crate::config;

/// The bounds `speed.rs` clamps every multiplier to.
const MIN_SPEED: f64 = 0.1;
const MAX_SPEED: f64 = 10.0;

fn on_off(value: u8) -> String {
    (value != 0).to_string()
}

macro_rules! speed {
    ($name:ident, $key:expr) => {
        static $name: NumberSetting = NumberSetting::float($key, MIN_SPEED, MAX_SPEED, None);
    };
}

speed!(PLAYER_ALL, "Player.All");
speed!(PLAYER_WALK, "Player.Walk");
speed!(PLAYER_RUN, "Player.Run");
speed!(PLAYER_SNEAK, "Player.Sneak");
speed!(PLAYER_JUMP, "Player.Jump");
speed!(PLAYER_ROLL, "Player.Roll");
speed!(PLAYER_LADDER, "Player.Ladder");
speed!(PLAYER_ATTACK, "Player.Attack");
speed!(PLAYER_CRITICAL, "Player.Critical");
speed!(PLAYER_SKILL, "Player.Skill");
speed!(PLAYER_CAST, "Player.Cast");
speed!(PLAYER_ITEM, "Player.Item");
speed!(PLAYER_OTHER, "Player.Other");
speed!(TORRENT_ALL, "Torrent.All");
speed!(TORRENT_WALK, "Torrent.Walk");
speed!(TORRENT_RUN, "Torrent.Run");
speed!(TORRENT_JUMP, "Torrent.Jump");
speed!(TORRENT_OTHER, "Torrent.Other");
speed!(SPIRIT_ALL, "Spirit.All");
speed!(ENEMY_ALL, "Enemy.All");
static LOG_FILE: Setting = Setting { key: "Logging.LogFile", format: on_off, apply: None };

fn log_file_row() -> u8 {
    config::get().logging.log_file as u8
}

struct Rows {
    registration: Registration,
    /// Each toggle row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
    /// The typed speed rows, with the function that reads each from the config.
    numbers: Vec<(&'static NumberSetting, fn() -> f64)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// After the file is written (and re-read): the mod reads its config every
/// frame, so only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

type SpeedRow = (&'static str, &'static str, &'static NumberSetting, fn() -> f64);

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("speed-multiplier", "Speed Multiplier", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Speed Multiplier",
        "Settings for the Speed Multiplier mod.",
        "Speed Multiplier",
        "Settings for the Speed Multiplier mod.",
    )?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();
    let mut numbers: Vec<(&'static NumberSetting, fn() -> f64)> = Vec::new();

    let mut add_speeds = |on_page: PageHandle, speeds: &[SpeedRow]| -> Result<(), String> {
        for (label, help, setting, read) in speeds {
            provider.add_number(on_page, label, help, setting, read())?;
            numbers.push((setting, *read));
        }
        Ok(())
    };

    add_speeds(
        page,
        &[(
            "All Player Actions",
            "Speed of everything you do. Anything but 1 overrides the individual player speeds.",
            &PLAYER_ALL,
            || f64::from(config::get().player.all),
        )],
    )?;

    let (movement, _entry) = provider.add_submenu(
        page,
        "Player Movement",
        "How fast you move.",
        "Player Movement",
        "Ignored while All Player Actions is not 1.",
    )?;
    add_speeds(
        movement,
        &[
            ("Walk", "Walking speed.", &PLAYER_WALK, || f64::from(config::get().player.walk)),
            ("Run", "Running speed.", &PLAYER_RUN, || f64::from(config::get().player.run)),
            ("Sneak", "Sneaking speed.", &PLAYER_SNEAK, || f64::from(config::get().player.sneak)),
            ("Jump", "Jumping and landing speed.", &PLAYER_JUMP, || f64::from(config::get().player.jump)),
            ("Roll", "Rolling and backstepping speed.", &PLAYER_ROLL, || f64::from(config::get().player.roll)),
            ("Ladder", "Climbing and sliding speed.", &PLAYER_LADDER, || f64::from(config::get().player.ladder)),
        ],
    )?;

    let (actions, _entry) = provider.add_submenu(
        page,
        "Player Actions",
        "How fast you attack, cast and use items.",
        "Player Actions",
        "Ignored while All Player Actions is not 1.",
    )?;
    add_speeds(
        actions,
        &[
            ("Attack", "Speed of normal attacks.", &PLAYER_ATTACK, || f64::from(config::get().player.attack)),
            ("Critical", "Speed of critical hits.", &PLAYER_CRITICAL, || f64::from(config::get().player.critical)),
            ("Skill", "Speed of weapon skills.", &PLAYER_SKILL, || f64::from(config::get().player.skill)),
            ("Cast", "Speed of casting sorceries and incantations.", &PLAYER_CAST, || f64::from(config::get().player.cast)),
            ("Item", "Speed of using items such as flasks.", &PLAYER_ITEM, || f64::from(config::get().player.item)),
            ("Other", "Speed of any action not listed here.", &PLAYER_OTHER, || f64::from(config::get().player.other)),
        ],
    )?;

    let (torrent, _entry) = provider.add_submenu(
        page,
        "Torrent",
        "How fast Torrent moves.",
        "Torrent",
        "All overrides the individual speeds when it is not 1.",
    )?;
    add_speeds(
        torrent,
        &[
            ("All", "Speed of everything Torrent does. Anything but 1 overrides the rows below.", &TORRENT_ALL, || {
                f64::from(config::get().torrent.all)
            }),
            ("Walk", "Torrent's walking speed.", &TORRENT_WALK, || f64::from(config::get().torrent.walk)),
            ("Run", "Torrent's running speed.", &TORRENT_RUN, || f64::from(config::get().torrent.run)),
            ("Jump", "Torrent's jumping and landing speed.", &TORRENT_JUMP, || f64::from(config::get().torrent.jump)),
            ("Other", "Speed of any other Torrent action.", &TORRENT_OTHER, || f64::from(config::get().torrent.other)),
        ],
    )?;

    let (others, _entry) = provider.add_submenu(
        page,
        "Spirit & Enemy",
        "How fast summoned spirits and enemies act.",
        "Spirit & Enemy",
        "One speed for everything they do.",
    )?;
    add_speeds(
        others,
        &[
            ("Spirit", "Speed of summoned spirit ashes.", &SPIRIT_ALL, || f64::from(config::get().spirit.all)),
            ("Enemy", "Speed of hostile enemies and bosses.", &ENEMY_ALL, || f64::from(config::get().enemy.all)),
        ],
    )?;

    rows.push((
        provider.add_toggle(
            page,
            "Log file",
            "Write a log file next to the DLL, for troubleshooting.",
            log_file_row() != 0,
            erui::setting_changed,
            LOG_FILE.user_data(),
        )?,
        log_file_row,
    ));
    let registration = provider.commit()?;
    Ok(Rows { registration, rows, numbers })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(|changes| config::save(changes), Some(saved));
    std::thread::spawn(move || {
        let connection = match erui::connect(Duration::from_secs(10)) {
            Ok(c) => c,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the config file only."));
                return;
            }
        };
        match build(hmodule, connection) {
            Ok(rows) => {
                let _ = ROWS.set(rows);
                logger::log("In-game menu: registered.");
            }
            Err(reason) => logger::log(&format!("In-game menu registration failed: {reason}")),
        }
    });
}

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(rows) = ROWS.get() {
        for (row, read) in &rows.rows {
            if let Err(reason) = rows.registration.set_row_value(*row, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
        for (setting, read) in &rows.numbers {
            if let Err(reason) = setting.show(&rows.registration, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_rows_are_float_rows() {
        // A TOML float field: a typed `2` must be saved as `2.0`.
        for setting in [&PLAYER_ALL, &PLAYER_ROLL, &TORRENT_OTHER, &SPIRIT_ALL, &ENEMY_ALL] {
            assert!(setting.decimal_point, "{}", setting.key);
        }
    }

    #[test]
    fn bounds_match_the_speed_clamp() {
        assert_eq!((PLAYER_ALL.min, PLAYER_ALL.max), (MIN_SPEED, MAX_SPEED));
    }
}
