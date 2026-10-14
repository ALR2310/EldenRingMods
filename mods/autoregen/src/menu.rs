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

use common::config;
use common::erui::{self, NumberSetting, Setting};

static TICK_ENABLED: Setting = Setting::flag("Regen.PerTick.Enabled");
static TICK_TRIGGER: Setting = Setting::int("Regen.PerTick.Trigger");
static TICK_INTERVAL: NumberSetting = NumberSetting::whole("Regen.PerTick.Interval", 0.0, 3_600_000.0, None);
static TICK_VALUE_TYPE: Setting = Setting::int("Regen.PerTick.ValueType");
static TICK_HP: NumberSetting = NumberSetting::new("Regen.PerTick.HP", -1000.0, 1000.0, None);
static TICK_FP: NumberSetting = NumberSetting::new("Regen.PerTick.FP", -1000.0, 1000.0, None);
static TICK_SP: NumberSetting = NumberSetting::new("Regen.PerTick.SP", -1000.0, 1000.0, None);
static TICK_CAP: Setting = Setting::int("Regen.PerTick.Cap");
static HIT_ENABLED: Setting = Setting::flag("Regen.PerHit.Enabled");
static HIT_MODE: Setting = Setting::int("Regen.PerHit.Mode");
static HIT_TYPE: Setting = Setting::int("Regen.PerHit.DamageType");
static HIT_EXCLUDE_AOW: Setting = Setting::flag("Regen.PerHit.ExcludeAow");
static HIT_HP: NumberSetting = NumberSetting::new("Regen.PerHit.HP", -1000.0, 1000.0, None);
static HIT_FP: NumberSetting = NumberSetting::new("Regen.PerHit.FP", -1000.0, 1000.0, None);
static HIT_SP: NumberSetting = NumberSetting::new("Regen.PerHit.SP", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

fn enum_row(key: &str, default: i32, max: i32) -> u8 {
    config::get_int(key, default).clamp(0, max) as u8
}

fn cap_row() -> u8 {
    config::get_double("Regen.PerTick.Cap", 100.0).round().clamp(1.0, 100.0) as u8
}

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn build(b: &mut erui::MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Auto Regen", "Settings for the Auto Regen mod.")?;
    let tick = b.submenu(page, "Per Tick", "Restore HP, FP and SP over time.")?;
    let hit = b.submenu(page, "Per Hit", "Restore HP, FP and SP when you land a hit.")?;

    b.toggle(tick, "Enabled", "Turns regeneration over time on or off.", &TICK_ENABLED, || {
        config::get_bool("Regen.PerTick.Enabled", true) as u8
    })?;
    b.choice(
        tick,
        "When",
        "When it applies. During gesture uses the gesture list in the config file.",
        &["Always", "Out of combat", "In combat", "Idle", "During gesture"],
        &TICK_TRIGGER,
        || enum_row("Regen.PerTick.Trigger", 0, 4),
    )?;
    b.number(
        tick,
        "Interval (ms)",
        "How often it restores, in milliseconds (1000 = 1 second). 0 turns it off. Shorter is smoother but costs a little more.",
        &TICK_INTERVAL,
        || config::get_int("Regen.PerTick.Interval", 1000) as f64,
    )?;
    b.choice(
        tick,
        "Value Type",
        "What the amounts below mean: points, percent of max, or percent of what is missing.",
        &["Points", "Percent of max", "Percent of missing"],
        &TICK_VALUE_TYPE,
        || enum_row("Regen.PerTick.ValueType", 0, 2),
    )?;
    b.number(tick, "HP", "HP restored each interval, in the unit chosen above.", &TICK_HP, || {
        config::get_double("Regen.PerTick.HP", 0.0)
    })?;
    b.number(tick, "FP", "FP restored each interval, in the unit chosen above.", &TICK_FP, || {
        config::get_double("Regen.PerTick.FP", 0.0)
    })?;
    b.number(tick, "SP", "SP restored each interval, in the unit chosen above.", &TICK_SP, || {
        config::get_double("Regen.PerTick.SP", 0.0)
    })?;
    b.slider(
        tick,
        "Max Fill",
        "Regeneration stops at this percent of the maximum. 100 means no limit.",
        1,
        100,
        &TICK_CAP,
        cap_row,
    )?;

    b.toggle(hit, "Enabled", "Turns regeneration on hit on or off.", &HIT_ENABLED, || {
        config::get_bool("Regen.PerHit.Enabled", false) as u8
    })?;
    b.choice(
        hit,
        "Mode",
        "What the amounts below mean: points, percent of max, or percent of the damage you dealt.",
        &["Points", "Percent of max", "Percent of damage"],
        &HIT_MODE,
        || enum_row("Regen.PerHit.Mode", 0, 2),
    )?;
    b.choice(hit, "Hit Type", "Which hits count.", &["Melee", "Ranged and spells", "Both"], &HIT_TYPE, || {
        enum_row("Regen.PerHit.DamageType", 0, 2)
    })?;
    b.toggle(hit, "Skip Ash of War", "Hits from an Ash of War restore nothing.", &HIT_EXCLUDE_AOW, || {
        config::get_bool("Regen.PerHit.ExcludeAow", false) as u8
    })?;
    b.number(hit, "HP", "HP restored per hit, in the unit chosen above.", &HIT_HP, || config::get_double("Regen.PerHit.HP", 0.0))?;
    b.number(hit, "FP", "FP restored per hit, in the unit chosen above.", &HIT_FP, || config::get_double("Regen.PerHit.FP", 0.0))?;
    b.number(hit, "SP", "SP restored per hit, in the unit chosen above.", &HIT_SP, || config::get_double("Regen.PerHit.SP", 0.0))?;

    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, || {
        config::get_bool("LogFile", false) as u8
    })?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup. Changes are only saved to the ini
/// (and the shared config map); the regen tick reads them live.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "auto-regen", "Auto Regen", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
