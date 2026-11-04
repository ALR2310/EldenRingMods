//! `InfiniteAilment.toml`. Defaults live only in the embedded template; `Default` on these structs
//! is the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../InfiniteAilment.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub scarlet_rot: Ailment,
    pub poison: Ailment,
    pub black_flame: Ailment,
    pub destined_death: DestinedDeath,
    pub logging: Logging,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct General {
    pub config_version: u32,
    #[serde(deserialize_with = "shared::toml_config::key_name")]
    pub reload_key: String,
    pub reload_banner: bool,
}

/// The three knobs of one damage-over-time ailment.
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Ailment {
    pub duration: f64,
    pub percent_damage: f64,
    pub fixed_damage: i32,
}

/// The burn like Black Flame's, plus how long the max HP loss lasts.
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct DestinedDeath {
    pub duration: f64,
    pub percent_damage: f64,
    pub fixed_damage: i32,
    pub max_hp_duration: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

/// The settings of the `.ini` file the 1.x releases used, laid over the template. Its keys are
/// already `Ailment.Key`, so they map to themselves.
pub fn import_ini(ini: &str) -> String {
    shared::toml_config::import_ini(TEMPLATE, ini, |key| match key {
        "ReloadKey" | "ReloadBanner" => Some(format!("General.{key}")),
        "LogFile" => Some("Logging.LogFile".to_string()),
        _ => Some(key.to_string()),
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

    fn parse_cfg(text: &str) -> Result<Config, String> {
        shared::toml_config::parse(text, TEMPLATE)
    }

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
        let ini = "[Scarlet Rot]\nScarletRot.Duration=120\nScarletRot.FixedDamage=20\n\
                   [Poison]\nPoison.PercentDamage=0.5\n\
                   [Destined Death]\nDestinedDeath.MaxHpDuration=30\n[Logging]\nLogFile=false\n";
        let config = parse_cfg(&import_ini(ini)).unwrap();
        assert_eq!(config.scarlet_rot.duration, 120.0);
        assert_eq!(config.scarlet_rot.fixed_damage, 20);
        assert_eq!(config.poison.percent_damage, 0.5);
        assert_eq!(config.destined_death.max_hp_duration, 30.0);
        assert!(!config.logging.log_file);
        assert_eq!(
            config.black_flame,
            parse_cfg("").unwrap().black_flame,
            "keys the ini lacks keep the template"
        );
    }
}
