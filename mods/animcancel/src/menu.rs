//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! Changes are saved to `AnimCancel.toml` a second after the player stops, then re-read. The probes
//! stay file-only.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::logger;
use shared::menu::{self, MenuBuilder, Setting};

use crate::config;

struct ActionSettings {
    enabled: Setting,
    attack: Setting,
    skill: Setting,
    cast: Setting,
    item: Setting,
}

macro_rules! action_settings {
    ($section:literal) => {
        ActionSettings {
            enabled: Setting::flag(concat!($section, ".Enabled")),
            attack: Setting::flag(concat!($section, ".Attack")),
            skill: Setting::flag(concat!($section, ".Skill")),
            cast: Setting::flag(concat!($section, ".Cast")),
            item: Setting::flag(concat!($section, ".Item")),
        }
    };
}

static GUARD: ActionSettings = action_settings!("Guard");
static DODGE: ActionSettings = action_settings!("Dodge");
static WEAPON_SWAP: ActionSettings = action_settings!("WeaponSwap");
static JUMP: ActionSettings = action_settings!("Jump");
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

/// The mod reads its config every frame; only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

/// A macro because the row readers must be plain `fn`s (no captures).
macro_rules! action_page {
    ($b:expr, $parent:expr, $title:expr, $help:expr, $settings:expr, $field:ident) => {{
        let page = $b.submenu($parent, $title, $help)?;
        $b.toggle(page, &t("enabled.label"), &t("enabled.help"), &$settings.enabled, || {
            config::get().$field.enabled as u8
        })?;
        $b.toggle(page, &t("attacks.label"), &t("attacks.help"), &$settings.attack, || {
            config::get().$field.attack as u8
        })?;
        $b.toggle(page, &t("skills.label"), &t("skills.help"), &$settings.skill, || {
            config::get().$field.skill as u8
        })?;
        $b.toggle(page, &t("spells.label"), &t("spells.help"), &$settings.cast, || {
            config::get().$field.cast as u8
        })?;
        $b.toggle(page, &t("items.label"), &t("items.help"), &$settings.item, || {
            config::get().$field.item as u8
        })?;
    }};
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Anim Cancel", &t("mod.help"))?;

    action_page!(b, page, &t("guard.title"), &t("guard.help"), GUARD, guard);
    action_page!(b, page, &t("dodge.title"), &t("dodge.help"), DODGE, dodge);
    action_page!(
        b,
        page,
        &t("weapon_swap.title"),
        &t("weapon_swap.help"),
        WEAPON_SWAP,
        weapon_swap
    );
    action_page!(b, page, &t("jump.title"), &t("jump.help"), JUMP, jump);

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "anim-cancel", "Anim Cancel", build);
}

pub fn sync_from_config() {
    menu::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu uses is in every language file, and the files have the same keys.
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 14);
    }
}
