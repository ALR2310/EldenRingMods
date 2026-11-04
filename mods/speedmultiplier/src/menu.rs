//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! Every speed is a slider from 0.1 to 10 in steps of 0.1 (a text row on older hosts). Changes are
//! saved to the config file a second after the player stops, then re-read. `[[Override]]` blocks
//! and the probes stay file-only.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::logger;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

/// The bounds `speed.rs` clamps every multiplier to.
const MIN_SPEED: f64 = 0.1;
const MAX_SPEED: f64 = 10.0;
const SPEED_STEP: f64 = 0.1;

macro_rules! speed {
    ($name:ident, $key:expr) => {
        static $name: NumberSetting =
            NumberSetting::new($key, MIN_SPEED, MAX_SPEED, None).stepped(SPEED_STEP);
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

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

/// The mod reads its config every frame; only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Speed Multiplier", &t("mod.help"))?;

    b.number(page, &t("all.label"), &t("all.help"), &PLAYER_ALL, || {
        f64::from(config::get().player.all)
    })?;

    let movement = b.submenu(page, &t("movement.title"), &t("movement.help"))?;
    b.number(movement, &t("walk.label"), &t("walk.help"), &PLAYER_WALK, || {
        f64::from(config::get().player.walk)
    })?;
    b.number(movement, &t("run.label"), &t("run.help"), &PLAYER_RUN, || {
        f64::from(config::get().player.run)
    })?;
    b.number(movement, &t("sneak.label"), &t("sneak.help"), &PLAYER_SNEAK, || {
        f64::from(config::get().player.sneak)
    })?;
    b.number(movement, &t("jump.label"), &t("jump.help"), &PLAYER_JUMP, || {
        f64::from(config::get().player.jump)
    })?;
    b.number(movement, &t("roll.label"), &t("roll.help"), &PLAYER_ROLL, || {
        f64::from(config::get().player.roll)
    })?;
    b.number(movement, &t("ladder.label"), &t("ladder.help"), &PLAYER_LADDER, || {
        f64::from(config::get().player.ladder)
    })?;

    let actions = b.submenu(page, &t("actions.title"), &t("actions.help"))?;
    b.number(actions, &t("attack.label"), &t("attack.help"), &PLAYER_ATTACK, || {
        f64::from(config::get().player.attack)
    })?;
    b.number(actions, &t("critical.label"), &t("critical.help"), &PLAYER_CRITICAL, || {
        f64::from(config::get().player.critical)
    })?;
    b.number(actions, &t("skill.label"), &t("skill.help"), &PLAYER_SKILL, || {
        f64::from(config::get().player.skill)
    })?;
    b.number(actions, &t("cast.label"), &t("cast.help"), &PLAYER_CAST, || {
        f64::from(config::get().player.cast)
    })?;
    b.number(actions, &t("item.label"), &t("item.help"), &PLAYER_ITEM, || {
        f64::from(config::get().player.item)
    })?;
    b.number(actions, &t("other.label"), &t("other.help"), &PLAYER_OTHER, || {
        f64::from(config::get().player.other)
    })?;

    let torrent = b.submenu(page, &t("torrent.title"), &t("torrent.help"))?;
    b.number(torrent, &t("torrent_all.label"), &t("torrent_all.help"), &TORRENT_ALL, || {
        f64::from(config::get().torrent.all)
    })?;
    b.number(torrent, &t("torrent_walk.label"), &t("torrent_walk.help"), &TORRENT_WALK, || {
        f64::from(config::get().torrent.walk)
    })?;
    b.number(torrent, &t("torrent_run.label"), &t("torrent_run.help"), &TORRENT_RUN, || {
        f64::from(config::get().torrent.run)
    })?;
    b.number(torrent, &t("torrent_jump.label"), &t("torrent_jump.help"), &TORRENT_JUMP, || {
        f64::from(config::get().torrent.jump)
    })?;
    b.number(torrent, &t("torrent_other.label"), &t("torrent_other.help"), &TORRENT_OTHER, || {
        f64::from(config::get().torrent.other)
    })?;

    let others = b.submenu(page, &t("others.title"), &t("others.help"))?;
    b.number(others, &t("spirit.label"), &t("spirit.help"), &SPIRIT_ALL, || {
        f64::from(config::get().spirit.all)
    })?;
    b.number(others, &t("enemy.label"), &t("enemy.help"), &ENEMY_ALL, || {
        f64::from(config::get().enemy.all)
    })?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "speed-multiplier", "Speed Multiplier", build);
}

pub fn sync_from_config() {
    menu::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_match_the_speed_clamp() {
        assert_eq!((PLAYER_ALL.min, PLAYER_ALL.max), (MIN_SPEED, MAX_SPEED));
    }

    /// Every key the menu uses is in every language file, and the files have the same keys.
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 41);
    }
}
