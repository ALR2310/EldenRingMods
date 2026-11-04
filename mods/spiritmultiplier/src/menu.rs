//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! One page per area: spirit count, summoning, health and look, behavior and teleport, plus a switch
//! for the log file. The mod reads every setting live, once a second or on every tick, so a change
//! applies once it is saved, except `MaxSpirits` (sizes the summon slot array) and `HubHpBars`
//! (patches the HUD movie), which are read once at start-up and say so. Numbers are typed (the file
//! holds decimals and values wider than a byte); the regen mode is a choice row, and a value that is
//! not one of its entries shows the closest one and is left alone until the player changes the row.
//! `Multiplier` and `RegenValue` start at 0 because the mod ignores values of 0 or less.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, Setting};

use crate::config;

/// The text each choice is saved as, in row order (the labels come from the language files).
const REGEN_MODE_CHOICES: &[&str] = &["1", "2", "3"];

/// Index of the table entry closest to `value`.
fn nearest(table: &[&str], value: f64) -> u8 {
    let mut best = (0usize, f64::MAX);
    for (i, text) in table.iter().enumerate() {
        let distance = (text.parse::<f64>().unwrap_or(0.0) - value).abs();
        if distance < best.1 {
            best = (i, distance);
        }
    }
    best.0 as u8
}

fn regen_mode_row() -> u8 {
    nearest(REGEN_MODE_CHOICES, f64::from(config::get().spirit.regen_mode))
}

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

static MULTIPLIER: NumberSetting =
    NumberSetting::new("Spirit.Multiplier", 0.0, 1000.0, None).slider(0.0, 10.0, 0.25);
static AMOUNT: NumberSetting = NumberSetting::whole("Spirit.Amount", 0.0, 1000.0, None);
static MAX_SPIRITS: NumberSetting = NumberSetting::whole("Spirit.MaxSpirits", 60.0, 1000.0, None);
static HP_BARS: NumberSetting = NumberSetting::whole("Spirit.HubHpBars", 5.0, 30.0, None);
static MULTI_SPIRIT: Setting = Setting::flag("Spirit.MultiSpirit");
static CLONE_SPIRIT: Setting = Setting::flag("Spirit.CloneSpirit");
static NO_REST: Setting = Setting::flag("Spirit.NoRestResummon");
static ANYWHERE: Setting = Setting::flag("Spirit.SummonAnywhere");
static REGEN_MODE: Setting = Setting {
    key: "Spirit.RegenMode",
    format: |index| {
        REGEN_MODE_CHOICES[(index as usize).min(REGEN_MODE_CHOICES.len() - 1)].to_string()
    },
    apply: None,
};
static REGEN_VALUE: NumberSetting = NumberSetting::new("Spirit.RegenValue", 0.0, 1000.0, None);
static GHOST_COLOR: Setting = Setting::flag("Spirit.GhostColor");
static INSTANT_MIMIC: Setting = Setting::flag("Spirit.InstantMimic");
static SENSES: Setting = Setting::flag("Spirit.ImproveSenses");
static AGGRESSION: Setting = Setting::flag("Spirit.ImproveAggression");
static FOLLOW: Setting = Setting::flag("Spirit.ImproveFollow");
static WARP_WHEN_FAR: Setting = Setting::flag("Spirit.WarpWhenFar");
static WARP_DISTANCE: NumberSetting = NumberSetting::new("Spirit.WarpDistance", 0.0, 1000.0, None);
static WARP_BLOCKED: NumberSetting = NumberSetting::new("Spirit.WarpBlockedTime", 0.0, 600.0, None);
static WARP_STUCK_TIME: NumberSetting =
    NumberSetting::new("Spirit.WarpStuckTime", 0.0, 600.0, None);
