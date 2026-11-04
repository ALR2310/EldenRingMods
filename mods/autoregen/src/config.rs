//! `AutoRegen.toml`. Defaults live only in the embedded template; `Default` on these structs is the
//! derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Value};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig, set_value};

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
    let template: DocumentMut = TEMPLATE.parse().expect("the template is valid TOML");
    let mut doc = template.clone();
    for line in ini.lines() {
        let Some((key, text)) = line.trim().split_once('=') else { continue };
        let path = match key.trim() {
            "ReloadKey" | "ReloadBanner" => format!("General.{}", key.trim()),
            "LogFile" => "Logging.LogFile".to_string(),
            key => match key.strip_prefix("Regen.") {
                Some(rest) => rest.to_string(),
                None => continue,
            },
        };
        if let Some(value) = typed_like_template(&template, &path, text.trim()) {
            set_value(&mut doc, &template, &path, value);
        }
    }
    doc.to_string()
}

/// `text` as the kind of value the template holds at `path`; `None` if it is not one.
fn typed_like_template(template: &DocumentMut, path: &str, text: &str) -> Option<Value> {
    let (section, key) = path.split_once('.')?;
    match template.get(section)?.get(key)?.as_value()? {
        Value::Boolean(_) => text.parse::<bool>().ok().map(Value::from),
        Value::Integer(_) => text.parse::<i64>().ok().map(Value::from),
        Value::Float(_) => text.parse::<f64>().ok().map(Value::from),
        Value::String(_) => Some(Value::from(text)),
        Value::Array(_) => {
            let ids: Option<Vec<i64>> = text.split(',').map(|id| id.trim().parse().ok()).collect();
            ids.map(|ids| ids.into_iter().collect::<Array>().into())
        }
        _ => None,
    }
}

/// 2.x kept its settings in `<name>.ini`; writes them as the new file when there is none yet.
pub fn adopt_ini(path: &str, ini_path: &str) {
    if std::path::Path::new(path).exists() {
        return;
    }
    if let Ok(ini) = std::fs::read_to_string(ini_path) {
        let _ = std::fs::write(path, import_ini(&ini));
    }
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
