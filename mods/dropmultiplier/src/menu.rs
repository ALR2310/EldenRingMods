//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! Multipliers are sliders from 0 to 10 in steps of 0.5 (a text row up to 1000 on older hosts), the
//! fixed chance a whole-percent slider. A change is saved a second after the player stops, then
//! applied again from the original drop tables, like the reload key.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

/// Upper bound of a typed multiplier; the mod only clamps at 0 below.
const MAX_MULTIPLIER: f64 = 1000.0;

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static MODE: Setting = Setting::int("Drop.Mode");
static MULTIPLIER: NumberSetting =
    NumberSetting::new("Drop.Multiplier", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
// `Percentage` is a TOML float, so the slider's whole number is saved as `45.0`.
static PERCENTAGE: Setting = Setting { key: "Drop.Percentage", format: float_text, apply: None };
static CRAFTING: NumberSetting =
    NumberSetting::new("Materials.Crafting", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static UPGRADE: NumberSetting =
    NumberSetting::new("Materials.Upgrade", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static UNIQUE: NumberSetting =
    NumberSetting::new("Materials.Unique", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn float_text(value: u8) -> String {
    format!("{value}.0")
}

fn mode_row() -> u8 {
    config::get().drop.mode.clamp(0, 1) as u8
}

/// The fixed chance as a slider position: a whole percent from 0 to 100.
fn percentage_row() -> u8 {
    config::get().drop.percentage.round().clamp(0.0, 100.0) as u8
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Drop Multiplier", &t("mod.help"))?;
    let drops = b.submenu(page, &t("drops.title"), &t("drops.help"))?;
    let materials = b.submenu(page, &t("materials.title"), &t("materials.help"))?;

    let modes = [t("mode.options.multiply"), t("mode.options.fixed")];
    let modes: Vec<&str> = modes.iter().map(String::as_str).collect();
    b.choice(drops, &t("mode.label"), &t("mode.help"), &modes, &MODE, mode_row)?;
    b.number(drops, &t("multiplier.label"), &t("multiplier.help"), &MULTIPLIER, || {
        config::get().drop.multiplier
    })?;
    b.slider(
        drops,
        &t("percentage.label"),
        &t("percentage.help"),
        (0, 100),
        &PERCENTAGE,
        percentage_row,
    )?;
    b.number(materials, &t("crafting.label"), &t("crafting.help"), &CRAFTING, || {
        config::get().materials.crafting
    })?;
    b.number(materials, &t("upgrade.label"), &t("upgrade.help"), &UPGRADE, || {
        config::get().materials.upgrade
    })?;
    b.number(materials, &t("unique.label"), &t("unique.help"), &UNIQUE, || {
        config::get().materials.unique
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "drop-multiplier", "Drop Multiplier", build);
}

/// A saved change takes effect like a reload, minus the menu refresh: the rows already show it.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
    crate::drop_rate::request_apply();
}

pub fn sync_from_config() {
    menu::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu uses is in every language file, and the files have the same keys.
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 10);
    }

    #[test]
    fn the_fixed_chance_is_saved_as_a_float() {
        assert_eq!((PERCENTAGE.format)(45), "45.0");
    }
}
