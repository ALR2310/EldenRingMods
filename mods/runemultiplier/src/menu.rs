//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Rune Multiplier            -> opens a page with:
//!                                 Multiplier   number -1000..1000 (typed)
//!                                 Log file     on/off
//! ```
//!
//! The multiplier is a text row where a number is typed (fractions such as
//! 1.5, 0 and negative values work, like in the ini). Changes apply at once
//! and are written back to the ini shortly after the player stops, so they
//! survive a restart.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, NumberSetting, Setting};

/// Bounds of the typed multiplier. Negative values subtract runes (the mod
/// accepts them), 0 gives none.
const MIN_MULTIPLIER: f64 = -1000.0;
const MAX_MULTIPLIER: f64 = 1000.0;

static MULTIPLIER: NumberSetting =
    NumberSetting::new("Multiplier", MIN_MULTIPLIER, MAX_MULTIPLIER, Some(crate::hook::set_multiplier));
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn build(b: &mut erui::MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Rune Multiplier", "Settings for the Rune Multiplier mod.")?;
    b.number(
        page,
        "Multiplier",
        "Multiplies rune gains. 1 = vanilla, 2 = double, 0 = none, negative takes runes away.",
        &MULTIPLIER,
        || config::get_double("Multiplier", 1.0),
    )?;
    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, || {
        config::get_bool("LogFile", false) as u8
    })?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "rune-multiplier", "Rune Multiplier", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
