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
use std::time::Duration;

use common::erui::{self, NumberSetting, Registration, RowHandle, Setting};
use common::{config, logger};

static ENABLED: Setting = Setting { key: "Rune.Passive.Enabled", format: |value| (value != 0).to_string(), apply: None };
static INTERVAL: NumberSetting = NumberSetting::whole("Rune.Passive.Interval", 0.0, 3_600_000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Rune.Passive.Amount", -100_000.0, 100_000.0, None);
static PERCENT: NumberSetting = NumberSetting::new("Rune.Passive.Percent", -100.0, 100.0, None);
static INTEREST: NumberSetting = NumberSetting::new("Rune.Passive.Interest", -100.0, 100.0, None);
static LOG_FILE: Setting = Setting { key: "LogFile", format: |value| (value != 0).to_string(), apply: None };

fn enabled_row() -> u8 {
    config::get_bool("Rune.Passive.Enabled", true) as u8
}
fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

type NumberList = Vec<(&'static NumberSetting, fn() -> f64)>;

struct Rows {
    registration: Registration,
    /// Each toggle row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: NumberList,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("passive-runes", "Passive Runes", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Passive Runes",
        "Settings for the Passive Runes mod.",
        "Passive Runes",
        "Settings for the Passive Runes mod.",
    )?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();
    let mut numbers: NumberList = Vec::new();

    rows.push((
        provider.add_toggle(
            page,
            "Enabled",
            "Turns the passive rune rewards on or off.",
            enabled_row() != 0,
            erui::setting_changed,
            ENABLED.user_data(),
        )?,
        enabled_row,
    ));
    let typed: [(&str, &str, &'static NumberSetting, fn() -> f64); 4] = [
        (
            "Interval (ms)",
            "How often you get runes, in milliseconds (1000 = 1 second). 0 pauses the interval rewards.",
            &INTERVAL,
            || config::get_double("Rune.Passive.Interval", 10000.0),
        ),
        (
            "Amount",
            "Runes you get each time. Negative takes runes away.",
            &AMOUNT,
            || config::get_double("Rune.Passive.Amount", 50.0),
        ),
        (
            "Next Level (%)",
            "Each time, also get this percent of the runes needed for your next level. Negative takes runes away.",
            &PERCENT,
            || config::get_double("Rune.Passive.Percent", 0.25),
        ),
        (
            "Interest (%)",
            "Each time, also get this percent of the runes you hold. Grows fast. Negative takes runes away.",
            &INTEREST,
            || config::get_double("Rune.Passive.Interest", 0.0),
        ),
    ];
    for (label, help, setting, read) in typed {
        provider.add_number(page, label, help, setting, read())?;
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
    // The mod reads every key live each tick, so saving is all it takes.
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
