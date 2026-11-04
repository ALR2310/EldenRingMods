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
    /// The game's `isBasePoint`: the dungeon's main reference, as opposed to an extra one.
    base_point: bool,
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
        // The units digit is the grid scale (0 to 2) and the tens digit an event variant; a units
        // digit beyond 2 is another variant of a small tile (m60_44_37_16 after Radahn's fall,
        // read as a 1024 m tile it put the player far off the map).
        let scale = match block.index() % 10 {
            scale @ 0..=2 => u32::from(scale),
            _ => 0,
        };
        let tile = (256u32 << scale) as f32;
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
    let roundtable_hold = is_roundtable_hold(block);
    let conversion = conversions()
        .and_then(|conversions| legacy_conversion(conversions, block, local))
        .filter(|_| !roundtable_hold);
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

/// Whether `area` is a legacy dungeon, one of the large hand-made maps (Stormveil, Leyndell, Farum
/// Azula, Shadow Keep...) as opposed to catacombs, caves, tunnels and towers (areas 30 to 34, 40
/// to 43). The game has no flag for it; this is the list of their areas.
/// Roundtable Hold (`m11_10`).
pub fn is_roundtable_hold(block: BlockId) -> bool {
    (block.area(), block.block()) == (11, 10)
}

pub fn is_legacy_dungeon(area: u8) -> bool {
    matches!(area, 10 | 11 | 13 | 14 | 15 | 16 | 18 | 19 | 20 | 21 | 22 | 25 | 28 | 35)
}

/// Whether the player is in a cave, catacomb, tunnel or the like: any block outside the
/// overworld, except the legacy dungeons, the Divine Towers (area 34) and the underground
/// (area 12), which have a minimap of their own to show.
pub fn in_cave(block: BlockId) -> bool {
    !block.is_overworld() && !matches!(block.area(), 12 | 34) && !is_legacy_dungeon(block.area())
}

/// How far the distance between two reference points on the world map may differ from their
/// distance inside the dungeon before the points count as disagreeing.
const REFERENCE_TOLERANCE: f32 = 500.0;

/// The rows that lead `src` to the world map, in the order of the param.
fn world_rows(conversions: &[Conversion], src: (u8, u8, u8)) -> Vec<&Conversion> {
    conversions.iter().filter(|c| c.src == src && WORLD_AREAS.contains(&c.dst.0)).collect()
}

/// Whether `rows` agree with one another: points that are as far apart on the map as inside the
/// dungeon. Most dungeons put every point on the same spot or shift them alike, which agrees.
/// Crumbling Farum Azula does not (kilometers apart), and then the game's own map is not the one
/// around the nearest point either.
fn agree(rows: &[&Conversion]) -> bool {
    let on_map = |c: &Conversion| {
        [f32::from(c.dst.1) * 256.0 + c.dst_pos[0], f32::from(c.dst.2) * 256.0 + c.dst_pos[1]]
    };
    rows.iter().all(|a| {
        rows.iter().all(|b| {
            let outside = distance_squared(on_map(a), on_map(b)).sqrt();
            let inside = distance_squared(a.src_pos, b.src_pos).sqrt();
            (outside - inside).abs() <= REFERENCE_TOLERANCE
        })
    })
}

