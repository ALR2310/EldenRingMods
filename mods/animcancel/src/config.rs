//! `AnimCancel.toml`. Defaults live only in the embedded template; `Default` on these structs is
//! the derived zero value the loader needs while parsing, never a config in use.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use toml_edit::DocumentMut;

use shared::toml_config::{LoadReport, Migration, Step, TomlConfig, remove_value, set_value};

pub const TEMPLATE: &str = include_str!("../AnimCancel.toml");

/// Migration steps for `General.ConfigVersion`: `STEPS[i]` takes a file from version `i + 1` to
/// `i + 2`. A released step never changes.
const STEPS: &[Step] = &[v1_to_v2];

/// 1 -> 2: `[Cancel]` and `[Animations]` became one section per action, each with its own
/// `Enabled` and animation set. Every action takes over the old shared set, so a migrated file
/// behaves as before.
fn v1_to_v2(doc: &mut DocumentMut, template: &DocumentMut) {
    const ACTIONS: [&str; 3] = ["Guard", "Dodge", "WeaponSwap"];
    const KINDS: [&str; 4] = ["Attack", "Skill", "Cast", "Item"];
    for section in ACTIONS {
        if let Some(value) = remove_value(doc, &format!("Cancel.{section}")) {
            set_value(doc, template, &format!("{section}.Enabled"), value);
        }
    }
    for kind in KINDS {
        if let Some(value) = remove_value(doc, &format!("Animations.{kind}")) {
            for section in ACTIONS {
                set_value(doc, template, &format!("{section}.{kind}"), value.clone());
            }
        }
    }
    // Whatever is left of the old sections would only be an unknown key.
    doc.as_table_mut().remove("Cancel");
    doc.as_table_mut().remove("Animations");
}

const MIGRATION: Migration =
    Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &[] };

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Config {
    pub general: General,
    pub guard: Action,
    pub dodge: Action,
    pub weapon_swap: Action,
    pub jump: Action,
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

/// One action that may cut an animation short, and which animations it may cut.
#[derive(Deserialize, Serialize, Default, Debug, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
pub struct Action {
    pub enabled: bool,
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

/// 1.2.0 named the file `<name>V1.toml`; keeps its settings when the new name does not exist yet.
pub fn adopt_legacy_file(path: &str, legacy: &str) {
    if !std::path::Path::new(path).exists() && std::path::Path::new(legacy).exists() {
        let _ = std::fs::copy(legacy, path);
    }
}

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
    fn the_old_file_name_is_adopted_once() {
        let dir = std::env::temp_dir().join(format!("animcancel_legacy_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (new, old) = (dir.join("a.toml"), dir.join("aV1.toml"));
        std::fs::write(&old, "old").unwrap();
        adopt_legacy_file(new.to_str().unwrap(), old.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "old");
        std::fs::write(&old, "changed").unwrap();
        adopt_legacy_file(new.to_str().unwrap(), old.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "old", "an existing file is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn template_sets_every_field() {
        let missing = shared::toml_config::template_missing_keys::<Config>(TEMPLATE, &[]);
        assert!(missing.is_empty(), "template lacks {missing:?}");
    }

    #[test]
    fn template_defaults() {
        let config = template();
        assert_eq!(config.general.config_version, 2);
        for action in [config.guard, config.dodge, config.weapon_swap, config.jump] {
            assert!(action.enabled && action.attack && action.skill && action.cast && action.item);
        }
        assert!(config.logging.log_file);
        assert!(!config.logging.cancel_probe && !config.logging.flag_probe);
    }

    #[test]
    fn user_keys_override_the_template() {
        let parsed: Config = shared::toml_config::parse(
            "[Dodge]
Enabled = false
",
            TEMPLATE,
        )
        .unwrap();
        assert!(!parsed.dodge.enabled);
        assert!(
            parsed.dodge.cast && parsed.guard.enabled,
            "a missing key keeps the template's value"
        );
    }

    #[test]
    fn unknown_key_is_an_error() {
        assert!(
            shared::toml_config::parse::<Config>(
                "[Dodge]
Roll = true
",
                TEMPLATE
            )
            .is_err()
        );
    }

    #[test]
    fn steps_cover_every_version() {
        let doc: toml::Value = toml::from_str(TEMPLATE).unwrap();
        let current = doc["General"]["ConfigVersion"].as_integer().unwrap();
        assert_eq!(STEPS.len() as i64, current - 1);
    }

    /// A version 1 file keeps its behavior: each action's switch moves over and every action takes
    /// the old shared animation set.
    #[test]
    fn version_1_file_migrates() {
        let old = "[General]
ConfigVersion = 1

[Cancel]
Guard = false
Dodge = true
WeaponSwap = true

[Animations]
Attack = true
Skill = false
Cast = true
Item = false
";
        let path =
            std::env::temp_dir().join(format!("animcancel_migrate_{}.toml", std::process::id()));
        std::fs::write(&path, old).unwrap();
        let (loaded, report) =
            TomlConfig::<Config>::load_or_create(&path.to_string_lossy(), TEMPLATE, MIGRATION);
        let config = loaded.get();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(report.migrated, Some((1, 2)));
        assert!(report.unknown.is_empty(), "{:?}", report.unknown);
        assert!(!config.guard.enabled && config.dodge.enabled && config.weapon_swap.enabled);
        for action in [config.guard, config.dodge, config.weapon_swap] {
            assert!(action.attack && !action.skill && action.cast && !action.item);
        }
        assert!(!text.contains("[Cancel]") && !text.contains("[Animations]"), "{text}");
    }

    /// A save from the in-game menu changes values only; every template comment stays.
    #[test]
    fn menu_save_keeps_every_comment() {
        let path =
            std::env::temp_dir().join(format!("animcancel_menu_save_{}.toml", std::process::id()));
        let path_text = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        init(&path_text);
        assert!(save(&[
            ("Guard.Enabled", "false".to_string()),
            ("Guard.Cast", "false".to_string()),
            ("Logging.LogFile", "false".to_string())
        ]));
        let config = get();
        assert!(!config.guard.enabled && !config.guard.cast && config.dodge.cast);
        assert!(!config.logging.log_file);
        let text = std::fs::read_to_string(&path).unwrap();
        let comments = |text: &str| -> Vec<String> {
            text.lines()
                .filter_map(|line| line.find('#').map(|i| line[i..].trim_end().to_string()))
                .collect()
        };
        assert_eq!(comments(&text), comments(TEMPLATE));
        assert_eq!(text.lines().count(), TEMPLATE.lines().count());
        let _ = std::fs::remove_file(&path);
    }
}
