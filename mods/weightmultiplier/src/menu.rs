//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Weight Multiplier          -> opens a page with:
//!   Mode                       Multiply / Fixed value
//!   Multiplier                 number -1000..1000 (used in Multiply mode)
//!   Fixed Value                number -1000..1000 (used in Fixed value mode)
//!   Log file                   on/off
//! ```
//!
//! This mod has no game task: its own reload (the ReloadKey poll thread) just
//! re-reads the config and stores the new value for the hook's stub. A menu
//! change takes the same path (`hook::reload_from_config`) once it has been
//! written to the ini (about a second after the player stops changing rows).
//! The multiplier and the fixed value are typed into text rows.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::i18n::Catalog;

static MODE: Setting = Setting::int("Mode");
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", -1000.0, 1000.0, None).slider(0.0, 5.0, 0.05);
static FIXED_VALUE: NumberSetting = NumberSetting::new("FixedValue", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn mode_row() -> u8 {
    config::get_int("Mode", 0).clamp(0, 1) as u8
}

fn fixed_value() -> f64 {
    config::get_double("FixedValue", 0.0)
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

/// After the ini is written: apply it to the hook, like the reload key does.
fn apply_saved() {
    crate::hook::reload_from_config();
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
    let page = b.submenu(b.root(), "Weight Multiplier", &t("mod.help"))?;
    let options = [t("mode.options.multiply"), t("mode.options.fixed")];
    let refs: Vec<&str> = options.iter().map(String::as_str).collect();
    b.choice(page, &t("mode.label"), &t("mode.help"), &refs, &MODE, mode_row)?;
    b.number(
        page,
        &t("multiplier.label"),
        &t("multiplier.help"),
        &MULTIPLIER,
        crate::hook::configured_multiplier,
    )?;
    b.number(page, &t("fixed_value.label"), &t("fixed_value.help"), &FIXED_VALUE, fixed_value)?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, log_file_row)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(apply_saved));
    erui::spawn_menu(&MENU, hmodule, "weight-multiplier", "Weight Multiplier", build);
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
        assert!(used.len() >= 9, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }

    /// Every row meant to be a native slider has a valid range and step (else
    /// it silently stays a typed row).
    #[test]
    fn slider_rows_have_a_valid_range() {
        for setting in [&MULTIPLIER] {
            assert!(setting.has_slider(), "{} would not be a slider", setting.key);
        }
    }
}
