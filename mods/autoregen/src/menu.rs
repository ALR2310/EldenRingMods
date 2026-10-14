//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Auto Regen                 -> opens a page with:
//!   Per Tick                 -> Enabled, When, Interval, Value Type,
//!                               HP, FP, SP, Max Fill
//!   Per Hit                  -> Enabled, Mode, Hit Type, Skip Ash of War,
//!                               HP, FP, SP
//!   Log file                 on/off
//! ```
//!
//! The regen code reads the config live every frame, so a change takes effect
//! as soon as it is saved (a second after the player stops). The interval and
//! the amounts are text rows where a number is typed (the ini holds decimals
//! such as 0.5, which a slider byte cannot). The gesture list stays ini-only.

use std::sync::OnceLock;
use std::time::Duration;

use common::erui::{self, NumberSetting, PageHandle, Provider, Registration, RowHandle, Setting};
use common::{config, logger};

fn number(value: u8) -> String {
    value.to_string()
}

fn flag(value: u8) -> String {
    (value != 0).to_string()
}

static TICK_ENABLED: Setting = Setting { key: "Regen.PerTick.Enabled", format: flag, apply: None };
static TICK_TRIGGER: Setting = Setting { key: "Regen.PerTick.Trigger", format: number, apply: None };
static TICK_INTERVAL: NumberSetting = NumberSetting::whole("Regen.PerTick.Interval", 0.0, 3_600_000.0, None);
static TICK_VALUE_TYPE: Setting = Setting { key: "Regen.PerTick.ValueType", format: number, apply: None };
static TICK_HP: NumberSetting = NumberSetting::new("Regen.PerTick.HP", -1000.0, 1000.0, None);
static TICK_FP: NumberSetting = NumberSetting::new("Regen.PerTick.FP", -1000.0, 1000.0, None);
static TICK_SP: NumberSetting = NumberSetting::new("Regen.PerTick.SP", -1000.0, 1000.0, None);
static TICK_CAP: Setting = Setting { key: "Regen.PerTick.Cap", format: number, apply: None };
static HIT_ENABLED: Setting = Setting { key: "Regen.PerHit.Enabled", format: flag, apply: None };
static HIT_MODE: Setting = Setting { key: "Regen.PerHit.Mode", format: number, apply: None };
static HIT_TYPE: Setting = Setting { key: "Regen.PerHit.DamageType", format: number, apply: None };
static HIT_EXCLUDE_AOW: Setting = Setting { key: "Regen.PerHit.ExcludeAow", format: flag, apply: None };
static HIT_HP: NumberSetting = NumberSetting::new("Regen.PerHit.HP", -1000.0, 1000.0, None);
static HIT_FP: NumberSetting = NumberSetting::new("Regen.PerHit.FP", -1000.0, 1000.0, None);
static HIT_SP: NumberSetting = NumberSetting::new("Regen.PerHit.SP", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting { key: "LogFile", format: flag, apply: None };

fn enum_row(key: &str, default: i32, max: i32) -> u8 {
    config::get_int(key, default).clamp(0, max) as u8
}

fn cap_row() -> u8 {
    config::get_double("Regen.PerTick.Cap", 100.0).round().clamp(1.0, 100.0) as u8
}

type Reader = fn() -> u8;
type RowList = Vec<(RowHandle, Reader)>;

struct Rows {
    registration: Registration,
    /// Each row with the function that reads its current value from the config.
    rows: RowList,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: Vec<(&'static NumberSetting, fn() -> f64)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

fn toggle(
    p: &Provider,
    rows: &mut RowList,
    page: PageHandle,
    label: &str,
    help: &str,
    setting: &'static Setting,
    read: Reader,
) -> Result<(), String> {
    let row = p.add_toggle(page, label, help, read() != 0, erui::setting_changed, setting.user_data())?;
    rows.push((row, read));
    Ok(())
}

fn choice(
    p: &Provider,
    rows: &mut RowList,
    page: PageHandle,
    label: &str,
    help: &str,
    options: &[&str],
    setting: &'static Setting,
    read: Reader,
) -> Result<(), String> {
    let row = p.add_inline_choice(page, label, help, options, read(), erui::setting_changed, setting.user_data())?;
    rows.push((row, read));
    Ok(())
}

type NumberList = Vec<(&'static NumberSetting, fn() -> f64)>;

fn number_row(
    p: &Provider,
    numbers: &mut NumberList,
    page: PageHandle,
    label: &str,
    help: &str,
    setting: &'static NumberSetting,
    read: fn() -> f64,
) -> Result<(), String> {
    p.add_number(page, label, help, setting, read())?;
    numbers.push((setting, read));
    Ok(())
}

/// The HP / FP / SP rows of one page.
fn amount_rows(
    p: &Provider,
    numbers: &mut NumberList,
    page: PageHandle,
    settings: [&'static NumberSetting; 3],
    readers: [fn() -> f64; 3],
    helps: [&str; 3],
) -> Result<(), String> {
    for (i, label) in ["HP", "FP", "SP"].into_iter().enumerate() {
        number_row(p, numbers, page, label, helps[i], settings[i], readers[i])?;
    }
    Ok(())
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("auto-regen", "Auto Regen", hmodule)?;
    let p = &provider;
    let (page, _entry) = p.add_submenu(
        p.root_page(),
        "Auto Regen",
        "Settings for the Auto Regen mod.",
        "Auto Regen",
        "Settings for the Auto Regen mod.",
    )?;
    let (tick, _entry) = p.add_submenu(
        page,
        "Per Tick",
        "Restore HP, FP and SP over time.",
        "Per Tick",
        "Restore HP, FP and SP over time.",
    )?;
    let (hit, _entry) = p.add_submenu(
        page,
        "Per Hit",
        "Restore HP, FP and SP when you land a hit.",
        "Per Hit",
        "Restore HP, FP and SP when you land a hit.",
    )?;
    let mut rows: RowList = Vec::new();
    let mut numbers: NumberList = Vec::new();
    let r = &mut rows;

    toggle(p, r, tick, "Enabled", "Turns regeneration over time on or off.", &TICK_ENABLED, || {
        config::get_bool("Regen.PerTick.Enabled", true) as u8
    })?;
    choice(
        p,
        r,
        tick,
        "When",
        "When it applies. During gesture uses the gesture list in the config file.",
        &["Always", "Out of combat", "In combat", "Idle", "During gesture"],
        &TICK_TRIGGER,
        || enum_row("Regen.PerTick.Trigger", 0, 4),
    )?;
    number_row(
        p,
        &mut numbers,
        tick,
        "Interval (ms)",
        "How often it restores, in milliseconds (1000 = 1 second). 0 turns it off. Shorter is smoother but costs a little more.",
        &TICK_INTERVAL,
        || config::get_int("Regen.PerTick.Interval", 1000) as f64,
    )?;
    choice(
        p,
        r,
        tick,
        "Value Type",
        "What the amounts below mean: points, percent of max, or percent of what is missing.",
        &["Points", "Percent of max", "Percent of missing"],
        &TICK_VALUE_TYPE,
        || enum_row("Regen.PerTick.ValueType", 0, 2),
    )?;
    amount_rows(
        p,
        &mut numbers,
        tick,
        [&TICK_HP, &TICK_FP, &TICK_SP],
        [
            || config::get_double("Regen.PerTick.HP", 0.0),
            || config::get_double("Regen.PerTick.FP", 0.0),
            || config::get_double("Regen.PerTick.SP", 0.0),
        ],
        [
            "HP restored each interval, in the unit chosen above.",
            "FP restored each interval, in the unit chosen above.",
            "SP restored each interval, in the unit chosen above.",
        ],
    )?;
    let cap = p.add_slider(
        tick,
        "Max Fill",
        "Regeneration stops at this percent of the maximum. 100 means no limit.",
        1,
        100,
        1,
        cap_row(),
        erui::setting_changed,
        TICK_CAP.user_data(),
    )?;
    r.push((cap, cap_row));

    toggle(p, r, hit, "Enabled", "Turns regeneration on hit on or off.", &HIT_ENABLED, || {
        config::get_bool("Regen.PerHit.Enabled", false) as u8
    })?;
    choice(
        p,
        r,
        hit,
        "Mode",
        "What the amounts below mean: points, percent of max, or percent of the damage you dealt.",
        &["Points", "Percent of max", "Percent of damage"],
        &HIT_MODE,
        || enum_row("Regen.PerHit.Mode", 0, 2),
    )?;
    choice(
        p,
        r,
        hit,
        "Hit Type",
        "Which hits count.",
        &["Melee", "Ranged and spells", "Both"],
        &HIT_TYPE,
        || enum_row("Regen.PerHit.DamageType", 0, 2),
    )?;
    toggle(p, r, hit, "Skip Ash of War", "Hits from an Ash of War restore nothing.", &HIT_EXCLUDE_AOW, || {
        config::get_bool("Regen.PerHit.ExcludeAow", false) as u8
    })?;
    amount_rows(
        p,
        &mut numbers,
        hit,
        [&HIT_HP, &HIT_FP, &HIT_SP],
        [
            || config::get_double("Regen.PerHit.HP", 0.0),
            || config::get_double("Regen.PerHit.FP", 0.0),
            || config::get_double("Regen.PerHit.SP", 0.0),
        ],
        [
            "HP restored per hit, in the unit chosen above.",
            "FP restored per hit, in the unit chosen above.",
            "SP restored per hit, in the unit chosen above.",
        ],
    )?;

    toggle(p, r, page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, || {
        config::get_bool("LogFile", false) as u8
    })?;
    let registration = provider.commit()?;
    Ok(Rows { registration, rows, numbers })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup. Changes are only saved to the ini
/// (and the shared config map); the regen tick reads them live.
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
