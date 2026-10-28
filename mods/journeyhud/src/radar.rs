//! Game-side reader of what the radar shows: the camera, the player's facing
//! and the offsets of the other players, the player's spirits and the locked-on
//! enemy. Runs on the frame task and copies the result into [LATEST], so the
//! render thread never touches live game objects. Positions are only ever
//! used as offsets from the player (all in the same Havok space), so no map
//! coordinate conversion is needed. Details: `docs/radar.md`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use eldenring::cs::{
    BossHealthDisplayEntry, CSCamera, CSEventFlagMan, CSFade, CSFeManHudState, CSFeManImp, SoloParamRepository, WorldAreaTime, WorldChrMan, WorldMapPieceParam,
};
use fromsoftware_shared::FromStatic;

use common::{logger, params};

use crate::map::tiles::MapId;
use crate::map::{beacons, graces, mfg, points, world};
use crate::settings;

// The layouts read here were checked in IDA on 2.6.2.0 to 2.7.2.0, see `docs/offsets.md`: a
// `fromsoftware-rs` that moves them stops the build.
const _: () = {
    assert!(std::mem::offset_of!(CSFeManImp, boss_health_displays) == 0x5BF0);
    assert!(std::mem::size_of::<BossHealthDisplayEntry>() == 0x20);
    assert!(std::mem::offset_of!(WorldAreaTime, clock) == 0);
    assert!(std::mem::offset_of!(eldenring::cs::GameDataMan, death_count) == 0x94);
};

/// The player's death count (`GameDataMan::death_count`). `GameDataMan::instance()` goes through the
/// RVA table and panics on 2.7.2.0, so the global is found by an AOB that is unique on every exe
/// version (`docs/offsets.md`). `None` until the manager exists.
fn death_count() -> Option<u32> {
    static GAME_DATA_MAN: common::memscan::CachedAddr = common::memscan::CachedAddr::new();
    static SCANNED: AtomicBool = AtomicBool::new(false);

    let address = GAME_DATA_MAN.get_or_resolve(|| {
        if SCANNED.swap(true, Ordering::Relaxed) {
            return None;
        }
        let hit = common::memscan::find_pattern_in_module("48 8B 05 ?? ?? ?? ?? 48 85 C0 74 05 48 8B 40 08 C3")? as usize;
        let offset = common::memscan::read_safe::<i32>(hit + 3)?;
        Some((hit + 7).wrapping_add_signed(offset as isize))
    })?;
    let manager = common::memscan::read_safe::<usize>(address).filter(|&manager| manager != 0)?;
    common::memscan::read_safe::<u32>(manager + std::mem::offset_of!(eldenring::cs::GameDataMan, death_count))
}

/// Another character as the radar draws it.
#[derive(Clone, Copy)]
pub struct Dot {
    /// Meters from the player along the world's X and Z axes.
    pub offset: [f32; 2],
    /// Horizontal direction the character faces.
    pub facing: [f32; 2],
}

#[derive(Clone)]
pub struct Snapshot {
    /// Where the player is on the world map, in meters on the overworld grid
    /// (see `map::world`).
    pub position: [f32; 2],
    /// The space the position is in: positions only compare within the same one.
    pub space: i32,
    /// The world map that shows the player's position, if any.
    pub map: Option<MapId>,
    /// Offsets of the sites of grace the player has found, within reach.
    pub graces: Vec<[f32; 2]>,
    /// The visible points of interest of the world map, within reach.
    pub points: Vec<points::Point>,
    /// The markers of Map for Goblins within reach (empty without its DLL).
    pub goblins: Vec<mfg::Marker>,
    /// Offsets of the beacons the player has placed on this map.
    pub beacons: Vec<[f32; 2]>,
    /// Horizontal direction the player faces.
    pub facing: [f32; 2],
    /// The camera's right and forward directions on the ground.
    pub camera_right: [f32; 2],
    pub camera_forward: [f32; 2],
    /// Screen fade from 0 to 1; loading screens and warps sit behind a full one.
    pub fade: f32,
    /// False while a menu has the HUD hidden or changed.
    pub hud_visible: bool,
    /// A boss health bar is on screen.
    pub boss_fight: bool,
    /// The player is in a cave, catacomb or other dungeon block.
    pub in_cave: bool,
    /// The map fragments the player has, one bit per fragment (all set when the whole map shows).
    pub fragments: u32,
    /// The in-game time of day: hours and minutes.
    pub clock: Option<[u8; 2]>,
    /// The player's death count.
    pub deaths: Option<u32>,
    pub partners: Vec<Dot>,
    pub spirits: Vec<Dot>,
    /// Offset of the locked-on enemy.
    pub target: Option<[f32; 2]>,
}

