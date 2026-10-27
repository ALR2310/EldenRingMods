//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Journey HUD                -> opens a page with:
//!                                 Minimap         on/off
//!                                 Map             on/off
//!                                 Graces          on/off
//!                                 Points          on/off
//!                                 Beacons         on/off
//!                                 Minimap range   slider
//!                                 Minimap size    slider
//!                                 Target bar      on/off
//!                                 Health numbers  on/off
//!                                 Bar length      slider
//!                                 Size            slider
//!                                 Right margin    slider
//!                                 Top margin      slider
//! ```
//!
//! Changes apply at once and are written back to the ini shortly after the
//! player stops, so they survive a restart.

use std::sync::OnceLock;

use common::config;
use common::erui::{self, NumberSetting, Setting};
use common::i18n::Catalog;

use crate::settings;

static ENABLED: Setting = Setting { key: "Enabled", format: erui::format_bool, apply: Some(settings::set_enabled) };
static SHOW_NUMBERS: Setting =
    Setting { key: "ShowNumbers", format: erui::format_bool, apply: Some(settings::set_show_numbers) };
static WIDTH: NumberSetting =
    NumberSetting::whole("Width", 100.0, 1600.0, Some(settings::set_width)).slider(100.0, 800.0, 10.0);
static SCALE: NumberSetting = NumberSetting::new("Scale", 0.5, 3.0, Some(settings::set_scale)).stepped(0.1);
static OFFSET_X: NumberSetting =
    NumberSetting::whole("OffsetX", 0.0, 400.0, Some(settings::set_offset_x)).slider(0.0, 300.0, 5.0);
static OFFSET_Y: NumberSetting =
    NumberSetting::whole("OffsetY", 0.0, 400.0, Some(settings::set_offset_y)).slider(0.0, 300.0, 5.0);

static MINIMAP: Setting =
    Setting { key: "MinimapEnabled", format: erui::format_bool, apply: Some(settings::set_minimap_enabled) };
static MINIMAP_MAP: Setting =
    Setting { key: "MinimapMap", format: erui::format_bool, apply: Some(settings::set_minimap_map) };
static MINIMAP_GRACES: Setting =
    Setting { key: "MinimapGraces", format: erui::format_bool, apply: Some(settings::set_minimap_graces) };
static MINIMAP_POINTS: Setting =
    Setting { key: "MinimapPoints", format: erui::format_bool, apply: Some(settings::set_minimap_points) };
static MINIMAP_BEACONS: Setting =
    Setting { key: "MinimapBeacons", format: erui::format_bool, apply: Some(settings::set_minimap_beacons) };
static MINIMAP_RANGE: NumberSetting =
    NumberSetting::whole("MinimapRange", 20.0, 400.0, Some(settings::set_minimap_range)).slider(40.0, 200.0, 10.0);
static MINIMAP_SIZE: NumberSetting =
    NumberSetting::whole("MinimapSize", 40.0, 200.0, Some(settings::set_minimap_size)).slider(50.0, 160.0, 5.0);

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
    let page = b.submenu(b.root(), "Journey HUD", &t("mod.help"))?;
    b.toggle(page, &t("enabled.label"), &t("enabled.help"), &ENABLED, || config::get_bool("Enabled", true) as u8)?;
    b.toggle(page, &t("numbers.label"), &t("numbers.help"), &SHOW_NUMBERS, || {
        config::get_bool("ShowNumbers", false) as u8
    })?;
    b.number(page, &t("width.label"), &t("width.help"), &WIDTH, || config::get_int("Width", 220) as f64)?;
    b.number(page, &t("scale.label"), &t("scale.help"), &SCALE, || config::get_double("Scale", 1.0))?;
    b.number(page, &t("offset_x.label"), &t("offset_x.help"), &OFFSET_X, || config::get_int("OffsetX", 48) as f64)?;
    b.number(page, &t("offset_y.label"), &t("offset_y.help"), &OFFSET_Y, || config::get_int("OffsetY", 47) as f64)?;
    b.toggle(page, &t("minimap.label"), &t("minimap.help"), &MINIMAP, || config::get_bool("MinimapEnabled", true) as u8)?;
    b.toggle(page, &t("minimap_map.label"), &t("minimap_map.help"), &MINIMAP_MAP, || config::get_bool("MinimapMap", true) as u8)?;
    b.toggle(page, &t("minimap_graces.label"), &t("minimap_graces.help"), &MINIMAP_GRACES, || {
        config::get_bool("MinimapGraces", true) as u8
    })?;
    b.toggle(page, &t("minimap_points.label"), &t("minimap_points.help"), &MINIMAP_POINTS, || {
        config::get_bool("MinimapPoints", true) as u8
    })?;
    b.toggle(page, &t("minimap_beacons.label"), &t("minimap_beacons.help"), &MINIMAP_BEACONS, || {
        config::get_bool("MinimapBeacons", true) as u8
    })?;
    b.number(page, &t("minimap_range.label"), &t("minimap_range.help"), &MINIMAP_RANGE, || config::get_int("MinimapRange", 100) as f64)?;
    b.number(page, &t("minimap_size.label"), &t("minimap_size.help"), &MINIMAP_SIZE, || config::get_int("MinimapSize", 110) as f64)?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "journey-hud", "Journey HUD", build);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the menu passes to `t` is in every language file, and the files
    /// have the same keys (nothing missing, nothing left over).
    #[test]
    fn language_files_match_the_menu_keys() {
        TEXT.assert_matches_source(include_str!("menu.rs"), 27);
    }

    /// Every row meant to be a native slider has a valid range and step (else
    /// it silently stays a typed row).
    #[test]
    fn slider_rows_have_a_valid_range() {
        for setting in [&WIDTH, &SCALE, &OFFSET_X, &OFFSET_Y, &MINIMAP_RANGE, &MINIMAP_SIZE] {
            assert!(setting.has_slider(), "{} would not be a slider", setting.key);
        }
    }
}
