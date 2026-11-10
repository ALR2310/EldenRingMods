//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! The multiplier is a slider from 0 to 5 in steps of 0.05 (a text row from -1000 to 1000 on older
//! hosts), the fixed value a typed number. A change is saved a second after the player stops and
//! then applied to the hook, like the reload key.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

static MODE: Setting = Setting::int("Features.Mode");
static MULTIPLIER: NumberSetting =
    NumberSetting::new("Features.Multiplier", -1000.0, 1000.0, None).slider(0.0, 5.0, 0.05);
static FIXED_VALUE: NumberSetting =
    NumberSetting::new("Features.FixedValue", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn mode_row() -> u8 {
    config::get().features.mode.clamp(0, 1) as u8
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Weight Multiplier", &t("mod.help"))?;
    let options = [t("mode.options.multiply"), t("mode.options.fixed")];
    let options: Vec<&str> = options.iter().map(String::as_str).collect();
    b.choice(page, &t("mode.label"), &t("mode.help"), &options, &MODE, mode_row)?;
    b.number(page, &t("multiplier.label"), &t("multiplier.help"), &MULTIPLIER, || {
        config::get().features.multiplier
    })?;
    b.number(page, &t("fixed_value.label"), &t("fixed_value.help"), &FIXED_VALUE, || {
        config::get().features.fixed_value
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "weight-multiplier", "Weight Multiplier", build);
}

/// After the file is written: apply it to the hook, like the reload key does.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
    crate::hook::reload_from_config();
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 9);
    }
}
