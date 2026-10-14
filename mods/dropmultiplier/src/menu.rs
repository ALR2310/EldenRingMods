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
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, NumberSetting, Registration, RowHandle, Setting};
use common::{config, logger};

/// Upper bound of the typed multipliers (the mod only clamps at 0 below).
const MAX_MULTIPLIER: f64 = 1000.0;

fn number(value: u8) -> String {
    value.to_string()
}

static MODE: Setting = Setting { key: "Mode", format: number, apply: None };
static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", 0.0, MAX_MULTIPLIER, None);
static PERCENTAGE: Setting = Setting { key: "Percentage", format: number, apply: None };
static CRAFTING: NumberSetting = NumberSetting::new("Crafting", 0.0, MAX_MULTIPLIER, None);
static UPGRADE: NumberSetting = NumberSetting::new("Upgrade", 0.0, MAX_MULTIPLIER, None);
static UNIQUE: NumberSetting = NumberSetting::new("Unique", 0.0, MAX_MULTIPLIER, None);
static LOG_FILE: Setting = Setting { key: "LogFile", format: |value| (value != 0).to_string(), apply: None };

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

struct Rows {
    registration: Registration,
    /// Each row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: Vec<(&'static NumberSetting, fn() -> f64)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// After the ini is written: ask the mod to re-apply, like the reload key.
fn reapply() {
    common::reload::RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("drop-multiplier", "Drop Multiplier", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Drop Multiplier",
        "Settings for the Drop Multiplier mod.",
        "Drop Multiplier",
        "Settings for the Drop Multiplier mod.",
    )?;
    let (drops, _entry) = provider.add_submenu(
        page,
        "Drop Chance",
        "How likely enemies are to drop items.",
        "Drop Chance",
        "How likely enemies are to drop items.",
    )?;
    let (materials, _entry) = provider.add_submenu(
        page,
        "Materials",
        "How many materials you get from each drop or pickup.",
        "Materials",
        "How many materials you get from each drop or pickup.",
    )?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();

    rows.push((
        provider.add_inline_choice(
            drops,
            "Mode",
            "Multiply chance: an item's drop chance is multiplied. Fixed chance: every drop is set to one chance.",
            &["Multiply chance", "Fixed chance"],
            mode_row(),
            erui::setting_changed,
            MODE.user_data(),
        )?,
        mode_row,
    ));
    let mut numbers: Vec<(&'static NumberSetting, fn() -> f64)> = Vec::new();
    provider.add_number(
        drops,
        "Multiplier",
        "Drop-rate multiplier, used when Mode is Multiply chance. 1 = vanilla, 0 = no drops.",
        &MULTIPLIER,
        config::get_double("Multiplier", 2.0),
    )?;
    numbers.push((&MULTIPLIER, || config::get_double("Multiplier", 2.0)));
    rows.push((
        provider.add_slider(
            drops,
            "Percentage (%)",
            "Fixed drop chance in percent, used when Mode is Fixed chance. 100 guarantees a drop.",
            0,
            100,
            1,
            percentage_row(),
            erui::setting_changed,
            PERCENTAGE.user_data(),
        )?,
        percentage_row,
    ));
    let materials_rows: [(&str, &str, &'static NumberSetting, fn() -> f64); 3] = [
        ("Crafting", "Multiplies the crafting materials you get (Rowa Fruit, Herba, bones...). 1 = vanilla.", &CRAFTING, || config::get_double("Crafting", 1.0)),
        ("Upgrade", "Multiplies the upgrade materials you get (Smithing Stones, Gloveworts...). 1 = vanilla.", &UPGRADE, || config::get_double("Upgrade", 1.0)),
        ("Unique", "Multiplies materials found only once (Sacred Tears, Scadutree Fragments...). 1 = vanilla.", &UNIQUE, || config::get_double("Unique", 1.0)),
    ];
    for (label, help, setting, read) in materials_rows {
        provider.add_number(materials, label, help, setting, read())?;
        numbers.push((setting, read));
    }
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
    Ok(Rows { registration, rows, numbers })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(reapply));
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
