//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! The mod reads its settings on every tick, so a change applies once it is saved, a second after
//! the player stops. Numbers are typed into text rows. The milestone list stays file-only.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

static ENABLED: Setting = Setting::flag("Passive.Enabled");
static INTERVAL: NumberSetting = NumberSetting::whole("Passive.Interval", 0.0, 3_600_000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Passive.Amount", -100_000.0, 100_000.0, None);
static PERCENT: NumberSetting = NumberSetting::new("Passive.Percent", -100.0, 100.0, None);
static INTEREST: NumberSetting = NumberSetting::new("Passive.Interest", -100.0, 100.0, None);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Passive Runes", &t("mod.help"))?;
    b.toggle(page, &t("enabled.label"), &t("enabled.help"), &ENABLED, || {
        config::get().passive.enabled as u8
    })?;
    b.number(page, &t("interval.label"), &t("interval.help"), &INTERVAL, || {
        config::get().passive.interval as f64
    })?;
    b.number(page, &t("amount.label"), &t("amount.help"), &AMOUNT, || {
        config::get().passive.amount as f64
    })?;
    b.number(page, &t("next_level.label"), &t("next_level.help"), &PERCENT, || {
        config::get().passive.percent
    })?;
    b.number(page, &t("interest.label"), &t("interest.help"), &INTEREST, || {
        config::get().passive.interest
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "passive-runes", "Passive Runes", build);
}

/// The mod reads every setting live; only the log switch needs following.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 7);
    }
}
