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
use common::i18n::Catalog;

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
    let page = b.submenu(b.root(), "Auto Regen", &t("mod.help"))?;
    let tick = b.submenu(page, &t("tick.title"), &t("tick.help"))?;
    let hit = b.submenu(page, &t("hit.title"), &t("hit.help"))?;

    b.toggle(tick, &t("tick_enabled.label"), &t("tick_enabled.help"), &TICK_ENABLED, || {
        config::get_bool("Regen.PerTick.Enabled", true) as u8
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
        enum_row("Regen.PerTick.Trigger", 0, 4)
    })?;
    b.number(tick, &t("interval.label"), &t("interval.help"), &TICK_INTERVAL, || {
        config::get_int("Regen.PerTick.Interval", 1000) as f64
    })?;
    let value_type = [
        t("value_type.options.points"),
        t("value_type.options.percent_of_max"),
        t("value_type.options.percent_of_missing"),
    ];
    let value_type: Vec<&str> = value_type.iter().map(String::as_str).collect();
    b.choice(tick, &t("value_type.label"), &t("value_type.help"), &value_type, &TICK_VALUE_TYPE, || {
        enum_row("Regen.PerTick.ValueType", 0, 2)
    })?;
    b.number(tick, &t("tick_hp.label"), &t("tick_hp.help"), &TICK_HP, || {
        config::get_double("Regen.PerTick.HP", 0.0)
    })?;
    b.number(tick, &t("tick_fp.label"), &t("tick_fp.help"), &TICK_FP, || {
        config::get_double("Regen.PerTick.FP", 0.0)
    })?;
    b.number(tick, &t("tick_sp.label"), &t("tick_sp.help"), &TICK_SP, || {
        config::get_double("Regen.PerTick.SP", 0.0)
    })?;
    b.slider(tick, &t("max_fill.label"), &t("max_fill.help"), 1, 100, &TICK_CAP, cap_row)?;

    b.toggle(hit, &t("hit_enabled.label"), &t("hit_enabled.help"), &HIT_ENABLED, || {
        config::get_bool("Regen.PerHit.Enabled", false) as u8
    })?;
    let mode = [
        t("mode.options.points"),
        t("mode.options.percent_of_max"),
        t("mode.options.percent_of_damage"),
    ];
    let mode: Vec<&str> = mode.iter().map(String::as_str).collect();
    b.choice(hit, &t("mode.label"), &t("mode.help"), &mode, &HIT_MODE, || enum_row("Regen.PerHit.Mode", 0, 2))?;
    let hit_type = [t("hit_type.options.melee"), t("hit_type.options.ranged"), t("hit_type.options.both")];
    let hit_type: Vec<&str> = hit_type.iter().map(String::as_str).collect();
    b.choice(hit, &t("hit_type.label"), &t("hit_type.help"), &hit_type, &HIT_TYPE, || {
        enum_row("Regen.PerHit.DamageType", 0, 2)
    })?;
    b.toggle(hit, &t("skip_aow.label"), &t("skip_aow.help"), &HIT_EXCLUDE_AOW, || {
        config::get_bool("Regen.PerHit.ExcludeAow", false) as u8
    })?;
    b.number(hit, &t("hit_hp.label"), &t("hit_hp.help"), &HIT_HP, || config::get_double("Regen.PerHit.HP", 0.0))?;
    b.number(hit, &t("hit_fp.label"), &t("hit_fp.help"), &HIT_FP, || config::get_double("Regen.PerHit.FP", 0.0))?;
    b.number(hit, &t("hit_sp.label"), &t("hit_sp.help"), &HIT_SP, || config::get_double("Regen.PerHit.SP", 0.0))?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || config::get_bool("LogFile", false) as u8)?;
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
    erui::sync(&MENU);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu passes to `t` is in every language file, and the files
    /// have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 41);
    }
}
