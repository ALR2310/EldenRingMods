//! Text of the in-game menu in more than one language. A mod keeps one JSON
//! file per language (`locales/en.json`, `locales/vi.json`), each with the same
//! keys; the menu code only names a key, e.g. `t("walk.label")`. A key missing
//! in the selected language falls back to English, then to the key itself.
//! Layout and conventions: `shared/docs/i18n.md`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::logger;

/// The language every mod must have a file for; the fallback.
pub const ENGLISH: &str = "english";

static LANGUAGE: Mutex<String> = Mutex::new(String::new());

/// Sets the language from the ERNativeUI host's exact identifier (called by
/// `erui::connect`), e.g. `"vietnamese"`. A language the mod has no file for
/// shows English.
pub fn set_language(identifier: &str) {
    *LANGUAGE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = identifier.to_string();
}

/// The language set by [`set_language`] (empty until the host reported one).
pub fn language() -> String {
    LANGUAGE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

/// Flattens a JSON file into `dotted.key -> text`: `{"walk": {"label": "x"}}`
/// becomes `walk.label = "x"`. Only strings are texts.
pub fn parse_texts(text: &str) -> Result<HashMap<String, String>, String> {
    use serde_json::Value;

    fn collect(prefix: &str, object: &serde_json::Map<String, Value>, out: &mut HashMap<String, String>) {
        for (key, value) in object {
            let name = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
            match value {
                Value::String(text) => {
                    out.insert(name, text.clone());
                }
                Value::Object(inner) => collect(&name, inner, out),
                _ => {}
            }
        }
    }
    let Value::Object(root) = serde_json::from_str(text).map_err(|e| e.to_string())? else {
        return Err("the file must hold one JSON object".to_string());
    };
    let mut texts = HashMap::new();
    collect("", &root, &mut texts);
    Ok(texts)
}

/// One mod's translations, embedded in the DLL. Declare it as a `static`:
///
/// ```ignore
/// static TEXT: Catalog = Catalog::new(&[
///     ("english", include_str!("../locales/en.json")),
///     ("vietnamese", include_str!("../locales/vi.json")),
/// ]);
/// ```
pub struct Catalog {
    files: &'static [(&'static str, &'static str)],
    parsed: OnceLock<HashMap<&'static str, HashMap<String, String>>>,
}

impl Catalog {
    /// `files`: `(host language identifier, JSON text)`, English included.
    pub const fn new(files: &'static [(&'static str, &'static str)]) -> Self {
        Catalog { files, parsed: OnceLock::new() }
    }

    fn languages(&self) -> &HashMap<&'static str, HashMap<String, String>> {
        self.parsed.get_or_init(|| {
            self.files
                .iter()
                .map(|(name, text)| {
                    let texts = parse_texts(text).unwrap_or_else(|reason| {
                        logger::log(&format!("i18n: {name} invalid ({reason})"));
                        HashMap::new()
                    });
                    (*name, texts)
                })
                .collect()
        })
    }

    /// The text of `key` in the current language, else in English, else `key`.
    pub fn tr(&self, key: &str) -> String {
        let languages = self.languages();
        [language().as_str(), ENGLISH]
            .iter()
            .find_map(|name| languages.get(name).and_then(|texts| texts.get(key)))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// The sorted keys of one language's file (for tests that compare the
    /// files with each other and with the keys a menu uses).
    pub fn keys(&self, language: &str) -> Vec<String> {
        let mut keys: Vec<String> = self.languages().get(language).map(|t| t.keys().cloned().collect()).unwrap_or_default();
        keys.sort();
        keys
    }

    /// For a menu's test: every `t("key")` call in `source` (the menu's own
    /// file, `include_str!("menu.rs")`) is in every language file and the files
    /// hold nothing else. `min_keys` guards against the scan finding nothing.
    pub fn assert_matches_source(&self, source: &str, min_keys: usize) {
        let mut used: Vec<String> = Vec::new();
        for (at, _) in source.match_indices("t(\"") {
            if source[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &source[at + 3..];
            if let Some(end) = rest.find('"') {
                used.push(rest[..end].to_string());
            }
        }
        used.sort();
        used.dedup();
        assert!(used.len() >= min_keys, "found only {} menu keys", used.len());
        for (name, _) in self.files {
            assert_eq!(used, self.keys(name), "menu keys and the {name} language file differ");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CATALOG: Catalog = Catalog::new(&[
        ("english", r#"{"log": {"label": "Log file", "help": "Write a log."}, "only_en": {"label": "English only"}}"#),
        ("vietnamese", r#"{"log": {"label": "Tệp log", "help": "Ghi tệp log."}}"#),
    ]);

    // One test: the language is process-wide state.
    #[test]
    fn picks_the_language_and_falls_back_to_english_then_to_the_key() {
        set_language("english");
        assert_eq!(CATALOG.tr("log.label"), "Log file");

        set_language("vietnamese");
        assert_eq!(CATALOG.tr("log.label"), "Tệp log");
        assert_eq!(CATALOG.tr("only_en.label"), "English only");
        assert_eq!(CATALOG.tr("nothing.here"), "nothing.here");

        set_language("french"); // no file: English
        assert_eq!(CATALOG.tr("log.help"), "Write a log.");

        assert_eq!(CATALOG.keys("english"), ["log.help", "log.label", "only_en.label"]);
        assert_eq!(CATALOG.keys("vietnamese"), ["log.help", "log.label"]);
        set_language("");
    }

    #[test]
    fn tables_are_flattened_and_a_broken_file_is_an_error() {
        let texts = parse_texts(r#"{"top": "a", "x": {"y": "b", "z": {"w": "c"}}, "number": 3}"#).unwrap();
        assert_eq!(texts.len(), 3);
        assert_eq!(texts["top"], "a");
        assert_eq!(texts["x.y"], "b");
        assert_eq!(texts["x.z.w"], "c");
        assert!(parse_texts("not json").is_err());
        assert!(parse_texts("[1, 2]").is_err());
        assert!(Catalog::new(&[("english", "{bad")]).keys("english").is_empty());
    }
}
