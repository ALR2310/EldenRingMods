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
use std::time::Duration;

use common::erui::{self, NumberSetting, Registration, RowHandle, Setting};
use common::{config, logger};

/// Bounds of the typed multiplier. Negative values subtract runes (the mod
/// accepts them), 0 gives none.
const MIN_MULTIPLIER: f64 = -1000.0;
const MAX_MULTIPLIER: f64 = 1000.0;

static MULTIPLIER: NumberSetting =
    NumberSetting::new("Multiplier", MIN_MULTIPLIER, MAX_MULTIPLIER, Some(crate::hook::set_multiplier));
static LOG_FILE: Setting = Setting { key: "LogFile", format: |value| (value != 0).to_string(), apply: None };

struct Rows {
    registration: Registration,
    log_file: RowHandle,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

fn multiplier_value() -> f64 {
    config::get_double("Multiplier", 1.0)
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("rune-multiplier", "Rune Multiplier", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Rune Multiplier",
        "Settings for the Rune Multiplier mod.",
        "Rune Multiplier",
        "Settings for the Rune Multiplier mod.",
    )?;
    provider.add_number(
        page,
        "Multiplier",
        "Multiplies rune gains. 1 = vanilla, 2 = double, 0 = none, negative takes runes away.",
        &MULTIPLIER,
        multiplier_value(),
    )?;
    let log_file = provider.add_toggle(
        page,
        "Log file",
        "Write a log file next to the DLL, for troubleshooting.",
        log_file_row() != 0,
        erui::setting_changed,
        LOG_FILE.user_data(),
    )?;
    let registration = provider.commit()?;
    Ok(Rows { registration, log_file })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, None);
    std::thread::spawn(move || {
        let connection = match erui::connect(Duration::from_secs(10)) {
            Ok(c) => c,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the ini only."));
                return;
            }
        };
        match build(hmodule, connection) {
            Ok(rows) => {
                let _ = ROWS.set(rows);
                logger::log("In-game menu: registered.");
            }
            Err(reason) => logger::log(&format!("In-game menu registration failed: {reason}")),
        }
    });
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(rows) = ROWS.get() {
        if let Err(reason) = rows.registration.set_row_value(rows.log_file, log_file_row()) {
            logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
        }
        if let Err(reason) = MULTIPLIER.show(&rows.registration, multiplier_value()) {
            logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
        }
    }
}
