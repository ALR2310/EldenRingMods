//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Infinite Ailment           -> opens a page with:
//!   Scarlet Rot              -> Duration (s)   number -1..86400 (-1 = infinite)
//!                               Damage (%)     number -100..100 (% of max HP per tick, negative heals)
//!                               Damage         number -1000..1000 (flat, per tick, negative heals)
//!   Poison                   -> same three rows
//!   Log file                 on/off
//! ```
//!
//! Numbers are typed into text rows (the ini holds decimals such as 0.18).
//! Changes are written to the ini once the player stops for a second, then
//! the mod is told to re-apply to the game's params - the same path as the
//! reload key.

use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, NumberSetting, PageHandle, Registration, RowHandle, Setting};
use common::{config, logger};

static ROT_DURATION: NumberSetting = NumberSetting::new("ScarletRot.Duration", -1.0, 86_400.0, None);
static ROT_PERCENT: NumberSetting = NumberSetting::new("ScarletRot.PercentDamage", -100.0, 100.0, None);
static ROT_FIXED: NumberSetting = NumberSetting::whole("ScarletRot.FixedDamage", -1000.0, 1000.0, None);
static POISON_DURATION: NumberSetting = NumberSetting::new("Poison.Duration", -1.0, 86_400.0, None);
static POISON_PERCENT: NumberSetting = NumberSetting::new("Poison.PercentDamage", -100.0, 100.0, None);
static POISON_FIXED: NumberSetting = NumberSetting::whole("Poison.FixedDamage", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting { key: "LogFile", format: |value| (value != 0).to_string(), apply: None };

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

/// After the ini is written: ask the mod to re-apply, like the reload key.
fn reapply() {
    common::reload::RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// One ailment's page: duration, percent damage, flat damage.
fn add_ailment_page(
    provider: &erui::Provider,
    parent: PageHandle,
    name: &str,
    help: &str,
    settings: [&'static NumberSetting; 3],
    reads: [fn() -> f64; 3],
    vanilla: [&str; 3],
    numbers: &mut NumberList,
) -> Result<(), String> {
    let (page, _entry) = provider.add_submenu(parent, name, help, name, help)?;
    let rows = [
        ("Duration (s)", format!("How long the damage lasts, in seconds. -1 = infinite. Vanilla: {}.", vanilla[0])),
        ("Damage (%)", format!("Percent of your max HP lost each tick; negative heals. Vanilla: {}.", vanilla[1])),
        ("Damage", format!("Flat damage each tick, added on top; negative heals. Vanilla: {}.", vanilla[2])),
    ];
    for (i, (label, help)) in rows.iter().enumerate() {
        provider.add_number(page, label, help, settings[i], reads[i]())?;
        numbers.push((settings[i], reads[i]));
    }
    Ok(())
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("infinite-ailment", "Infinite Ailment", hmodule)?;
    let (page, _entry) = provider.add_submenu(
        provider.root_page(),
        "Infinite Ailment",
        "Settings for the Infinite Ailment mod.",
        "Infinite Ailment",
        "Settings for the Infinite Ailment mod.",
    )?;
    let mut numbers: NumberList = Vec::new();
    add_ailment_page(
        &provider,
        page,
        "Scarlet Rot",
        "Scarlet Rot damage over time.",
        [&ROT_DURATION, &ROT_PERCENT, &ROT_FIXED],
        [
            || config::get_double("ScarletRot.Duration", -1.0),
            || config::get_double("ScarletRot.PercentDamage", 0.18),
            || config::get_double("ScarletRot.FixedDamage", 15.0),
        ],
        ["90", "0.18", "15"],
        &mut numbers,
    )?;
    add_ailment_page(
        &provider,
        page,
        "Poison",
        "Poison damage over time.",
        [&POISON_DURATION, &POISON_PERCENT, &POISON_FIXED],
        [
            || config::get_double("Poison.Duration", -1.0),
            || config::get_double("Poison.PercentDamage", 0.07),
            || config::get_double("Poison.FixedDamage", 7.0),
        ],
        ["90", "0.07", "7"],
        &mut numbers,
    )?;
    let rows = vec![(
        provider.add_toggle(
            page,
            "Log file",
            "Write a log file next to the DLL, for troubleshooting.",
            log_file_row() != 0,
            erui::setting_changed,
            LOG_FILE.user_data(),
        )?,
        log_file_row as fn() -> u8,
    )];
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
