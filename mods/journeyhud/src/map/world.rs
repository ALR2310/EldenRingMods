// The conversion follows kubilaiswf-minimap (MIT, Copyright (c) 2026 kubilaiswf, see
// THIRD-PARTY-NOTICES.txt), `game.rs`: same idea and constants, written against this
// crate's helpers. See docs/map.md.
//! Where the player is on the world map: a block-local position converted to the
//! map's coordinates (meters on the overworld grid), which space it belongs to
//! and which of the game's two world maps shows it.

use std::sync::OnceLock;

use eldenring::cs::{BlockId, SoloParamRepository, WorldMapLegacyConvParam};
use fromsoftware_shared::FromStatic;

use common::{logger, params};

use super::tiles::MapId;

/// The overworld and the Shadow of the Erdtree world, the areas the world map is drawn for.
const WORLD_AREAS: [u8; 2] = [60, 61];

/// One row of `WorldMapLegacyConvParam`: a point in a dungeon and the same point either
/// on the world map grid or in another dungeon.
struct Conversion {
    src: (u8, u8, u8),
    src_pos: [f32; 2],
    dst: (u8, u8, u8),
    dst_pos: [f32; 2],
}

static CONVERSIONS: OnceLock<Vec<Conversion>> = OnceLock::new();

/// Converts a block-local position to `(position, space, map)`: `position` is in
/// meters on the overworld grid where the world map can place it (otherwise local to
/// `space`), positions are only comparable within the same `space`, and `map` is
/// the world map that shows it, if any.
pub fn locate(block: BlockId, local: [f32; 2]) -> ([f32; 2], i32, Option<MapId>) {
    if block.is_overworld() {
        // Overworld blocks are mAA_XX_ZZ_LL grid tiles of (256 << LL) meters with positions
        // relative to the tile center. Larger tiles are centered on their 2x2 or 4x4 group of
        // small tiles, hence the extra half-tile offsets.
        // Only the units digit is the grid scale; the tens digit is an event variant.
        let tile = (256u32 << (block.index() % 10).min(2)) as f32;
        let center = (tile - 256.0) / 2.0;
        let position = [
            local[0] + f32::from(block.block()) * tile + center,
            local[1] + f32::from(block.region()) * tile + center,
        ];
        // The base map is area 60, the Land of Shadow area 61.
        let map = match block.area() {
            60 => Some(MapId::Overworld),
            61 => Some(MapId::Shadow),
            _ => None,
        };
        return (position, i32::from(block.area()), map);
    }
    // Roundtable Hold has a world map spot but is not anywhere on the map itself.
    let roundtable_hold = (block.area(), block.block()) == (11, 10);
    let conversion = legacy_conversion(block, local).filter(|_| !roundtable_hold);
    let Some((position, dst_area)) = conversion else {
        return (local, i32::from(block), None);
    };
    // Underground areas share the overworld's coordinates but have their own map.
    let (space, map) = match (dst_area, block.area()) {
        (60, 12) => (12, Some(MapId::Underground)),
        (60, _) => (60, Some(MapId::Overworld)),
        (61, _) => (61, Some(MapId::Shadow)),
        (area, _) => (i32::from(area), None),
    };
    (position, space, map)
}

