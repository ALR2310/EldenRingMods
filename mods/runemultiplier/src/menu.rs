//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! The multiplier is a slider from -10 to 10 in steps of 0.5 (a text row from -1000 to 1000 on
//! older hosts). A change applies at once and is saved a second after the player stops.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

/// Bounds of the typed multiplier. A negative value takes runes away, 0 gives none.
const MIN_MULTIPLIER: f64 = -1000.0;
const MAX_MULTIPLIER: f64 = 1000.0;

static MULTIPLIER: NumberSetting = NumberSetting::new(
    "Settings.Multiplier",
    MIN_MULTIPLIER,
    MAX_MULTIPLIER,
    Some(crate::hook::set_multiplier),
)
.slider(-10.0, 10.0, 0.5);
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
    let page = b.submenu(b.root(), "Rune Multiplier", &t("mod.help"))?;
    b.number(page, &t("multiplier.label"), &t("multiplier.help"), &MULTIPLIER, || {
        config::get().settings.multiplier
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "rune-multiplier", "Rune Multiplier", build);
}

/// The multiplier was applied when the row changed; only the log switch needs following.
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 5);
    }
}
