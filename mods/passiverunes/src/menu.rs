//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Passive Runes              -> opens a page with:
//!   Enabled                    on/off
//!   Interval (ms)              number 0..3600000 (0 = off)
//!   Amount                     number -100000..100000
//!   Next Level (%)             number -100..100
//!   Interest (%)               number -100..100
//!   Log file                   on/off
//! ```
//!
//! The mod re-reads its ini keys every tick, so a change applies as soon as
//! it is written (shortly after the player stops changing rows). Numbers are
//! typed into text rows. The milestone list stays ini-only.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::i18n::Catalog;

static ENABLED: Setting = Setting::flag("Rune.Passive.Enabled");
static INTERVAL: NumberSetting = NumberSetting::whole("Rune.Passive.Interval", 0.0, 3_600_000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Rune.Passive.Amount", -100_000.0, 100_000.0, None);
static PERCENT: NumberSetting = NumberSetting::new("Rune.Passive.Percent", -100.0, 100.0, None);
static INTEREST: NumberSetting = NumberSetting::new("Rune.Passive.Interest", -100.0, 100.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn enabled_row() -> u8 {
    config::get_bool("Rune.Passive.Enabled", true) as u8
}
fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

/// The menu texts, one file per language (`locales/`); English is the fallback.
static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

/// The text of `key` in the host language.
fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Passive Runes", &t("mod.help"))?;
    b.toggle(page, &t("enabled.label"), &t("enabled.help"), &ENABLED, enabled_row)?;
    b.number(page, &t("interval.label"), &t("interval.help"), &INTERVAL, || {
        config::get_double("Rune.Passive.Interval", 10000.0)
    })?;
    b.number(page, &t("amount.label"), &t("amount.help"), &AMOUNT, || {
        config::get_double("Rune.Passive.Amount", 50.0)
    })?;
    b.number(page, &t("next_level.label"), &t("next_level.help"), &PERCENT, || {
        config::get_double("Rune.Passive.Percent", 0.25)
    })?;
    b.number(page, &t("interest.label"), &t("interest.help"), &INTEREST, || {
        config::get_double("Rune.Passive.Interest", 0.0)
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, log_file_row)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    // The mod reads every key live each tick, so saving is all it takes.
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "passive-runes", "Passive Runes", build);
}

/// After an ini reload: show the reloaded values in the menu.
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 7);
    }
}
