//! The settings the overlay and the target reader use, held in atomics so a
//! change from the in-game menu applies at once (the ini is written a second
//! later by the menu's saver). Defaults and meaning: `JourneyHud.ini`.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use common::config;
use common::input::parse_virtual_key;

const VK_OEM_PLUS: i32 = 0xBB;
const VK_OEM_MINUS: i32 = 0xBD;

/// The minimap ranges (meters from the center to the rim) the zoom keys step through.
const ZOOM_LEVELS: [i32; 3] = [50, 100, 200];

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHOW_NUMBERS: AtomicBool = AtomicBool::new(false);
static WIDTH: AtomicI32 = AtomicI32::new(220);
static OFFSET_X: AtomicI32 = AtomicI32::new(48);
static OFFSET_Y: AtomicI32 = AtomicI32::new(47);
/// `f32` bits.
static SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);
static MINIMAP_ENABLED: AtomicBool = AtomicBool::new(true);
static MINIMAP_MAP: AtomicBool = AtomicBool::new(true);
static MINIMAP_GRACES: AtomicBool = AtomicBool::new(true);
static MINIMAP_POINTS: AtomicBool = AtomicBool::new(true);
static MINIMAP_BEACONS: AtomicBool = AtomicBool::new(true);
static MINIMAP_RANGE: AtomicI32 = AtomicI32::new(100);
/// Virtual-key codes of the zoom keys.
static ZOOM_IN_KEY: AtomicI32 = AtomicI32::new(VK_OEM_PLUS);
static ZOOM_OUT_KEY: AtomicI32 = AtomicI32::new(VK_OEM_MINUS);
static MINIMAP_SIZE: AtomicI32 = AtomicI32::new(110);

