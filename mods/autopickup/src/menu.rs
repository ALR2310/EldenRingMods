//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the TOML
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Auto Pickup                -> opens a page with:
//!   Materials / Items / Corpse loot / Lost runes / Chests -> each a page with: Enabled, Range
//!   Pause in combat          on/off
//!   Log file                 on/off
//! ```
//!
//! Changes are written to `AutoPickup.toml` (comments kept) once the player
//! stops for a second, and the mod applies them when it re-reads the file right
//! after (about a second after the player stops).

use std::sync::OnceLock;

use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::i18n::Catalog;
use common::logger;

use crate::config;
use crate::range::{MAX_RANGE, MIN_RANGE};

/// Slider step: 100 positions from 0.1 to 10.
const RANGE_STEP: f64 = 0.1;

/// The two settings of one group's page; `$section` is its TOML section.
struct GroupSettings {
    enabled: Setting,
    range: NumberSetting,
}

macro_rules! group_settings {
    ($section:literal) => {
        GroupSettings {
            enabled: Setting::flag(concat!($section, ".Enabled")),
            range: NumberSetting::float(concat!($section, ".Range"), MIN_RANGE, MAX_RANGE, None).stepped(RANGE_STEP),
        }
    };
}

static MATERIALS: GroupSettings = group_settings!("Materials");
static ITEMS: GroupSettings = group_settings!("Items");
static CORPSE_LOOT: GroupSettings = group_settings!("CorpseLoot");
static LOST_RUNES: GroupSettings = group_settings!("LostRunes");
static CHESTS: GroupSettings = group_settings!("Chests");
static PAUSE_IN_COMBAT: Setting = Setting::flag("General.PauseInCombat");
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

/// The menu texts, one file per language (`locales/`); English is the fallback.
static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

/// The text of `key` in the host language.
fn t(key: &str) -> String {
    TEXT.tr(key)
}

/// After the file is written (and re-read): the mod follows its config on its
/// own, so only the log switch needs following here.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

/// One group's page: the switch, then the range. A macro because the row
/// readers must be plain `fn`s (no captures).
macro_rules! group_page {
    ($b:expr, $parent:expr, $title:expr, $help:expr, $settings:expr, $field:ident) => {{
        let page = $b.submenu($parent, $title, $help)?;
        $b.toggle(page, &t("enabled.label"), &t("enabled.help"), &$settings.enabled, || config::get().$field.enabled as u8)?;
        $b.number(page, &t("range.label"), &t("range.help"), &$settings.range, || config::get().$field.range)?;
    }};
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Auto Pickup", &t("mod.help"))?;

    group_page!(b, page, &t("materials.title"), &t("materials.help"), MATERIALS, materials);
    group_page!(b, page, &t("items.title"), &t("items.help"), ITEMS, items);
    group_page!(b, page, &t("corpse_loot.title"), &t("corpse_loot.help"), CORPSE_LOOT, corpse_loot);
    group_page!(b, page, &t("lost_runes.title"), &t("lost_runes.help"), LOST_RUNES, lost_runes);
    group_page!(b, page, &t("chests.title"), &t("chests.help"), CHESTS, chests);

    b.toggle(page, &t("combat.label"), &t("combat.help"), &PAUSE_IN_COMBAT, || config::get().general.pause_in_combat as u8)?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || config::get().logging.log_file as u8)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(config::save, Some(saved));
    erui::spawn_menu(&MENU, hmodule, "auto-pickup", "Auto Pickup", build);
}

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    erui::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_rows_are_float_rows() {
        // A TOML float field: a typed `2` must be saved as `2.0`.
        for group in [&MATERIALS, &ITEMS, &CORPSE_LOOT, &LOST_RUNES, &CHESTS] {
            assert!(group.range.decimal_point, "{}", group.range.key);
        }
    }

    #[test]
    fn bounds_match_the_range_clamp() {
        assert_eq!((MATERIALS.range.min, MATERIALS.range.max), (MIN_RANGE, MAX_RANGE));
    }

    /// Every key the menu passes to `t` is in every language file, and the files
    /// have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 19);
    }
}
