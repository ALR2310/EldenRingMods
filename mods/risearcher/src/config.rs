//! `RiseArcher.toml`. Defaults live only in the embedded template; `Default` on these structs is the
//! derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../RiseArcher.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub bow: Bow,
    pub crossbow: Launcher,
    pub ballista: Launcher,
    pub bullet: Bullet,
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

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Bow {
    pub damage_multiplier: f64,
    pub scaling_multiplier: f64,
    #[serde(rename = "UnlockAOW")]
    pub unlock_aow: bool,
    pub weight_multiplier: f64,
    pub sell_value_multiplier: f64,
}

/// What a crossbow and a ballista have: no scaling, and no Ash of War to unlock.
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Launcher {
    pub damage_multiplier: f64,
    pub weight_multiplier: f64,
    pub sell_value_multiplier: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Bullet {
    pub arrow_max_quantity: i32,
    pub bolt_max_quantity: i32,
    pub speed_multiplier: f64,
    pub range_multiplier: f64,
    pub spread_multiplier: f64,
    pub rain_of_arrows_count: i32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

/// The settings of the `.ini` file the 2.0 release used, laid over the template. The weapon keys
/// (`Bow.X`, `Crossbow.X`, `Ballista.X`) are the same in the table form; the ammo keys became
/// `Bullet.ArrowMaxQuantity` and `Bullet.BoltMaxQuantity`.
pub fn import_ini(ini: &str) -> String {
    shared::toml_config::import_ini(TEMPLATE, ini, |key| match key {
        "ReloadKey" | "ReloadBanner" => Some(format!("General.{key}")),
        "LogFile" => Some("Logging.LogFile".to_string()),
        "Bullet.Arrow.MaxQuantity" => Some("Bullet.ArrowMaxQuantity".to_string()),
        "Bullet.Bolt.MaxQuantity" => Some("Bullet.BoltMaxQuantity".to_string()),
        _ => Some(key.to_string()),
    })
}

/// 2.0 kept its settings in `<name>.ini`; they become the new file when there is none yet.
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
    fn the_ini_of_the_old_release_is_imported() {
        let ini = "[Damage Multipliers]\nBow.DamageMultiplier=2\nCrossbow.DamageMultiplier=3.5\n\
                   [Common]\nBow.UnlockAOW=false\nBallista.WeightMultiplier=0.25\n\
                   [Bullet Tuning]\nBullet.Arrow.MaxQuantity=99\nBullet.RainOfArrowsCount=8\n\
                   [Logging]\nLogFile=false\n";
        let config = parse_cfg(&import_ini(ini)).unwrap();
        assert_eq!(config.bow.damage_multiplier, 2.0);
        assert_eq!(config.crossbow.damage_multiplier, 3.5);
        assert!(!config.bow.unlock_aow);
        assert_eq!(config.ballista.weight_multiplier, 0.25);
        assert_eq!(config.bullet.arrow_max_quantity, 99);
        assert_eq!(config.bullet.rain_of_arrows_count, 8);
        assert!(!config.logging.log_file);
        assert_eq!(config.bullet.bolt_max_quantity, 255, "keys the ini lacks keep the template");
    }
}
