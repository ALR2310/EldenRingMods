//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Passive Runes              -> opens a page with:
//!   Enabled                    on/off
//!   Interval (ms)              number 0..3600000 (0 = off)
//!   Amount                     number -100000..100000
//!   Next Level (%)             number -100..100
//!   Interest (%)               number -100..100
//!   Log file                   on/off
//! ```
//!
//! The mod re-reads its ini keys every tick, so a change applies as soon as
//! it is written (shortly after the player stops changing rows). Numbers are
//! typed into text rows. The milestone list stays ini-only.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, Setting};

static ENABLED: Setting = Setting::flag("Rune.Passive.Enabled");
static INTERVAL: NumberSetting = NumberSetting::whole("Rune.Passive.Interval", 0.0, 3_600_000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Rune.Passive.Amount", -100_000.0, 100_000.0, None);
static PERCENT: NumberSetting = NumberSetting::new("Rune.Passive.Percent", -100.0, 100.0, None);
static INTEREST: NumberSetting = NumberSetting::new("Rune.Passive.Interest", -100.0, 100.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn enabled_row() -> u8 {
    config::get_bool("Rune.Passive.Enabled", true) as u8
}
fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Passive Runes", "Settings for the Passive Runes mod.")?;
    b.toggle(page, "Enabled", "Turns the passive rune rewards on or off.", &ENABLED, enabled_row)?;
    b.number(
        page,
        "Interval (ms)",
        "How often you get runes, in milliseconds (1000 = 1 second). 0 pauses the interval rewards.",
        &INTERVAL,
        || config::get_double("Rune.Passive.Interval", 10000.0),
    )?;
    b.number(page, "Amount", "Runes you get each time. Negative takes runes away.", &AMOUNT, || {
        config::get_double("Rune.Passive.Amount", 50.0)
    })?;
    b.number(
        page,
        "Next Level (%)",
        "Each time, also get this percent of the runes needed for your next level. Negative takes runes away.",
        &PERCENT,
        || config::get_double("Rune.Passive.Percent", 0.25),
    )?;
    b.number(
        page,
        "Interest (%)",
        "Each time, also get this percent of the runes you hold. Grows fast. Negative takes runes away.",
        &INTEREST,
        || config::get_double("Rune.Passive.Interest", 0.0),
    )?;
    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, log_file_row)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    // The mod reads every key live each tick, so saving is all it takes.
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "passive-runes", "Passive Runes", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