static WARP_STUCK_RANGE: NumberSetting =
    NumberSetting::new("Spirit.WarpStuckRange", 0.0, 100.0, None).slider(0.0, 100.0, 1.0);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Spirit Multiplier", &t("mod.help"))?;

    let count = b.submenu(page, &t("count.title"), &t("count.help"))?;
    b.number(count, &t("multiplier.label"), &t("multiplier.help"), &MULTIPLIER, || {
        config::get().spirit.multiplier
    })?;
    b.number(count, &t("fixed_count.label"), &t("fixed_count.help"), &AMOUNT, || {
        f64::from(config::get().spirit.amount)
    })?;
    b.number(count, &t("max_spirits.label"), &t("max_spirits.help"), &MAX_SPIRITS, || {
        f64::from(config::get().spirit.max_spirits)
    })?;

    let summoning = b.submenu(page, &t("summoning.title"), &t("summoning.help"))?;
    b.toggle(summoning, &t("multi_spirit.label"), &t("multi_spirit.help"), &MULTI_SPIRIT, || {
        config::get().spirit.multi_spirit as u8
    })?;
    b.toggle(summoning, &t("clone_spirit.label"), &t("clone_spirit.help"), &CLONE_SPIRIT, || {
        config::get().spirit.clone_spirit as u8
    })?;
    b.toggle(summoning, &t("no_rest.label"), &t("no_rest.help"), &NO_REST, || {
        config::get().spirit.no_rest_resummon as u8
    })?;
    b.toggle(summoning, &t("anywhere.label"), &t("anywhere.help"), &ANYWHERE, || {
        config::get().spirit.summon_anywhere as u8
    })?;

    let health = b.submenu(page, &t("health.title"), &t("health.help"))?;
    let options = [
        t("regen_mode.options.flat"),
        t("regen_mode.options.percent_max"),
        t("regen_mode.options.percent_missing"),
    ];
    let labels: Vec<&str> = options.iter().map(String::as_str).collect();
    b.choice(
        health,
        &t("regen_mode.label"),
        &t("regen_mode.help"),
        &labels,
        &REGEN_MODE,
        regen_mode_row,
    )?;
    b.number(health, &t("regen_amount.label"), &t("regen_amount.help"), &REGEN_VALUE, || {
        config::get().spirit.regen_value
    })?;
    b.toggle(health, &t("ghost.label"), &t("ghost.help"), &GHOST_COLOR, || {
        config::get().spirit.ghost_color as u8
    })?;
    b.number(health, &t("hp_bars.label"), &t("hp_bars.help"), &HP_BARS, || {
        f64::from(config::get().spirit.hub_hp_bars)
    })?;
    b.toggle(health, &t("mimic.label"), &t("mimic.help"), &INSTANT_MIMIC, || {
        config::get().spirit.instant_mimic as u8
    })?;

    let behavior = b.submenu(page, &t("behavior.title"), &t("behavior.help"))?;
    b.toggle(behavior, &t("senses.label"), &t("senses.help"), &SENSES, || {
        config::get().spirit.improve_senses as u8
    })?;
    b.toggle(behavior, &t("aggression.label"), &t("aggression.help"), &AGGRESSION, || {
        config::get().spirit.improve_aggression as u8
    })?;
    b.toggle(behavior, &t("follow.label"), &t("follow.help"), &FOLLOW, || {
        config::get().spirit.improve_follow as u8
    })?;

    let teleport = b.submenu(page, &t("teleport.title"), &t("teleport.help"))?;
    b.toggle(teleport, &t("warp_far.label"), &t("warp_far.help"), &WARP_WHEN_FAR, || {
        config::get().spirit.warp_when_far as u8
    })?;
    b.number(
        teleport,
        &t("warp_distance.label"),
        &t("warp_distance.help"),
        &WARP_DISTANCE,
        || config::get().spirit.warp_distance,
    )?;
    b.number(teleport, &t("warp_blocked.label"), &t("warp_blocked.help"), &WARP_BLOCKED, || {
        config::get().spirit.warp_blocked_time
    })?;
    b.number(
        teleport,
        &t("warp_stuck_time.label"),
        &t("warp_stuck_time.help"),
        &WARP_STUCK_TIME,
        || config::get().spirit.warp_stuck_time,
    )?;
    b.number(
        teleport,
        &t("warp_stuck_range.label"),
        &t("warp_stuck_range.help"),
        &WARP_STUCK_RANGE,
        || config::get().spirit.warp_stuck_range,
    )?;

    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "spirit-multiplier", "Spirit Multiplier", build);
}

/// The mod reads every setting live; only the log switch needs following.
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 31);
    }

    #[test]
    fn a_regen_mode_outside_the_table_shows_the_closest_entry() {
        assert_eq!(nearest(REGEN_MODE_CHOICES, 2.0), 1);
        assert_eq!(nearest(REGEN_MODE_CHOICES, 9.0), 2);
        assert_eq!(nearest(REGEN_MODE_CHOICES, -4.0), 0);
    }
}