/// Whether the game is in normal play (no menu changing the HUD, no fade), and when that was
/// read. The overlay's target panel follows it even when the minimap is off.
static PLAY: Mutex<Option<(bool, Instant)>> = Mutex::new(None);

/// True while the game is in normal play; false in a menu, on a loading screen or a fade, or when
/// it was not read lately.
pub fn in_control() -> bool {
    let Ok(guard) = PLAY.lock() else { return false };
    guard.is_some_and(|(play, at)| play && at.elapsed() < STALE_AFTER)
}

/// The latest snapshot and when it was read; [current] drops it once stale.
static LATEST: Mutex<Option<(Snapshot, Instant)>> = Mutex::new(None);

const STALE_AFTER: Duration = Duration::from_millis(500);

pub fn current() -> Option<Snapshot> {
    let guard = LATEST.lock().ok()?;
    let (snapshot, at) = guard.as_ref()?;
    (at.elapsed() < STALE_AFTER).then(|| snapshot.clone())
}

/// Reads the world for this frame; `target` is the locked-on enemy's offset.
pub fn update(target: Option<[f32; 2]>) {
    log_piece_rows();
    if let Ok(mut guard) = PLAY.lock() {
        *guard = play_state().map(|play| (play, Instant::now()));
    }
    let snapshot = if settings::minimap_enabled() { read(target) } else { None };
    if let Ok(mut guard) = LATEST.lock() {
        *guard = snapshot.map(|snapshot| (snapshot, Instant::now()));
    }
}

/// Logs once which rows of `WorldMapPieceParam` hold a map fragment (they come in runs: the
/// overworld's, the underground's and the Land of Shadow's), so the base of each run in
/// [map_fragments] can be checked. Tried each frame until the params are loaded, for a while.
fn log_piece_rows() {
    static DONE: AtomicBool = AtomicBool::new(false);
    static TRIES: AtomicU32 = AtomicU32::new(0);
    if DONE.load(Ordering::Relaxed) || TRIES.fetch_add(1, Ordering::Relaxed) > 1200 {
        return;
    }
    let Ok(repo) = (unsafe { SoloParamRepository::instance() }) else { return };
    if params::check::<WorldMapPieceParam>(repo).is_err() {
        DONE.store(true, Ordering::Relaxed);
        return;
    }
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for id in 0..400 {
        if repo.get::<WorldMapPieceParam>(id).is_some_and(|row| row.open_event_flag_id() != 0) {
            match runs.last_mut() {
                Some((_, end)) if *end + 1 == id => *end = id,
                _ => runs.push((id, id)),
            }
        }
    }
    if runs.is_empty() {
        return;
    }
    DONE.store(true, Ordering::Relaxed);
    let runs: Vec<String> = runs.iter().map(|(from, to)| format!("{from}-{to}")).collect();
    logger::log(&format!("Map: piece rows {}", runs.join(", ")));
}

/// Whether the game is in normal play, `None` while its HUD and fade are not there to read.
fn play_state() -> Option<bool> {
    let fade = unsafe { CSFade::instance() }
        .map_or(0.0, |fade| fade.fade_plates.iter().map(|plate| plate.current_color.a).fold(0.0, f32::max));
    let hud = unsafe { CSFeManImp::instance() }.ok()?;
    Some(hud.hud_state == CSFeManHudState::Default && fade < 0.05)
}

