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
//! stops for a second; the mod reads its config snapshot every frame, so they
//! apply at once. The probes stay file-only.

use std::sync::OnceLock;
use std::time::Duration;

use common::erui::{self, PageHandle, Registration, RowHandle, Setting};
use common::logger;

use crate::config;

fn on_off(value: u8) -> String {
    (value != 0).to_string()
}

macro_rules! toggle {
    ($name:ident, $key:expr) => {
        static $name: Setting = Setting { key: $key, format: on_off, apply: None };
    };
}

toggle!(GUARD, "Cancel.Guard");
toggle!(DODGE, "Cancel.Dodge");
toggle!(WEAPON_SWAP, "Cancel.WeaponSwap");
toggle!(ATTACK, "Animations.Attack");
toggle!(SKILL, "Animations.Skill");
toggle!(CAST, "Animations.Cast");
toggle!(ITEM, "Animations.Item");
toggle!(LOG_FILE, "Logging.LogFile");

struct Rows {
    registration: Registration,
    /// Each row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// After the file is written (and re-read): the mod reads its config every
/// frame, so only the log switch needs following.
fn saved() {
    logger::set_enabled(config::get().logging.log_file);
}

type ToggleRow = (&'static str, &'static str, &'static Setting, fn() -> u8);

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("anim-cancel", "Anim Cancel", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Anim Cancel",
        "Settings for the Anim Cancel mod.",
        "Anim Cancel",
        "Settings for the Anim Cancel mod.",
    )?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();

    let mut add_toggles = |on_page: PageHandle, toggles: &[ToggleRow]| -> Result<(), String> {
        for (label, help, setting, read) in toggles {
            rows.push((
                provider.add_toggle(on_page, label, help, read() != 0, erui::setting_changed, setting.user_data())?,
                *read,
            ));
        }
        Ok(())
    };

    let (cancel, _entry) = provider.add_submenu(
        page,
        "Cancel With",
        "Which actions may cut an animation short.",
        "Cancel With",
        "Which actions may cut an animation short.",
    )?;
    add_toggles(
        cancel,
        &[
            ("Guard", "Raising your guard cancels the animation.", &GUARD, || config::get().cancel.guard as u8),
            ("Dodge", "Rolling or backstepping cancels the animation.", &DODGE, || config::get().cancel.dodge as u8),
            ("Weapon swap", "Switching weapons cancels the animation.", &WEAPON_SWAP, || config::get().cancel.weapon_swap as u8),
        ],
    )?;

    let (animations, _entry) = provider.add_submenu(
        page,
        "Cancel What",
        "Which animations may be cut short.",
        "Cancel What",
        "Which animations may be cut short.",
    )?;
    add_toggles(
        animations,
        &[
            ("Attacks", "Normal attacks can be cancelled.", &ATTACK, || config::get().animations.attack as u8),
            ("Skills", "Weapon skills can be cancelled.", &SKILL, || config::get().animations.skill as u8),
            ("Spells", "Casting sorceries and incantations can be cancelled.", &CAST, || config::get().animations.cast as u8),
            ("Items", "Using items such as flasks can be cancelled.", &ITEM, || config::get().animations.item as u8),
        ],
    )?;

    add_toggles(
        page,
        &[(
            "Log file",
            "Write a log file next to the DLL, for troubleshooting.",
            &LOG_FILE,
            || config::get().logging.log_file as u8,
        )],
    )?;
    let registration = provider.commit()?;
    Ok(Rows { registration, rows })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64) {
    erui::start_saver_with(|changes| config::save(changes), Some(saved));
    std::thread::spawn(move || {
        let connection = match erui::connect(Duration::from_secs(10)) {
            Ok(c) => c,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the config file only."));
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

/// After a config reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(rows) = ROWS.get() {
        for (row, read) in &rows.rows {
            if let Err(reason) = rows.registration.set_row_value(*row, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}
