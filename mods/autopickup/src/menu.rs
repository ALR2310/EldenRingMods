//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! Changes are saved to `AutoPickup.toml` a second after the player stops, then re-read.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::logger;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;
use crate::range::{MAX_RANGE, MIN_RANGE};

const RANGE_STEP: f64 = 0.1;

struct GroupSettings {
    enabled: Setting,
    range: NumberSetting,
}

macro_rules! group_settings {
    ($section:literal) => {
        GroupSettings {
            enabled: Setting::flag(concat!($section, ".Enabled")),
            range: NumberSetting::new(concat!($section, ".Range"), MIN_RANGE, MAX_RANGE, None)
                .stepped(RANGE_STEP),
        }
    };
}

static MATERIALS: GroupSettings = group_settings!("Materials");
static ITEMS: GroupSettings = group_settings!("Items");
static CORPSE_LOOT: GroupSettings = group_settings!("CorpseLoot");
static LOST_RUNES: GroupSettings = group_settings!("LostRunes");
static CHESTS: GroupSettings = group_settings!("Chests");
static PAUSE_IN_COMBAT: Setting = Setting::flag("General.PauseInCombat");
static NO_PICKUP_ANIMATION: Setting = Setting::flag("General.NoPickupAnimation");
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

/// The mod follows its config on its own; only the log switch needs following here.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

/// A macro because the row readers must be plain `fn`s (no captures).
macro_rules! group_page {
    ($b:expr, $parent:expr, $title:expr, $help:expr, $settings:expr, $field:ident) => {{
        let page = $b.submenu($parent, $title, $help)?;
        $b.toggle(page, &t("enabled.label"), &t("enabled.help"), &$settings.enabled, || {
            config::get().$field.enabled as u8
        })?;
        $b.number(page, &t("range.label"), &t("range.help"), &$settings.range, || {
            config::get().$field.range
        })?;
    }};
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Auto Pickup", &t("mod.help"))?;

    group_page!(b, page, &t("materials.title"), &t("materials.help"), MATERIALS, materials);
    group_page!(b, page, &t("items.title"), &t("items.help"), ITEMS, items);
    group_page!(b, page, &t("corpse_loot.title"), &t("corpse_loot.help"), CORPSE_LOOT, corpse_loot);
    group_page!(b, page, &t("lost_runes.title"), &t("lost_runes.help"), LOST_RUNES, lost_runes);
    group_page!(b, page, &t("chests.title"), &t("chests.help"), CHESTS, chests);

    b.toggle(page, &t("combat.label"), &t("combat.help"), &PAUSE_IN_COMBAT, || {
        config::get().general.pause_in_combat as u8
    })?;
    b.toggle(page, &t("animation.label"), &t("animation.help"), &NO_PICKUP_ANIMATION, || {
        config::get().general.no_pickup_animation as u8
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "auto-pickup", "Auto Pickup", build);
}

pub fn sync_from_config() {
    menu::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_match_the_range_clamp() {
        assert_eq!((MATERIALS.range.min, MATERIALS.range.max), (MIN_RANGE, MAX_RANGE));
    }

    /// Every key the menu uses is in every language file, and the files have the same keys.
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 21);
    }
}
