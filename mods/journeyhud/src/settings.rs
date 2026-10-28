//! The settings the overlay and the target reader use, held in atomics so a
//! change from the in-game menu applies at once (the file is written a second
//! later by the menu's saver). Defaults and meaning: `JourneyHUD.toml`.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use common::input::parse_virtual_key;

use crate::config::Config;

const VK_OEM_PLUS: i32 = 0xBB;
const VK_OEM_MINUS: i32 = 0xBD;

/// How many keys each zoom direction takes (0 = none).
const ZOOM_KEYS: usize = 4;

/// The minimap ranges (meters from the center to the rim) the zoom keys step through.
const ZOOM_LEVELS: [i32; 3] = [50, 100, 200];

static ENABLED: AtomicBool = AtomicBool::new(true);
static SHOW_NUMBERS: AtomicBool = AtomicBool::new(true);
static BUILDUP: AtomicBool = AtomicBool::new(true);
static EFFECTS_TARGET: AtomicBool = AtomicBool::new(true);
static EFFECTS_PLAYER: AtomicBool = AtomicBool::new(true);
static WIDTH: AtomicI32 = AtomicI32::new(220);
static OFFSET_X: AtomicI32 = AtomicI32::new(48);
static OFFSET_Y: AtomicI32 = AtomicI32::new(47);
/// `f32` bits.
static SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);
/// `f32` bits.
static MINIMAP_SCALE: AtomicU32 = AtomicU32::new(0x3F80_0000);
static MINIMAP_ENABLED: AtomicBool = AtomicBool::new(true);
static MINIMAP_MAP: AtomicBool = AtomicBool::new(true);
static MINIMAP_GRACES: AtomicBool = AtomicBool::new(true);
static MINIMAP_POINTS: AtomicBool = AtomicBool::new(true);
static MINIMAP_BEACONS: AtomicBool = AtomicBool::new(true);
static MINIMAP_ROTATE: AtomicBool = AtomicBool::new(true);
static MINIMAP_COMPASS: AtomicBool = AtomicBool::new(true);
static MINIMAP_TRAIL: AtomicBool = AtomicBool::new(false);
static MINIMAP_CLOCK: AtomicBool = AtomicBool::new(true);
static MINIMAP_DEATHS: AtomicBool = AtomicBool::new(true);
static MINIMAP_FULL_MAP: AtomicBool = AtomicBool::new(true);
static MINIMAP_GOBLINS: AtomicBool = AtomicBool::new(true);
static MINIMAP_COOP: AtomicBool = AtomicBool::new(true);
static MINIMAP_SPIRITS: AtomicBool = AtomicBool::new(true);
static MINIMAP_HIDE_IN_CAVES: AtomicBool = AtomicBool::new(false);
static MINIMAP_HIDE_IN_BOSS_FIGHTS: AtomicBool = AtomicBool::new(false);
static MINIMAP_RANGE: AtomicI32 = AtomicI32::new(100);
/// Virtual-key codes of the zoom keys.
static ZOOM_IN_KEYS: [AtomicI32; ZOOM_KEYS] = [AtomicI32::new(VK_OEM_PLUS), AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];
static ZOOM_OUT_KEYS: [AtomicI32; ZOOM_KEYS] = [AtomicI32::new(VK_OEM_MINUS), AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];
static MINIMAP_SIZE: AtomicI32 = AtomicI32::new(110);