/// Reads every setting from the loaded config.
pub fn load() {
    set_enabled(config::get_bool("Enabled", true) as u8);
    set_show_numbers(config::get_bool("ShowNumbers", false) as u8);
    set_width(config::get_int("Width", 220) as f64);
    set_offset_x(config::get_int("OffsetX", 48) as f64);
    set_offset_y(config::get_int("OffsetY", 47) as f64);
    set_scale(config::get_double("Scale", 1.0));
    set_minimap_enabled(config::get_bool("MinimapEnabled", true) as u8);
    set_minimap_map(config::get_bool("MinimapMap", true) as u8);
    set_minimap_graces(config::get_bool("MinimapGraces", true) as u8);
    set_minimap_points(config::get_bool("MinimapPoints", true) as u8);
    set_minimap_beacons(config::get_bool("MinimapBeacons", true) as u8);
    set_minimap_range(config::get_int("MinimapRange", 100) as f64);
    ZOOM_IN_KEY.store(parse_virtual_key(&config::get_string("ZoomInKey", "0xBB"), VK_OEM_PLUS), Ordering::Relaxed);
    ZOOM_OUT_KEY.store(parse_virtual_key(&config::get_string("ZoomOutKey", "0xBD"), VK_OEM_MINUS), Ordering::Relaxed);
    set_minimap_size(config::get_int("MinimapSize", 110) as f64);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn show_numbers() -> bool {
    SHOW_NUMBERS.load(Ordering::Relaxed)
}

/// Bar length in pixels at 1080p.
pub fn width() -> f32 {
    WIDTH.load(Ordering::Relaxed) as f32
}

/// Offset of the right-hand column (minimap, then the target bar under it) from
/// the top-right corner of the game's picture, in pixels at 1080p.
pub fn offset() -> [f32; 2] {
    [OFFSET_X.load(Ordering::Relaxed) as f32, OFFSET_Y.load(Ordering::Relaxed) as f32]
}

pub fn scale() -> f32 {
    f32::from_bits(SCALE.load(Ordering::Relaxed))
}

pub fn minimap_enabled() -> bool {
    MINIMAP_ENABLED.load(Ordering::Relaxed)
}

/// Whether the world map is drawn under the minimap marks.
pub fn minimap_map() -> bool {
    MINIMAP_MAP.load(Ordering::Relaxed)
}

/// Whether the found sites of grace are marked.
pub fn minimap_graces() -> bool {
    MINIMAP_GRACES.load(Ordering::Relaxed)
}

/// Whether the world map's points of interest (ruins, churches, caves, bosses) are marked.
pub fn minimap_points() -> bool {
    MINIMAP_POINTS.load(Ordering::Relaxed)
}

/// Whether the beacons the player has placed on the world map are marked.
pub fn minimap_beacons() -> bool {
    MINIMAP_BEACONS.load(Ordering::Relaxed)
}

/// Meters from the center of the minimap to its rim.
pub fn minimap_range() -> f32 {
    MINIMAP_RANGE.load(Ordering::Relaxed) as f32
}

pub fn zoom_in_key() -> i32 {
    ZOOM_IN_KEY.load(Ordering::Relaxed)
}

pub fn zoom_out_key() -> i32 {
    ZOOM_OUT_KEY.load(Ordering::Relaxed)
}

/// Steps the range to the next level: smaller when zooming in, larger when zooming out, and
/// from the last level round to the first (a range set by hand moves to the level next to it).
pub fn zoom(zoom_in: bool) {
    MINIMAP_RANGE.store(next_range(MINIMAP_RANGE.load(Ordering::Relaxed), zoom_in), Ordering::Relaxed);
}

fn next_range(current: i32, zoom_in: bool) -> i32 {
    let [first, .., last] = ZOOM_LEVELS;
    if zoom_in {
        ZOOM_LEVELS.into_iter().rev().find(|&level| level < current).unwrap_or(last)
    } else {
        ZOOM_LEVELS.into_iter().find(|&level| level > current).unwrap_or(first)
    }
}

/// Radius of the minimap in pixels at 1080p.
pub fn minimap_size() -> f32 {
    MINIMAP_SIZE.load(Ordering::Relaxed) as f32
}

pub fn set_enabled(value: u8) {
    ENABLED.store(value != 0, Ordering::Relaxed);
}

pub fn set_show_numbers(value: u8) {
    SHOW_NUMBERS.store(value != 0, Ordering::Relaxed);
}

pub fn set_width(value: f64) {
    WIDTH.store(value.round().clamp(100.0, 1600.0) as i32, Ordering::Relaxed);
}

pub fn set_offset_x(value: f64) {
    OFFSET_X.store(value.round().clamp(0.0, 400.0) as i32, Ordering::Relaxed);
}

pub fn set_offset_y(value: f64) {
    OFFSET_Y.store(value.round().clamp(0.0, 400.0) as i32, Ordering::Relaxed);
}

pub fn set_scale(value: f64) {
    SCALE.store((value as f32).clamp(0.5, 3.0).to_bits(), Ordering::Relaxed);
}

pub fn set_minimap_enabled(value: u8) {
    MINIMAP_ENABLED.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_map(value: u8) {
    MINIMAP_MAP.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_graces(value: u8) {
    MINIMAP_GRACES.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_points(value: u8) {
    MINIMAP_POINTS.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_beacons(value: u8) {
    MINIMAP_BEACONS.store(value != 0, Ordering::Relaxed);
}

/// At most 200: the map window holds 768 m, which has to cover the range with room to spare.
pub fn set_minimap_range(value: f64) {
    MINIMAP_RANGE.store(value.round().clamp(20.0, 200.0) as i32, Ordering::Relaxed);
}

pub fn set_minimap_size(value: f64) {
    MINIMAP_SIZE.store(value.round().clamp(40.0, 200.0) as i32, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::next_range;

    #[test]
    fn zooming_in_steps_to_smaller_ranges_and_wraps() {
        assert_eq!(next_range(200, true), 100);
        assert_eq!(next_range(100, true), 50);
        assert_eq!(next_range(50, true), 200);
    }

    #[test]
    fn zooming_out_steps_to_larger_ranges_and_wraps() {
        assert_eq!(next_range(50, false), 100);
        assert_eq!(next_range(100, false), 200);
        assert_eq!(next_range(200, false), 50);
    }

    #[test]
    fn a_range_set_by_hand_moves_to_the_next_level() {
        assert_eq!(next_range(80, true), 50);
        assert_eq!(next_range(80, false), 100);
        assert_eq!(next_range(30, true), 200);
        assert_eq!(next_range(150, false), 200);
    }
}
