//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the
//! `JourneyHUD.toml` file only. Layout (System > Game Options):
//!
//! ```text
//! Journey HUD                -> two pages:
//!   Target panel             -> Target bar on/off, Health numbers on/off, Status buildup on/off, Effects on/off (target, you),
//!                               Bar length slider, Size stepped, Right margin slider, Top margin slider
//!   Minimap                  -> Minimap on/off, Map, Graces, Points, Beacons, Map for Goblins,
//!                               Co-op players, Spirits, Full map, Clock, Deaths, Trail, Compass, Turn with
//!                               camera, Hide in caves, Hide in boss fights (all on/off),
//!                               Minimap range and size (sliders), Minimap scale (stepped)
//! ```
//!
//! Changes apply at once and are written back to `JourneyHUD.toml` shortly after the
//! player stops, so they survive a restart.

use std::sync::OnceLock;

use common::erui::{self, NumberSetting, Setting};
use common::i18n::Catalog;

use crate::config;
use crate::settings;

static ENABLED: Setting = Setting { key: "Target.Enabled", format: erui::format_bool, apply: Some(settings::set_enabled) };
static SHOW_NUMBERS: Setting =
    Setting { key: "Target.Numbers", format: erui::format_bool, apply: Some(settings::set_show_numbers) };
static EFFECTS_TARGET: Setting =
    Setting { key: "Target.Effects", format: erui::format_bool, apply: Some(settings::set_effects_target) };
static EFFECTS_PLAYER: Setting =
    Setting { key: "Player.Effects", format: erui::format_bool, apply: Some(settings::set_effects_player) };
static BUILDUP: Setting =
    Setting { key: "Target.Buildup", format: erui::format_bool, apply: Some(settings::set_buildup) };
static PLAYER_SCALE: NumberSetting =
    NumberSetting::float("Player.Scale", 0.5, 3.0, Some(settings::set_player_scale)).stepped(0.1);
static WIDTH: NumberSetting =
    NumberSetting::whole("Target.Width", 100.0, 1600.0, Some(settings::set_width)).slider(100.0, 800.0, 10.0);
static SCALE: NumberSetting = NumberSetting::float("Target.Scale", 0.5, 3.0, Some(settings::set_scale)).stepped(0.1);
static OFFSET_X: NumberSetting =
    NumberSetting::whole("Layout.OffsetX", 0.0, 400.0, Some(settings::set_offset_x)).slider(0.0, 300.0, 5.0);
static OFFSET_Y: NumberSetting =
    NumberSetting::whole("Layout.OffsetY", 0.0, 400.0, Some(settings::set_offset_y)).slider(0.0, 300.0, 5.0);

static MINIMAP: Setting =
    Setting { key: "Minimap.Enabled", format: erui::format_bool, apply: Some(settings::set_minimap_enabled) };
static MINIMAP_MAP: Setting =
    Setting { key: "Minimap.Layers.Map", format: erui::format_bool, apply: Some(settings::set_minimap_map) };
static MINIMAP_GRACES: Setting =
    Setting { key: "Minimap.Layers.Graces", format: erui::format_bool, apply: Some(settings::set_minimap_graces) };
static MINIMAP_POINTS: Setting =
    Setting { key: "Minimap.Layers.Points", format: erui::format_bool, apply: Some(settings::set_minimap_points) };
static MINIMAP_BEACONS: Setting =
    Setting { key: "Minimap.Layers.Beacons", format: erui::format_bool, apply: Some(settings::set_minimap_beacons) };
static MINIMAP_COMPASS: Setting =
    Setting { key: "Minimap.Compass", format: erui::format_bool, apply: Some(settings::set_minimap_compass) };
static MINIMAP_CLOCK: Setting =
    Setting { key: "Minimap.Clock", format: erui::format_bool, apply: Some(settings::set_minimap_clock) };
static MINIMAP_DEATHS: Setting =
    Setting { key: "Minimap.Deaths", format: erui::format_bool, apply: Some(settings::set_minimap_deaths) };
static MINIMAP_TRAIL: Setting =
    Setting { key: "Minimap.Trail", format: erui::format_bool, apply: Some(settings::set_minimap_trail) };
static MINIMAP_FULL_MAP: Setting =
    Setting { key: "Minimap.FullMap", format: erui::format_bool, apply: Some(settings::set_minimap_full_map) };
static MINIMAP_GOBLINS: Setting =
    Setting { key: "Minimap.Layers.MapForGoblins", format: erui::format_bool, apply: Some(settings::set_minimap_goblins) };
