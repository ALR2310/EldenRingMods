// The pointer chain and the slot layout follow kubilaiswf-minimap (MIT, Copyright (c) 2026
// kubilaiswf, see THIRD-PARTY-NOTICES.txt), `game.rs`, which took them from soarqin's EROverlay
// notes. Written against this crate's helpers, with safe reads and a sanity check on the data.
//! The beacons the player has placed on the world map. They have no type in `fromsoftware-rs`;
//! the game keeps them in a `CSMenuMarkersSaveData` reached from the menu manager:
//!
//! ```text
//! CSMenuMan -> +0x80 -> +0x250 (the world map view) -> +0x338 (= the marker list at +0x300, its
//!   save data pointer at +0x38) -> +0x08 slots (16 bytes each), +0x10 slot count
//! slot: id i32 (negative = empty), x f32, y f32 (the world map screen's own coordinates), map u8
//!   (0 overworld, 1 underground), icon u8
//! ```
//!
//! From the save data on, the layout is checked against `eldenring.exe` 2.7.2.0 (the add function
//! `0x81A200` writes these fields, its caller keeps the list at view +0x300; `docs/map.md`). The
//! chain before it is from the source above and not checked on the other builds, so every pointer is
//! read with a function that fails instead of faulting, and the result is dropped unless it all
//! looks like beacons.

use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::FromStatic;

use super::tiles::{MapId, TILE};

const MENU_TO_OWNER: usize = 0x80;
const OWNER_TO_VIEW: usize = 0x250;
const VIEW_TO_SAVE: usize = 0x300 + 0x38;
const SAVE_SLOTS: usize = 0x08;
const SAVE_COUNT: usize = 0x10;
/// The save has at most 10 slots (the game allows 5 beacons at a time); more is not this list.
const MAX_SLOTS: u64 = 10;

/// A placed beacon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beacon {
    /// Where it is, in meters on the overworld grid like the player's position.
    pub position: [f32; 2],
    /// Its slot number, which is the number the game shows next to it.
    pub number: u8,
}

/// Reads a `T` at `address`, or `None` if that memory cannot be read.
fn read<T: Copy>(address: usize) -> Option<T> {
    common::memscan::read_safe(address)
}

/// A non-null pointer stored at `address`.
fn pointer(address: usize) -> Option<usize> {
    read::<usize>(address).filter(|&p| p != 0)
}

/// The beacons placed on `map`.
pub fn placed(map: MapId) -> Vec<Beacon> {
    let wanted = match map {
        MapId::Overworld => 0,
        MapId::Underground => 1,
        MapId::Shadow => 2,
    };
    let Ok(menu) = CSMenuManImp::instance_ptr() else { return Vec::new() };
    let save = pointer(menu as usize + MENU_TO_OWNER)
        .and_then(|owner| pointer(owner + OWNER_TO_VIEW))
        .and_then(|view| pointer(view + VIEW_TO_SAVE));
    let Some(save) = save else { return Vec::new() };
    let (Some(slots), Some(count)) = (pointer(save + SAVE_SLOTS), read::<u64>(save + SAVE_COUNT)) else {
        return Vec::new();
    };
    if count == 0 || count > MAX_SLOTS {
        return Vec::new();
    }
    let mut raw = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        match read::<[u8; 16]>(slots + i * 16) {
            Some(bytes) => raw.push(bytes),
            None => return Vec::new(),
        }
    }
    parse(&raw, wanted)
}

/// The beacons of `map` (0 or 1) in the raw slots. Anything that does not look like a slot
/// (an id past what the game hands out, a position off the map, an unknown map) means this is not
/// the data it was taken for, and no beacon is returned at all.
fn parse(slots: &[[u8; 16]], wanted: u8) -> Vec<Beacon> {
    let mut beacons = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        let id = i32::from_le_bytes(slot[0..4].try_into().unwrap());
        if id < 0 {
            continue;
        }
        let (x, y) = (f32::from_le_bytes(slot[4..8].try_into().unwrap()), f32::from_le_bytes(slot[8..12].try_into().unwrap()));
        let map = slot[12];
        // The world map screen is 41 cells of 256 px; ids count up from 1 per placement.
        let limit = 41.0 * TILE as f32;
        if id > 1_000_000 || map > 1 || !(0.0..=limit).contains(&x) || !(0.0..=limit).contains(&y) {
            return Vec::new();
        }
        if map == wanted {
            beacons.push(Beacon { position: from_game_map([x, y]), number: i as u8 + 1 });
        }
    }
    beacons
}

/// Converts a position of the world map screen (pixels, Y growing down over the 41-cell map) to
/// meters on the overworld grid. Map cell (i, j) shows overworld tile `m60_{i+28}_{j+24}`.
fn from_game_map([x, y]: [f32; 2]) -> [f32; 2] {
    const HALF: f32 = TILE as f32 / 2.0;
    let y_up = 41.0 * TILE as f32 - y;
    [x - HALF + 28.0 * TILE as f32, y_up - HALF + 24.0 * TILE as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: i32, x: f32, y: f32, map: u8) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&id.to_le_bytes());
        bytes[4..8].copy_from_slice(&x.to_le_bytes());
        bytes[8..12].copy_from_slice(&y.to_le_bytes());
        bytes[12] = map;
        bytes
    }

    #[test]
    fn game_map_positions_are_converted_to_overworld_meters() {
        // The map's center cell (20, 20) shows m60_48_44: its center, in meters.
        let (x, y) = (20.5 * 256.0, 20.5 * 256.0);
        assert_eq!(from_game_map([x, y]), [48.0 * 256.0, 44.0 * 256.0]);
        // Going down the screen (Y up) goes south: smaller Z.
        assert!(from_game_map([x, y + 10.0])[1] < from_game_map([x, y])[1]);
    }

    #[test]
    fn beacons_of_the_wanted_map_are_taken_with_their_slot_number() {
        let slots = [slot(-1, 0.0, 0.0, 0), slot(3, 100.0, 200.0, 0), slot(4, 300.0, 400.0, 1)];
        let overworld = parse(&slots, 0);
        assert_eq!(overworld.len(), 1);
        assert_eq!(overworld[0].number, 2);
        let underground = parse(&slots, 1);
        assert_eq!((underground.len(), underground[0].number), (1, 3));
    }

    #[test]
    fn data_that_does_not_look_like_beacons_gives_none() {
        // A position off the map, an unknown map, an id far too large, a non-finite position.
        for bad in [slot(1, 99999.0, 5.0, 0), slot(1, 5.0, 5.0, 7), slot(5_000_000, 5.0, 5.0, 0), slot(1, f32::NAN, 5.0, 0)] {
            assert!(parse(&[slot(2, 100.0, 100.0, 0), bad], 0).is_empty());
        }
    }

    #[test]
    fn reading_unreadable_memory_fails_instead_of_faulting() {
        assert_eq!(read::<u64>(8), None);
        let value = 0x1122_3344_5566_7788u64;
        assert_eq!(read::<u64>(&value as *const u64 as usize), Some(value));
    }
}