/// Returns the world map aligned position and its area. A dungeon lists several reference
/// points, each valid around itself, so the one nearest the player is used. Rows that lead to
/// another dungeon are only followed when no row leads to the world map.
fn legacy_conversion(block: BlockId, local: [f32; 2]) -> Option<([f32; 2], u8)> {
    let conversions = conversions()?;
    let (mut src, mut at) = ((block.area(), block.block(), block.region()), local);
    for _ in 0..4 {
        let nearest = |to_world: bool| {
            conversions
                .iter()
                .filter(|c| c.src == src && WORLD_AREAS.contains(&c.dst.0) == to_world)
                .min_by(|a, b| {
                    distance_squared(a.src_pos, at).total_cmp(&distance_squared(b.src_pos, at))
                })
        };
        if let Some(c) = nearest(true) {
            let position = [
                at[0] - c.src_pos[0] + f32::from(c.dst.1) * 256.0 + c.dst_pos[0],
                at[1] - c.src_pos[1] + f32::from(c.dst.2) * 256.0 + c.dst_pos[1],
            ];
            return Some((position, c.dst.0));
        }
        let c = nearest(false)?;
        at = [
            at[0] - c.src_pos[0] + c.dst_pos[0],
            at[1] - c.src_pos[1] + c.dst_pos[1],
        ];
        src = c.dst;
    }
    None
}

fn distance_squared(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

/// Whether the dungeon conversions are loaded, so `locate` is complete.
pub fn conversions_ready() -> bool {
    conversions().is_some()
}

/// The conversion rows, read once the params are loaded (they load after the player
/// can already exist, so this is retried each frame until it works).
fn conversions() -> Option<&'static [Conversion]> {
    if let Some(conversions) = CONVERSIONS.get() {
        return Some(conversions);
    }
    let repo = unsafe { SoloParamRepository::instance() }.ok()?;
    // fromsoftware-rs hardcodes the slot of each param; make sure it holds this one.
    if let Err(reason) = params::check::<WorldMapLegacyConvParam>(repo) {
        logger::warn(&format!(
            "Map: legacy conversion params unavailable ({reason})"
        ));
        return None;
    }
    let conversions: Vec<Conversion> = (0..)
        .map_while(|i| repo.get_row_by_index::<WorldMapLegacyConvParam>(i))
        .map(|row| Conversion {
            src: (row.src_area_no(), row.src_grid_x_no(), row.src_grid_z_no()),
            src_pos: [row.src_pos_x(), row.src_pos_z()],
            dst: (row.dst_area_no(), row.dst_grid_x_no(), row.dst_grid_z_no()),
            dst_pos: [row.dst_pos_x(), row.dst_pos_z()],
        })
        .collect();
    if conversions.is_empty() {
        return None;
    }
    logger::log(&format!(
        "Map: {} legacy conversion row(s)",
        conversions.len()
    ));
    Some(CONVERSIONS.get_or_init(|| conversions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overworld_positions_are_tile_origin_plus_local() {
        // m60_42_36_00: a 256 m tile, positions relative to its center.
        let block = BlockId::from_parts(60, 42, 36, 0);
        let (position, space, map) = locate(block, [10.0, -20.0]);
        assert_eq!(position, [10.0 + 42.0 * 256.0, -20.0 + 36.0 * 256.0]);
        assert_eq!((space, map), (60, Some(MapId::Overworld)));
    }

    #[test]
    fn larger_overworld_tiles_are_centered_on_their_group_of_small_tiles() {
        // m60_10_10_01 spans 2x2 small tiles: half a small tile further along each axis.
        let block = BlockId::from_parts(60, 10, 10, 1);
        let (position, ..) = locate(block, [0.0, 0.0]);
        assert_eq!(position, [10.0 * 512.0 + 128.0, 10.0 * 512.0 + 128.0]);
    }

    #[test]
    fn event_variants_keep_the_units_digit_grid_scale() {
        for (area, map) in [(60, MapId::Overworld), (61, MapId::Shadow)] {
            for lod in 0..=2 {
                let tile = (256u32 << lod) as f32;
                let expected = [
                    12.0 + 18.0 * tile + (tile - 256.0) / 2.0,
                    -9.0 + 16.0 * tile + (tile - 256.0) / 2.0,
                ];
                for variant in [0, 10] {
                    assert_eq!(
                        locate(
                            BlockId::from_parts(area, 18, 16, variant + lod),
                            [12.0, -9.0]
                        ),
                        (expected, i32::from(area), Some(map)),
                    );
                }
            }
        }
    }
}
