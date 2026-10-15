//! Text in more than one language for the in-game menu (English and
//! Vietnamese). English is the source text and the fallback: a mod writes its
//! labels in English and wraps them in [`Catalog::tr`]; a missing translation
//! (or any other language) shows the English text. Details and the file layout:
//! `shared/docs/i18n.md`.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::logger;

static VIETNAMESE: AtomicBool = AtomicBool::new(false);

/// Sets the language from the ERNativeUI host's exact identifier (called by
/// `erui::connect`). Only `"vietnamese"` selects a translation.
pub fn set_language(identifier: &str) {
    VIETNAMESE.store(identifier == "vietnamese", Ordering::Relaxed);
}

/// Whether the menu text is shown in Vietnamese.
pub fn is_vietnamese() -> bool {
    VIETNAMESE.load(Ordering::Relaxed)
}

/// `English text = "Vietnamese text"` pairs, parsed from a TOML file.
pub fn parse_catalog(text: &str) -> Result<HashMap<String, String>, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// One mod's Vietnamese texts, embedded in the DLL. Declare it as a `static`:
///
/// ```ignore
/// static TEXT: Catalog = Catalog::new(include_str!("../locales/vi.toml"));
/// ```
pub struct Catalog {
    vi: &'static str,
    parsed: OnceLock<HashMap<String, String>>,
}

impl Catalog {
    pub const fn new(vi: &'static str) -> Self {
        Catalog { vi, parsed: OnceLock::new() }
    }

    fn vietnamese(&self) -> &HashMap<String, String> {
        self.parsed.get_or_init(|| {
            parse_catalog(self.vi).unwrap_or_else(|reason| {
                logger::log(&format!("Menu translation file is invalid ({reason}); showing English."));
                HashMap::new()
            })
        })
    }

    /// `english` itself, or its Vietnamese translation when the language is
    /// Vietnamese and the catalog has one.
    pub fn tr(&self, english: &str) -> String {
        if is_vietnamese()
            && let Some(text) = self.vietnamese().get(english)
        {
            return text.clone();
        }
        english.to_string()
    }

    /// The English texts the catalog translates (for tests that compare it
    /// with the texts a menu uses).
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.vietnamese().keys().cloned().collect();
        keys.sort();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CATALOG: Catalog = Catalog::new("\"Log file\" = \"Tệp log\"\n\"Torrent's speed\" = \"Tốc độ của Torrent\"\n");

    // One test: the language is process-wide state.
    #[test]
    fn translates_only_in_vietnamese_and_falls_back_to_english() {
        set_language("english");
        assert_eq!(CATALOG.tr("Log file"), "Log file");

        set_language("vietnamese");
        assert_eq!(CATALOG.tr("Log file"), "Tệp log");
        assert_eq!(CATALOG.tr("Torrent's speed"), "Tốc độ của Torrent");
        assert_eq!(CATALOG.tr("Not translated"), "Not translated");

        set_language("french");
        assert_eq!(CATALOG.tr("Log file"), "Log file");
        assert_eq!(CATALOG.keys(), ["Log file", "Torrent's speed"]);
    }

    #[test]
    fn a_broken_catalog_shows_english() {
        assert!(parse_catalog("not toml =").is_err());
        let broken = Catalog::new("\"a\" = ");
        assert!(broken.vietnamese().is_empty());
    }
}
