//! TOML config for a mod: a serde struct `T` plus an embedded template holding every default. The
//! file is read in two passes (errors with their line, then the user's values over the template)
//! and brought in line with the template at start-up. See `docs/toml_config.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::{Arc, RwLock};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table};

use crate::logger;

pub struct TomlConfig<T> {
    path: String,
    template: String,
    current: RwLock<Arc<T>>,
}

/// One migration step: turns a file of the previous format version into the next.
pub type Step = fn(&mut DocumentMut, &DocumentMut);

#[derive(Clone, Copy)]
pub struct Migration<'a> {
    /// Dotted path of the format version, e.g. `"General.ConfigVersion"`; the template holds the
    /// current one.
    pub version_key: &'a str,
    /// `steps[i]`: version `i + 1` -> `i + 2`. Append only.
    pub steps: &'a [Step],
    /// Top-level names never treated as unknown (`"Override"`).
    pub keep: &'a [&'a str],
}

/// What [`TomlConfig::load_or_create`] did, for the mod to log once its logger is up.
#[derive(Default, Debug)]
pub struct LoadReport {
    /// The file didn't exist and the template was written.
    pub created: bool,
    /// `(from, to)` format versions, when migration steps ran.
    pub migrated: Option<(u32, u32)>,
    /// The file is from a newer format version and was left untouched: `(file, current)`.
    pub newer: Option<(u32, u32)>,
    pub added_keys: usize,
    /// Keys the template doesn't have, moved to the comment block at the end
    /// of the file (`"Speed.PlayerMovement = 1.2"`).
    pub unknown: Vec<String>,
    /// Parse error - the mod is running on the template's values.
    pub error: Option<String>,
}

impl LoadReport {
    /// One short line per thing loading did; nothing for a file that loaded unchanged.
    pub fn log(&self, file: &str) {
        if self.created {
            logger::log(&format!("{file}: created"));
        }
        if self.added_keys > 0 {
            logger::log(&format!("{file}: added {} key(s) from the template", self.added_keys));
        }
        if let Some((from, to)) = self.migrated {
            logger::log(&format!("{file}: migrated format v{from} -> v{to}"));
        }
        if let Some((found, supported)) = self.newer {
            logger::warn(&format!(
                "{file}: newer format v{found} (supported v{supported}), left as is"
            ));
        }
        if !self.unknown.is_empty() {
            logger::warn(&format!(
                "{file}: {} unused key(s) moved to comments ({})",
                self.unknown.len(),
                self.unknown.join(", ")
            ));
        }
        if let Some(err) = &self.error {
            logger::error(&format!("{file}: parse failed, using defaults\n{err}"));
        }
    }
}

impl<T> TomlConfig<T>
where
    T: DeserializeOwned + Serialize + Send + Sync,
{
    pub fn load_or_create(path: &str, template: &str, migration: Migration) -> (Self, LoadReport) {
        let mut report = LoadReport::default();
        match fs::read_to_string(path) {
            Ok(text) => {
                let sync = sync_with_template(&text, template, migration);
                report.newer = sync.newer;
                if let Some(new_text) = &sync.text
                    && write_atomic(path, new_text).is_ok()
                {
                    report.migrated = sync.migrated;
                    report.added_keys = sync.added;
                    report.unknown = sync.unknown;
                }
            }
            Err(_) => {
                report.created = write_atomic(path, template).is_ok();
            }
        }

        let config = match read(path, template) {
            Ok(config) => config,
            Err(err) => {
                report.error = Some(err);
                defaults(template)
            }
        };
        (
            Self {
                path: path.to_string(),
                template: template.to_string(),
                current: RwLock::new(Arc::new(config)),
            },
            report,
        )
    }

    /// The parsed config; take it once per tick.
    pub fn get(&self) -> Arc<T> {
        self.current.read().unwrap().clone()
    }

    /// Re-reads the file and logs every key whose value changed as
    /// `Config: Section.Key: old -> new`. On error the previous config stays in use.
    pub fn reload(&self) -> Result<(), String> {
        let config: Arc<T> = Arc::new(read(&self.path, &self.template)?);
        let previous = std::mem::replace(&mut *self.current.write().unwrap(), config.clone());
        for change in changed_keys(&*previous, &*config) {
            logger::log(&format!("Config: {change}"));
        }
        Ok(())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// Writes `("Section.Key", "text")` pairs into the file, keeping its comments and layout, then
    /// re-reads it. Each text becomes a bool, an integer, a float or a string. On error the file is
    /// left as it was.
    pub fn save_values(&self, values: &[(&str, String)]) -> Result<(), String> {
        let text = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
        let mut doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| e.to_string())?;
        let template: DocumentMut =
            self.template.parse().map_err(|e: toml_edit::TomlError| e.to_string())?;
        for (path, value) in values {
            set_value(&mut doc, &template, path, scalar(value));
        }
        write_atomic(&self.path, &doc.to_string()).map_err(|e| e.to_string())?;
        self.reload()
    }
}

