//! `SpiritMultiplier.toml`. Defaults live only in the embedded template; `Default` on these structs
//! is the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../SpiritMultiplier.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub spirit: Spirit,
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
pub struct Spirit {
    pub multiplier: f64,
    pub amount: i32,
    pub max_spirits: i32,
    pub ghost_color: bool,
    pub instant_mimic: bool,
    pub regen_mode: i32,
    pub regen_value: f64,
    pub multi_spirit: bool,
    pub clone_spirit: bool,
    pub no_rest_resummon: bool,
    pub summon_anywhere: bool,
    pub keep_spirits_on_rest: bool,
    pub improve_senses: bool,
    pub improve_aggression: bool,
    pub improve_follow: bool,
    pub warp_when_far: bool,
    pub warp_distance: f64,
    pub warp_blocked_time: f64,
    pub warp_stuck_time: f64,
    pub warp_stuck_range: f64,
    pub hub_hp_bars: i32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
    pub slot_probe: bool,
    pub effect_probe: bool,
    pub enemy_probe: bool,
    pub warp_probe: bool,
    pub persist_probe: bool,
    pub active_character_limit: u32,
}

/// The settings of the `.ini` file the 1.x releases used, laid over the template. Its keys had no
/// section, so every key but the general and logging ones goes to `[Spirit]`; `Regen` and `HpBars`
/// were renamed long ago.
pub fn import_ini(ini: &str) -> String {
    shared::toml_config::import_ini(TEMPLATE, ini, |key| match key {
        "ReloadKey" | "ReloadBanner" => Some(format!("General.{key}")),
        "LogFile"
        | "SlotProbe"
        | "EffectProbe"
        | "EnemyProbe"
        | "WarpProbe"
        | "PersistProbe"
        | "ActiveCharacterLimit" => Some(format!("Logging.{key}")),
        "Regen" => Some("Spirit.RegenValue".to_string()),
        "HpBars" => Some("Spirit.HubHpBars".to_string()),
        _ => Some(format!("Spirit.{key}")),
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
        let ini = "[General]\nReloadKey=F6\n[Spirit]\nMultiplier=3\nAmount=4\nGhostColor=true\n\
                   Regen=1.5\nHpBars=10\nWarpDistance=30\n[Logging]\nLogFile=false\nSlotProbe=true\nActiveCharacterLimit=120\n";
        let config = parse_cfg(&import_ini(ini)).unwrap();
        assert_eq!(config.general.reload_key, "F6");
        assert_eq!(config.spirit.multiplier, 3.0);
        assert_eq!(config.spirit.amount, 4);
        assert!(config.spirit.ghost_color);
        assert_eq!(config.spirit.regen_value, 1.5);
        assert_eq!(config.spirit.hub_hp_bars, 10);
        assert_eq!(config.spirit.warp_distance, 30.0);
        assert!(!config.logging.log_file);
        assert!(config.logging.slot_probe && !config.logging.warp_probe);
        assert_eq!(config.logging.active_character_limit, 120);
        assert!(config.spirit.multi_spirit, "keys the ini lacks keep the template");
    }
}
