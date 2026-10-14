//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Weight Multiplier          -> opens a page with:
//!   Mode                       Multiply / Fixed value
//!   Multiplier                 number -1000..1000 (used in Multiply mode)
//!   Fixed Value                number -1000..1000 (used in Fixed value mode)
//!   Log file                   on/off
//! ```
//!
//! This mod has no game task: its own reload (the ReloadKey poll thread) just
//! re-reads the config and stores the new value for the hook's stub. A menu
//! change takes the same path (`hook::reload_from_config`) once it has been
//! written to the ini (about a second after the player stops changing rows).
//! The multiplier and the fixed value are typed into text rows.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};

static MODE: Setting = Setting::int("Mode");
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", -1000.0, 1000.0, None);
static FIXED_VALUE: NumberSetting = NumberSetting::new("FixedValue", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn mode_row() -> u8 {
    config::get_int("Mode", 0).clamp(0, 1) as u8
}

fn fixed_value() -> f64 {
    config::get_double("FixedValue", 0.0)
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

/// After the ini is written: apply it to the hook, like the reload key does.
fn apply_saved() {
    crate::hook::reload_from_config();
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Weight Multiplier", "Settings for the Weight Multiplier mod.")?;
    b.choice(
        page,
        "Mode",
        "Multiply: your equip load is multiplied. Fixed value: your equip load is always one number.",
        &["Multiply", "Fixed value"],
        &MODE,
        mode_row,
    )?;
    b.number(
        page,
        "Multiplier",
        "Multiplies your equip load, used when Mode is Multiply. 1 = vanilla.",
        &MULTIPLIER,
        crate::hook::configured_multiplier,
    )?;
    b.number(
        page,
        "Fixed Value",
        "The equip load you always have, used when Mode is Fixed value.",
        &FIXED_VALUE,
        fixed_value,
    )?;
    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, log_file_row)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(apply_saved));
    erui::spawn_menu(&MENU, hmodule, "weight-multiplier", "Weight Multiplier", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