/// Writes through a temporary file and a rename, so a crash never leaves a half-written config.
fn write_atomic(path: &str, contents: &str) -> std::io::Result<()> {
    let temp = format!("{path}.tmp");
    fs::write(&temp, contents)?;
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

/// `"Section.Key: old -> new"` for every value that differs between two configs. A config that
/// cannot be turned into TOML has no changes to list.
fn changed_keys<T: Serialize>(old: &T, new: &T) -> Vec<String> {
    let (Ok(old), Ok(new)) = (toml::Table::try_from(old), toml::Table::try_from(new)) else {
        return Vec::new();
    };
    let (mut old_values, mut new_values) = (BTreeMap::new(), BTreeMap::new());
    flatten("", &old, &mut old_values);
    flatten("", &new, &mut new_values);
    let keys: BTreeSet<_> = old_values.keys().chain(new_values.keys()).collect();
    keys.into_iter()
        .filter(|key| old_values.get(*key) != new_values.get(*key))
        .map(|key| {
            let show = |values: &BTreeMap<String, toml::Value>| {
                values.get(key).map_or_else(|| "(none)".to_string(), ToString::to_string)
            };
            format!("{key}: {} -> {}", show(&old_values), show(&new_values))
        })
        .collect()
}

fn flatten(prefix: &str, table: &toml::Table, out: &mut BTreeMap<String, toml::Value>) {
    for (key, value) in table {
        let path = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
        match value {
            toml::Value::Table(inner) => flatten(&path, inner, out),
            other => {
                out.insert(path, other.clone());
            }
        }
    }
}

/// `true`/`false` -> bool, `42` -> integer, `1.5` -> float, anything else a string.
fn scalar(text: &str) -> toml_edit::Value {
    if let Ok(v) = text.parse::<bool>() {
        v.into()
    } else if let Ok(v) = text.parse::<i64>() {
        v.into()
    } else if let Ok(v) = text.parse::<f64>() {
        v.into()
    } else {
        text.into()
    }
}

/// `#[serde(deserialize_with = "shared::toml_config::key_name")]` for a
/// hotkey field read by [`crate::input::parse_virtual_key`]: accepts a string (`"F5"`,
/// `"0x74"`, `"116"`) or a bare number (`116`, the VK code in decimal), which TOML would
/// otherwise reject as the wrong type.
pub fn key_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum KeyName {
        Name(String),
        Code(i64),
    }
    Ok(match KeyName::deserialize(deserializer)? {
        KeyName::Name(name) => name,
        KeyName::Code(code) => code.to_string(),
    })
}

fn read<T: DeserializeOwned>(path: &str, template: &str) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("can't read the file: {e}"))?;
    parse(&text, template)
}

fn toml_error(e: impl std::fmt::Display) -> String {
    e.to_string().trim_end().to_string()
}

/// `text` read as a `T`: errors with their line from `text` alone, then the values with every
/// missing key filled from `template`.
pub fn parse<T: DeserializeOwned>(text: &str, template: &str) -> Result<T, String> {
    toml::from_str::<T>(text).map_err(toml_error)?;
    let mut merged: toml::Table =
        toml::from_str(template).map_err(|e| format!("template: {}", toml_error(e)))?;
    let user: toml::Table = toml::from_str(text).map_err(toml_error)?;
    lay_over(&mut merged, user);
    T::deserialize(toml::Value::Table(merged)).map_err(toml_error)
}

