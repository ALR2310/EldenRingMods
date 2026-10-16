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
//!   Black Flame              -> same three rows
//!   Destined Death           -> same three rows, plus Max HP Loss (s) number -1..86400
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
use common::i18n::Catalog;

static ROT_DURATION: NumberSetting = NumberSetting::new("ScarletRot.Duration", -1.0, 86_400.0, None);
static ROT_PERCENT: NumberSetting = NumberSetting::new("ScarletRot.PercentDamage", -100.0, 100.0, None);
static ROT_FIXED: NumberSetting = NumberSetting::whole("ScarletRot.FixedDamage", -1000.0, 1000.0, None);
static POISON_DURATION: NumberSetting = NumberSetting::new("Poison.Duration", -1.0, 86_400.0, None);
static POISON_PERCENT: NumberSetting = NumberSetting::new("Poison.PercentDamage", -100.0, 100.0, None);
static POISON_FIXED: NumberSetting = NumberSetting::whole("Poison.FixedDamage", -1000.0, 1000.0, None);
static FLAME_DURATION: NumberSetting = NumberSetting::new("BlackFlame.Duration", -1.0, 86_400.0, None);
static FLAME_PERCENT: NumberSetting = NumberSetting::new("BlackFlame.PercentDamage", -100.0, 100.0, None);
static FLAME_FIXED: NumberSetting = NumberSetting::whole("BlackFlame.FixedDamage", -1000.0, 1000.0, None);
static DESTINED_DURATION: NumberSetting = NumberSetting::new("DestinedDeath.Duration", -1.0, 86_400.0, None);
static DESTINED_PERCENT: NumberSetting = NumberSetting::new("DestinedDeath.PercentDamage", -100.0, 100.0, None);
static DESTINED_FIXED: NumberSetting = NumberSetting::whole("DestinedDeath.FixedDamage", -1000.0, 1000.0, None);
static DESTINED_MAX_HP: NumberSetting = NumberSetting::new("DestinedDeath.MaxHpDuration", -1.0, 86_400.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

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
) -> Result<PageHandle, String> {
    let page = b.submenu(parent, name, help)?;
    let rows = [
        (t("duration.label"), t("duration.help").replace("{vanilla}", vanilla[0])),
        (t("percent.label"), t("percent.help").replace("{vanilla}", vanilla[1])),
        (t("fixed.label"), t("fixed.help").replace("{vanilla}", vanilla[2])),
    ];
    for (i, (label, help)) in rows.iter().enumerate() {
        b.number(page, label, help, settings[i], reads[i])?;
    }
    Ok(page)
}

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Infinite Ailment", &t("mod.help"))?;
    add_ailment_page(
        b,
        page,
        &t("scarlet_rot.title"),
        &t("scarlet_rot.help"),
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
        &t("poison.title"),
        &t("poison.help"),
        [&POISON_DURATION, &POISON_PERCENT, &POISON_FIXED],
        [
            || config::get_double("Poison.Duration", -1.0),
            || config::get_double("Poison.PercentDamage", 0.07),
            || config::get_double("Poison.FixedDamage", 7.0),
        ],
        ["90", "0.07", "7"],
    )?;
    add_ailment_page(
        b,
        page,
        &t("black_flame.title"),
        &t("black_flame.help"),
        [&FLAME_DURATION, &FLAME_PERCENT, &FLAME_FIXED],
        [
            || config::get_double("BlackFlame.Duration", 10.0),
            || config::get_double("BlackFlame.PercentDamage", 0.1),
            || config::get_double("BlackFlame.FixedDamage", 1.0),
        ],
        ["2", "0.1", "1"],
    )?;
    let destined = add_ailment_page(
        b,
        page,
        &t("destined_death.title"),
        &t("destined_death.help"),
        [&DESTINED_DURATION, &DESTINED_PERCENT, &DESTINED_FIXED],
        [
            || config::get_double("DestinedDeath.Duration", 10.0),
            || config::get_double("DestinedDeath.PercentDamage", 0.1),
            || config::get_double("DestinedDeath.FixedDamage", 1.0),
        ],
        ["2", "0.1", "1"],
    )?;
    b.number(
        destined,
        &t("max_hp_loss.label"),
        &t("max_hp_loss.help"),
        &DESTINED_MAX_HP,
        || config::get_double("DestinedDeath.MaxHpDuration", -1.0),
    )?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, log_file_row)
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
        assert!(used.len() >= 10, "found only {} menu keys", used.len());
        assert_eq!(used, TEXT.keys("english"), "menu.rs keys and locales/en.json differ");
        assert_eq!(used, TEXT.keys("vietnamese"), "menu.rs keys and locales/vi.json differ");
    }
}
