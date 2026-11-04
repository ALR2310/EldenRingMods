//! `AutoPickup.toml`. Defaults live only in the embedded template; `Default` on these structs is
//! the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../AutoPickup.toml");

/// Migration steps for `General.ConfigVersion`; a released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub materials: Group,
    pub items: Group,
    pub corpse_loot: Group,
    pub lost_runes: Group,
    pub chests: Group,
    pub logging: Logging,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct General {
    pub config_version: u32,
    #[serde(deserialize_with = "shared::toml_config::key_name")]
    pub reload_key: String,
    pub reload_banner: bool,
    pub pause_in_combat: bool,
    pub no_pickup_animation: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Group {
    pub enabled: bool,
    pub range: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
    pub pickup_probe: bool,
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
        None => Arc::new(template()),
    }
}

pub fn template() -> Config {
    shared::toml_config::defaults(TEMPLATE)
}

pub fn reload() -> Result<(), String> {
    match CONFIG.get() {
        Some(config) => config.reload(),
        None => Ok(()),
    }
}

/// For the in-game menu: writes the pairs into the file and re-reads it.
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
        let missing = shared::toml_config::template_missing_keys::<Config>(TEMPLATE, &[]);
        assert!(missing.is_empty(), "template lacks {missing:?}");
    }

    #[test]
    fn template_defaults() {
        let config = template();
        assert_eq!(config.general.config_version, 1);
        assert!(config.general.pause_in_combat);
        assert!(config.general.no_pickup_animation);
        for group in
            [config.materials, config.items, config.corpse_loot, config.lost_runes, config.chests]
        {
            assert_eq!(group.range, 2.0);
        }
        for group in [config.materials, config.items, config.corpse_loot, config.lost_runes] {
            assert!(group.enabled);
        }
        assert!(!config.chests.enabled);
        assert!(config.logging.log_file);
        assert!(!config.logging.pickup_probe);
    }

    #[test]
    fn user_keys_override_the_template() {
        let parsed: Config =
            shared::toml_config::parse("[Items]\nEnabled = false\nRange = 2.5\n", TEMPLATE)
                .unwrap();
        assert!(!parsed.items.enabled && parsed.items.range == 2.5);
        assert!(parsed.materials.enabled, "a missing key keeps the template's value");
    }

    #[test]
    fn unknown_key_is_an_error() {
        assert!(shared::toml_config::parse::<Config>("[Items]\nRadius = 2.0\n", TEMPLATE).is_err());
    }

    #[test]
    fn steps_cover_every_version() {
        let doc: toml::Value = toml::from_str(TEMPLATE).unwrap();
        let current = doc["General"]["ConfigVersion"].as_integer().unwrap();
        assert_eq!(STEPS.len() as i64, current - 1);
    }
}