/// `user`'s values over `base`: tables merge key by key, anything else
/// (values, arrays, `[[lists]]`) is replaced whole.
fn lay_over(base: &mut toml::Table, user: toml::Table) {
    for (key, value) in user {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(base_table)), toml::Value::Table(user_table)) => {
                lay_over(base_table, user_table)
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// The template's values. Panics if the template doesn't parse (a mod test covers that).
pub fn defaults<T: DeserializeOwned>(template: &str) -> T {
    toml::from_str(template).expect("the embedded template parses")
}

/// Dotted paths of every field `T` has but `template` doesn't set, which [parse] would leave at
/// zero. Names in `keep` are skipped. For a mod's unit test.
pub fn template_missing_keys<T: DeserializeOwned + Serialize>(
    template: &str,
    keep: &[&str],
) -> Vec<String> {
    let fields = toml::Table::try_from(defaults::<T>(template)).expect("config serializes");
    let present: toml::Table = toml::from_str(template).expect("the embedded template parses");
    let mut missing = Vec::new();
    missing_keys(&fields, &present, "", keep, &mut missing);
    missing
}

fn missing_keys(
    fields: &toml::Table,
    present: &toml::Table,
    prefix: &str,
    keep: &[&str],
    out: &mut Vec<String>,
) {
    for (key, value) in fields {
        if prefix.is_empty() && keep.contains(&key.as_str()) {
            continue;
        }
        let path = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
        match (value, present.get(key)) {
            (_, None) => out.push(path),
            (toml::Value::Table(f), Some(toml::Value::Table(p))) => {
                missing_keys(f, p, &path, keep, out)
            }
            _ => {}
        }
    }
}

const UNKNOWN_HEADER: &str = "\n# ---- Keys this version doesn't use (renamed or removed) - kept here as\n# comments so nothing you set is lost. Safe to delete. ----\n";

#[derive(Default)]
struct Synced {
    text: Option<String>,
    migrated: Option<(u32, u32)>,
    newer: Option<(u32, u32)>,
    added: usize,
    unknown: Vec<String>,
}

fn version_at(doc: &DocumentMut, path: &str) -> Option<u32> {
    let (parent, key) = split_path(path);
    get_table(doc.as_table(), &parent)?.get(key)?.as_integer().and_then(|v| u32::try_from(v).ok())
}

/// `user` brought in line with `template`; text that isn't valid TOML is left as it is.
fn sync_with_template(user: &str, template: &str, migration: Migration) -> Synced {
    let mut out = Synced::default();
    let (Ok(mut doc), Ok(template)) =
        (user.parse::<DocumentMut>(), template.parse::<DocumentMut>())
    else {
        return out;
    };
    let current = version_at(&template, migration.version_key).unwrap_or(1);
    let file_version = version_at(&doc, migration.version_key).unwrap_or(1);
    if file_version > current {
        out.newer = Some((file_version, current));
        return out;
    }

    let mut changed = false;
    if file_version < current {
        for step in migration
            .steps
            .iter()
            .skip(file_version as usize - 1)
            .take((current - file_version) as usize)
        {
            step(&mut doc, &template);
        }
        out.migrated = Some((file_version, current));
        changed = true;
    }
    // Stamp the current version when the file lacks it or is behind, before the merge below
    // adds the template's value.
    if version_at(&doc, migration.version_key) != Some(current) {
        set_value(&mut doc, &template, migration.version_key, current as i64);
        changed = true;
    }

    out.added = merge_table(doc.as_table_mut(), template.as_table());
    take_unknown(doc.as_table_mut(), template.as_table(), "", migration.keep, &mut out.unknown);
    if !changed && out.added == 0 && out.unknown.is_empty() {
        return out;
    }

    let mut text = doc.to_string();
    if !out.unknown.is_empty() {
        if !text.contains(UNKNOWN_HEADER.trim_start()) {
            text = text.trim_end().to_string() + "\n" + UNKNOWN_HEADER;
        }
        for line in &out.unknown {
            text.push_str("# ");
            text.push_str(line);
            text.push('\n');
        }
    }
    out.text = Some(text);
    out
}

/// For migration steps: moves the value at `old` to `new` (dotted paths), keeping the template's
/// comments. Does nothing if `old` isn't a plain value or `new` is already set; returns whether it
/// moved.
pub fn rename_key(doc: &mut DocumentMut, template: &DocumentMut, old: &str, new: &str) -> bool {
    let (old_parent, old_key) = split_path(old);
    let (new_parent, new_key) = split_path(new);
    if get_table(doc.as_table(), &new_parent).is_some_and(|t| t.contains_key(new_key)) {
        return false;
    }
    let Some(old_table) = get_table_mut(doc.as_table_mut(), &old_parent) else {
        return false;
    };
    if !old_table.get(old_key).is_some_and(Item::is_value) {
        return false;
    }
    let item = old_table.remove(old_key).unwrap();
    let toml_edit::Item::Value(v) = item else { return false };
    set_value(doc, template, new, v);
    true
}

/// For migration steps: removes the value at `path`, returning it.
pub fn remove_value(doc: &mut DocumentMut, path: &str) -> Option<toml_edit::Value> {
    let (parent, key) = split_path(path);
    let table = get_table_mut(doc.as_table_mut(), &parent)?;
    if !table.get(key).is_some_and(Item::is_value) {
        return None;
    }
    table.remove(key)?.into_value().ok()
}

/// For migration steps: sets `path` to `v`, creating its tables with the template's comments.
pub fn set_value(
    doc: &mut DocumentMut,
    template: &DocumentMut,
    path: &str,
    v: impl Into<toml_edit::Value>,
) {
    let (parent, key) = split_path(path);
    let mut table = doc.as_table_mut();
    let mut template_table = Some(template.as_table());
    for part in &parent {
        let from_template = template_table.and_then(|t| t.get(part)).and_then(Item::as_table);
        if !table.contains_key(part) {
            let mut fresh = Table::new();
            if let Some(t) = from_template {
                fresh.decor_mut().clone_from(t.decor());
            }
            table.insert(part, Item::Table(fresh));
        }
        template_table = from_template;
        table = table.get_mut(part).and_then(Item::as_table_mut).unwrap();
    }
    let v: toml_edit::Value = v.into();
    if let Some(existing) = table.get_mut(key).and_then(Item::as_value_mut) {
        // Keep the user's inline comment / spacing around the value.
        let decor = existing.decor().clone();
        *existing = v;
        *existing.decor_mut() = decor;
        return;
    }
    match template_table.and_then(|t| t.get_key_value(key)) {
        Some((template_key, template_item)) => {
            let mut v = v;
            if let Some(template_value) = template_item.as_value() {
                *v.decor_mut() = template_value.decor().clone();
            }
            table.insert_formatted(template_key, Item::Value(v));
        }
        None => {
            table.insert(key, Item::Value(v));
        }
    }
}

/// The settings of an old `.ini` file laid over `template`, as TOML text that keeps the template's
/// comments. `path_of` maps an ini key to the `"Section.Key"` it became, `None` to skip it. A value
/// is read as the kind the template holds there (bool, integer, float, string, or a comma-separated
/// list of integers for an array); one that does not read as that kind keeps the template's value.
pub fn import_ini(template: &str, ini: &str, path_of: impl Fn(&str) -> Option<String>) -> String {
    let template: DocumentMut = template.parse().expect("the template is valid TOML");
    let mut doc = template.clone();
    for line in ini.lines() {
        let Some((key, text)) = line.trim().split_once('=') else { continue };
        let Some(path) = path_of(key.trim()) else { continue };
        if let Some(value) = typed_like(&template, &path, text.trim()) {
            set_value(&mut doc, &template, &path, value);
        }
    }
    doc.to_string()
}

fn typed_like(template: &DocumentMut, path: &str, text: &str) -> Option<toml_edit::Value> {
    use toml_edit::Value;
    let (section, key) = path.split_once('.')?;
    match template.get(section)?.get(key)?.as_value()? {
        Value::Boolean(_) => text.parse::<bool>().ok().map(Value::from),
        Value::Integer(_) => text.parse::<i64>().ok().map(Value::from),
        Value::Float(_) => text.parse::<f64>().ok().map(Value::from),
        Value::String(_) => Some(Value::from(text)),
        Value::Array(_) => {
            let ids: Option<Vec<i64>> = text.split(',').map(|id| id.trim().parse().ok()).collect();
            ids.map(|ids| ids.into_iter().collect::<toml_edit::Array>().into())
        }
        _ => None,
    }
}

/// For a mod that used to keep its settings in an `.ini`: when `path` does not exist yet and
/// `ini_path` does, writes `import(ini text)` to `path`. The ini file is left alone.
pub fn adopt_ini(path: &str, ini_path: &str, import: impl FnOnce(&str) -> String) {
    if std::path::Path::new(path).exists() {
        return;
    }
    if let Ok(ini) = fs::read_to_string(ini_path) {
        let _ = write_atomic(path, &import(&ini));
    }
}

fn split_path(path: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = path.split('.').collect();
    let key = parts.pop().unwrap_or_default();
    (parts, key)
}

fn get_table<'a>(mut table: &'a Table, path: &[&str]) -> Option<&'a Table> {
    for part in path {
        table = table.get(part)?.as_table()?;
    }
    Some(table)
}

