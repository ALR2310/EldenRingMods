//! Game-side reader of what the radar shows: the camera, the player's facing
//! and the offsets of the other players, the player's spirits and the locked-on
//! enemy. Runs on the frame task and copies the result into [LATEST], so the
//! render thread never touches live game objects. Positions are only ever
//! used as offsets from the player (all in the same Havok space), so no map
//! coordinate conversion is needed. Details: `docs/radar.md`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use eldenring::cs::{CSCamera, CSFade, CSFeManHudState, CSFeManImp, WorldChrMan};
use fromsoftware_shared::FromStatic;

use crate::settings;

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
    /// Horizontal direction the player faces.
    pub facing: [f32; 2],
    /// The camera's right and forward directions on the ground.
    pub camera_right: [f32; 2],
    pub camera_forward: [f32; 2],
    /// Screen fade from 0 to 1; loading screens and warps sit behind a full one.
    pub fade: f32,
    /// False while a menu has the HUD hidden or changed.
    pub hud_visible: bool,
    pub partners: Vec<Dot>,
    pub spirits: Vec<Dot>,
    /// Offset of the locked-on enemy.
    pub target: Option<[f32; 2]>,
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
    let snapshot = if settings::minimap_enabled() { read(target) } else { None };
    if let Ok(mut guard) = LATEST.lock() {
        *guard = snapshot.map(|snapshot| (snapshot, Instant::now()));
    }
}

fn read(target: Option<[f32; 2]>) -> Option<Snapshot> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let camera = unsafe { CSCamera::instance() }.ok()?;
    let player = world_chr_man.main_player.as_ref()?;

    let fade = unsafe { CSFade::instance() }
        .map_or(0.0, |fade| fade.fade_plates.iter().map(|plate| plate.current_color.a).fold(0.0, f32::max));
    let hud_visible = unsafe { CSFeManImp::instance() }.map_or(true, |fe| fe.hud_state == CSFeManHudState::Default);

    // The camera and the physics matrices share axes and scale with the
    // world, so their direction vectors apply to offsets directly.
    let view = &camera.pers_cam_1.matrix;
    let model = &player.chr_ins.chr_ctrl.physics_model_matrix;
    let origin = model.3;
    let area = player.chr_ins.block_id.area();
    // Beyond this the other character is in some other part of the world
    // whose coordinates only look near.
    let reach = settings::minimap_range() * 3.0;

    let dot = |chr: &eldenring::cs::ChrIns| {
        let model = &chr.chr_ctrl.physics_model_matrix;
        let offset = [model.3.0 - origin.0, model.3.2 - origin.2];
        let near = offset[0].hypot(offset[1]) <= reach;
        // Characters face down their model's -Z axis.
        (chr.block_id.area() == area && near).then_some(Dot { offset, facing: [-model.2.0, -model.2.2] })
    };

    let partners = world_chr_man
        .player_chr_set
        .characters()
        .filter(|other| !std::ptr::eq(&**other, &**player))
        .filter_map(|other| dot(&other.chr_ins))
        .collect();
    let mut spirits = Vec::new();
    common::player::for_each_active_chr(&world_chr_man.summon_buddy_chr_set, |chr| spirits.extend(dot(chr)));

    Some(Snapshot {
        facing: [-model.2.0, -model.2.2],
        camera_right: [view.0.0, view.0.2],
        camera_forward: [view.2.0, view.2.2],
        fade,
        hud_visible,
        partners,
        spirits,
        target,
    })
}
