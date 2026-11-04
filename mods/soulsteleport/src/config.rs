//! `SoulsTeleport.toml`. Defaults live only in the embedded template; `Default` on these structs is
//! the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../SoulsTeleport.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub menu: Menu,
    pub logging: Logging,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct General {
    pub config_version: u32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Menu {
    #[serde(deserialize_with = "shared::toml_config::key_name")]
    pub key: String,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

/// The settings of the `.ini` file the 1.x releases used, laid over the template: `MenuKey`,
/// `MenuScale`, `MenuX`... became `Menu.Key`, `Menu.Scale`, `Menu.X`...
pub fn import_ini(ini: &str) -> String {
    shared::toml_config::import_ini(TEMPLATE, ini, |key| match key {
        "LogFile" => Some("Logging.LogFile".to_string()),
        _ => key.strip_prefix("Menu").map(|rest| format!("Menu.{rest}")),
    })
}

/// 1.x kept its settings in `<name>.ini`; they become the new file when there is none yet.
pub fn adopt_ini(path: &str, ini_path: &str) {
    shared::toml_config::adopt_ini(path, ini_path, import_ini);
}

static CONFIG: OnceLock<TomlConfig<Config>> = OnceLock::new();

/// The report is for the caller to log once the logger is up.
pub fn init(path: &str) -> LoadReport {
    let (config, report) = TomlConfig::load_or_create(path, TEMPLATE, MIGRATION);
    let _ = CONFIG.set(config);
    report
}

pub fn get() -> Arc<Config> {
    match CONFIG.get() {
        Some(config) => config.get(),
        None => Arc::new(shared::toml_config::defaults(TEMPLATE)),
    }
}

/// Writes the pairs into the file and re-reads it.
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
    let refs: Vec<(&str, String)> =
        mapped.iter().map(|(key, text)| (key.as_str(), text.clone())).collect();
    save(&refs)
}

/// What `shared::overlay` needs to keep the menu's layout and scale in this file.
pub fn overlay_store() -> shared::overlay::Store {
    shared::overlay::Store { get: overlay_get, save: overlay_save }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_sets_every_field() {
        let missing = shared::toml_config::template_missing_keys::<Config>(TEMPLATE, &[]);
        assert!(missing.is_empty(), "template lacks {missing:?}");
    }

    #[test]
    fn steps_cover_every_version() {
        let doc: toml::Value = toml::from_str(TEMPLATE).unwrap();
        let current = doc["General"]["ConfigVersion"].as_integer().unwrap();
        assert_eq!(STEPS.len() as i64, current - 1);
    }

    #[test]
    fn the_ini_of_the_old_releases_is_imported() {
        let ini = "[Menu]\nMenuKey=0x77\nMenuScale=1.5\nMenuX=100\nMenuHeight=300\n[Logging]\nLogFile=false\n";
        let config: Config = shared::toml_config::parse(&import_ini(ini), TEMPLATE).unwrap();
        assert_eq!(config.menu.key, "0x77");
        assert_eq!(config.menu.scale, 1.5);
        assert_eq!((config.menu.x, config.menu.y), (100.0, -1.0));
        assert_eq!(config.menu.height, 300.0);
        assert!(!config.logging.log_file);
    }

    /// The overlay's `MenuX`... keys land in `[Menu]` and every comment of the template survives.
    #[test]
    fn overlay_layout_is_saved_into_the_menu_table() {
        let path = std::env::temp_dir()
            .join(format!("soulsteleport_overlay_save_{}.toml", std::process::id()));
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