fn get_table_mut<'a>(mut table: &'a mut Table, path: &[&str]) -> Option<&'a mut Table> {
    for part in path {
        table = table.get_mut(part)?.as_table_mut()?;
    }
    Some(table)
}

fn merge_table(user: &mut Table, template: &Table) -> usize {
    let mut added = 0;
    for (name, item) in template.iter() {
        match user.get_mut(name) {
            None => {
                let (key, _) = template.get_key_value(name).unwrap();
                added += count_values(item);
                user.insert_formatted(key, item.clone());
            }
            Some(existing) => {
                if let (Some(existing), Some(item)) = (existing.as_table_mut(), item.as_table()) {
                    added += merge_table(existing, item);
                }
            }
        }
    }
    added
}

fn count_values(item: &Item) -> usize {
    match item.as_table() {
        Some(table) => table.iter().map(|(_, item)| count_values(item)).sum(),
        None => 1,
    }
}

/// Removes every key of `user` that `template` doesn't have, adding a
/// `path = value` line per removed value to `out`.
fn take_unknown(
    user: &mut Table,
    template: &Table,
    prefix: &str,
    keep: &[&str],
    out: &mut Vec<String>,
) {
    let names: Vec<String> = user.iter().map(|(name, _)| name.to_string()).collect();
    for name in names {
        let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}.{name}") };
        if prefix.is_empty() && keep.contains(&name.as_str()) {
            continue;
        }
        match template.get(&name) {
            Some(template_item) => {
                if let (Some(user_table), Some(template_table)) =
                    (user.get_mut(&name).and_then(Item::as_table_mut), template_item.as_table())
                {
                    take_unknown(user_table, template_table, &path, keep, out);
                }
            }
            None => {
                if let Some(item) = user.remove(&name) {
                    describe(&item, &path, out);
                }
            }
        }
    }
}

