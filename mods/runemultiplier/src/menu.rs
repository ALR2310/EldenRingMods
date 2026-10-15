//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Rune Multiplier            -> opens a page with:
//!                                 Multiplier   number -1000..1000 (typed)
//!                                 Log file     on/off
//! ```
//!
//! The multiplier is a text row where a number is typed (fractions such as
//! 1.5, 0 and negative values work, like in the ini). Changes apply at once
//! and are written back to the ini shortly after the player stops, so they
//! survive a restart.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, NumberSetting, Setting};
use common::i18n::Catalog;

/// Bounds of the typed multiplier. Negative values subtract runes (the mod
/// accepts them), 0 gives none.
const MIN_MULTIPLIER: f64 = -1000.0;
const MAX_MULTIPLIER: f64 = 1000.0;

static MULTIPLIER: NumberSetting =
    NumberSetting::new("Multiplier", MIN_MULTIPLIER, MAX_MULTIPLIER, Some(crate::hook::set_multiplier));
static LOG_FILE: Setting = Setting::flag("LogFile");

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
    let page = b.submenu(b.root(), "Rune Multiplier", &t("mod.help"))?;
    b.number(page, &t("multiplier.label"), &t("multiplier.help"), &MULTIPLIER, || {
        config::get_double("Multiplier", 1.0)
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || config::get_bool("LogFile", false) as u8)?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "rune-multiplier", "Rune Multiplier", build);
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

    /// Every key the menu passes to the translation function is in every
    /// language file, and the files have the same keys (nothing missing,
    /// nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        let source = include_str!("menu.rs");
        let mut used: Vec<String> = Vec::new();
        for (at, _) in source.match_indices("t(\"") {
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
        assert!(used.len() >= 5, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }
}
