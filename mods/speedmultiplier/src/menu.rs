//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the TOML
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Speed Multiplier           -> opens a page with:
//!   All Player Actions       slider 0.1..10  (overrides the rows below)
//!   Player Movement          -> Walk, Run, Sneak, Jump, Roll, Ladder
//!   Player Actions           -> Attack, Critical, Skill, Cast, Item, Other
//!   Torrent                  -> All, Walk, Run, Jump, Other
//!   Spirit & Enemy           -> Spirit, Enemy
//!   Log file                 on/off
//! ```
//!
//! Every speed row is a native slider in steps of 0.1 (ERNativeUI 1.2 or newer;
//! older hosts get a text row where the multiplier is typed, `1.5` or `1,5`).
//! The bounds 0.1..10 are the ones the mod clamps to when it applies a speed.
//! Values are saved as TOML floats (`2` becomes `2.0`).
//! Changes are written to `SpeedMultiplier.toml` (comments kept) once the
//! player stops for a second, and the mod applies them when it re-reads the
//! file right after (about a second after the player stops): it reads its
//! config snapshot every frame. `[[Override]]` blocks and the probes stay
//! file-only.

use std::sync::OnceLock;

use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::i18n::Catalog;
use common::logger;

use crate::config;

/// The bounds `speed.rs` clamps every multiplier to.
const MIN_SPEED: f64 = 0.1;
const MAX_SPEED: f64 = 10.0;
/// Slider step: 100 positions from 0.1 to 10.
const SPEED_STEP: f64 = 0.1;

macro_rules! speed {
    ($name:ident, $key:expr) => {
        static $name: NumberSetting = NumberSetting::float($key, MIN_SPEED, MAX_SPEED, None).stepped(SPEED_STEP);
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
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

/// Vietnamese texts of the menu (`locales/vi.toml`); English is the source.
static TEXT: Catalog = Catalog::new(include_str!("../locales/vi.toml"));

/// The menu text in the host language (English without a translation).
fn t(english: &str) -> String {
    TEXT.tr(english)
}

/// After the file is written (and re-read): the mod reads its config every
/// frame, so only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Speed Multiplier", &t("Settings for the Speed Multiplier mod."))?;

    b.number(
        page,
        &t("All Player Actions"),
        &t("Speed of everything you do. Anything but 1 overrides the individual player speeds."),
        &PLAYER_ALL,
        || f64::from(config::get().player.all),
    )?;

    let movement = b.submenu(page, &t("Player Movement"), &t("How fast you move."))?;
    b.number(movement, &t("Walk"), &t("Walking speed."), &PLAYER_WALK, || f64::from(config::get().player.walk))?;
    b.number(movement, &t("Run"), &t("Running speed."), &PLAYER_RUN, || f64::from(config::get().player.run))?;
    b.number(movement, &t("Sneak"), &t("Sneaking speed."), &PLAYER_SNEAK, || f64::from(config::get().player.sneak))?;
    b.number(movement, &t("Jump"), &t("Jumping and landing speed."), &PLAYER_JUMP, || f64::from(config::get().player.jump))?;
    b.number(movement, &t("Roll"), &t("Rolling and backstepping speed."), &PLAYER_ROLL, || f64::from(config::get().player.roll))?;
    b.number(movement, &t("Ladder"), &t("Climbing and sliding speed."), &PLAYER_LADDER, || f64::from(config::get().player.ladder))?;

    let actions = b.submenu(page, &t("Player Actions"), &t("How fast you attack, cast and use items."))?;
    b.number(actions, &t("Attack"), &t("Speed of normal attacks."), &PLAYER_ATTACK, || f64::from(config::get().player.attack))?;
    b.number(actions, &t("Critical"), &t("Speed of critical hits."), &PLAYER_CRITICAL, || f64::from(config::get().player.critical))?;
    b.number(actions, &t("Skill"), &t("Speed of weapon skills."), &PLAYER_SKILL, || f64::from(config::get().player.skill))?;
    b.number(actions, &t("Cast"), &t("Speed of casting sorceries and incantations."), &PLAYER_CAST, || f64::from(config::get().player.cast))?;
    b.number(actions, &t("Item"), &t("Speed of using items such as flasks."), &PLAYER_ITEM, || f64::from(config::get().player.item))?;
    b.number(actions, &t("Other"), &t("Speed of any action not listed here."), &PLAYER_OTHER, || f64::from(config::get().player.other))?;

    let torrent = b.submenu(page, &t("Torrent"), &t("How fast Torrent moves."))?;
    b.number(
        torrent,
        &t("All"),
        &t("Speed of everything Torrent does. Anything but 1 overrides the rows below."),
        &TORRENT_ALL,
        || f64::from(config::get().torrent.all),
    )?;
    b.number(torrent, &t("Walk"), &t("Torrent's walking speed."), &TORRENT_WALK, || f64::from(config::get().torrent.walk))?;
    b.number(torrent, &t("Run"), &t("Torrent's running speed."), &TORRENT_RUN, || f64::from(config::get().torrent.run))?;
    b.number(torrent, &t("Jump"), &t("Torrent's jumping and landing speed."), &TORRENT_JUMP, || f64::from(config::get().torrent.jump))?;
    b.number(torrent, &t("Other"), &t("Speed of any other Torrent action."), &TORRENT_OTHER, || f64::from(config::get().torrent.other))?;

    let others = b.submenu(page, &t("Spirit & Enemy"), &t("How fast summoned spirits and enemies act."))?;
    b.number(others, &t("Spirit"), &t("Speed of summoned spirit ashes."), &SPIRIT_ALL, || f64::from(config::get().spirit.all))?;
    b.number(others, &t("Enemy"), &t("Speed of hostile enemies and bosses."), &ENEMY_ALL, || f64::from(config::get().enemy.all))?;

    b.toggle(page, &t("Log file"), &t("Write a log file next to the DLL, for troubleshooting."), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(config::save, Some(saved));
    erui::spawn_menu(&MENU, hmodule, "speed-multiplier", "Speed Multiplier", build);
}

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
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

    /// Every text passed to `t("...")` has a Vietnamese translation, and the
    /// catalog has nothing the menu does not use.
    #[test]
    fn translations_match_the_menu_texts() {
        let source = include_str!("menu.rs");
        let mut used: Vec<String> = Vec::new();
        for part in source.split("&t(\"").skip(1) {
            if let Some(end) = part.find("\")") {
                used.push(part[..end].to_string());
            }
        }
        used.sort();
        used.dedup();
        assert!(used.len() > 40, "found only {} menu texts", used.len());
        assert_eq!(used, TEXT.keys(), "menu.rs texts and locales/vi.toml differ");
    }
}
