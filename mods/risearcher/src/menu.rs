//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Rise Archer                -> opens a page with:
//!   Damage                   -> Bow / Crossbow / Ballista damage,
//!                               Bow Scaling, Bow Ash of War
//!   Weight & Value           -> weight and sell value of each weapon type
//!   Ammo & Shots             -> Arrow / Bolt carry limit, Speed, Range,
//!                               Spread, Rain of Arrows count
//!   Log file                 on/off
//! ```
//!
//! Multipliers are text rows where a number is typed (the ini holds decimals
//! such as 1.5, which a slider byte cannot); the carry limits and the Rain of
//! Arrows count stay sliders (whole numbers that fit a byte). Changes are written to the ini once the player stops for a
//! second, then the mod is told to re-apply from its cached original param
//! values - the same path as the reload key - so it rescans once and never
//! compounds. The reload task only runs once the player is in the world.

use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, NumberSetting, PageHandle, Provider, Registration, RowHandle, Setting};
use common::{config, logger};

fn number(value: u8) -> String {
    value.to_string()
}

fn flag(value: u8) -> String {
    (value != 0).to_string()
}

static BOW_DAMAGE: NumberSetting = NumberSetting::new("Bow.DamageMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_DAMAGE: NumberSetting = NumberSetting::new("Crossbow.DamageMultiplier", -1000.0, 1000.0, None);
static BALLISTA_DAMAGE: NumberSetting = NumberSetting::new("Ballista.DamageMultiplier", -1000.0, 1000.0, None);
static BOW_SCALING: NumberSetting = NumberSetting::new("Bow.ScalingMultiplier", -1000.0, 1000.0, None);
static BOW_AOW: Setting = Setting { key: "Bow.UnlockAOW", format: flag, apply: None };
static BOW_WEIGHT: NumberSetting = NumberSetting::new("Bow.WeightMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_WEIGHT: NumberSetting = NumberSetting::new("Crossbow.WeightMultiplier", -1000.0, 1000.0, None);
static BALLISTA_WEIGHT: NumberSetting = NumberSetting::new("Ballista.WeightMultiplier", -1000.0, 1000.0, None);
static BOW_SELL: NumberSetting = NumberSetting::new("Bow.SellValueMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_SELL: NumberSetting = NumberSetting::new("Crossbow.SellValueMultiplier", -1000.0, 1000.0, None);
static BALLISTA_SELL: NumberSetting = NumberSetting::new("Ballista.SellValueMultiplier", -1000.0, 1000.0, None);
static ARROW_MAX: Setting = Setting { key: "Bullet.Arrow.MaxQuantity", format: number, apply: None };
static BOLT_MAX: Setting = Setting { key: "Bullet.Bolt.MaxQuantity", format: number, apply: None };
static BULLET_SPEED: NumberSetting = NumberSetting::new("Bullet.SpeedMultiplier", -1000.0, 1000.0, None);
static BULLET_RANGE: NumberSetting = NumberSetting::new("Bullet.RangeMultiplier", -1000.0, 1000.0, None);
static BULLET_SPREAD: NumberSetting = NumberSetting::new("Bullet.SpreadMultiplier", -1000.0, 1000.0, None);
static RAIN_COUNT: Setting = Setting { key: "Bullet.RainOfArrowsCount", format: number, apply: None };
static LOG_FILE: Setting = Setting { key: "LogFile", format: flag, apply: None };

fn count_row(key: &str, default: i32, max: i32) -> u8 {
    config::get_int(key, default).clamp(1, max) as u8
}

type Reader = fn() -> u8;
type RowList = Vec<(RowHandle, Reader)>;
type NumberList = Vec<(&'static NumberSetting, fn() -> f64)>;

struct Rows {
    registration: Registration,
    /// Each row with the function that reads its current value from the config.
    rows: RowList,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: NumberList,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

/// After the ini is written: ask the mod to re-apply, like the reload key.
fn reapply() {
    common::reload::RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// One multiplier row (a typed number, 0 to 1000) on `page`.
fn multiplier(
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

/// One whole-number slider row on `page`.
fn count(
    p: &Provider,
    rows: &mut RowList,
    page: PageHandle,
    label: &str,
    help: &str,
    max: i32,
    setting: &'static Setting,
    read: Reader,
) -> Result<(), String> {
    let row = p.add_slider(page, label, help, 1, max, 1, read(), erui::setting_changed, setting.user_data())?;
    rows.push((row, read));
    Ok(())
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("rise-archer", "Rise Archer", hmodule)?;
    let p = &provider;
    let (page, _entry) = p.add_submenu(
        p.root_page(),
        "Rise Archer",
        "Settings for the Rise Archer mod.",
        "Rise Archer",
        "Settings for the Rise Archer mod.",
    )?;
    let (damage, _entry) = p.add_submenu(
        page,
        "Damage",
        "How hard bows, crossbows and ballistae hit.",
        "Damage",
        "How hard bows, crossbows and ballistae hit.",
    )?;
    let (upkeep, _entry) = p.add_submenu(
        page,
        "Weight & Value",
        "How heavy each weapon is and what it sells for.",
        "Weight & Value",
        "How heavy each weapon is and what it sells for.",
    )?;
    let (ammo, _entry) = p.add_submenu(
        page,
        "Ammo & Shots",
        "How much ammo you carry and how arrows and bolts fly.",
        "Ammo & Shots",
        "How much ammo you carry and how arrows and bolts fly.",
    )?;
    let mut rows: RowList = Vec::new();
    let mut numbers: NumberList = Vec::new();
    let r = &mut rows;

    multiplier(p, &mut numbers, damage, "Bow Damage", "Damage multiplier for bows and greatbows. 1x = vanilla.", &BOW_DAMAGE, || config::get_double("Bow.DamageMultiplier", 1.5))?;
    multiplier(p, &mut numbers, damage, "Crossbow Damage", "Damage multiplier for crossbows. 1x = vanilla.", &CROSSBOW_DAMAGE, || config::get_double("Crossbow.DamageMultiplier", 3.0))?;
    multiplier(p, &mut numbers, damage, "Ballista Damage", "Damage multiplier for ballistae. 1x = vanilla.", &BALLISTA_DAMAGE, || config::get_double("Ballista.DamageMultiplier", 4.0))?;
    multiplier(p, &mut numbers, damage, "Bow Scaling", "Stat scaling multiplier for bows and greatbows. 1x = vanilla.", &BOW_SCALING, || config::get_double("Bow.ScalingMultiplier", 2.0))?;
    let aow_read: Reader = || config::get_bool("Bow.UnlockAOW", true) as u8;
    let aow = p.add_toggle(
        damage,
        "Bow Ash of War",
        "Lets every bow take an Ash of War.",
        aow_read() != 0,
        erui::setting_changed,
        BOW_AOW.user_data(),
    )?;
    r.push((aow, aow_read));

    multiplier(p, &mut numbers, upkeep, "Bow Weight", "Weight multiplier for bows and greatbows.", &BOW_WEIGHT, || config::get_double("Bow.WeightMultiplier", 0.5))?;
    multiplier(p, &mut numbers, upkeep, "Crossbow Weight", "Weight multiplier for crossbows.", &CROSSBOW_WEIGHT, || config::get_double("Crossbow.WeightMultiplier", 0.5))?;
    multiplier(p, &mut numbers, upkeep, "Ballista Weight", "Weight multiplier for ballistae.", &BALLISTA_WEIGHT, || config::get_double("Ballista.WeightMultiplier", 0.5))?;
    multiplier(p, &mut numbers, upkeep, "Bow Sell Value", "Sell price multiplier for bows and greatbows.", &BOW_SELL, || config::get_double("Bow.SellValueMultiplier", 2.0))?;
    multiplier(p, &mut numbers, upkeep, "Crossbow Sell Value", "Sell price multiplier for crossbows.", &CROSSBOW_SELL, || config::get_double("Crossbow.SellValueMultiplier", 2.0))?;
    multiplier(p, &mut numbers, upkeep, "Ballista Sell Value", "Sell price multiplier for ballistae.", &BALLISTA_SELL, || config::get_double("Ballista.SellValueMultiplier", 2.0))?;

    count(p, r, ammo, "Arrow Limit", "How many arrows you can carry. Vanilla is lower.", 255, &ARROW_MAX, || {
        count_row("Bullet.Arrow.MaxQuantity", 255, 255)
    })?;
    count(p, r, ammo, "Bolt Limit", "How many bolts you can carry. Vanilla is lower.", 255, &BOLT_MAX, || {
        count_row("Bullet.Bolt.MaxQuantity", 255, 255)
    })?;
    multiplier(p, &mut numbers, ammo, "Projectile Speed", "How fast arrows and bolts fly. 1x = vanilla.", &BULLET_SPEED, || config::get_double("Bullet.SpeedMultiplier", 3.5))?;
    multiplier(p, &mut numbers, ammo, "Rain Range", "How far Rain of Arrows reaches. 1x = vanilla.", &BULLET_RANGE, || config::get_double("Bullet.RangeMultiplier", 2.0))?;
    multiplier(p, &mut numbers, ammo, "Rain Spread", "How wide Rain of Arrows spreads. 1x = vanilla.", &BULLET_SPREAD, || config::get_double("Bullet.SpreadMultiplier", 2.0))?;
    count(p, r, ammo, "Rain Arrows", "How many arrows Rain of Arrows fires.", 50, &RAIN_COUNT, || {
        count_row("Bullet.RainOfArrowsCount", 6, 50)
    })?;

    let log_read: Reader = || config::get_bool("LogFile", false) as u8;
    let log = p.add_toggle(
        page,
        "Log file",
        "Write a log file next to the DLL, for troubleshooting.",
        log_read() != 0,
        erui::setting_changed,
        LOG_FILE.user_data(),
    )?;
    r.push((log, log_read));

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
