// The visibility rule follows kubilaiswf-minimap (MIT, Copyright (c) 2026 kubilaiswf, see
// THIRD-PARTY-NOTICES.txt), `game.rs` `visible_points`, which took it from the game via
// soarqin's EROverlay notes: a row shows when its open flag is set and, unless it may go
// without text, one of its texts is active. Written against this crate's helpers.
//! The points of interest of the world map (ruins, churches, caves, bosses, ...), from
//! `WorldMapPointParam`, as offsets from the player. The icon of each is the game's own
//! (`MENU_MAP_<icon id>` in the menu sprite sheets, see `sprites`).

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use eldenring::cs::{BlockId, CSEventFlagMan, SoloParamRepository, WorldMapPointParam};
use fromsoftware_shared::FromStatic;

use common::{logger, params};

use super::cache::Refreshed;
use super::tiles::MapId;
use super::world;

/// One text line of a point: it is active when its enable flags are set and its disable flags are not.
#[derive(Clone, Copy)]
struct Text {
    id: i32,
    enable: [u32; 2],
    disable: [u32; 2],
    /// 1 marks the text that switches a point to its alternative icon.
    kind: u8,
}

/// One `WorldMapPointParam` row, reduced to what deciding its visibility takes.
struct Row {
    position: [f32; 2],
    space: i32,
    on_overworld: bool,
    on_underground: bool,
    on_shadow: bool,
    icon: u16,
    alt_icon: u16,
    open_flag: u32,
    /// `u32::MAX` when the point cannot be cleared.
    cleared_flag: u32,
    /// Rows allowed without text skip the text condition.
    needs_text: bool,
    texts: [Text; 8],
}

/// Icon ids drawn above the places: the people (an NPC, a summon sign, an invader).
pub fn is_npc(icon: u16) -> bool {
    matches!(icon, 80 | 86 | 87 | 88)
}

/// A point of interest to draw.
#[derive(Clone, Copy)]
pub struct Point {
    /// Meters from the player along the world's X and Z axes.
    pub offset: [f32; 2],
    /// The game's map icon id.
    pub icon: u16,
    /// Cleared, like a defeated boss: the game dims these.
    pub cleared: bool,
}

static ROWS: OnceLock<Vec<Row>> = OnceLock::new();

/// A point whose flags were checked: where it is on the overworld grid and how to draw it.
struct Visible {
    position: [f32; 2],
    icon: u16,
    cleared: bool,
}

/// The visible points of the map and space last asked for. Flags change rarely (a boss falls, a
/// place is found), so they are checked once a second over all rows, not every frame.
static VISIBLE: Mutex<Refreshed<(MapId, i32), Visible>> = Mutex::new(Refreshed::new());
const RECHECK: Duration = Duration::from_secs(1);

/// Icon ids that are not places to draw: none, the sites of grace (drawn from their own
/// table), and the light beams that guide toward graces and the like.
fn skipped(icon: u16) -> bool {
    matches!(icon, 0..=2 | 83..=85)
}

/// The visible points on `map` within `reach` meters of `position` (in `space`).
pub fn nearby(map: MapId, position: [f32; 2], space: i32, reach: f32) -> Vec<Point> {
    let mut cache = VISIBLE.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get((map, space), RECHECK, || visible(map, space))
        .iter()
        .map(|point| (point, [point.position[0] - position[0], point.position[1] - position[1]]))
        .filter(|(_, offset)| offset[0].hypot(offset[1]) <= reach)
        .map(|(point, offset)| Point { offset, icon: point.icon, cleared: point.cleared })
        .collect()
}

/// Every point of `space` on `map` that the game shows now, or `None` while the table or the
/// flags are not there yet.
fn visible(map: MapId, space: i32) -> Option<Vec<Visible>> {
    let (rows, Ok(flags)) = (rows()?, unsafe { CSEventFlagMan::instance() }) else { return None };
    let flags = &flags.virtual_memory_flag;
    let on = |flag: u32| flag != 0 && flags.get_flag(flag);
    Some(
        rows.iter()
            .filter(|row| match map {
                MapId::Overworld => row.on_overworld,
                MapId::Underground => row.on_underground,
                MapId::Shadow => row.on_shadow,
            })
            .filter(|row| row.space == space)
            .filter(|row| row.open_flag == 0 || on(row.open_flag))
            .filter_map(|row| {
                let active = |text: &&Text| {
                    text.id > 0
                        && text.enable.iter().all(|&flag| flag == 0 || on(flag))
                        && !text.disable.iter().any(|&flag| on(flag))
                };
                let mut active_texts = row.texts.iter().filter(active).peekable();
                if row.needs_text && active_texts.peek().is_none() {
                    return None;
                }
                let icon = if active_texts.any(|text| text.kind == 1) && row.alt_icon != 0 { row.alt_icon } else { row.icon };
                (!skipped(icon)).then(|| Visible {
                    position: row.position,
                    icon,
                    cleared: row.cleared_flag != u32::MAX && on(row.cleared_flag),
                })
            })
            .collect(),
    )
}

