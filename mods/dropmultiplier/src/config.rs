//! `DropMultiplier.toml`. Defaults live only in the embedded template; `Default` on these structs is
//! the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Value};

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig, set_value};

pub const TEMPLATE: &str = include_str!("../DropMultiplier.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[];

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub drop: Drop,
    pub materials: Materials,
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
pub struct Drop {
    pub mode: i32,
    pub multiplier: f64,
    pub percentage: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Materials {
    pub crafting: f64,
    pub upgrade: f64,
    pub unique: f64,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Logging {
    pub log_file: bool,
}

/// The settings of the `.ini` file the 1.x releases used, laid over the template. The ini keys had
/// no section; `ChancePercent` is the old name of `Percentage`.
pub fn import_ini(ini: &str) -> String {
    let template: DocumentMut = TEMPLATE.parse().expect("the template is valid TOML");
    let mut doc = template.clone();
    for line in ini.lines() {
        let Some((key, text)) = line.trim().split_once('=') else { continue };
        let path = match key.trim() {
            "ReloadKey" | "ReloadBanner" => format!("General.{}", key.trim()),
            "Mode" | "Multiplier" | "Percentage" => format!("Drop.{}", key.trim()),
            "ChancePercent" => "Drop.Percentage".to_string(),
            "Crafting" | "Upgrade" | "Unique" => format!("Materials.{}", key.trim()),
            "LogFile" => "Logging.LogFile".to_string(),
            _ => continue,
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
        _ => None,
    }
}

/// 1.x kept its settings in `<name>.ini`; writes them as the new file when there is none yet.
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
    fn a_misspelled_key_is_an_error_with_its_line() {
        let err = parse_cfg("[Drop]\nModee = 1\n").unwrap_err();
        assert!(err.contains("Modee") && err.contains("line 2"), "{err}");
    }

    #[test]
    fn the_ini_of_the_old_releases_is_imported() {
        let ini = "[General]\nReloadKey=F6\n\n[Drop]\nMode=1\nMultiplier=2\nChancePercent=45\n\n\
                   [Materials]\nCrafting=4\nUnique=1.5\n\n[Logging]\nLogFile=false\n";
        let config = parse_cfg(&import_ini(ini)).unwrap();
        assert_eq!(config.general.reload_key, "F6");
        assert_eq!(config.drop.mode, 1);
        assert_eq!(config.drop.multiplier, 2.0);
        assert_eq!(config.drop.percentage, 45.0);
        assert_eq!(config.materials.crafting, 4.0);
        assert_eq!(config.materials.unique, 1.5);
        assert_eq!(config.materials.upgrade, template().materials.upgrade);
        assert!(!config.logging.log_file);
    }

    #[test]
    fn a_bad_ini_value_keeps_the_template_value() {
        let config = parse_cfg(&import_ini("Mode=fixed\n")).unwrap();
        assert_eq!(config.drop.mode, template().drop.mode);
    }
}