/// `path = value` lines for a removed item (one per value in a table).
fn describe(item: &Item, path: &str, out: &mut Vec<String>) {
    match item {
        Item::Value(value) => out.push(format!("{path} = {}", value.to_string().trim())),
        Item::Table(table) => {
            for (name, item) in table.iter() {
                describe(item, &format!("{path}.{name}"), out);
            }
        }
        Item::ArrayOfTables(array) => {
            for table in array.iter() {
                out.push(format!("[[{path}]]"));
                for (name, item) in table.iter() {
                    describe(item, name, out);
                }
            }
        }
        Item::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format version 3 of a made-up mod; see the steps below.
    const TEMPLATE: &str = "\
[General]
ConfigVersion = 3   # managed by the mod
# Reload key
ReloadKey = \"F5\"

[Player]
# all actions
All = 1.0
# walking
Walk = 1.2
# running
Run = 1.2

[Logging]
LogFile = true
";

    #[test]
    fn an_ini_is_laid_over_the_template() {
        let ini = "ReloadKey=F6
All=2
Walk=oops
LogFile=false
Other=1
";
        let text = import_ini(TEMPLATE, ini, |key| match key {
            "ReloadKey" => Some("General.ReloadKey".to_string()),
            "All" | "Walk" => Some(format!("Player.{key}")),
            "LogFile" => Some("Logging.LogFile".to_string()),
            _ => None,
        });
        let doc: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(doc["General"]["ReloadKey"].as_str(), Some("F6"));
        assert_eq!(doc["Player"]["All"].as_float(), Some(2.0));
        assert_eq!(doc["Player"]["Walk"].as_float(), Some(1.2), "a bad value keeps the template's");
        assert_eq!(doc["Logging"]["LogFile"].as_bool(), Some(false));
        assert!(text.contains("# running"), "the template's comments are kept");
    }

    /// 1 -> 2: `Speed.PlayerAll` / `Speed.PlayerMove` moved under `[Player]`.
    fn step_1_to_2(doc: &mut DocumentMut, template: &DocumentMut) {
        rename_key(doc, template, "Speed.PlayerAll", "Player.All");
        rename_key(doc, template, "Speed.PlayerMove", "Player.Move");
    }

    /// 2 -> 3: `Player.Move` split into `Player.Walk` + `Player.Run`.
    fn step_2_to_3(doc: &mut DocumentMut, template: &DocumentMut) {
        if let Some(v) = remove_value(doc, "Player.Move") {
            set_value(doc, template, "Player.Walk", v.clone());
            set_value(doc, template, "Player.Run", v);
        }
    }

    const STEPS: &[Step] = &[step_1_to_2, step_2_to_3];

    fn migration() -> Migration<'static> {
        Migration { version_key: "General.ConfigVersion", steps: STEPS, keep: &["Override"] }
    }

    fn sync(user: &str) -> Synced {
        sync_with_template(user, TEMPLATE, migration())
    }

    fn doc(text: &str) -> DocumentMut {
        text.parse().unwrap()
    }

    #[test]
    fn complete_file_is_left_alone() {
        assert!(sync(TEMPLATE).text.is_none());
    }

    #[test]
    fn version_1_file_runs_every_step() {
        let s = sync("[Speed]\nPlayerAll = 2.0\nPlayerMove = 1.5\n");
        assert_eq!(s.migrated, Some((1, 3)));
        let d = doc(s.text.as_deref().unwrap());
        assert_eq!(d["General"]["ConfigVersion"].as_integer(), Some(3));
        assert_eq!(d["Player"]["All"].as_float(), Some(2.0));
        assert_eq!(d["Player"]["Walk"].as_float(), Some(1.5));
        assert_eq!(d["Player"]["Run"].as_float(), Some(1.5));
        assert!(s.unknown.is_empty(), "{:?}", s.unknown);
        assert!(sync(s.text.as_deref().unwrap()).text.is_none());
    }

    #[test]
    fn version_2_file_runs_only_the_last_step() {
        let s = sync("[General]\nConfigVersion = 2\n\n[Player]\nAll = 3.0\nMove = 0.5\n");
        assert_eq!(s.migrated, Some((2, 3)));
        let d = doc(s.text.as_deref().unwrap());
        assert_eq!(d["Player"]["All"].as_float(), Some(3.0));
        assert_eq!(d["Player"]["Walk"].as_float(), Some(0.5));
        assert_eq!(d["Player"]["Run"].as_float(), Some(0.5));
    }

    #[test]
    fn steps_never_rerun_on_a_current_file() {
        // v3 file with a stray old key: step 1->2 must not move it.
        let user = format!("{TEMPLATE}\n[Speed]\nPlayerAll = 9.0\n");
        let s = sync(&user);
        assert_eq!(s.migrated, None);
        let d = doc(s.text.as_deref().unwrap());
        assert_eq!(d["Player"]["All"].as_float(), Some(1.0));
        assert_eq!(s.unknown, vec!["Speed.PlayerAll = 9.0"]);
    }

    #[test]
    fn newer_file_is_not_touched() {
        let user =
            TEMPLATE.replace("ConfigVersion = 3", "ConfigVersion = 4") + "\n[Future]\nX = 1\n";
        let s = sync(&user);
        assert_eq!(s.newer, Some((4, 3)));
        assert!(s.text.is_none());
    }

    #[test]
    fn missing_key_is_added_with_its_comment_and_user_values_kept() {
        let user = "\
[General]
ConfigVersion = 3
ReloadKey = \"F6\" # mine

[Player]
All = 2.0
Walk = 1.0

[Logging]
LogFile = false
";
        let s = sync(user);
        assert_eq!(s.added, 1);
        let text = s.text.unwrap();
        assert!(text.contains("ReloadKey = \"F6\" # mine"));
        assert!(text.contains("All = 2.0"));
        assert!(text.contains("# running\nRun = 1.2"));
        assert!(text.contains("LogFile = false"));
    }

    #[test]
    fn missing_table_is_added() {
        let s = sync("[General]\nConfigVersion = 3\nReloadKey = \"F5\"\n");
        assert_eq!(s.added, 4);
        let d = doc(s.text.as_deref().unwrap());
        assert_eq!(d["Player"]["Run"].as_float(), Some(1.2));
        assert_eq!(d["Logging"]["LogFile"].as_bool(), Some(true));
    }

    #[test]
    fn invalid_user_file_is_not_touched() {
        assert!(sync("[Player\nAll = ").text.is_none());
    }

    #[test]
    fn unknown_keys_become_comments_at_the_end() {
        let user = format!("{TEMPLATE}\n[Old]\nA = 1\nB = \"x\"\n")
            .replace("Run = 1.2", "Run = 1.2\nTypo = 3");
        let s = sync(&user);
        assert_eq!(s.unknown, vec!["Player.Typo = 3", "Old.A = 1", "Old.B = \"x\""]);
        let text = s.text.unwrap();
        let d = doc(&text);
        assert!(d.get("Old").is_none());
        assert!(text.contains("# Player.Typo = 3\n"));
        assert!(sync(&text).text.is_none());
    }

    #[test]
    fn kept_names_are_not_unknown() {
        let user = format!("{TEMPLATE}\n[[Override]]\nSpEffect = 1\n");
        assert!(sync(&user).text.is_none());
    }

    #[test]
    fn rename_never_overwrites_a_set_key() {
        let template = doc(TEMPLATE);
        let mut d = doc("[Player]\nAll = 5.0\n[Speed]\nPlayerAll = 2.0\n");
        assert!(!rename_key(&mut d, &template, "Speed.PlayerAll", "Player.All"));
        assert_eq!(d["Player"]["All"].as_float(), Some(5.0));
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct Cfg {
        player: PlayerCfg,
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct PlayerCfg {
        walk: f32,
        run: f32,
    }

    const CFG_TEMPLATE: &str = "[Player]
Walk = 1.1
Run = 1.2
";

    #[test]
    fn missing_keys_take_the_template_value() {
        let cfg: Cfg = parse(
            "[Player]
Run = 3.0
",
            CFG_TEMPLATE,
        )
        .unwrap();
        assert_eq!(cfg.player.walk, 1.1);
        assert_eq!(cfg.player.run, 3.0);
        let cfg: Cfg = parse("", CFG_TEMPLATE).unwrap();
        assert_eq!(cfg, defaults::<Cfg>(CFG_TEMPLATE));
    }

    #[test]
    fn errors_keep_the_users_line() {
        let err = parse::<Cfg>(
            "[Player]
Run = 1.0
Rn = 2.0
",
            CFG_TEMPLATE,
        )
        .unwrap_err();
        assert!(err.contains("Rn") && err.contains("line 3"), "{err}");
        let err = parse::<Cfg>(
            "[Player]
Run = \"fast\"
",
            CFG_TEMPLATE,
        )
        .unwrap_err();
        assert!(err.contains("line 2"), "{err}");
    }

    #[test]
    fn template_missing_keys_finds_unset_fields() {
        assert!(template_missing_keys::<Cfg>(CFG_TEMPLATE, &[]).is_empty());
        assert_eq!(
            template_missing_keys::<Cfg>(
                "[Player]
Walk = 1.1
",
                &[]
            ),
            vec!["Player.Run"]
        );
    }

    #[test]
    fn set_value_keeps_the_users_inline_comment() {
        let template = doc(TEMPLATE);
        let mut d = doc("[Player]\nRun = 1.0 # mine\n");
        set_value(&mut d, &template, "Player.Run", 2.0);
        assert!(d.to_string().contains("Run = 2.0 # mine"), "{}", d);
    }

    #[test]
    fn scalar_types_the_text() {
        assert_eq!(scalar("true").as_bool(), Some(true));
        assert_eq!(scalar("false").as_bool(), Some(false));
        assert_eq!(scalar("42").as_integer(), Some(42));
        assert_eq!(scalar("-3").as_integer(), Some(-3));
        assert_eq!(scalar("1.5").as_float(), Some(1.5));
        assert_eq!(scalar("1.0").as_float(), Some(1.0));
        assert_eq!(scalar("F5").as_str(), Some("F5"));
        assert_eq!(scalar("").as_str(), Some(""));
        // Not a number: stays text.
        assert_eq!(scalar("1.2.3").as_str(), Some("1.2.3"));
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct SaveCfg {
        general: SaveGeneral,
        player: SavePlayer,
        extra: SaveExtra,
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct SaveGeneral {
        reload_key: String,
        count: i64,
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct SavePlayer {
        walk: f32,
        run: f32,
        guard: bool,
    }

    #[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, PartialEq)]
    #[serde(default, deny_unknown_fields, rename_all = "PascalCase")]
    struct SaveExtra {
        // Present in the template only, so a save has to create it.
        boost: f32,
    }

    const SAVE_TEMPLATE: &str = "\
[General]
# Reload key
ReloadKey = \"F5\"
Count = 1

[Player]
# Groups of actions.
Walk = 1.1          # walking
Run = 1.2           # running
Guard = true        # raising the guard

[Extra]
# Added by a newer version.
Boost = 2.0         # more speed
";

    /// A config over a fresh temp file holding `text`.
    fn save_config(name: &str, text: &str) -> (TomlConfig<SaveCfg>, String) {
        let path = std::env::temp_dir()
            .join(format!("toml_config_test_{}_{name}.toml", std::process::id()));
        fs::write(&path, text).unwrap();
        let path = path.to_string_lossy().to_string();
        let current = parse::<SaveCfg>(text, SAVE_TEMPLATE).unwrap();
        let config = TomlConfig {
            path: path.clone(),
            template: SAVE_TEMPLATE.to_string(),
            current: RwLock::new(Arc::new(current)),
        };
        (config, path)
    }

    const USER_FILE: &str = "\
[General]
# my own note on the key
ReloadKey = \"F6\"   # mine
Count = 1

# Groups of actions.
[Player]
Walk = 1.1          # walking
Run = 1.2           # running
Guard = true        # raising the guard
";

    #[test]
    fn save_values_keeps_comments_and_layout_and_updates_the_snapshot() {
        let (config, path) = save_config("layout", USER_FILE);
        config.save_values(&[("Player.Run", "1.5".to_string())]).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        // Only that value changed: same text with the one line swapped.
        assert_eq!(text, USER_FILE.replace("Run = 1.2 ", "Run = 1.5 "));
        assert!(text.contains("# my own note on the key"));
        assert!(text.contains("ReloadKey = \"F6\"   # mine"));
        assert!(text.contains("Walk = 1.1          # walking"));
        // The snapshot was re-read from the file.
        assert_eq!(config.get().player.run, 1.5);
        assert_eq!(config.get().player.walk, 1.1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_values_types_bool_int_float_and_string() {
        let (config, path) = save_config("types", USER_FILE);
        config
            .save_values(&[
                ("Player.Guard", "false".to_string()),
                ("General.Count", "7".to_string()),
                ("Player.Walk", "0.5".to_string()),
                ("General.ReloadKey", "F7".to_string()),
            ])
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let d = doc(&text);
        assert_eq!(d["Player"]["Guard"].as_bool(), Some(false));
        assert_eq!(d["General"]["Count"].as_integer(), Some(7));
        assert_eq!(d["Player"]["Walk"].as_float(), Some(0.5));
        assert_eq!(d["General"]["ReloadKey"].as_str(), Some("F7"));
        assert!(text.contains("Guard = false"));
        assert!(text.contains("ReloadKey = \"F7\""));
        // Inline comments stay.
        assert!(text.contains("# mine") && text.contains("# raising the guard"));
        let now = config.get();
        assert!(!now.player.guard);
        assert_eq!(now.general.count, 7);
        assert_eq!(now.general.reload_key, "F7");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_values_creates_an_unknown_key_with_the_templates_comment() {
        let (config, path) = save_config("create", USER_FILE);
        assert!(!USER_FILE.contains("[Extra]"));
        config.save_values(&[("Extra.Boost", "3.0".to_string())]).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        // The table and the key come with the template's comments.
        assert!(text.contains("[Extra]\n# Added by a newer version.\nBoost = 3.0"), "{text}");
        assert!(text.contains("Boost = 3.0         # more speed"), "{text}");
        // The rest is untouched.
        assert!(text.starts_with(USER_FILE), "{text}");
        assert_eq!(config.get().extra.boost, 3.0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_values_creates_a_missing_key_in_an_existing_table() {
        let user = "[Player]\nWalk = 1.1\n";
        let (config, path) = save_config("missing", user);
        config.save_values(&[("Player.Run", "2.5".to_string())]).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("Walk = 1.1"));
        assert!(text.contains("Run = 2.5") && text.contains("# running"), "{text}");
        assert_eq!(config.get().player.run, 2.5);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_values_on_a_missing_file_is_an_error() {
        let (config, path) = save_config("gone", USER_FILE);
        fs::remove_file(&path).unwrap();
        assert!(config.save_values(&[("Player.Run", "2.0".to_string())]).is_err());
    }

    #[test]
    fn changed_keys_lists_only_what_differs() {
        #[derive(Serialize)]
        struct Group {
            enabled: bool,
            range: f64,
        }
        #[derive(Serialize)]
        struct Settings {
            items: Group,
            key: String,
        }
        let old = Settings { items: Group { enabled: true, range: 2.0 }, key: "F5".into() };
        let new = Settings { items: Group { enabled: false, range: 2.0 }, key: "F6".into() };
        assert_eq!(
            changed_keys(&old, &new),
            vec!["items.enabled: true -> false", "key: \"F5\" -> \"F6\""]
        );
        assert!(changed_keys(&old, &old).is_empty());
    }
}
