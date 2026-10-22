//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Spirit Multiplier           -> opens a page with:
//!   Spirit Count              -> Multiplier   number 0..1000 (0 = off, vanilla count)
//!                                Fixed Count  number 0..1000 (0 = use Multiplier)
//!                                Max Spirits  number 60..1000 (restart only)
//!   Summoning                 -> Multi Spirit, Clone Spirit, Resummon Without Rest,
//!                                Summon Anywhere (toggles)
//!   Health and Look           -> Regen Mode (choice), Regen Amount (number 0..1000,
//!                                0 = no healing), Ghost Color (toggle)
//!   Behavior                  -> Senses, Aggression, Follow (toggles)
//!   Teleport                  -> Warp When Far (toggle), Distance, Blocked Time,
//!                                Stuck Time, Stuck Range (numbers, 0 = game default)
//!   Log file                  on/off
//! ```
//!
//! Every setting except `MaxSpirits` is re-read by the mod once a second (or
//! every tick), so a change is live as soon as the saver has written it to
//! the config map. `MaxSpirits` sizes the summon slot array at startup, so
//! its row says it needs a restart. Numbers are typed into text rows (the ini
//! holds decimals and values wider than a byte); the regen mode stays a choice
//! row, and an ini value that is not one of its entries shows the closest one
//! and is left alone until the player changes the row. `Multiplier` and
//! `RegenValue` start at 0 because the mod ignores values <= 0 (no multiplying,
//! no healing), so a negative number would only mean the same as 0.

use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::config;
use common::i18n::Catalog;

