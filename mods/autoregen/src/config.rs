//! `AutoRegen.toml`. Defaults live only in the embedded template; `Default` on these structs is the
//! derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use shared::toml_config::{LoadReport, Migration, Step, TomlConfig};

pub const TEMPLATE: &str = include_str!("../AutoRegen.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub per_tick: PerTick,
    pub per_hit: PerHit,
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
pub struct PerTick {
    pub enabled: bool,
    pub trigger: i32,
    pub gesture_id: Vec<i32>,
    pub interval: i32,
    pub value_type: i32,
    #[serde(rename = "HP")]
    pub hp: f64,
    #[serde(rename = "FP")]
    pub fp: f64,
    #[serde(rename = "SP")]
    pub sp: f64,
    pub cap: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct PerHit {
    pub enabled: bool,
    pub mode: i32,
    pub damage_type: i32,
    pub exclude_aow: bool,
    #[serde(rename = "HP")]
    pub hp: f64,
    #[serde(rename = "FP")]
    pub fp: f64,
    #[serde(rename = "SP")]
    pub sp: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

/// The settings of the `.ini` file the 2.x releases used, laid over the template. The ini keys map
/// one to one: `Regen.PerTick.HP` is `PerTick.HP`.
pub fn import_ini(ini: &str) -> String {
    shared::toml_config::import_ini(TEMPLATE, ini, |key| match key {
        "ReloadKey" | "ReloadBanner" => Some(format!("General.{key}")),
        "LogFile" => Some("Logging.LogFile".to_string()),
        _ => key.strip_prefix("Regen.").map(str::to_string),
    })
}

/// 2.x kept its settings in `<name>.ini`; they become the new file when there is none yet.
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
    fn missing_keys_fall_back_to_the_template() {
        let parsed = parse_cfg("[PerTick]\nTrigger = 3\n").unwrap();
        assert_eq!(parsed.per_tick.trigger, 3);
        assert_eq!(parsed.per_tick.interval, template().per_tick.interval);
        assert!(parsed.logging.log_file);
    }

    #[test]
    fn a_misspelled_key_is_an_error_with_its_line() {
        let err = parse_cfg("[PerTick]\nTrigr = 3\n").unwrap_err();
        assert!(err.contains("Trigr") && err.contains("line 2"), "{err}");
    }

    #[test]
    fn the_ini_of_the_old_releases_is_imported() {
        let ini = "[General]\n; comment\nReloadKey=F6\nReloadBanner=true\n\n\
                   [Regen Per Tick]\nRegen.PerTick.Enabled=false\nRegen.PerTick.Trigger=4\n\
                   Regen.PerTick.GestureId=80, 90,100\nRegen.PerTick.Interval=2000\n\
                   Regen.PerTick.HP=2\nRegen.PerTick.Cap=80\n\
                   [Regen Per Hit]\nRegen.PerHit.Enabled=true\nRegen.PerHit.SP=1.5\n\
                   [Logging]\nLogFile=false\n";
        let config = parse_cfg(&import_ini(ini)).unwrap();
        assert_eq!(config.general.reload_key, "F6");
        assert!(config.general.reload_banner);
        assert!(!config.per_tick.enabled);
        assert_eq!(config.per_tick.trigger, 4);
        assert_eq!(config.per_tick.gesture_id, vec![80, 90, 100]);
        assert_eq!(config.per_tick.interval, 2000);
        assert_eq!(config.per_tick.hp, 2.0);
        assert_eq!(config.per_tick.cap, 80.0);
        assert!(config.per_hit.enabled);
        assert_eq!(config.per_hit.sp, 1.5);
        assert!(!config.logging.log_file);
        assert_eq!(
            config.per_tick.fp,
            template().per_tick.fp,
            "keys the ini lacks keep the template"
        );
    }

    #[test]
    fn a_bad_ini_value_keeps_the_template_value() {
        let config = parse_cfg(&import_ini("Regen.PerTick.Interval=soon\n")).unwrap();
        assert_eq!(config.per_tick.interval, template().per_tick.interval);
    }

    #[test]
    fn the_imported_file_keeps_the_template_comments() {
        let text = import_ini("Regen.PerTick.HP=3\n");
        assert!(text.contains("# Amount restored per tick."));
    }
}
