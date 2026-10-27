//! `AutoPickup.toml`, loaded through `common::toml_config` (same layout as
//! AnimCancel's). Defaults live only in the embedded template: `Default` on
//! these structs is the derived zero value the loader needs while parsing,
//! never a config in use - see the test `template_sets_every_field`.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use common::toml_config::{LoadReport, Migration, Step, TomlConfig};

/// Embedded verbatim at compile time - single source of truth for the
/// default file.
pub const TEMPLATE: &str = include_str!("../AutoPickup.toml");

/// Format migration steps for `General.ConfigVersion`; none yet (1 is the
/// first format). Append only - a released step must never change.
const STEPS: &[Step] = &[];

const MIGRATION: Migration = Migration {
    version_key: "General.ConfigVersion",
    steps: STEPS,
    keep: &[],
};

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
    /// File format version, managed by the mod.
    pub config_version: u32,
    #[serde(deserialize_with = "common::toml_config::key_name")]
    pub reload_key: String,
    pub reload_banner: bool,
    pub pause_in_combat: bool,
    pub no_pickup_animation: bool,
}

/// `[Materials]`, `[Items]`, `[CorpseLoot]`, `[LostRunes]`, `[Chests]`.
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
}

static CONFIG: OnceLock<TomlConfig<Config>> = OnceLock::new();

/// Loads (or creates) the file at `path`. The returned report is for the
/// caller to log once the logger is up.
pub fn init(path: &str) -> LoadReport {
    let (config, report) = TomlConfig::load_or_create(path, TEMPLATE, MIGRATION);
    let _ = CONFIG.set(config);
    report
}

/// The config in use (a cheap `Arc` snapshot).
pub fn get() -> Arc<Config> {
    match CONFIG.get() {
        Some(config) => config.get(),
        None => Arc::new(template()),
    }
}

/// The template's values: what a fresh install runs on.
pub fn template() -> Config {
    common::toml_config::defaults(TEMPLATE)
}

/// Re-reads the file; on an error the last good config stays in use.
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
    fn template_defaults() {
        let config = template();
        assert_eq!(config.general.config_version, 1);
        assert!(config.general.pause_in_combat);
        assert!(!config.general.no_pickup_animation);
        // Twice the game's own reach out of the box.
        for group in [config.materials, config.items, config.corpse_loot, config.lost_runes, config.chests] {
            assert_eq!(group.range, 2.0);
        }
        for group in [config.materials, config.items, config.corpse_loot, config.lost_runes] {
            assert!(group.enabled);
        }
        // Chests stay shut by default: a mimic opens the same way.
        assert!(!config.chests.enabled);
        assert!(config.logging.log_file);
    }

    #[test]
    fn user_keys_override_the_template() {
        let parsed: Config = common::toml_config::parse("[Items]\nEnabled = false\nRange = 2.5\n", TEMPLATE).unwrap();
        assert!(!parsed.items.enabled && parsed.items.range == 2.5);
        assert!(parsed.materials.enabled, "a missing key keeps the template's value");
    }

    #[test]
    fn unknown_key_is_an_error() {
        assert!(common::toml_config::parse::<Config>("[Items]\nRadius = 2.0\n", TEMPLATE).is_err());
    }

    #[test]
    fn steps_cover_every_version() {
        let doc: toml::Value = toml::from_str(TEMPLATE).unwrap();
        let current = doc["General"]["ConfigVersion"].as_integer().unwrap();
        assert_eq!(STEPS.len() as i64, current - 1);
    }
}
