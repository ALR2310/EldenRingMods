//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the TOML
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Anim Cancel                -> opens a page with:
//!   Guard / Dodge / Weapon swap -> each a page with: Enabled, Attacks, Skills,
//!                                  Spells, Items
//!   Log file                 on/off
//! ```
//!
//! Changes are written to `AnimCancel.toml` (comments kept) once the player
//! stops for a second, and the mod applies them when it re-reads the file right
//! after (about a second after the player stops): it reads its config snapshot
//! every frame. The probes stay file-only.

use std::sync::OnceLock;

use common::erui::{self, MenuBuilder, Setting};
use common::logger;

use crate::config;

/// The five switches of one action's page; `$section` is its TOML section.
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
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

/// After the file is written (and re-read): the mod reads its config every
/// frame, so only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

/// One action's page: the switch for the action, then the animations it may
/// cancel. A macro because the row readers must be plain `fn`s (no captures).
macro_rules! action_page {
    ($b:expr, $parent:expr, $title:expr, $help:expr, $settings:expr, $field:ident) => {{
        let page = $b.submenu($parent, $title, $help)?;
        $b.toggle(page, "Enabled", "Turn this action's cancelling on or off.", &$settings.enabled, || config::get().$field.enabled as u8)?;
        $b.toggle(page, "Attacks", "Can cancel normal attacks.", &$settings.attack, || config::get().$field.attack as u8)?;
        $b.toggle(page, "Skills", "Can cancel weapon skills.", &$settings.skill, || config::get().$field.skill as u8)?;
        $b.toggle(page, "Spells", "Can cancel casting sorceries and incantations.", &$settings.cast, || config::get().$field.cast as u8)?;
        $b.toggle(page, "Items", "Can cancel using items such as flasks.", &$settings.item, || config::get().$field.item as u8)?;
    }};
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Anim Cancel", "Settings for the Anim Cancel mod.")?;

    action_page!(b, page, "Guard", "Raising your guard cancels the animations you pick.", GUARD, guard);
    action_page!(b, page, "Dodge", "Rolling or backstepping cancels the animations you pick.", DODGE, dodge);
    action_page!(b, page, "Weapon swap", "Switching weapons cancels the animations you pick.", WEAPON_SWAP, weapon_swap);

    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(config::save, Some(saved));
    erui::spawn_menu(&MENU, hmodule, "anim-cancel", "Anim Cancel", build);
}

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
