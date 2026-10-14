//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the TOML
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Anim Cancel                -> opens a page with:
//!   Cancel With              -> Guard, Dodge, Weapon swap
//!   Cancel What              -> Attacks, Skills, Spells, Items
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

static GUARD: Setting = Setting::flag("Cancel.Guard");
static DODGE: Setting = Setting::flag("Cancel.Dodge");
static WEAPON_SWAP: Setting = Setting::flag("Cancel.WeaponSwap");
static ATTACK: Setting = Setting::flag("Animations.Attack");
static SKILL: Setting = Setting::flag("Animations.Skill");
static CAST: Setting = Setting::flag("Animations.Cast");
static ITEM: Setting = Setting::flag("Animations.Item");
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

/// After the file is written (and re-read): the mod reads its config every
/// frame, so only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Anim Cancel", "Settings for the Anim Cancel mod.")?;

    let cancel = b.submenu(page, "Cancel With", "Which actions may cut an animation short.")?;
    b.toggle(cancel, "Guard", "Raising your guard cancels the animation.", &GUARD, || config::get().cancel.guard as u8)?;
    b.toggle(cancel, "Dodge", "Rolling or backstepping cancels the animation.", &DODGE, || config::get().cancel.dodge as u8)?;
    b.toggle(cancel, "Weapon swap", "Switching weapons cancels the animation.", &WEAPON_SWAP, || {
        config::get().cancel.weapon_swap as u8
    })?;

    let animations = b.submenu(page, "Cancel What", "Which animations may be cut short.")?;
    b.toggle(animations, "Attacks", "Normal attacks can be cancelled.", &ATTACK, || config::get().animations.attack as u8)?;
    b.toggle(animations, "Skills", "Weapon skills can be cancelled.", &SKILL, || config::get().animations.skill as u8)?;
    b.toggle(animations, "Spells", "Casting sorceries and incantations can be cancelled.", &CAST, || {
        config::get().animations.cast as u8
    })?;
    b.toggle(animations, "Items", "Using items such as flasks can be cancelled.", &ITEM, || config::get().animations.item as u8)?;

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