/// Takes every setting from the loaded config.
pub fn load(config: &Config) {
    let (target, minimap) = (&config.target, &config.minimap);
    set_enabled(target.enabled as u8);
    set_show_numbers(target.show_numbers as u8);
    set_buildup(target.buildup as u8);
    set_effects_target(config.effects.target as u8);
    set_effects_player(config.effects.player as u8);
    set_width(f64::from(target.width));
    set_offset_x(f64::from(config.layout.offset_x));
    set_offset_y(f64::from(config.layout.offset_y));
    set_scale(target.scale);
    set_minimap_enabled(minimap.enabled as u8);
    set_minimap_map(minimap.layers.map as u8);
    set_minimap_graces(minimap.layers.graces as u8);
    set_minimap_points(minimap.layers.points as u8);
    set_minimap_beacons(minimap.layers.beacons as u8);
    set_minimap_rotate(minimap.rotate as u8);
    set_minimap_compass(minimap.compass as u8);
    set_minimap_trail(minimap.trail as u8);
    set_minimap_clock(minimap.clock as u8);
    set_minimap_deaths(minimap.deaths as u8);
    set_minimap_full_map(minimap.full_map as u8);
    set_minimap_goblins(minimap.layers.map_for_goblins as u8);
    set_minimap_coop(minimap.layers.coop_players as u8);
    set_minimap_spirits(minimap.layers.spirits as u8);
    set_minimap_hide_in_caves(minimap.hide.in_caves as u8);
    set_minimap_hide_in_boss_fights(minimap.hide.in_boss_fights as u8);
    set_minimap_range(f64::from(minimap.range));
    store_keys(&ZOOM_IN_KEYS, &minimap.zoom_in_key);
    store_keys(&ZOOM_OUT_KEYS, &minimap.zoom_out_key);
    set_minimap_size(f64::from(minimap.size));
    set_minimap_scale(minimap.scale);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Whether the effects of the target (poison, rot, frost, sleep... with the time left) are shown.
pub fn effects_target() -> bool {
    EFFECTS_TARGET.load(Ordering::Relaxed)
}

/// Whether the effects of the player are shown.
pub fn effects_player() -> bool {
    EFFECTS_PLAYER.load(Ordering::Relaxed)
}

/// Whether the status buildup bars of the target are shown.
pub fn buildup() -> bool {
    BUILDUP.load(Ordering::Relaxed)
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

/// Size multiplier of the minimap.
pub fn minimap_scale() -> f32 {
    f32::from_bits(MINIMAP_SCALE.load(Ordering::Relaxed))
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

/// Whether the minimap turns with the camera (else north is up).
pub fn minimap_rotate() -> bool {
    MINIMAP_ROTATE.load(Ordering::Relaxed)
}

/// Whether the N, E, S and W letters are drawn on the rim.
pub fn minimap_compass() -> bool {
    MINIMAP_COMPASS.load(Ordering::Relaxed)
}

/// Whether the in-game time is shown under the minimap.
pub fn minimap_clock() -> bool {
    MINIMAP_CLOCK.load(Ordering::Relaxed)
}

/// Whether the death count is shown under the minimap.
pub fn minimap_deaths() -> bool {
    MINIMAP_DEATHS.load(Ordering::Relaxed)
}

/// Whether the path the player has walked is drawn.
pub fn minimap_trail() -> bool {
    MINIMAP_TRAIL.load(Ordering::Relaxed)
}

/// Whether the whole world map shows, or only what the player's map fragments reveal.
pub fn minimap_full_map() -> bool {
    MINIMAP_FULL_MAP.load(Ordering::Relaxed)
}

/// Whether the markers of Map for Goblins are drawn.
pub fn minimap_goblins() -> bool {
    MINIMAP_GOBLINS.load(Ordering::Relaxed)
}

/// Whether the co-op partners are marked.
pub fn minimap_coop() -> bool {
    MINIMAP_COOP.load(Ordering::Relaxed)
}

/// Whether the player's summoned spirits are marked.
pub fn minimap_spirits() -> bool {
    MINIMAP_SPIRITS.load(Ordering::Relaxed)
}

/// Whether the minimap hides in caves, catacombs and other dungeons.
pub fn minimap_hide_in_caves() -> bool {
    MINIMAP_HIDE_IN_CAVES.load(Ordering::Relaxed)
}

/// Whether the minimap hides while a boss health bar is on screen.
pub fn minimap_hide_in_boss_fights() -> bool {
    MINIMAP_HIDE_IN_BOSS_FIGHTS.load(Ordering::Relaxed)
}

/// Meters from the center of the minimap to its rim.
pub fn minimap_range() -> f32 {
    MINIMAP_RANGE.load(Ordering::Relaxed) as f32
}

/// The virtual-key codes that zoom in (0 = unused).
pub fn zoom_in_keys() -> [i32; ZOOM_KEYS] {
    ZOOM_IN_KEYS.each_ref().map(|key| key.load(Ordering::Relaxed))
}

pub fn zoom_out_keys() -> [i32; ZOOM_KEYS] {
    ZOOM_OUT_KEYS.each_ref().map(|key| key.load(Ordering::Relaxed))
}

/// Puts the first `ZOOM_KEYS` of `names` in `slots`; a name that is no key leaves its slot unused.
fn store_keys(slots: &[AtomicI32; ZOOM_KEYS], names: &[String]) {
    for (i, slot) in slots.iter().enumerate() {
        let code = names.get(i).map_or(0, |name| parse_virtual_key(name, 0));
        slot.store(code, Ordering::Relaxed);
    }
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

pub fn set_effects_target(value: u8) {
    EFFECTS_TARGET.store(value != 0, Ordering::Relaxed);
}

pub fn set_effects_player(value: u8) {
    EFFECTS_PLAYER.store(value != 0, Ordering::Relaxed);
}

pub fn set_buildup(value: u8) {
    BUILDUP.store(value != 0, Ordering::Relaxed);
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

pub fn set_minimap_scale(value: f64) {
    MINIMAP_SCALE.store((value as f32).clamp(0.5, 3.0).to_bits(), Ordering::Relaxed);
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

pub fn set_minimap_rotate(value: u8) {
    MINIMAP_ROTATE.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_compass(value: u8) {
    MINIMAP_COMPASS.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_clock(value: u8) {
    MINIMAP_CLOCK.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_deaths(value: u8) {
    MINIMAP_DEATHS.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_trail(value: u8) {
    MINIMAP_TRAIL.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_full_map(value: u8) {
    MINIMAP_FULL_MAP.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_goblins(value: u8) {
    MINIMAP_GOBLINS.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_coop(value: u8) {
    MINIMAP_COOP.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_spirits(value: u8) {
    MINIMAP_SPIRITS.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_hide_in_caves(value: u8) {
    MINIMAP_HIDE_IN_CAVES.store(value != 0, Ordering::Relaxed);
}

pub fn set_minimap_hide_in_boss_fights(value: u8) {
    MINIMAP_HIDE_IN_BOSS_FIGHTS.store(value != 0, Ordering::Relaxed);
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