/// The point table, read once the params and the dungeon conversions are ready (they load
/// after the player can already exist, so this is retried each frame until it works).
fn rows() -> Option<&'static [Row]> {
    if let Some(rows) = ROWS.get() {
        return Some(rows);
    }
    if !world::conversions_ready() {
        return None;
    }
    let repo = unsafe { SoloParamRepository::instance() }.ok()?;
    if let Err(reason) = params::check::<WorldMapPointParam>(repo) {
        logger::warn(&format!("Map: point params unavailable ({reason})"));
        return None;
    }
    let rows: Vec<Row> = (0..)
        .map_while(|i| repo.get_row_by_index::<WorldMapPointParam>(i))
        // Area labels name whole regions rather than places.
        .filter(|row| !row.is_area_icon())
        .map(|row| {
            let block = BlockId::from_parts(row.area_no(), row.grid_x_no(), row.grid_z_no(), 0);
            let (position, space, _) = world::locate(block, [row.pos_x(), row.pos_z()]);
            // The second set of text flags is signed; non-positive means unused.
            let flag2 = |flag: i32| u32::try_from(flag).unwrap_or(0);
            let text = |id: i32, enable: u32, disable: u32, enable2: i32, disable2: i32, kind: u8| Text {
                id,
                enable: [enable, flag2(enable2)],
                disable: [disable, flag2(disable2)],
                kind,
            };
            let texts = [
                text(row.text_id1(), row.text_enable_flag_id1(), row.text_disable_flag_id1(), row.text_enable_flag2_id1(), row.text_disable_flag2_id1(), row.text_type1()),
                text(row.text_id2(), row.text_enable_flag_id2(), row.text_disable_flag_id2(), row.text_enable_flag2_id2(), row.text_disable_flag2_id2(), row.text_type2()),
                text(row.text_id3(), row.text_enable_flag_id3(), row.text_disable_flag_id3(), row.text_enable_flag2_id3(), row.text_disable_flag2_id3(), row.text_type3()),
                text(row.text_id4(), row.text_enable_flag_id4(), row.text_disable_flag_id4(), row.text_enable_flag2_id4(), row.text_disable_flag2_id4(), row.text_type4()),
                text(row.text_id5(), row.text_enable_flag_id5(), row.text_disable_flag_id5(), row.text_enable_flag2_id5(), row.text_disable_flag2_id5(), row.text_type5()),
                text(row.text_id6(), row.text_enable_flag_id6(), row.text_disable_flag_id6(), row.text_enable_flag2_id6(), row.text_disable_flag2_id6(), row.text_type6()),
                text(row.text_id7(), row.text_enable_flag_id7(), row.text_disable_flag_id7(), row.text_enable_flag2_id7(), row.text_disable_flag2_id7(), row.text_type7()),
                text(row.text_id8(), row.text_enable_flag_id8(), row.text_disable_flag_id8(), row.text_enable_flag2_id8(), row.text_disable_flag2_id8(), row.text_type8()),
            ];
            Row {
                position,
                space,
                on_overworld: row.disp_mask00(),
                on_underground: row.disp_mask01(),
                on_shadow: row.disp_mask02(),
                icon: row.icon_id(),
                alt_icon: row.alt_icon_id(),
                open_flag: row.event_flag_id(),
                cleared_flag: row.cleared_event_flag_id(),
                needs_text: !row.is_enable_no_text(),
                texts,
            }
        })
        .collect();
    logger::log(&format!("Map: {} point(s)", rows.len()));
    Some(ROWS.get_or_init(|| rows))
}

#[cfg(test)]
mod tests {
    use super::skipped;

    #[test]
    fn people_are_the_npc_layer_places_are_not() {
        assert!([80, 86, 87, 88].into_iter().all(super::is_npc));
        assert!(![4, 66, 79, 89, 208].into_iter().any(super::is_npc));
    }

    #[test]
    fn graces_and_beams_are_not_drawn_as_points() {
        assert!([0, 1, 2, 83, 84, 85].into_iter().all(skipped));
        assert!(![3, 82, 86, 100, 208].into_iter().any(skipped));
    }
}
