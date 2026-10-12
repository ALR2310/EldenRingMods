//! `AnimCancel.toml`, loaded through `common::toml_config` (same layout as
//! SpeedMultiplier's). Defaults live only in the embedded template: `Default`
//! on these structs is the derived zero value the loader needs while parsing,
//! never a config in use - see [template] and the test
//! `template_sets_every_field`. `General.ConfigVersion` is the file format
//! version for future migrations ([STEPS]); 1 is the first format.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use common::toml_config::{LoadReport, Migration, Step, TomlConfig};

/// Embedded verbatim at compile time - single source of truth for the
/// default file.
pub const TEMPLATE: &str = include_str!("../AnimCancel.toml");

/// Format migration steps for `General.ConfigVersion` (see
/// `common::toml_config`): `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. Append only - a released step must never change.
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
    pub cancel: Cancel,
    pub animations: Animations,
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

/// `[Cancel]`: the actions that may cut an animation short.
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Cancel {
    pub guard: bool,
    pub dodge: bool,
    pub weapon_swap: bool,
}

/// `[Animations]`: the animations that may be cut short.
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Animations {
    pub attack: bool,
    pub skill: bool,
    pub cast: bool,
    pub item: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
    pub cancel_probe: bool,
    pub flag_probe: bool,
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
        assert!(config.cancel.guard && config.cancel.dodge && config.cancel.weapon_swap);
        assert!(config.animations.attack && config.animations.skill && config.animations.cast);
        // Off until the early-cancel of a flask is known to be safe for the player.
        assert!(!config.animations.item);
        // Like the other mods: the log is on, the (verbose) probes are off.
        assert!(config.logging.log_file);
        assert!(!config.logging.cancel_probe && !config.logging.flag_probe);
    }

    #[test]
    fn user_keys_override_the_template() {
        let parsed: Config = common::toml_config::parse("[Cancel]\nDodge = false\n", TEMPLATE).unwrap();
        assert!(!parsed.cancel.dodge);
        assert!(parsed.cancel.guard, "a missing key keeps the template's value");
    }

    #[test]
    fn unknown_key_is_an_error() {
        assert!(common::toml_config::parse::<Config>("[Cancel]\nRoll = true\n", TEMPLATE).is_err());
    }
}