/// The ini text of each choice, in row order (the labels come from the language files).
type Table = [&'static str];

const REGEN_MODE_CHOICES: &Table = &["1", "2", "3"];

/// Index of the table entry closest to `value`.
fn nearest(table: &Table, value: f64) -> u8 {
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
    nearest(REGEN_MODE_CHOICES, config::get_double("RegenMode", 2.0))
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

static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", 0.0, 1000.0, None).slider(0.0, 10.0, 0.25);
static AMOUNT: NumberSetting = NumberSetting::whole("Amount", 0.0, 1000.0, None);
static MAX_SPIRITS: NumberSetting = NumberSetting::whole("MaxSpirits", 60.0, 1000.0, None);
static MULTI_SPIRIT: Setting = Setting::flag("MultiSpirit");
static CLONE_SPIRIT: Setting = Setting::flag("CloneSpirit");
static NO_REST: Setting = Setting::flag("NoRestResummon");
static ANYWHERE: Setting = Setting::flag("SummonAnywhere");
static REGEN_MODE: Setting = Setting {
    key: "RegenMode",
    format: |index| REGEN_MODE_CHOICES[(index as usize).min(REGEN_MODE_CHOICES.len() - 1)].to_string(),
    apply: None,
};
static REGEN_VALUE: NumberSetting = NumberSetting::new("RegenValue", 0.0, 1000.0, None);
static GHOST_COLOR: Setting = Setting::flag("GhostColor");
static INSTANT_MIMIC: Setting = Setting::flag("InstantMimic");
static SENSES: Setting = Setting::flag("ImproveSenses");
static AGGRESSION: Setting = Setting::flag("ImproveAggression");
static FOLLOW: Setting = Setting::flag("ImproveFollow");
static WARP_WHEN_FAR: Setting = Setting::flag("WarpWhenFar");
static WARP_DISTANCE: NumberSetting = NumberSetting::new("WarpDistance", 0.0, 1000.0, None);
static WARP_BLOCKED: NumberSetting = NumberSetting::new("WarpBlockedTime", 0.0, 600.0, None);
static WARP_STUCK_TIME: NumberSetting = NumberSetting::new("WarpStuckTime", 0.0, 600.0, None);
static WARP_STUCK_RANGE: NumberSetting = NumberSetting::new("WarpStuckRange", 0.0, 100.0, None).slider(0.0, 100.0, 1.0);
static LOG_FILE: Setting = Setting::flag("LogFile");

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Spirit Multiplier", &t("mod.help"))?;

    let count = b.submenu(page, &t("count.title"), &t("count.help"))?;
    b.number(
        count,
        &t("multiplier.label"),
        &t("multiplier.help"),
        &MULTIPLIER,
        || config::get_double("Multiplier", 2.0),
    )?;
    b.number(
        count,
        &t("fixed_count.label"),
        &t("fixed_count.help"),
        &AMOUNT,
        || config::get_double("Amount", 0.0),
    )?;
    b.number(
        count,
        &t("max_spirits.label"),
        &t("max_spirits.help"),
        &MAX_SPIRITS,
        || config::get_double("MaxSpirits", 60.0),
    )?;

    let summoning = b.submenu(page, &t("summoning.title"), &t("summoning.help"))?;
    b.toggle(
        summoning,
        &t("multi_spirit.label"),
        &t("multi_spirit.help"),
        &MULTI_SPIRIT,
        || config::get_bool("MultiSpirit", true) as u8,
    )?;
    b.toggle(
        summoning,
        &t("clone_spirit.label"),
        &t("clone_spirit.help"),
        &CLONE_SPIRIT,
        || config::get_bool("CloneSpirit", false) as u8,
    )?;
    b.toggle(
        summoning,
        &t("no_rest.label"),
        &t("no_rest.help"),
        &NO_REST,
        || config::get_bool("NoRestResummon", true) as u8,
    )?;
    b.toggle(
        summoning,
        &t("anywhere.label"),
        &t("anywhere.help"),
        &ANYWHERE,
        || config::get_bool("SummonAnywhere", true) as u8,
    )?;

    let health = b.submenu(page, &t("health.title"), &t("health.help"))?;
    let options = [t("regen_mode.options.flat"), t("regen_mode.options.percent_max"), t("regen_mode.options.percent_missing")];
    let labels: Vec<&str> = options.iter().map(String::as_str).collect();
    b.choice(health, &t("regen_mode.label"), &t("regen_mode.help"), &labels, &REGEN_MODE, regen_mode_row)?;
    b.number(
        health,
        &t("regen_amount.label"),
        &t("regen_amount.help"),
        &REGEN_VALUE,
        || config::get_double("RegenValue", 0.5),
    )?;
    b.toggle(
        health,
        &t("ghost.label"),
        &t("ghost.help"),
        &GHOST_COLOR,
        || config::get_bool("GhostColor", true) as u8,
    )?;
    b.toggle(
        health,
        &t("mimic.label"),
        &t("mimic.help"),
        &INSTANT_MIMIC,
        || config::get_bool("InstantMimic", true) as u8,
    )?;

    let behavior = b.submenu(page, &t("behavior.title"), &t("behavior.help"))?;
    b.toggle(
        behavior,
        &t("senses.label"),
        &t("senses.help"),
        &SENSES,
        || config::get_bool("ImproveSenses", true) as u8,
    )?;
    b.toggle(
        behavior,
        &t("aggression.label"),
        &t("aggression.help"),
        &AGGRESSION,
        || config::get_bool("ImproveAggression", true) as u8,
    )?;
    b.toggle(
        behavior,
        &t("follow.label"),
        &t("follow.help"),
        &FOLLOW,
        || config::get_bool("ImproveFollow", true) as u8,
    )?;

    let teleport = b.submenu(page, &t("teleport.title"), &t("teleport.help"))?;
    b.toggle(
        teleport,
        &t("warp_far.label"),
        &t("warp_far.help"),
        &WARP_WHEN_FAR,
        || config::get_bool("WarpWhenFar", false) as u8,
    )?;
    b.number(
        teleport,
        &t("warp_distance.label"),
        &t("warp_distance.help"),
        &WARP_DISTANCE,
        || config::get_double("WarpDistance", 0.0),
    )?;
    b.number(
        teleport,
        &t("warp_blocked.label"),
        &t("warp_blocked.help"),
        &WARP_BLOCKED,
        || config::get_double("WarpBlockedTime", 0.0),
    )?;
    b.number(
        teleport,
        &t("warp_stuck_time.label"),
        &t("warp_stuck_time.help"),
        &WARP_STUCK_TIME,
        || config::get_double("WarpStuckTime", 0.0),
    )?;
    b.number(
        teleport,
        &t("warp_stuck_range.label"),
        &t("warp_stuck_range.help"),
        &WARP_STUCK_RANGE,
        || config::get_double("WarpStuckRange", 0.0),
    )?;

    b.toggle(
        page,
        &t("log.label"),
        &t("log.help"),
        &LOG_FILE,
        || config::get_bool("LogFile", false) as u8,
    )?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    // Every setting is re-read by the mod on its own; no reload request needed.
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "spirit-multiplier", "Spirit Multiplier", build);
    // Follow the reload key: show the reloaded values in the menu.
    std::thread::spawn(|| {
        let mut seen = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let now = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if now != seen {
                seen = now;
                sync_from_config();
            }
        }
    });
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

    /// Every key the menu passes to the text lookup is in every language file,
    /// and the files have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        let source = include_str!("menu.rs");
        let mut used: Vec<String> = Vec::new();
        for (at, _) in source.match_indices("t(\"") {
            let before = source[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &source[at + 3..];
            if let Some(end) = rest.find('"') {
                used.push(rest[..end].to_string());
            }
        }
        used.sort();
        used.dedup();
        assert!(used.len() > 30, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }

    /// Every row meant to be a native slider has a valid range and step (else
    /// it silently stays a typed row).
    #[test]
    fn slider_rows_have_a_valid_range() {
        for setting in [&MULTIPLIER, &WARP_STUCK_RANGE] {
            assert!(setting.has_slider(), "{} would not be a slider", setting.key);
        }
    }
}