/// Returns the world map aligned position and its area. A dungeon lists several reference
/// points, each valid around itself, so the one nearest the player is used; when they disagree
/// (see [`agree`]) the first one that is a base point is used for the whole dungeon, which puts
/// the graces of Crumbling Farum Azula on the whirlpool island where the game's map draws it
/// (the nearest point there is a placeholder in Flame Peak). Rows that lead to another dungeon
/// are only followed when no row leads to the world map.
fn legacy_conversion(conversions: &[Conversion], block: BlockId, local: [f32; 2]) -> Option<([f32; 2], u8)> {
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
        if let Some(nearest) = nearest(true) {
            let rows = world_rows(conversions, src);
            let c = if agree(&rows) { nearest } else { rows.iter().find(|c| c.base_point).copied().unwrap_or(nearest) };
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
            base_point: row.is_base_point(),
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

    fn row(src: (u8, u8, u8), src_pos: [f32; 2], dst: (u8, u8, u8), dst_pos: [f32; 2], base_point: bool) -> Conversion {
        Conversion { src, src_pos, dst, dst_pos, base_point }
    }

    #[test]
    fn a_dungeon_whose_reference_points_disagree_uses_its_first_base_point() {
        // Crumbling Farum Azula (rows 10, 11 and 132 of the base game): a grace of it is on the
        // whirlpool island, around m60_61_46, not at Flame Peak where the nearest point leads.
        let farum_azula = [
            row((13, 0, 0), [668.01, -2509.77], (60, 51, 43), [-36.71, 0.32], false),
            row((13, 0, 0), [-2509.61, -668.01], (60, 51, 43), [-36.71, 0.32], true),
            row((13, 0, 0), [0.0, 0.0], (60, 54, 53), [0.0, 0.0], true),
        ];
        let (position, area) =
            legacy_conversion(&farum_azula, BlockId::from_parts(13, 0, 0, 0), [116.66, 161.07]).unwrap();
        assert_eq!(area, 60);
        assert!((position[0] - 15645.56).abs() < 0.1 && (position[1] - 11837.4).abs() < 0.1, "{position:?}");
        // Stormveil Castle: every point on the same spot, the nearest one decides as before.
        let stormveil = [
            row((10, 0, 0), [100.0, 50.0], (60, 42, 36), [10.0, 20.0], true),
            row((10, 0, 0), [100.0, 50.0], (60, 42, 36), [10.0, 20.0], true),
        ];
        let (position, area) = legacy_conversion(&stormveil, BlockId::from_parts(10, 0, 0, 0), [110.0, 70.0]).unwrap();
        assert_eq!(area, 60);
        assert_eq!(position, [20.0 + 42.0 * 256.0, 40.0 + 36.0 * 256.0]);
    }

    #[test]
    fn roundtable_hold_is_m11_10() {
        assert!(is_roundtable_hold(BlockId::from_parts(11, 10, 0, 0)));
        assert!(!is_roundtable_hold(BlockId::from_parts(11, 0, 0, 0)));
        assert!(!is_roundtable_hold(BlockId::from_parts(10, 10, 0, 0)));
    }

    #[test]
    fn legacy_dungeons_are_not_caves() {
        assert!(!in_cave(BlockId::from_parts(13, 0, 0, 0)));
        assert!(!in_cave(BlockId::from_parts(10, 0, 0, 0)));
        // Catacombs and caves still count as caves; the underground and the overworld do not.
        assert!(in_cave(BlockId::from_parts(30, 0, 0, 0)));
        assert!(in_cave(BlockId::from_parts(31, 3, 0, 0)));
        // A Divine Tower is not one (area 34).
        assert!(!in_cave(BlockId::from_parts(34, 10, 0, 0)));
        assert!(!in_cave(BlockId::from_parts(12, 2, 0, 0)));
        assert!(!in_cave(BlockId::from_parts(60, 42, 36, 0)));
        for area in [10, 11, 13, 14, 15, 16, 18, 19, 20, 21, 22, 25, 28, 35] {
            assert!(is_legacy_dungeon(area), "area {area}");
        }
        for area in [12, 30, 31, 32, 34, 40, 41, 42, 43, 60, 61] {
            assert!(!is_legacy_dungeon(area), "area {area}");
        }
    }

    #[test]
    fn larger_overworld_tiles_are_centered_on_their_group_of_small_tiles() {
        // m60_10_10_01 spans 2x2 small tiles: half a small tile further along each axis.
        let block = BlockId::from_parts(60, 10, 10, 1);
        let (position, ..) = locate(block, [0.0, 0.0]);
        assert_eq!(position, [10.0 * 512.0 + 128.0, 10.0 * 512.0 + 128.0]);
    }

    #[test]
    fn a_units_digit_beyond_two_is_a_small_tile() {
        // m60_44_37_16, where the Mistwood is after Radahn: 256 m tile, not a 1024 m one.
        let (position, space, map) = locate(BlockId::from_parts(60, 44, 37, 16), [10.0, -20.0]);
        assert_eq!(position, [10.0 + 44.0 * 256.0, -20.0 + 37.0 * 256.0]);
        assert_eq!((space, map), (60, Some(MapId::Overworld)));
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
