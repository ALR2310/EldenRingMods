//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! Three pages (damage, weight and value, ammo) and a switch for the log file. Multipliers are
//! sliders from 0 to 10 (a typed row up to 1000 on older hosts), the carry limits are sliders from 0
//! to 255 and the Rain of Arrows count is typed. A change is saved a second after the player stops,
//! then applied again from the cached original param values, like the reload key.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

const MIN: f64 = -1000.0;
const MAX: f64 = 1000.0;

static BOW_DAMAGE: NumberSetting =
    NumberSetting::new("Bow.DamageMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static CROSSBOW_DAMAGE: NumberSetting =
    NumberSetting::new("Crossbow.DamageMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static BALLISTA_DAMAGE: NumberSetting =
    NumberSetting::new("Ballista.DamageMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static BOW_SCALING: NumberSetting =
    NumberSetting::new("Bow.ScalingMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static BOW_AOW: Setting = Setting::flag("Bow.UnlockAOW");
static BOW_WEIGHT: NumberSetting =
    NumberSetting::new("Bow.WeightMultiplier", MIN, MAX, None).slider(0.0, 5.0, 0.1);
static CROSSBOW_WEIGHT: NumberSetting =
    NumberSetting::new("Crossbow.WeightMultiplier", MIN, MAX, None).slider(0.0, 5.0, 0.1);
static BALLISTA_WEIGHT: NumberSetting =
    NumberSetting::new("Ballista.WeightMultiplier", MIN, MAX, None).slider(0.0, 5.0, 0.1);
static BOW_SELL: NumberSetting =
    NumberSetting::new("Bow.SellValueMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.5);
static CROSSBOW_SELL: NumberSetting =
    NumberSetting::new("Crossbow.SellValueMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.5);
static BALLISTA_SELL: NumberSetting =
    NumberSetting::new("Ballista.SellValueMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.5);
static ARROW_MAX: Setting = Setting::int("Bullet.ArrowMaxQuantity");
static BOLT_MAX: Setting = Setting::int("Bullet.BoltMaxQuantity");
static BULLET_SPEED: NumberSetting =
    NumberSetting::new("Bullet.SpeedMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static BULLET_RANGE: NumberSetting =
    NumberSetting::new("Bullet.RangeMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static BULLET_SPREAD: NumberSetting =
    NumberSetting::new("Bullet.SpreadMultiplier", MIN, MAX, None).slider(0.0, 10.0, 0.1);
static RAIN_COUNT: NumberSetting =
    NumberSetting::whole("Bullet.RainOfArrowsCount", 0.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

/// A carry limit as a slider position: the mod accepts 0 to 255.
fn limit_row(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Rise Archer", &t("mod.help"))?;
    let damage = b.submenu(page, &t("damage.title"), &t("damage.help"))?;
    let upkeep = b.submenu(page, &t("upkeep.title"), &t("upkeep.help"))?;
    let ammo = b.submenu(page, &t("ammo.title"), &t("ammo.help"))?;

    b.number(damage, &t("bow_damage.label"), &t("bow_damage.help"), &BOW_DAMAGE, || {
        config::get().bow.damage_multiplier
    })?;
    b.number(
        damage,
        &t("crossbow_damage.label"),
        &t("crossbow_damage.help"),
        &CROSSBOW_DAMAGE,
        || config::get().crossbow.damage_multiplier,
    )?;
    b.number(
        damage,
        &t("ballista_damage.label"),
        &t("ballista_damage.help"),
        &BALLISTA_DAMAGE,
        || config::get().ballista.damage_multiplier,
    )?;
    b.number(damage, &t("bow_scaling.label"), &t("bow_scaling.help"), &BOW_SCALING, || {
        config::get().bow.scaling_multiplier
    })?;
    b.toggle(damage, &t("bow_aow.label"), &t("bow_aow.help"), &BOW_AOW, || {
        config::get().bow.unlock_aow as u8
    })?;

    b.number(upkeep, &t("bow_weight.label"), &t("bow_weight.help"), &BOW_WEIGHT, || {
        config::get().bow.weight_multiplier
    })?;
    b.number(
        upkeep,
        &t("crossbow_weight.label"),
        &t("crossbow_weight.help"),
        &CROSSBOW_WEIGHT,
        || config::get().crossbow.weight_multiplier,
    )?;
    b.number(
        upkeep,
        &t("ballista_weight.label"),
        &t("ballista_weight.help"),
        &BALLISTA_WEIGHT,
        || config::get().ballista.weight_multiplier,
    )?;
    b.number(upkeep, &t("bow_sell.label"), &t("bow_sell.help"), &BOW_SELL, || {
        config::get().bow.sell_value_multiplier
    })?;
    b.number(upkeep, &t("crossbow_sell.label"), &t("crossbow_sell.help"), &CROSSBOW_SELL, || {
        config::get().crossbow.sell_value_multiplier
    })?;
    b.number(upkeep, &t("ballista_sell.label"), &t("ballista_sell.help"), &BALLISTA_SELL, || {
        config::get().ballista.sell_value_multiplier
    })?;

    b.slider(ammo, &t("arrow_limit.label"), &t("arrow_limit.help"), (0, 255), &ARROW_MAX, || {
        limit_row(config::get().bullet.arrow_max_quantity)
    })?;
    b.slider(ammo, &t("bolt_limit.label"), &t("bolt_limit.help"), (0, 255), &BOLT_MAX, || {
        limit_row(config::get().bullet.bolt_max_quantity)
    })?;
    b.number(ammo, &t("bullet_speed.label"), &t("bullet_speed.help"), &BULLET_SPEED, || {
        config::get().bullet.speed_multiplier
    })?;
    b.number(ammo, &t("rain_range.label"), &t("rain_range.help"), &BULLET_RANGE, || {
        config::get().bullet.range_multiplier
    })?;
    b.number(ammo, &t("rain_spread.label"), &t("rain_spread.help"), &BULLET_SPREAD, || {
        config::get().bullet.spread_multiplier
    })?;
    b.number(ammo, &t("rain_count.label"), &t("rain_count.help"), &RAIN_COUNT, || {
        f64::from(config::get().bullet.rain_of_arrows_count)
    })?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "rise-archer", "Rise Archer", build);
}

/// A saved change takes effect like a reload, minus the menu refresh: the rows already show it.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
    crate::request_apply();
}

pub fn sync_from_config() {
    menu::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu uses is in every language file, and the files have the same keys.
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 21);
    }

    #[test]
    fn carry_limits_stay_on_the_slider() {
        assert_eq!((limit_row(-3), limit_row(99), limit_row(900)), (0, 99, 255));
    }
}
