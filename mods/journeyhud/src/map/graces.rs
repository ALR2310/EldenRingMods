// The idea follows kubilaiswf-minimap (MIT, Copyright (c) 2026 kubilaiswf, see
// THIRD-PARTY-NOTICES.txt), `game.rs`: the grace table is `BonfireWarpParam`, a grace is found
// when its event flag is set. Written against this crate's helpers. See docs/map.md.
//! The sites of grace the player has found, as offsets from the player.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use eldenring::cs::{BlockId, BonfireWarpParam, CSEventFlagMan, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use common::{logger, params};

use super::cache::Refreshed;
use super::world;

struct Grace {
    /// Set once the player has found the grace.
    event_flag: u32,
    /// On the overworld grid, like the player's position (see `world::locate`).
    position: [f32; 2],
    space: i32,
}

static GRACES: OnceLock<Vec<Grace>> = OnceLock::new();

/// Where the found graces of the space last asked for are, on the overworld grid. A grace is
/// found rarely, so the flags are checked once a second, not every frame.
static FOUND: Mutex<Refreshed<i32, [f32; 2]>> = Mutex::new(Refreshed::new());
const RECHECK: Duration = Duration::from_secs(1);

/// Meters from `position` (in `space`) to every found grace within `reach` of it.
pub fn nearby(position: [f32; 2], space: i32, reach: f32) -> Vec<[f32; 2]> {
    let mut cache = FOUND.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get(space, RECHECK, || found(space))
        .iter()
        .map(|grace| [grace[0] - position[0], grace[1] - position[1]])
        .filter(|offset| offset[0].hypot(offset[1]) <= reach)
        .collect()
}

/// Every found grace of `space`, or `None` while the table or the flags are not there yet.
fn found(space: i32) -> Option<Vec<[f32; 2]>> {
    let (graces, Ok(flags)) = (graces()?, unsafe { CSEventFlagMan::instance() }) else { return None };
    Some(
        graces
            .iter()
            .filter(|grace| grace.space == space)
            .filter(|grace| flags.virtual_memory_flag.get_flag(grace.event_flag))
            .map(|grace| grace.position)
            .collect(),
    )
}

/// The grace table, read once the params and the dungeon conversions are ready (they load
/// after the player can already exist, so this is retried each frame until it works).
fn graces() -> Option<&'static [Grace]> {
    if let Some(graces) = GRACES.get() {
        return Some(graces);
    }
    if !world::conversions_ready() {
        return None;
    }
    let repo = unsafe { SoloParamRepository::instance() }.ok()?;
    if let Err(reason) = params::check::<BonfireWarpParam>(repo) {
        logger::warn(&format!("Map: grace params unavailable ({reason})"));
        return None;
    }
    let graces: Vec<Grace> = (0..)
        .map_while(|i| repo.get_row_by_index::<BonfireWarpParam>(i))
        .filter(|row| row.eventflag_id() != 0)
        .map(|row| {
            let block = BlockId::from_parts(row.area_no(), row.grid_x_no(), row.grid_z_no(), 0);
            let (position, space, _) = world::locate(block, [row.pos_x(), row.pos_z()]);
            Grace { event_flag: row.eventflag_id(), position, space }
        })
        .collect();
    logger::log(&format!("Map: {} grace(s)", graces.len()));
    Some(GRACES.get_or_init(|| graces))
}
