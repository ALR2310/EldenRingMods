//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Drop Multiplier            -> opens a page with:
//!   Drop Chance              -> Mode        Multiply chance / Fixed chance
//!                               Multiplier  number 0..1000   (Mode: multiply)
//!                               Percentage  slider 0..100    (Mode: fixed)
//!   Materials                -> Crafting     number 0..1000
//!                               Upgrade      number 0..1000
//!                               Unique       number 0..1000
//!   Log file                 on/off
//! ```
//!
//! The multipliers are native sliders (0..10, step 0.5) when the host has
//! them, else typed text rows (the ini holds decimals up to 1000); the
//! percentage is a whole-percent slider. Changes are written to the ini once
//! the player stops for a second, then re-applied from the original drop
//! tables through the same path as the reload key.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::i18n::Catalog;

/// Upper bound of the typed multipliers (the mod only clamps at 0 below).
const MAX_MULTIPLIER: f64 = 1000.0;

static MENU: OnceLock<erui::Menu> = OnceLock::new();

static MODE: Setting = Setting::int("Mode");
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static PERCENTAGE: Setting = Setting::int("Percentage");
static CRAFTING: NumberSetting = NumberSetting::new("Crafting", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static UPGRADE: NumberSetting = NumberSetting::new("Upgrade", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static UNIQUE: NumberSetting = NumberSetting::new("Unique", 0.0, MAX_MULTIPLIER, None).slider(0.0, 10.0, 0.5);
static LOG_FILE: Setting = Setting::flag("LogFile");

/// The menu texts, one file per language (`locales/`); English is the fallback.
static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

/// The text of `key` in the host language.
fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn mode_row() -> u8 {
    config::get_int("Mode", 0).clamp(0, 1) as u8
}

/// The fixed chance as a slider position (whole percent, 0..=100).
fn percentage_row() -> u8 {
    config::get_double("Percentage", 50.0).round().clamp(0.0, 100.0) as u8
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Drop Multiplier", &t("mod.help"))?;
    let drops = b.submenu(page, &t("drops.title"), &t("drops.help"))?;
    let materials = b.submenu(page, &t("materials.title"), &t("materials.help"))?;

    let modes = [t("mode.options.multiply"), t("mode.options.fixed")];
    let mode_refs: Vec<&str> = modes.iter().map(String::as_str).collect();
    b.choice(drops, &t("mode.label"), &t("mode.help"), &mode_refs, &MODE, mode_row)?;
    b.number(
        drops,
        &t("multiplier.label"),
        &t("multiplier.help"),
        &MULTIPLIER,
        || config::get_double("Multiplier", 2.0),
    )?;
    b.slider(
        drops,
        &t("percentage.label"),
        &t("percentage.help"),
        0,
        100,
        &PERCENTAGE,
        percentage_row,
    )?;
    b.number(
        materials,
        &t("crafting.label"),
        &t("crafting.help"),
        &CRAFTING,
        || config::get_double("Crafting", 1.0),
    )?;
    b.number(
        materials,
        &t("upgrade.label"),
        &t("upgrade.help"),
        &UPGRADE,
        || config::get_double("Upgrade", 1.0),
    )?;
    b.number(
        materials,
        &t("unique.label"),
        &t("unique.help"),
        &UNIQUE,
        || config::get_double("Unique", 1.0),
    )?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, log_file_row)?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(erui::reload_from_menu));
    erui::spawn_menu(&MENU, hmodule, "drop-multiplier", "Drop Multiplier", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu passes to the text lookup is in every language file,
    /// and the files have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        let source = include_str!("menu.rs");
        let mut used: Vec<String> = Vec::new();
        for (at, _) in source.match_indices("t(\"") {
            // Skip longer names that merely end in the letter t.
            let before = source[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &source[at + 3..];
            if let Some(end) = rest.find('"') {
                used.push(rest[..end].to_string());
            }
        }
        used.sort();
        used.dedup();
        assert!(used.len() >= 10, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }

    /// Every row meant to be a native slider has a valid range and step (else
    /// it silently stays a typed row).
    #[test]
    fn slider_rows_have_a_valid_range() {
        for setting in [&MULTIPLIER, &CRAFTING, &UPGRADE, &UNIQUE] {
            assert!(setting.has_slider(), "{} would not be a slider", setting.key);
        }
    }
}