static MINIMAP_COOP: Setting =
    Setting { key: "Minimap.Layers.CoopPlayers", format: erui::format_bool, apply: Some(settings::set_minimap_coop) };
static MINIMAP_SPIRITS: Setting =
    Setting { key: "Minimap.Layers.Spirits", format: erui::format_bool, apply: Some(settings::set_minimap_spirits) };
static MINIMAP_ROTATE: Setting =
    Setting { key: "Minimap.Rotate", format: erui::format_bool, apply: Some(settings::set_minimap_rotate) };
static MINIMAP_HIDE_IN_CAVES: Setting =
    Setting { key: "Minimap.Hide.InCaves", format: erui::format_bool, apply: Some(settings::set_minimap_hide_in_caves) };
static MINIMAP_HIDE_IN_BOSS_FIGHTS: Setting = Setting {
    key: "Minimap.Hide.InBossFights",
    format: erui::format_bool,
    apply: Some(settings::set_minimap_hide_in_boss_fights),
};
static MINIMAP_SCALE: NumberSetting =
    NumberSetting::float("Minimap.Scale", 0.5, 3.0, Some(settings::set_minimap_scale)).stepped(0.1);
static MINIMAP_RANGE: NumberSetting =
    NumberSetting::whole("Minimap.Range", 20.0, 400.0, Some(settings::set_minimap_range)).slider(40.0, 200.0, 10.0);
static MINIMAP_SIZE: NumberSetting =
    NumberSetting::whole("Minimap.Size", 40.0, 200.0, Some(settings::set_minimap_size)).slider(50.0, 160.0, 5.0);

static MENU: OnceLock<erui::Menu> = OnceLock::new();

/// The menu texts, one file per language (`locales/`); English is the fallback.
static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

