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
//!
//! The menu texts live in `locales/` (one JSON file per language, English is
//! the fallback); the mod name stays English.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, NumberSetting, Setting};
use common::i18n::Catalog;

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

/// The menu texts, one file per language (`locales/`); English is the fallback.
static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

/// The text of `key` in the host language.
fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn build(b: &mut erui::MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Rise Archer", &t("mod.help"))?;
    let damage = b.submenu(page, &t("damage.title"), &t("damage.help"))?;
    let upkeep = b.submenu(page, &t("upkeep.title"), &t("upkeep.help"))?;
    let ammo = b.submenu(page, &t("ammo.title"), &t("ammo.help"))?;

    b.number(damage, &t("bow_damage.label"), &t("bow_damage.help"), &BOW_DAMAGE, || config::get_double("Bow.DamageMultiplier", 1.5))?;
    b.number(damage, &t("crossbow_damage.label"), &t("crossbow_damage.help"), &CROSSBOW_DAMAGE, || config::get_double("Crossbow.DamageMultiplier", 3.0))?;
    b.number(damage, &t("ballista_damage.label"), &t("ballista_damage.help"), &BALLISTA_DAMAGE, || config::get_double("Ballista.DamageMultiplier", 4.0))?;
    b.number(damage, &t("bow_scaling.label"), &t("bow_scaling.help"), &BOW_SCALING, || config::get_double("Bow.ScalingMultiplier", 2.0))?;
    b.toggle(damage, &t("bow_aow.label"), &t("bow_aow.help"), &BOW_AOW, || {
        config::get_bool("Bow.UnlockAOW", true) as u8
    })?;

    b.number(upkeep, &t("bow_weight.label"), &t("bow_weight.help"), &BOW_WEIGHT, || config::get_double("Bow.WeightMultiplier", 0.5))?;
    b.number(upkeep, &t("crossbow_weight.label"), &t("crossbow_weight.help"), &CROSSBOW_WEIGHT, || config::get_double("Crossbow.WeightMultiplier", 0.5))?;
    b.number(upkeep, &t("ballista_weight.label"), &t("ballista_weight.help"), &BALLISTA_WEIGHT, || config::get_double("Ballista.WeightMultiplier", 0.5))?;
    b.number(upkeep, &t("bow_sell.label"), &t("bow_sell.help"), &BOW_SELL, || config::get_double("Bow.SellValueMultiplier", 2.0))?;
    b.number(upkeep, &t("crossbow_sell.label"), &t("crossbow_sell.help"), &CROSSBOW_SELL, || config::get_double("Crossbow.SellValueMultiplier", 2.0))?;
    b.number(upkeep, &t("ballista_sell.label"), &t("ballista_sell.help"), &BALLISTA_SELL, || config::get_double("Ballista.SellValueMultiplier", 2.0))?;

    b.slider(ammo, &t("arrow_limit.label"), &t("arrow_limit.help"), 0, 255, &ARROW_MAX, || {
        limit_row("Bullet.Arrow.MaxQuantity")
    })?;
    b.slider(ammo, &t("bolt_limit.label"), &t("bolt_limit.help"), 0, 255, &BOLT_MAX, || {
        limit_row("Bullet.Bolt.MaxQuantity")
    })?;
    b.number(ammo, &t("bullet_speed.label"), &t("bullet_speed.help"), &BULLET_SPEED, || config::get_double("Bullet.SpeedMultiplier", 3.5))?;
    b.number(ammo, &t("rain_range.label"), &t("rain_range.help"), &BULLET_RANGE, || config::get_double("Bullet.RangeMultiplier", 2.0))?;
    b.number(ammo, &t("rain_spread.label"), &t("rain_spread.help"), &BULLET_SPREAD, || config::get_double("Bullet.SpreadMultiplier", 2.0))?;
    b.number(ammo, &t("rain_count.label"), &t("rain_count.help"), &RAIN_COUNT, || {
        config::get_int("Bullet.RainOfArrowsCount", 6) as f64
    })?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu passes to the lookup function is in every language
    /// file, and the files have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        let source = include_str!("menu.rs");
        let mut used: Vec<String> = Vec::new();
        for (at, _) in source.match_indices("t(\"") {
            // Skip names that merely end in the letter t (for example a call to `format(`).
            let glued = source[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
            if glued {
                continue;
            }
            let rest = &source[at + 3..];
            if let Some(end) = rest.find('"') {
                used.push(rest[..end].to_string());
            }
        }
        used.sort();
        used.dedup();
        assert!(used.len() > 20, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }
}
