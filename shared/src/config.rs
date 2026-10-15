//! Minimal flat INI reader: `key=value` per line, `#` or `;` starts a comment,
//! on its own line or after the value when preceded by whitespace
//! (`Width=25   ; metres`). Section headers are accepted but ignored: all keys
//! share one namespace. Every entry point that needs a default takes the mod's
//! `include_str!` template as a parameter, since this crate has no ini of its
//! own. How the template is merged into a user's file: `shared/docs/config.md`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::str::FromStr;
use std::sync::{LazyLock, RwLock};

static VALUES: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Cuts a trailing comment off a value: the first `;` or `#` that follows
/// whitespace (`true    ; teleport far spirits` -> `true`). Without this the
/// whole text counted as the value, `get_bool`/`get_double` could not parse it
/// and the mod silently fell back to its default. A `;` or `#` glued to the
/// value (`a;b`) is part of it.
fn strip_inline_comment(raw: &str) -> &str {
    let mut previous_is_space = false;
    for (index, c) in raw.char_indices() {
        if previous_is_space && (c == ';' || c == '#') {
            return &raw[..index];
        }
        previous_is_space = c.is_whitespace();
    }
    raw
}

/// `(key, value)` of a `key=value` line, the value without its trailing
/// comment; `None` for blank lines, comments, section headers and lines
/// without `=`.
fn split_kv(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(['#', ';', '[']) {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), strip_inline_comment(value).trim()))
}