/// The text of `key` in the host language.
fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn build(b: &mut erui::MenuBuilder) -> Result<(), String> {
    let root = b.submenu(b.root(), "Journey HUD", &t("mod.help"))?;
    let target = b.submenu(root, &t("target_page.title"), &t("target_page.help"))?;
    let minimap = b.submenu(root, &t("minimap_page.title"), &t("minimap_page.help"))?;
    b.toggle(target, &t("enabled.label"), &t("enabled.help"), &ENABLED, || config::get().target.enabled as u8)?;
    b.toggle(target, &t("numbers.label"), &t("numbers.help"), &SHOW_NUMBERS, || {
        config::get().target.numbers as u8
    })?;
    b.toggle(target, &t("buildup.label"), &t("buildup.help"), &BUILDUP, || config::get().target.buildup as u8)?;
    b.toggle(target, &t("effects.label"), &t("effects.help"), &EFFECTS_TARGET, || config::get().target.effects as u8)?;
    b.toggle(target, &t("effects_player.label"), &t("effects_player.help"), &EFFECTS_PLAYER, || {
        config::get().player.effects as u8
    })?;
    b.number(target, &t("player_scale.label"), &t("player_scale.help"), &PLAYER_SCALE, || config::get().player.scale)?;
    b.number(target, &t("width.label"), &t("width.help"), &WIDTH, || f64::from(config::get().target.width))?;
    b.number(target, &t("scale.label"), &t("scale.help"), &SCALE, || config::get().target.scale)?;
    b.number(target, &t("offset_x.label"), &t("offset_x.help"), &OFFSET_X, || f64::from(config::get().layout.offset_x))?;
    b.number(target, &t("offset_y.label"), &t("offset_y.help"), &OFFSET_Y, || f64::from(config::get().layout.offset_y))?;
    b.toggle(minimap, &t("minimap.label"), &t("minimap.help"), &MINIMAP, || config::get().minimap.enabled as u8)?;
    b.toggle(minimap, &t("minimap_map.label"), &t("minimap_map.help"), &MINIMAP_MAP, || config::get().minimap.layers.map as u8)?;
    b.toggle(minimap, &t("minimap_graces.label"), &t("minimap_graces.help"), &MINIMAP_GRACES, || {
        config::get().minimap.layers.graces as u8
    })?;
    b.toggle(minimap, &t("minimap_points.label"), &t("minimap_points.help"), &MINIMAP_POINTS, || {
        config::get().minimap.layers.points as u8
    })?;
    b.toggle(minimap, &t("minimap_beacons.label"), &t("minimap_beacons.help"), &MINIMAP_BEACONS, || {
        config::get().minimap.layers.beacons as u8
    })?;
    b.toggle(minimap, &t("minimap_goblins.label"), &t("minimap_goblins.help"), &MINIMAP_GOBLINS, || {
        config::get().minimap.layers.map_for_goblins as u8
    })?;
    b.toggle(minimap, &t("minimap_coop.label"), &t("minimap_coop.help"), &MINIMAP_COOP, || {
        config::get().minimap.layers.coop_players as u8
    })?;
    b.toggle(minimap, &t("minimap_spirits.label"), &t("minimap_spirits.help"), &MINIMAP_SPIRITS, || {
        config::get().minimap.layers.spirits as u8
    })?;
    b.toggle(minimap, &t("minimap_full_map.label"), &t("minimap_full_map.help"), &MINIMAP_FULL_MAP, || {
        config::get().minimap.full_map as u8
    })?;
    b.toggle(minimap, &t("minimap_clock.label"), &t("minimap_clock.help"), &MINIMAP_CLOCK, || {
        config::get().minimap.clock as u8
    })?;
    b.toggle(minimap, &t("minimap_deaths.label"), &t("minimap_deaths.help"), &MINIMAP_DEATHS, || {
        config::get().minimap.deaths as u8
    })?;
    b.toggle(minimap, &t("minimap_trail.label"), &t("minimap_trail.help"), &MINIMAP_TRAIL, || {
        config::get().minimap.trail as u8
    })?;
    b.toggle(minimap, &t("minimap_compass.label"), &t("minimap_compass.help"), &MINIMAP_COMPASS, || {
        config::get().minimap.compass as u8
    })?;
    b.toggle(minimap, &t("minimap_rotate.label"), &t("minimap_rotate.help"), &MINIMAP_ROTATE, || {
        config::get().minimap.rotate as u8
    })?;
    b.toggle(minimap, &t("minimap_hide_caves.label"), &t("minimap_hide_caves.help"), &MINIMAP_HIDE_IN_CAVES, || {
        config::get().minimap.hide.in_caves as u8
    })?;
    b.toggle(minimap, &t("minimap_hide_boss.label"), &t("minimap_hide_boss.help"), &MINIMAP_HIDE_IN_BOSS_FIGHTS, || {
        config::get().minimap.hide.in_boss_fights as u8
    })?;
    b.number(minimap, &t("minimap_range.label"), &t("minimap_range.help"), &MINIMAP_RANGE, || f64::from(config::get().minimap.range))?;
    b.number(minimap, &t("minimap_scale.label"), &t("minimap_scale.help"), &MINIMAP_SCALE, || config::get().minimap.scale)?;
    b.number(minimap, &t("minimap_size.label"), &t("minimap_size.help"), &MINIMAP_SIZE, || f64::from(config::get().minimap.size))?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(config::save, None);
    erui::spawn_menu(&MENU, hmodule, "journey-hud", "Journey HUD", build);
}

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    erui::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu passes to `t` is in every language file, and the files
    /// have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 55);
    }

    /// Every key a row saves to is in the template, so a save never invents a key.
    #[test]
    fn menu_keys_are_in_the_template() {
        let doc: toml::Value = toml::from_str(config::TEMPLATE).unwrap();
        let settings = [&ENABLED, &SHOW_NUMBERS, &BUILDUP, &EFFECTS_TARGET, &EFFECTS_PLAYER, &MINIMAP, &MINIMAP_MAP, &MINIMAP_GRACES, &MINIMAP_POINTS, &MINIMAP_BEACONS, &MINIMAP_COMPASS, &MINIMAP_CLOCK, &MINIMAP_DEATHS, &MINIMAP_TRAIL, &MINIMAP_GOBLINS, &MINIMAP_FULL_MAP, &MINIMAP_COOP, &MINIMAP_SPIRITS];
        let numbers = [&WIDTH, &SCALE, &PLAYER_SCALE, &OFFSET_X, &OFFSET_Y, &MINIMAP_RANGE, &MINIMAP_SIZE, &MINIMAP_SCALE];
        let keys = settings.iter().map(|s| s.key).chain(numbers.iter().map(|n| n.key));
        for key in keys.chain([MINIMAP_ROTATE.key, MINIMAP_HIDE_IN_CAVES.key, MINIMAP_HIDE_IN_BOSS_FIGHTS.key]) {
            assert!(key.split('.').try_fold(&doc, |table, part| table.get(part)).is_some(), "{key} is not in the template");
        }
    }

    /// Every row meant to be a native slider has a valid range and step (else
    /// it silently stays a typed row).
    #[test]
    fn slider_rows_have_a_valid_range() {
        for setting in [&WIDTH, &SCALE, &OFFSET_X, &OFFSET_Y, &MINIMAP_RANGE, &MINIMAP_SIZE] {
            assert!(setting.has_slider(), "{} would not be a slider", setting.key);
        }
    }
}
