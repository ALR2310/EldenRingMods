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

use common::config;
use common::erui::{self, MenuBuilder, NumberSetting, PageHandle, Setting};

static ROT_DURATION: NumberSetting = NumberSetting::new("ScarletRot.Duration", -1.0, 86_400.0, None);
static ROT_PERCENT: NumberSetting = NumberSetting::new("ScarletRot.PercentDamage", -100.0, 100.0, None);
static ROT_FIXED: NumberSetting = NumberSetting::whole("ScarletRot.FixedDamage", -1000.0, 1000.0, None);
static POISON_DURATION: NumberSetting = NumberSetting::new("Poison.Duration", -1.0, 86_400.0, None);
static POISON_PERCENT: NumberSetting = NumberSetting::new("Poison.PercentDamage", -100.0, 100.0, None);
static POISON_FIXED: NumberSetting = NumberSetting::whole("Poison.FixedDamage", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn log_file_row() -> u8 {
    config::get_bool("LogFile", false) as u8
}

/// One ailment's page: duration, percent damage, flat damage.
fn add_ailment_page(
    b: &mut MenuBuilder,
    parent: PageHandle,
    name: &str,
    help: &str,
    settings: [&'static NumberSetting; 3],
    reads: [fn() -> f64; 3],
    vanilla: [&str; 3],
) -> Result<(), String> {
    let page = b.submenu(parent, name, help)?;
    let rows = [
        ("Duration (s)", format!("How long the damage lasts, in seconds. -1 = infinite. Vanilla: {}.", vanilla[0])),
        ("Damage (%)", format!("Percent of your max HP lost each tick; negative heals. Vanilla: {}.", vanilla[1])),
        ("Damage", format!("Flat damage each tick, added on top; negative heals. Vanilla: {}.", vanilla[2])),
    ];
    for (i, (label, help)) in rows.iter().enumerate() {
        b.number(page, label, help, settings[i], reads[i])?;
    }
    Ok(())
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Infinite Ailment", "Settings for the Infinite Ailment mod.")?;
    add_ailment_page(
        b,
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
    )?;
    add_ailment_page(
        b,
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
    )?;
    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, log_file_row)
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(erui::reload_from_menu));
    erui::spawn_menu(&MENU, hmodule, "infinite-ailment", "Infinite Ailment", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