fn read(target: Option<[f32; 2]>) -> Option<Snapshot> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let camera = unsafe { CSCamera::instance() }.ok()?;
    let player = world_chr_man.main_player.as_ref()?;

    let fade = unsafe { CSFade::instance() }
        .map_or(0.0, |fade| fade.fade_plates.iter().map(|plate| plate.current_color.a).fold(0.0, f32::max));
    let fe = unsafe { CSFeManImp::instance() }.ok();
    let hud_visible = fe.as_ref().map_or(true, |fe| fe.hud_state == CSFeManHudState::Default);
    // A boss health bar is on screen while one of its slots names a character.
    let boss_fight = fe.is_some_and(|fe| fe.boss_health_displays.iter().any(|boss| !boss.field_ins_handle.is_empty()));
    // Catacombs, caves, tunnels and the like: any block outside the overworld, except the
    // underground (area 12), which has a world map of its own.
    let block = player.current_block_id;
    let in_cave = !block.is_overworld() && block.area() != 12;

    // The camera and the physics matrices share axes and scale with the
    // world, so their direction vectors apply to offsets directly.
    let view = &camera.pers_cam_1.matrix;
    let model = &player.chr_ins.chr_ctrl.physics_model_matrix;
    let origin = model.3;
    let (position, space, map) =
        world::locate(player.current_block_id, [player.block_position.x, player.block_position.z]);
    // Beyond this the other character is in some other part of the world
    // whose coordinates only look near.
    let reach = settings::minimap_range() * 3.0;

    let dot = |chr: &eldenring::cs::ChrIns| {
        let model = &chr.chr_ctrl.physics_model_matrix;
        let offset = [model.3.0 - origin.0, model.3.2 - origin.2];
        let near = offset[0].hypot(offset[1]) <= reach;
        // Characters face down their model's -Z axis.
        (near && world::locate(chr.block_id, [0.0, 0.0]).1 == space).then_some(Dot { offset, facing: [-model.2.0, -model.2.2] })
    };

    let partners = if settings::minimap_coop() {
        world_chr_man
            .player_chr_set
            .characters()
            .filter(|other| !std::ptr::eq(&**other, &**player))
            .filter_map(|other| dot(&other.chr_ins))
            .collect()
    } else {
        Vec::new()
    };
    let mut spirits = Vec::new();
    if settings::minimap_spirits() {
        common::player::for_each_active_chr(&world_chr_man.summon_buddy_chr_set, |chr| spirits.extend(dot(chr)));
    }
    let clock = unsafe { WorldAreaTime::instance() }.ok().map(|time| [time.clock.hours(), time.clock.minutes()]);
    let deaths = if settings::minimap_deaths() { death_count() } else { None };
    let fragments = if settings::minimap_full_map() { u32::MAX } else { map_fragments(map) };

    Some(Snapshot {
        position,
        space,
        map,
        beacons: match (settings::minimap_beacons(), map) {
            (true, Some(map)) => beacons::placed(map)
                .into_iter()
                .map(|beacon| [beacon.position[0] - position[0], beacon.position[1] - position[1]])
                .collect(),
            _ => Vec::new(),
        },
        goblins: match (settings::minimap_goblins(), map) {
            (true, Some(map)) => mfg::nearby(map, position, space, settings::minimap_range() * 1.2),
            _ => Vec::new(),
        },
        points: match (settings::minimap_points(), map) {
            (true, Some(map)) => points::nearby(map, position, space, settings::minimap_range() * 1.2),
            _ => Vec::new(),
        },
        graces: if settings::minimap_graces() {
            graces::nearby(position, space, settings::minimap_range() * 1.2)
        } else {
            Vec::new()
        },
        facing: [-model.2.0, -model.2.2],
        camera_right: [view.0.0, view.0.2],
        camera_forward: [view.2.0, view.2.2],
        fade,
        hud_visible,
        boss_fight,
        in_cave,
        fragments,
        clock,
        deaths,
        partners,
        spirits,
        target,
    })
}

/// The map fragments the player has, one bit per fragment of the map they are on, as the map
/// tile masks number them: bit b is `WorldMapPieceParam` row `base + b`, base 0 for the
/// overworld, 100 for the underground and 200 for the Land of Shadow. Everything counts as found until the params and the
/// flags are loaded, or when the player is not on a map.
fn map_fragments(map: Option<MapId>) -> u32 {
    let base = match map {
        Some(MapId::Overworld) => 0,
        Some(MapId::Underground) => 100,
        // The Land of Shadow keeps its five fragments in the rows after the underground's (to be
        // confirmed by the `Map: piece rows` log line).
        Some(MapId::Shadow) => 200,
        None => return u32::MAX,
    };
    static READABLE: OnceLock<bool> = OnceLock::new();
    let (Ok(repo), Ok(flags)) = (unsafe { SoloParamRepository::instance() }, unsafe { CSEventFlagMan::instance() })
    else {
        return u32::MAX;
    };
    let readable = *READABLE.get_or_init(|| match params::check::<WorldMapPieceParam>(repo) {
        Ok(()) => true,
        Err(reason) => {
            logger::warn(&format!("Map: piece params unavailable ({reason})"));
            false
        }
    });
    if !readable {
        return u32::MAX;
    }
    (0..32)
        .filter(|&bit| {
            repo.get::<WorldMapPieceParam>(base + bit)
                .map(|row| row.open_event_flag_id())
                .is_some_and(|flag| flag != 0 && flags.virtual_memory_flag.get_flag(flag))
        })
        .fold(0, |bits, bit| bits | 1 << bit)
}
