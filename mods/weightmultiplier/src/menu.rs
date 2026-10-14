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
use std::time::Duration;

use common::erui::{self, NumberSetting, Registration, RowHandle, Setting};
use common::{config, logger};

fn number(value: u8) -> String {
    value.to_string()
}

static MODE: Setting = Setting { key: "Mode", format: number, apply: None };
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", -1000.0, 1000.0, None);
static FIXED_VALUE: NumberSetting = NumberSetting::new("FixedValue", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting { key: "LogFile", format: |value| (value != 0).to_string(), apply: None };

fn mode_row() -> u8 {
    config::get_int("Mode", 0).clamp(0, 1) as u8
}

fn multiplier_value() -> f64 {
    config::get_double("Multiplier", 0.5)
}

fn fixed_value() -> f64 {
    config::get_double("FixedValue", 0.0)
}

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

struct Rows {
    registration: Registration,
    /// Each toggle/choice row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: Vec<(&'static NumberSetting, fn() -> f64)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// After the ini is written: apply it to the hook, like the reload key does.
fn apply_saved() {
    crate::hook::reload_from_config();
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("weight-multiplier", "Weight Multiplier", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Weight Multiplier",
        "Settings for the Weight Multiplier mod.",
        "Weight Multiplier",
        "Settings for the Weight Multiplier mod.",
    )?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();

    rows.push((
        provider.add_inline_choice(
            page,
            "Mode",
            "Multiply: your equip load is multiplied. Fixed value: your equip load is always one number.",
            &["Multiply", "Fixed value"],
            mode_row(),
            erui::setting_changed,
            MODE.user_data(),
        )?,
        mode_row,
    ));
    provider.add_number(
        page,
        "Multiplier",
        "Multiplies your equip load, used when Mode is Multiply. 1 = vanilla.",
        &MULTIPLIER,
        multiplier_value(),
    )?;
    provider.add_number(
        page,
        "Fixed Value",
        "The equip load you always have, used when Mode is Fixed value.",
        &FIXED_VALUE,
        fixed_value(),
    )?;
    rows.push((
        provider.add_toggle(
            page,
            "Log file",
            "Write a log file next to the DLL, for troubleshooting.",
            log_file_row() != 0,
            erui::setting_changed,
            LOG_FILE.user_data(),
        )?,
        log_file_row,
    ));
    let registration = provider.commit()?;
    let numbers: Vec<(&'static NumberSetting, fn() -> f64)> =
        vec![(&MULTIPLIER, multiplier_value), (&FIXED_VALUE, fixed_value)];
    Ok(Rows { registration, rows, numbers })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(apply_saved));
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
        for (row, read) in &rows.rows {
            if let Err(reason) = rows.registration.set_row_value(*row, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
        for (setting, read) in &rows.numbers {
            if let Err(reason) = setting.show(&rows.registration, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}
