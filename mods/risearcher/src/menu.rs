//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Rise Archer                -> opens a page with:
//!   Damage                   -> Bow / Crossbow / Ballista damage,
//!                               Bow Scaling, Bow Ash of War
//!   Weight & Value           -> weight and sell value of each weapon type
//!   Ammo & Shots             -> Arrow / Bolt carry limit (slider 0..255),
//!                               Speed, Range, Spread, Rain Arrows (number 0..1000)
//!   Log file                 on/off
//! ```
//!
//! Multipliers are text rows where a number is typed, -1000..1000 (the ini
//! holds decimals such as 1.5, which a slider byte cannot); so is the Rain of
//! Arrows count (a whole number; the mod accepts more, 1000 is a menu cap).
//! The carry limits stay sliders (whole numbers that fit a byte). Changes are
//! written to the ini once the player stops for a second, then the mod is told
//! to re-apply from its cached original param values - the same path as the
//! reload key - so it rescans once and never compounds. The reload task only
//! runs once the player is in the world.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, NumberSetting, Setting};

static BOW_DAMAGE: NumberSetting = NumberSetting::new("Bow.DamageMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_DAMAGE: NumberSetting = NumberSetting::new("Crossbow.DamageMultiplier", -1000.0, 1000.0, None);
static BALLISTA_DAMAGE: NumberSetting = NumberSetting::new("Ballista.DamageMultiplier", -1000.0, 1000.0, None);
static BOW_SCALING: NumberSetting = NumberSetting::new("Bow.ScalingMultiplier", -1000.0, 1000.0, None);
static BOW_AOW: Setting = Setting::flag("Bow.UnlockAOW");
static BOW_WEIGHT: NumberSetting = NumberSetting::new("Bow.WeightMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_WEIGHT: NumberSetting = NumberSetting::new("Crossbow.WeightMultiplier", -1000.0, 1000.0, None);
static BALLISTA_WEIGHT: NumberSetting = NumberSetting::new("Ballista.WeightMultiplier", -1000.0, 1000.0, None);
static BOW_SELL: NumberSetting = NumberSetting::new("Bow.SellValueMultiplier", -1000.0, 1000.0, None);
static CROSSBOW_SELL: NumberSetting = NumberSetting::new("Crossbow.SellValueMultiplier", -1000.0, 1000.0, None);
static BALLISTA_SELL: NumberSetting = NumberSetting::new("Ballista.SellValueMultiplier", -1000.0, 1000.0, None);
static ARROW_MAX: Setting = Setting::int("Bullet.Arrow.MaxQuantity");
static BOLT_MAX: Setting = Setting::int("Bullet.Bolt.MaxQuantity");
static BULLET_SPEED: NumberSetting = NumberSetting::new("Bullet.SpeedMultiplier", -1000.0, 1000.0, None);
static BULLET_RANGE: NumberSetting = NumberSetting::new("Bullet.RangeMultiplier", -1000.0, 1000.0, None);
static BULLET_SPREAD: NumberSetting = NumberSetting::new("Bullet.SpreadMultiplier", -1000.0, 1000.0, None);
static RAIN_COUNT: NumberSetting = NumberSetting::whole("Bullet.RainOfArrowsCount", 0.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

/// A carry limit as a slider position (the mod accepts 0..=255).
fn limit_row(key: &str) -> u8 {
    config::get_int(key, 255).clamp(0, 255) as u8
}

static MENU: OnceLock<erui::Menu> = OnceLock::new();

fn build(b: &mut erui::MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Rise Archer", "Settings for the Rise Archer mod.")?;
    let damage = b.submenu(page, "Damage", "How hard bows, crossbows and ballistae hit.")?;
    let upkeep = b.submenu(page, "Weight & Value", "How heavy each weapon is and what it sells for.")?;
    let ammo = b.submenu(page, "Ammo & Shots", "How much ammo you carry and how arrows and bolts fly.")?;

    b.number(damage, "Bow Damage", "Damage multiplier for bows and greatbows. 1x = vanilla.", &BOW_DAMAGE, || config::get_double("Bow.DamageMultiplier", 1.5))?;
    b.number(damage, "Crossbow Damage", "Damage multiplier for crossbows. 1x = vanilla.", &CROSSBOW_DAMAGE, || config::get_double("Crossbow.DamageMultiplier", 3.0))?;
    b.number(damage, "Ballista Damage", "Damage multiplier for ballistae. 1x = vanilla.", &BALLISTA_DAMAGE, || config::get_double("Ballista.DamageMultiplier", 4.0))?;
    b.number(damage, "Bow Scaling", "Stat scaling multiplier for bows and greatbows. 1x = vanilla.", &BOW_SCALING, || config::get_double("Bow.ScalingMultiplier", 2.0))?;
    b.toggle(damage, "Bow Ash of War", "Lets every bow take an Ash of War.", &BOW_AOW, || {
        config::get_bool("Bow.UnlockAOW", true) as u8
    })?;

    b.number(upkeep, "Bow Weight", "Weight multiplier for bows and greatbows.", &BOW_WEIGHT, || config::get_double("Bow.WeightMultiplier", 0.5))?;
    b.number(upkeep, "Crossbow Weight", "Weight multiplier for crossbows.", &CROSSBOW_WEIGHT, || config::get_double("Crossbow.WeightMultiplier", 0.5))?;
    b.number(upkeep, "Ballista Weight", "Weight multiplier for ballistae.", &BALLISTA_WEIGHT, || config::get_double("Ballista.WeightMultiplier", 0.5))?;
    b.number(upkeep, "Bow Sell Value", "Sell price multiplier for bows and greatbows.", &BOW_SELL, || config::get_double("Bow.SellValueMultiplier", 2.0))?;
    b.number(upkeep, "Crossbow Sell Value", "Sell price multiplier for crossbows.", &CROSSBOW_SELL, || config::get_double("Crossbow.SellValueMultiplier", 2.0))?;
    b.number(upkeep, "Ballista Sell Value", "Sell price multiplier for ballistae.", &BALLISTA_SELL, || config::get_double("Ballista.SellValueMultiplier", 2.0))?;

    b.slider(ammo, "Arrow Limit", "How many arrows you can carry. Vanilla is lower.", 0, 255, &ARROW_MAX, || {
        limit_row("Bullet.Arrow.MaxQuantity")
    })?;
    b.slider(ammo, "Bolt Limit", "How many bolts you can carry. Vanilla is lower.", 0, 255, &BOLT_MAX, || {
        limit_row("Bullet.Bolt.MaxQuantity")
    })?;
    b.number(ammo, "Projectile Speed", "How fast arrows and bolts fly. 1x = vanilla.", &BULLET_SPEED, || config::get_double("Bullet.SpeedMultiplier", 3.5))?;
    b.number(ammo, "Rain Range", "How far Rain of Arrows reaches. 1x = vanilla.", &BULLET_RANGE, || config::get_double("Bullet.RangeMultiplier", 2.0))?;
    b.number(ammo, "Rain Spread", "How wide Rain of Arrows spreads. 1x = vanilla.", &BULLET_SPREAD, || config::get_double("Bullet.SpreadMultiplier", 2.0))?;
    b.number(ammo, "Rain Arrows", "How many arrows Rain of Arrows fires.", &RAIN_COUNT, || {
        config::get_int("Bullet.RainOfArrowsCount", 6) as f64
    })?;

    b.toggle(page, "Log file", "Write a log file next to the DLL, for troubleshooting.", &LOG_FILE, || {
        config::get_bool("LogFile", false) as u8
    })?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, Some(erui::reload_from_menu));
    erui::spawn_menu(&MENU, hmodule, "rise-archer", "Rise Archer", build);
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
