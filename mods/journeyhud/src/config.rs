//! `JourneyHUD.toml`, loaded through `common::toml_config`. Defaults live only in
//! the embedded template: `Default` on these structs is the derived zero value the
//! loader needs while parsing, never a config in use (see the test
//! `template_sets_every_field`).

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use common::toml_config::{LoadReport, Migration, Step, TomlConfig};

/// Embedded verbatim at compile time - single source of truth for the default file.
pub const TEMPLATE: &str = include_str!("../JourneyHUD.toml");

/// Format migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from
/// version `i + 1` to `i + 2`. Append only - a released step must never change.
const STEPS: &[Step] = &[];

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub target: Target,
    pub effects: Effects,
    pub layout: Layout,
    pub minimap: Minimap,
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
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Target {
    pub enabled: bool,
    pub width: i32,
    pub scale: f64,
    pub show_numbers: bool,
    pub buildup: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Effects {
    pub target: bool,
    pub player: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Layout {
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Minimap {
    pub enabled: bool,
    pub size: i32,
    pub scale: f64,
    pub range: i32,
    pub rotate: bool,
    pub compass: bool,
    pub trail: bool,
    pub clock: bool,
    pub deaths: bool,
    pub full_map: bool,
    #[serde(deserialize_with = "key_list")]
    pub zoom_in_key: Vec<String>,
    #[serde(deserialize_with = "key_list")]
    pub zoom_out_key: Vec<String>,
    pub layers: Layers,
    pub hide: Hide,
}

/// A hotkey field that takes one key or a list of them: `"0xBB"`, `116` or `["0xBB", "0x6B"]`,
/// each a key name or a virtual-key code (see `common::toml_config::key_name`).
fn key_list<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Key {
        Name(String),
        Code(i64),
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Keys {
        One(Key),
        Many(Vec<Key>),
    }
    let name = |key: Key| match key {
        Key::Name(name) => name,
        Key::Code(code) => code.to_string(),
    };
    Ok(match Keys::deserialize(deserializer)? {
        Keys::One(key) => vec![name(key)],
        Keys::Many(keys) => keys.into_iter().map(name).collect(),
    })
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Layers {
    pub map: bool,
    pub graces: bool,
    pub points: bool,
    pub beacons: bool,
    pub map_for_goblins: bool,
    pub coop_players: bool,
    pub spirits: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Hide {
    pub in_caves: bool,
    pub in_boss_fights: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

const MIGRATION: Migration = Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

static CONFIG: OnceLock<TomlConfig<Config>> = OnceLock::new();

/// Loads (creating / bringing in line with the template) `path`. Call once, first
/// thing in `DllMain`'s thread; the report is for logging once the logger is up.
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

/// Writes `("Section.Key", text)` pairs into the file (comments kept) and re-reads it;
/// for the in-game menu. Returns whether it was saved.
pub fn save(values: &[(&str, String)]) -> bool {
    match CONFIG.get() {
        Some(config) => config.save_values(values).is_ok(),
        None => false,
    }
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
        let parsed: Config = common::toml_config::parse("[Minimap]\nRange = 50\n", TEMPLATE).unwrap();
        assert_eq!(parsed.minimap.range, 50);
        assert_eq!(parsed.minimap.size, template().minimap.size);
        assert!(parsed.minimap.layers.map);
    }

    #[test]
    fn misspelled_key_is_an_error_with_its_line() {
        let err = common::toml_config::parse::<Config>("[Minimap.Layers]\nMapp = true\n", TEMPLATE).unwrap_err();
        assert!(err.contains("Mapp") && err.contains("line 2"), "{err}");
    }

    /// A save from the in-game menu changes values only: every comment of the template is
    /// still in the file afterwards, nested tables included.
    #[test]
    fn menu_save_keeps_every_comment() {
        let path = std::env::temp_dir().join(format!("journeyhud_menu_save_{}.toml", std::process::id()));
        let path_text = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        init(&path_text);
        assert!(save(&[
            ("Minimap.Layers.Map", "false".to_string()),
            ("Minimap.Hide.InCaves", "true".to_string()),
            ("Minimap.Range", "50".to_string()),
            ("Target.Scale", "1.5".to_string()),
        ]));
        let config = get();
        assert!(!config.minimap.layers.map && config.minimap.hide.in_caves);
        assert_eq!((config.minimap.range, config.target.scale), (50, 1.5));
        let text = std::fs::read_to_string(&path).unwrap();
        let comments = |text: &str| -> Vec<String> {
            text.lines().filter_map(|line| line.find('#').map(|i| line[i..].trim_end().to_string())).collect()
        };
        assert_eq!(comments(&text), comments(TEMPLATE));
        assert_eq!(text.lines().count(), TEMPLATE.lines().count());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn zoom_keys_take_a_name_or_a_number() {
        let parsed: Config =
            common::toml_config::parse("[Minimap]\nZoomInKey = \"F8\"\nZoomOutKey = 0xBD\n", TEMPLATE).unwrap();
        assert_eq!(parsed.minimap.zoom_in_key, ["F8"]);
        assert_eq!(common::input::parse_virtual_key(&parsed.minimap.zoom_out_key[0], 0), 0xBD);
    }

    #[test]
    fn a_zoom_key_can_be_a_list_of_keys() {
        let parsed: Config =
            common::toml_config::parse("[Minimap]\nZoomInKey = [\"0xBB\", 107]\n", TEMPLATE).unwrap();
        assert_eq!(parsed.minimap.zoom_in_key, ["0xBB", "107"]);
        // The template gives the numpad keys too.
        assert_eq!(template().minimap.zoom_out_key, ["0xBD", "0x6D"]);
    }
}