fn parse(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(split_kv)
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// Loads `ini_path` into the shared config map, replacing whatever was loaded
/// before. Keeps the previous values if the file can't be read (e.g. a
/// transient lock during a hot reload).
pub fn load(ini_path: &str) {
    if let Ok(content) = fs::read_to_string(ini_path) {
        *VALUES.write().unwrap() = parse(&content);
    }
}

/// Writes `default_ini` to `ini_path` if the file doesn't exist yet; otherwise
/// merges in the template's keys that the file is missing (see
/// `migrate_with_renames`). Then loads the file. Returns how many new keys
/// were merged, so the caller can log it once its logger is up.
pub fn load_or_create_default(ini_path: &str, default_ini: &str) -> usize {
    load_or_create_default_with_renames(ini_path, default_ini, &[])
}

/// Same as [load_or_create_default], but for each `(old, new)` in `renames` a
/// file that still has `old` but not `new` keeps its value under `new`
/// instead of resetting to the template default.
pub fn load_or_create_default_with_renames(
    ini_path: &str,
    default_ini: &str,
    renames: &[(&str, &str)],
) -> usize {
    let migrated = if fs::exists(ini_path).unwrap_or(false) {
        migrate_with_renames(ini_path, default_ini, renames)
    } else {
        let _ = fs::write(ini_path, default_ini);
        0
    };
    load(ini_path);
    migrated
}

#[cfg(test)]
fn migrate(ini_path: &str, default_ini: &str) -> usize {
    migrate_with_renames(ini_path, default_ini, &[])
}

/// Rewrites `ini_path` from the template layout: comments and order come from
/// `default_ini`, every key the user already has keeps their value, missing
/// keys get the template default. Keys the template no longer defines go to a
/// trailing `[Legacy]` block rather than being dropped. No disk write if there
/// is nothing to merge, rename or move. Returns the number of keys added; a
/// renamed key doesn't count, the user already had it.
fn migrate_with_renames(ini_path: &str, default_ini: &str, renames: &[(&str, &str)]) -> usize {
    let Ok(existing_content) = fs::read_to_string(ini_path) else {
        return 0;
    };
    let mut existing_values = parse(&existing_content);
    let mut renamed = false;
    for (old_key, new_key) in renames {
        if !existing_values.contains_key(*new_key)
            && let Some(value) = existing_values.remove(*old_key)
        {
            existing_values.insert(new_key.to_string(), value);
            renamed = true;
        }
    }
    let template_values = parse(default_ini);

    let missing_count = template_values
        .keys()
        .filter(|k| !existing_values.contains_key(*k))
        .count();
    let mut legacy: Vec<(&String, &String)> = existing_values
        .iter()
        .filter(|(k, _)| !template_values.contains_key(*k))
        .collect();
    if missing_count == 0 && legacy.is_empty() && !renamed {
        return 0;
    }

    let mut merged = String::new();
    for line in default_ini.lines() {
        match split_kv(line).and_then(|(key, _)| Some((key, existing_values.get(key)?))) {
            Some((key, value)) => {
                let _ = writeln!(merged, "{key}={value}");
            }
            None => {
                merged.push_str(line);
                merged.push('\n');
            }
        }
    }

    if !legacy.is_empty() {
        legacy.sort();
        merged.push_str(
            "\n[Legacy]\n\
             ; Keys below no longer exist in the shipped default ini template\n\
             ; (renamed or removed in a newer version) - kept here instead of being\n\
             ; silently discarded. Safe to delete once you've moved the value elsewhere.\n",
        );
        for (key, value) in legacy {
            let _ = writeln!(merged, "{key}={value}");
        }
    }

    let _ = fs::write(ini_path, merged);
    missing_count
}

/// Writes `contents` to `path` through a temporary file next to it and a rename,
/// so a crash or a second writer never leaves a half-written config behind.
pub(crate) fn write_atomic(path: &str, contents: &str) -> std::io::Result<()> {
    let temp = format!("{path}.tmp");
    fs::write(&temp, contents)?;
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

/// Writes `values` into `ini_path` and the shared config map, for a mod that
/// saves state changed in game (e.g. a menu position). An existing
/// `key=value` line is rewritten in place so comments and layout survive; a
/// key with no line yet is appended. Returns `false` (map still updated) if
/// the file couldn't be read or written.
pub fn set_values(ini_path: &str, values: &[(&str, String)]) -> bool {
    {
        let mut map = VALUES.write().unwrap();
        for (key, value) in values {
            map.insert((*key).to_string(), value.clone());
        }
    }
    let Ok(content) = fs::read_to_string(ini_path) else {
        return false;
    };
    let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let mut pending: Vec<&(&str, String)> = values.iter().collect();
    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        let slot = split_kv(line).and_then(|(key, _)| pending.iter().position(|(pk, _)| *pk == key));
        match slot {
            Some(pos) => {
                let (key, value) = pending.remove(pos);
                let indent = &line[..line.len() - line.trim_start().len()];
                lines.push(format!("{indent}{key}={value}"));
            }
            None => lines.push(line.to_string()),
        }
    }
    for (key, value) in pending {
        lines.push(format!("{key}={value}"));
    }
    let mut out = lines.join(newline);
    out.push_str(newline);
    write_atomic(ini_path, &out).is_ok()
}

pub fn get_string(key: &str, default: &str) -> String {
    VALUES
        .read()
        .unwrap()
        .get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn get_parsed<T: FromStr>(key: &str, default: T) -> T {
    VALUES
        .read()
        .unwrap()
        .get(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn get_int(key: &str, default: i32) -> i32 {
    get_parsed(key, default)
}

pub fn get_double(key: &str, default: f64) -> f64 {
    get_parsed(key, default)
}

pub fn get_bool(key: &str, default: bool) -> bool {
    match get_string(key, "").to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => true,
        "0" | "false" | "no" => false,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn trailing_comments_are_not_part_of_the_value() {
        let parsed = parse(
            "WarpWhenFar=true    ; Teleport far spirits\n\
             WarpDistance=25     ; metres\n\
             Tab=1\t# tab then hash\n\
             Empty=   ; nothing here\n\
             Glued=a;b\n\
             Hash=a#b\n\
             Milestone=1800:5000,3600:10000 ; ok\n\
             ; whole-line comment\n\
             Plain=7\n",
        );
        assert_eq!(parsed["WarpWhenFar"], "true");
        assert_eq!(parsed["WarpDistance"], "25");
        assert_eq!(parsed["Tab"], "1");
        assert_eq!(parsed["Empty"], "");
        assert_eq!(parsed["Glued"], "a;b");
        assert_eq!(parsed["Hash"], "a#b");
        assert_eq!(parsed["Milestone"], "1800:5000,3600:10000");
        assert_eq!(parsed["Plain"], "7");
        assert!(!parsed.contains_key("; whole-line comment"));
    }

    // A small stand-in template, not any real mod's ini - this crate has no
    // ini of its own, so the migrate() tests exercise it against a fixture
    // instead of a real `include_str!`-embedded default.
    const TEST_TEMPLATE: &str = "\
[General]
ReloadKey=F5

[Section]
Hp=0
Fp=0
";

    /// Unique-per-test scratch file under the OS temp dir, so tests running
    /// in parallel don't clobber each other. Best-effort cleanup on drop.
    struct TempIni(PathBuf);

    impl TempIni {
        fn new(name: &str, content: &str) -> Self {
            let path = std::env::temp_dir().join(format!("common_config_test_{name}.ini"));
            fs::write(&path, content).unwrap();
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }

        fn read(&self) -> String {
            fs::read_to_string(&self.0).unwrap()
        }
    }

    impl Drop for TempIni {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn set_values_rewrites_in_place_and_appends_missing() {
        let ini = TempIni::new("set_values", "[General]\n; comment\nReloadKey=F5\n\n[Section]\nHp=0\n");
        assert!(set_values(ini.path(), &[("Hp", "12".into()), ("New", "x".into())]));
        let text = ini.read();
        assert_eq!(text, "[General]\n; comment\nReloadKey=F5\n\n[Section]\nHp=12\nNew=x\n");
        assert_eq!(get_string("Hp", ""), "12");
    }

    #[test]
    fn migrate_adds_missing_key_with_template_default() {
        let without_reload_key: String = TEST_TEMPLATE
            .lines()
            .filter(|l| !l.trim_start().starts_with("ReloadKey="))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!parse(&without_reload_key).contains_key("ReloadKey"));

        let ini = TempIni::new("adds_missing", &without_reload_key);
        let migrated = migrate(ini.path(), TEST_TEMPLATE);
        assert_eq!(migrated, 1);

        let merged = parse(&ini.read());
        assert_eq!(merged.get("ReloadKey").map(String::as_str), Some("F5"));
    }

    #[test]
    fn migrate_preserves_existing_customized_values() {
        let customized = TEST_TEMPLATE.replace("Hp=0", "Hp=25");
        let ini = TempIni::new("preserves_custom", &customized);

        // Nothing missing yet - migrate must be a no-op.
        assert_eq!(migrate(ini.path(), TEST_TEMPLATE), 0);
        assert_eq!(ini.read(), customized);

        // Now also drop a different key so migrate actually rewrites the
        // file, and confirm the customized value survives the rewrite.
        let with_gap: String = customized
            .lines()
            .filter(|l| !l.trim_start().starts_with("Fp="))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&ini.0, &with_gap).unwrap();

        let migrated = migrate(ini.path(), TEST_TEMPLATE);
        assert_eq!(migrated, 1);
        let merged = parse(&ini.read());
        assert_eq!(merged.get("Hp").map(String::as_str), Some("25"));
    }

    #[test]
    fn migrate_moves_unknown_keys_to_legacy_section_instead_of_dropping_them() {
        let with_gap: String = TEST_TEMPLATE
            .lines()
            .filter(|l| !l.trim_start().starts_with("ReloadKey="))
            .collect::<Vec<_>>()
            .join("\n");
        let with_stale_key = format!("{with_gap}\nSomeRemovedFeature.Flag=true\n");
        let ini = TempIni::new("legacy_section", &with_stale_key);

        migrate(ini.path(), TEST_TEMPLATE);

        let content = ini.read();
        assert!(content.contains("[Legacy]"));
        let merged = parse(&content);
        assert_eq!(
            merged.get("SomeRemovedFeature.Flag").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn migrate_moves_stale_key_to_legacy_even_with_nothing_missing() {
        // Regression: the legacy check used to be short-circuited by the
        // "nothing missing" no-op return, so a stale key sat there forever
        // if the template itself hadn't gained any new keys since.
        let with_stale_key = format!("{TEST_TEMPLATE}\nSomeRemovedFeature.Flag=true\n");
        let ini = TempIni::new("stale_only", &with_stale_key);

        let migrated = migrate(ini.path(), TEST_TEMPLATE);

        assert_eq!(migrated, 0); // no new template key was merged
        let content = ini.read();
        assert!(content.contains("[Legacy]"));
        let merged = parse(&content);
        assert_eq!(
            merged.get("SomeRemovedFeature.Flag").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn migrate_carries_renamed_key_value_over() {
        let old_file = TEST_TEMPLATE.replace("Hp=0", "OldHp=25");
        let ini = TempIni::new("rename", &old_file);

        let migrated = migrate_with_renames(ini.path(), TEST_TEMPLATE, &[("OldHp", "Hp")]);

        assert_eq!(migrated, 0); // renamed, not new
        let content = ini.read();
        assert!(!content.contains("OldHp"));
        let merged = parse(&content);
        assert_eq!(merged.get("Hp").map(String::as_str), Some("25"));
    }

    #[test]
    fn migrate_is_a_noop_when_nothing_is_missing() {
        let ini = TempIni::new("noop", TEST_TEMPLATE);
        assert_eq!(migrate(ini.path(), TEST_TEMPLATE), 0);
        assert_eq!(ini.read(), TEST_TEMPLATE);
    }
}
