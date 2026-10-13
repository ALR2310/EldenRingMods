//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Rune Multiplier            -> opens a page with:
//!                                 Multiplier   slider x0..x10, steps of 1
//!                                 Log file     on/off
//! ```
//!
//! The slider holds a whole number (the ini can still hold fractions such as
//! 1.5). Changes apply at once and are written back to the ini shortly after
//! the player stops, so they survive a restart.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use common::erui::{self, Registration, RowHandle};
use common::{config, logger};

const MAX_MULTIPLIER: i32 = 10;

/// How long the player must stop changing things before the ini is written.
const SAVE_DELAY: Duration = Duration::from_millis(1000);

struct Rows {
    registration: Registration,
    multiplier: RowHandle,
    log_file: RowHandle,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// A change made in the menu, sent to the saver (which owns the ini write).
enum Change {
    Multiplier(u8),
    LogFile(bool),
}

static CHANGES: OnceLock<Sender<Change>> = OnceLock::new();

fn notify(change: Change) {
    if let Some(tx) = CHANGES.get() {
        let _ = tx.send(change);
    }
}

fn multiplier_to_slider(multiplier: f64) -> u8 {
    multiplier.round().clamp(0.0, MAX_MULTIPLIER as f64) as u8
}

// Both callbacks run on the host's worker thread: keep them short and never
// let a panic unwind into the C ABI.
unsafe extern "C" fn on_multiplier_changed(_user_data: *mut c_void, value: u8) {
    let _ = std::panic::catch_unwind(|| {
        crate::hook::set_multiplier(value as f64);
        notify(Change::Multiplier(value));
    });
}

unsafe extern "C" fn on_log_file_changed(_user_data: *mut c_void, value: u8) {
    let _ = std::panic::catch_unwind(|| notify(Change::LogFile(value != 0)));
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
    let multiplier = provider.add_slider(
        page,
        "Multiplier",
        "Multiplies rune gains. 1 = vanilla, 2 = double.",
        0,
        MAX_MULTIPLIER,
        1,
        multiplier_to_slider(config::get_double("Multiplier", 1.0)),
        on_multiplier_changed,
        std::ptr::null_mut(),
    )?;
    let log_file = provider.add_toggle(
        page,
        "Log file",
        "Write RuneMultiplier.log next to the DLL, for troubleshooting.",
        config::get_bool("LogFile", false),
        on_log_file_changed,
        std::ptr::null_mut(),
    )?;
    let registration = provider.commit()?;
    Ok(Rows { registration, multiplier, log_file })
}

/// Registers the menu on a worker thread, then keeps that thread as the
/// ini saver. Must start early: the host closes registration shortly after
/// startup.
pub fn start(hmodule: u64, ini_path: String) {
    std::thread::spawn(move || {
        let connection = match erui::connect(Duration::from_secs(10)) {
            Ok(c) => c,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the ini only."));
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        let _ = CHANGES.set(tx);
        match build(hmodule, connection) {
            Ok(rows) => {
                let _ = ROWS.set(rows);
                logger::log("In-game menu: registered.");
                save_changes(&ini_path, rx);
            }
            Err(reason) => logger::log(&format!("In-game menu registration failed: {reason}")),
        }
    });
}

/// Sleeps until the player changes something, waits for them to stop, then
/// writes the latest values into the ini so the choice survives a restart.
fn save_changes(ini_path: &str, changes: mpsc::Receiver<Change>) {
    let mut multiplier: Option<u8> = None;
    let mut log_file: Option<bool> = None;
    while let Ok(first) = changes.recv() {
        let mut next = Some(first);
        loop {
            match next.take() {
                Some(Change::Multiplier(v)) => multiplier = Some(v),
                Some(Change::LogFile(v)) => log_file = Some(v),
                None => {}
            }
            match changes.recv_timeout(SAVE_DELAY) {
                Ok(change) => next = Some(change),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        let mut values: Vec<(&str, String)> = Vec::new();
        if let Some(v) = multiplier.take() {
            values.push(("Multiplier", v.to_string()));
        }
        if let Some(v) = log_file.take() {
            values.push(("LogFile", v.to_string()));
        }
        // Also updates the shared config map, which is what switches the log
        // file on/off (see common::logger).
        if config::set_values(ini_path, &values) {
            logger::log(&format!("In-game menu: saved {values:?} to the ini."));
        } else {
            logger::log("In-game menu: couldn't write the ini; the change applies until the game closes.");
        }
    }
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(rows) = ROWS.get() {
        let multiplier = multiplier_to_slider(config::get_double("Multiplier", 1.0));
        let log_file = config::get_bool("LogFile", false) as u8;
        for (row, value) in [(rows.multiplier, multiplier), (rows.log_file, log_file)] {
            if let Err(reason) = rows.registration.set_row_value(row, value) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}
