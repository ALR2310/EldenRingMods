//! Optional in-game menu through the ERNativeUI host; without it the mod runs from the TOML file.
//! One page per ailment with its duration, percent damage and flat damage; Destined Death also has
//! how long its max HP loss lasts. Numbers are typed (the file holds decimals such as 0.18). A
//! change is saved a second after the player stops, then applied to the game's params like a reload.

use std::sync::OnceLock;

use shared::i18n::Catalog;
use shared::menu::{self, MenuBuilder, NumberSetting, PageHandle, Setting};

use crate::config;

static ROT_DURATION: NumberSetting =
    NumberSetting::new("ScarletRot.Duration", -1.0, 86_400.0, None);
static ROT_PERCENT: NumberSetting =
    NumberSetting::new("ScarletRot.PercentDamage", -100.0, 100.0, None);
static ROT_FIXED: NumberSetting =
    NumberSetting::whole("ScarletRot.FixedDamage", -1000.0, 1000.0, None);
static POISON_DURATION: NumberSetting = NumberSetting::new("Poison.Duration", -1.0, 86_400.0, None);
static POISON_PERCENT: NumberSetting =
    NumberSetting::new("Poison.PercentDamage", -100.0, 100.0, None);
static POISON_FIXED: NumberSetting =
    NumberSetting::whole("Poison.FixedDamage", -1000.0, 1000.0, None);
static FLAME_DURATION: NumberSetting =
    NumberSetting::new("BlackFlame.Duration", -1.0, 86_400.0, None);
static FLAME_PERCENT: NumberSetting =
    NumberSetting::new("BlackFlame.PercentDamage", -100.0, 100.0, None);
static FLAME_FIXED: NumberSetting =
    NumberSetting::whole("BlackFlame.FixedDamage", -1000.0, 1000.0, None);
static DESTINED_DURATION: NumberSetting =
    NumberSetting::new("DestinedDeath.Duration", -1.0, 86_400.0, None);
static DESTINED_PERCENT: NumberSetting =
    NumberSetting::new("DestinedDeath.PercentDamage", -100.0, 100.0, None);
static DESTINED_FIXED: NumberSetting =
    NumberSetting::whole("DestinedDeath.FixedDamage", -1000.0, 1000.0, None);
static DESTINED_MAX_HP: NumberSetting =
    NumberSetting::new("DestinedDeath.MaxHpDuration", -1.0, 86_400.0, None);
static LOG_FILE: Setting = Setting::flag("Logging.LogFile");

static MENU: OnceLock<menu::Menu> = OnceLock::new();

static TEXT: Catalog = Catalog::new(&[
    ("english", include_str!("../locales/en.json")),
    ("vietnamese", include_str!("../locales/vi.json")),
]);

fn t(key: &str) -> String {
    TEXT.tr(key)
}

/// One ailment's page: duration, percent damage and flat damage, with the vanilla values in the help.
fn add_ailment_page(
    b: &mut MenuBuilder,
    parent: PageHandle,
    (name, help): (String, String),
    settings: [&'static NumberSetting; 3],
    reads: [fn() -> f64; 3],
    vanilla: [&str; 3],
) -> Result<PageHandle, String> {
    let page = b.submenu(parent, &name, &help)?;
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
        (t("scarlet_rot.title"), t("scarlet_rot.help")),
        [&ROT_DURATION, &ROT_PERCENT, &ROT_FIXED],
        [
            || config::get().scarlet_rot.duration,
            || config::get().scarlet_rot.percent_damage,
            || f64::from(config::get().scarlet_rot.fixed_damage),
        ],
        ["90", "0.18", "15"],
    )?;
    add_ailment_page(
        b,
        page,
        (t("poison.title"), t("poison.help")),
        [&POISON_DURATION, &POISON_PERCENT, &POISON_FIXED],
        [
            || config::get().poison.duration,
            || config::get().poison.percent_damage,
            || f64::from(config::get().poison.fixed_damage),
        ],
        ["90", "0.07", "7"],
    )?;
    add_ailment_page(
        b,
        page,
        (t("black_flame.title"), t("black_flame.help")),
        [&FLAME_DURATION, &FLAME_PERCENT, &FLAME_FIXED],
        [
            || config::get().black_flame.duration,
            || config::get().black_flame.percent_damage,
            || f64::from(config::get().black_flame.fixed_damage),
        ],
        ["2", "0.1", "1"],
    )?;
    let destined = add_ailment_page(
        b,
        page,
        (t("destined_death.title"), t("destined_death.help")),
        [&DESTINED_DURATION, &DESTINED_PERCENT, &DESTINED_FIXED],
        [
            || config::get().destined_death.duration,
            || config::get().destined_death.percent_damage,
            || f64::from(config::get().destined_death.fixed_damage),
        ],
        ["2", "0.1", "1"],
    )?;
    b.number(destined, &t("max_hp_loss.label"), &t("max_hp_loss.help"), &DESTINED_MAX_HP, || {
        config::get().destined_death.max_hp_duration
    })?;
    b.toggle(page, &t("log.label"), &t("log.help"), &LOG_FILE, || {
        config::get().logging.log_file as u8
    })
}

/// Must start early: the host closes registration shortly after startup.
pub fn start(hmodule: u64) {
    menu::start_saver(config::save, Some(saved));
    menu::spawn_menu(&MENU, hmodule, "infinite-ailment", "Infinite Ailment", build);
}

/// A saved change takes effect like a reload, minus the menu refresh: the rows already show it.
fn saved() {
    shared::logger::set_enabled(config::get().logging.log_file);
    crate::status_effect::request_apply();
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
        TEXT.assert_matches_source(include_str!("menu.rs"), 10);
    }
}
