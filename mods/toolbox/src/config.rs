//! `Toolbox.toml`, loaded through `common::toml_config`. Defaults live only in the embedded
//! template: `Default` on these structs is the derived zero value the loader needs while
//! parsing, never a config in use (see the test `template_sets_every_field`).

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use common::toml_config::{LoadReport, Migration, Step, TomlConfig};

/// Embedded verbatim at compile time - single source of truth for the default file.
pub const TEMPLATE: &str = include_str!("../Toolbox.toml");

/// Format migration steps for `General.ConfigVersion`; none yet (1 is the first format).
/// Append only - a released step must never change.
const STEPS: &[Step] = &[];

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub menu: Menu,
    pub input_bridge: InputBridge,
    pub window_resize: WindowResize,
    pub taskbar: Taskbar,
    pub map_anywhere: Toggle,
    pub torrent_anywhere: Toggle,
    pub buildup: Buildup,
    pub logging: Logging,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct General {
    /// File format version, managed by the mod.
    pub config_version: u32,
    #[serde(deserialize_with = "common::toml_config::key_name")]
    pub reload_key: String,
    pub reload_banner: bool,
    pub load_delay: u32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Menu {
    #[serde(deserialize_with = "common::toml_config::key_name")]
    pub key: String,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct InputBridge {
    pub enabled: bool,
    pub background_keyboard: bool,
    pub spoof_foreground: bool,
    pub isolate_input: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct WindowResize {
    pub resizable: bool,
    pub width: i32,
    pub height: i32,
    pub aspect_width: i32,
    pub aspect_height: i32,
    pub min_width: i32,
    pub min_height: i32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Taskbar {
    /// `Game`, `Blank` or `Default` (any case), see `taskbar.rs`.
    pub icon: String,
    pub stop_flashing: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Toggle {
    pub enabled: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Buildup {
    pub amount: i32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

const MIGRATION: Migration = Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

static CONFIG: OnceLock<TomlConfig<Config>> = OnceLock::new();

/// Loads (creating / bringing in line with the template) `path`. Call once, first thing in
/// `DllMain`'s thread; the report is for logging once the logger is up.
pub fn init(path: &str) -> LoadReport {
    let (config, report) = TomlConfig::load_or_create(path, TEMPLATE, MIGRATION);
    let _ = CONFIG.set(config);
    report
}

/// The config in use.
pub fn get() -> Arc<Config> {
    match CONFIG.get() {
        Some(config) => config.get(),
        None => Arc::new(template()),
    }
}

/// The template's values - the defaults.
pub fn template() -> Config {
    common::toml_config::defaults(TEMPLATE)
}

pub fn reload() -> Result<(), String> {
    match CONFIG.get() {
        Some(config) => config.reload(),
        None => Ok(()),
    }
}

/// Writes `("Section.Key", text)` pairs into the file (comments kept) and re-reads it; for the
/// in-game menu. Returns whether it was saved.
pub fn save(values: &[(&str, String)]) -> bool {
    match CONFIG.get() {
        Some(config) => config.save_values(values).is_ok(),
        None => false,
    }
}

/// The overlay's settings keys (`MenuX`...) live in the `[Menu]` table here (`X`...).
fn menu_path(key: &str) -> Option<String> {
    key.strip_prefix("Menu").map(|rest| format!("Menu.{rest}"))
}

fn overlay_get(key: &str) -> Option<f64> {
    let menu = get().menu.clone();
    match menu_path(key)?.as_str() {
        "Menu.Scale" => Some(menu.scale),
        "Menu.X" => Some(menu.x),
        "Menu.Y" => Some(menu.y),
        "Menu.Width" => Some(menu.width),
        "Menu.Height" => Some(menu.height),
        _ => None,
    }
}

fn overlay_save(values: &[(&str, String)]) -> bool {
    let mapped: Vec<(String, String)> = values
        .iter()
        .map(|(key, text)| (menu_path(key).unwrap_or_else(|| (*key).to_string()), text.clone()))
        .collect();
    let refs: Vec<(&str, String)> = mapped.iter().map(|(key, text)| (key.as_str(), text.clone())).collect();
    save(&refs)
}

/// What `common::overlay` needs to keep the menu's layout and scale in this file.
pub fn overlay_store() -> common::overlay::Store {
    common::overlay::Store { get: overlay_get, save: overlay_save }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_sets_every_field() {
        let missing = common::toml_config::template_missing_keys::<Config>(TEMPLATE, &[]);
        assert!(missing.is_empty(), "template lacks {missing:?}");
    }

    #[test]
    fn steps_cover_every_version() {
        let doc: toml::Value = toml::from_str(TEMPLATE).unwrap();
        let current = doc["General"]["ConfigVersion"].as_integer().unwrap();
        assert_eq!(STEPS.len() as i64, current - 1);
    }

    #[test]
    fn missing_keys_fall_back_to_the_template() {
        let parsed: Config = common::toml_config::parse("[Buildup]\nAmount = 50\n", TEMPLATE).unwrap();
        assert_eq!(parsed.buildup.amount, 50);
        assert_eq!(parsed.general.load_delay, template().general.load_delay);
        assert!(parsed.map_anywhere.enabled);
    }

    #[test]
    fn misspelled_key_is_an_error_with_its_line() {
        let err = common::toml_config::parse::<Config>("[Buildup]\nAmmount = 1\n", TEMPLATE).unwrap_err();
        assert!(err.contains("Ammount") && err.contains("line 2"), "{err}");
    }

    /// The overlay's `MenuX`... keys land in `[Menu]` and every comment of the template survives.
    #[test]
    fn overlay_layout_is_saved_into_the_menu_table() {
        let path = std::env::temp_dir().join(format!("toolbox_overlay_save_{}.toml", std::process::id()));
        let path_text = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        init(&path_text);
        assert!(overlay_save(&[("MenuX", "120".to_string()), ("MenuHeight", "300".to_string())]));
        assert_eq!(overlay_get("MenuX"), Some(120.0));
        assert_eq!(overlay_get("MenuHeight"), Some(300.0));
        assert_eq!(overlay_get("MenuScale"), Some(1.0));
        assert_eq!(overlay_get("Nope"), None);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), TEMPLATE.lines().count());
        let _ = std::fs::remove_file(&path);
    }
}
