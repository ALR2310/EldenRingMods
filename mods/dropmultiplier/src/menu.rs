//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Drop Multiplier            -> opens a page with:
//!   Drop Chance              -> Mode        Multiply chance / Fixed chance
//!                               Multiplier  number 0..1000   (Mode: multiply)
//!                               Percentage  slider 0..100    (Mode: fixed)
//!   Materials                -> Crafting     number 0..1000
//!                               Upgrade      number 0..1000
//!                               Unique       number 0..1000
//!   Log file                 on/off
//! ```
//!
//! Multipliers are text rows where a number is typed (the ini holds
//! decimals); the percentage is a slider (whole percent). Changes are written to the ini once the player stops
//! for a second, then the mod is told to re-apply from its original drop
//! tables - the same path as the reload key - so it rescans once, not on
//! every edit.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};

/// Upper bound of the typed multipliers (the mod only clamps at 0 below).
const MAX_MULTIPLIER: f64 = 1000.0;

static MENU: OnceLock<erui::Menu> = OnceLock::new();

static MODE: Setting = Setting::int("Mode");
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", 0.0, MAX_MULTIPLIER, None);
static PERCENTAGE: Setting = Setting::int("Percentage");
static CRAFTING: NumberSetting = NumberSetting::new("Crafting", 0.0, MAX_MULTIPLIER, None);
static UPGRADE: NumberSetting = NumberSetting::new("Upgrade", 0.0, MAX_MULTIPLIER, None);
static UNIQUE: NumberSetting = NumberSetting::new("Unique", 0.0, MAX_MULTIPLIER, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

fn mode_row() -> u8 {
    config::get_int("Mode", 0).clamp(0, 1) as u8
}

/// The fixed chance as a slider position (whole percent, 0..=100).
fn percentage_row() -> u8 {
    config::get_double("Percentage", 50.0).round().clamp(0.0, 100.0) as u8
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Drop Multiplier", "Settings for the Drop Multiplier mod.")?;
    let drops = b.submenu(page, "Drop Chance", "How likely enemies are to drop items.")?;
    let materials = b.submenu(page, "Materials", "How many materials you get from each drop or pickup.")?;

    b.choice(
        drops,
        "Mode",
        "Multiply chance: an item's drop chance is multiplied. Fixed chance: every drop is set to one chance.",
        &["Multiply chance", "Fixed chance"],
        &MODE,
        mode_row,
    )?;
    b.number(
        drops,
        "Multiplier",
        "Drop-rate multiplier, used when Mode is Multiply chance. 1 = vanilla, 0 = no drops.",
        &MULTIPLIER,
        || config::get_double("Multiplier", 2.0),
    )?;
    b.slider(
        drops,
        "Percentage (%)",
        "Fixed drop chance in percent, used when Mode is Fixed chance. 100 guarantees a drop.",
        0,
        100,
        &PERCENTAGE,
        percentage_row,
    )?;
    b.number(
        materials,
        "Crafting",
        "Multiplies the crafting materials you get (Rowa Fruit, Herba, bones...). 1 = vanilla.",
        &CRAFTING,
        || config::get_double("Crafting", 1.0),
    )?;
    b.number(
        materials,
        "Upgrade",
        "Multiplies the upgrade materials you get (Smithing Stones, Gloveworts...). 1 = vanilla.",
        &UPGRADE,
        || config::get_double("Upgrade", 1.0),
    )?;
    b.number(
        materials,
        "Unique",
        "Multiplies materials found only once (Sacred Tears, Scadutree Fragments...). 1 = vanilla.",
        &UNIQUE,
        || config::get_double("Unique", 1.0),
    )?;
    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, log_file_row)?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(erui::reload_from_menu));
    erui::spawn_menu(&MENU, hmodule, "drop-multiplier", "Drop Multiplier", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
