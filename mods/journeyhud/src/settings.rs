//! The settings the overlay and the target reader use, held in atomics so a
//! change from the in-game menu applies at once (the ini is written a second
//! later by the menu's saver). Defaults and meaning: `JourneyHud.ini`.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use common::config;

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHOW_NUMBERS: AtomicBool = AtomicBool::new(false);
static WIDTH: AtomicI32 = AtomicI32::new(220);
static OFFSET_X: AtomicI32 = AtomicI32::new(48);
static OFFSET_Y: AtomicI32 = AtomicI32::new(47);
/// `f32` bits.
static SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);
static MINIMAP_ENABLED: AtomicBool = AtomicBool::new(true);
static MINIMAP_RANGE: AtomicI32 = AtomicI32::new(80);
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
    set_minimap_range(config::get_int("MinimapRange", 80) as f64);
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

/// Meters from the center of the minimap to its rim.
pub fn minimap_range() -> f32 {
    MINIMAP_RANGE.load(Ordering::Relaxed) as f32
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

pub fn set_minimap_range(value: f64) {
    MINIMAP_RANGE.store(value.round().clamp(20.0, 400.0) as i32, Ordering::Relaxed);
}

pub fn set_minimap_size(value: f64) {
    MINIMAP_SIZE.store(value.round().clamp(40.0, 200.0) as i32, Ordering::Relaxed);
}
