//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! The regen code reads the config every frame, so a change takes effect once it is saved, a second
//! after the player stops. Amounts and the interval are text rows (the file holds decimals such as
//! 0.5). The gesture list stays file-only.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

static TICK_ENABLED: Setting = Setting::flag("PerTick.Enabled");
static TICK_TRIGGER: Setting = Setting::int("PerTick.Trigger");
static TICK_INTERVAL: NumberSetting =
    NumberSetting::whole("PerTick.Interval", 0.0, 3_600_000.0, None);
static TICK_VALUE_TYPE: Setting = Setting::int("PerTick.ValueType");
static TICK_HP: NumberSetting = NumberSetting::new("PerTick.HP", -1000.0, 1000.0, None);
static TICK_FP: NumberSetting = NumberSetting::new("PerTick.FP", -1000.0, 1000.0, None);
static TICK_SP: NumberSetting = NumberSetting::new("PerTick.SP", -1000.0, 1000.0, None);
// `Cap` is a TOML float, so the slider's whole number is saved as `80.0`.
static TICK_CAP: Setting = Setting { key: "PerTick.Cap", format: float_text, apply: None };
static HIT_ENABLED: Setting = Setting::flag("PerHit.Enabled");
static HIT_MODE: Setting = Setting::int("PerHit.Mode");
static HIT_TYPE: Setting = Setting::int("PerHit.DamageType");
static HIT_EXCLUDE_AOW: Setting = Setting::flag("PerHit.ExcludeAow");
static HIT_HP: NumberSetting = NumberSetting::new("PerHit.HP", -1000.0, 1000.0, None);
static HIT_FP: NumberSetting = NumberSetting::new("PerHit.FP", -1000.0, 1000.0, None);
static HIT_SP: NumberSetting = NumberSetting::new("PerHit.SP", -1000.0, 1000.0, None);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

fn float_text(value: u8) -> String {
    format!("{value}.0")
}

/// A choice row's index, kept inside its options.
fn choice_row(value: i32, last: u8) -> u8 {
    value.clamp(0, i32::from(last)) as u8
}

fn cap_row() -> u8 {
    config::get().per_tick.cap.round().clamp(1.0, 100.0) as u8
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
    let page = b.submenu(b.root(), "Auto Regen", &t("mod.help"))?;
    let tick = b.submenu(page, &t("tick.title"), &t("tick.help"))?;
    let hit = b.submenu(page, &t("hit.title"), &t("hit.help"))?;

    b.toggle(tick, &t("tick_enabled.label"), &t("tick_enabled.help"), &TICK_ENABLED, || {
        config::get().per_tick.enabled as u8
    })?;
    let when = [
        t("when.options.always"),
        t("when.options.out_of_combat"),
        t("when.options.in_combat"),
        t("when.options.idle"),
        t("when.options.during_gesture"),
    ];
    let when: Vec<&str> = when.iter().map(String::as_str).collect();
    b.choice(tick, &t("when.label"), &t("when.help"), &when, &TICK_TRIGGER, || {
        choice_row(config::get().per_tick.trigger, 4)
    })?;
    b.number(tick, &t("interval.label"), &t("interval.help"), &TICK_INTERVAL, || {
        f64::from(config::get().per_tick.interval)
    })?;
    let value_type = [
        t("value_type.options.points"),
        t("value_type.options.percent_of_max"),
        t("value_type.options.percent_of_missing"),
    ];
    let value_type: Vec<&str> = value_type.iter().map(String::as_str).collect();
    b.choice(
        tick,
        &t("value_type.label"),
        &t("value_type.help"),
        &value_type,
        &TICK_VALUE_TYPE,
        || choice_row(config::get().per_tick.value_type, 2),
    )?;
    b.number(tick, &t("tick_hp.label"), &t("tick_hp.help"), &TICK_HP, || {
        config::get().per_tick.hp
    })?;
    b.number(tick, &t("tick_fp.label"), &t("tick_fp.help"), &TICK_FP, || {
        config::get().per_tick.fp
    })?;
    b.number(tick, &t("tick_sp.label"), &t("tick_sp.help"), &TICK_SP, || {
        config::get().per_tick.sp
    })?;
    b.slider(tick, &t("max_fill.label"), &t("max_fill.help"), (1, 100), &TICK_CAP, cap_row)?;

    b.toggle(hit, &t("hit_enabled.label"), &t("hit_enabled.help"), &HIT_ENABLED, || {
        config::get().per_hit.enabled as u8
    })?;
    let mode = [
        t("mode.options.points"),
        t("mode.options.percent_of_max"),
        t("mode.options.percent_of_damage"),
    ];
    let mode: Vec<&str> = mode.iter().map(String::as_str).collect();
    b.choice(hit, &t("mode.label"), &t("mode.help"), &mode, &HIT_MODE, || {
        choice_row(config::get().per_hit.mode, 2)
    })?;
    let hit_type =
        [t("hit_type.options.melee"), t("hit_type.options.ranged"), t("hit_type.options.both")];
    let hit_type: Vec<&str> = hit_type.iter().map(String::as_str).collect();
    b.choice(hit, &t("hit_type.label"), &t("hit_type.help"), &hit_type, &HIT_TYPE, || {
        choice_row(config::get().per_hit.damage_type, 2)
    })?;
    b.toggle(hit, &t("skip_aow.label"), &t("skip_aow.help"), &HIT_EXCLUDE_AOW, || {
        config::get().per_hit.exclude_aow as u8
    })?;
    b.number(hit, &t("hit_hp.label"), &t("hit_hp.help"), &HIT_HP, || config::get().per_hit.hp)?;
    b.number(hit, &t("hit_fp.label"), &t("hit_fp.help"), &HIT_FP, || config::get().per_hit.fp)?;
    b.number(hit, &t("hit_sp.label"), &t("hit_sp.help"), &HIT_SP, || config::get().per_hit.sp)?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "auto-regen", "Auto Regen", build);
}

/// The regen code reads its config every frame; only the log switch needs following.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 41);
    }

    #[test]
    fn choice_rows_stay_inside_their_options() {
        assert_eq!(choice_row(-1, 4), 0);
        assert_eq!(choice_row(3, 4), 3);
        assert_eq!(choice_row(9, 2), 2);
    }
}
