//! Game-side reader of the locked-on enemy. Runs as a frame task and copies
//! what the panel shows into [SNAPSHOT], so the render thread never touches
//! live game objects. Details: `docs/design.md`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use eldenring::cs::{
    CSChrDataModule, CSChrSuperArmorModule, CSTaskGroupIndex, ChrIns, ChrInsModuleContainer, ChrSet, FieldInsHandle,
    PlayerIns, WorldChrMan,
};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::FromStatic;

use common::logger;

use crate::settings;

// The layout the fields are read at, checked against eldenring.exe 2.6.2.0 to
// 2.7.2.0 (`docs/offsets.md`). A `fromsoftware-rs` update that moves one stops
// the build instead of reading the wrong memory.
const _: () = {
    assert!(std::mem::offset_of!(PlayerIns, locked_on_enemy) == 0x6B0);
    assert!(std::mem::offset_of!(ChrIns, modules) == 0x190);
    assert!(std::mem::offset_of!(ChrInsModuleContainer, data) == 0x00);
    assert!(std::mem::offset_of!(ChrInsModuleContainer, super_armor) == 0x40);
    assert!(std::mem::offset_of!(CSChrDataModule, hp) == 0x138);
    assert!(std::mem::offset_of!(CSChrDataModule, max_hp) == 0x13C);
    assert!(std::mem::offset_of!(CSChrSuperArmorModule, sa_durability) == 0x10);
    assert!(std::mem::offset_of!(CSChrSuperArmorModule, sa_durability_max) == 0x14);
};

/// What the panel shows for the locked-on enemy.
#[derive(Clone)]
pub struct Target {
    /// The chr's address: stable for one enemy, differs between enemies.
    pub id: usize,
    pub npc_param_id: i32,
    pub hp: i32,
    pub max_hp: i32,
    /// `None` when the enemy has no poise bar (max is 0).
    pub poise: Option<(f32, f32)>,
}

/// The latest target and when it was read; [current] drops it once stale
/// (task stopped, e.g. a loading screen).
static SNAPSHOT: Mutex<Option<(Target, Instant)>> = Mutex::new(None);

const STALE_AFTER: Duration = Duration::from_millis(500);

pub fn current() -> Option<Target> {
    let guard = SNAPSHOT.lock().ok()?;
    let (target, at) = guard.as_ref()?;
    (at.elapsed() < STALE_AFTER).then(|| target.clone())
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    let mut last = None;
    common::task::run_recurring_safe(cs_task, "Target", CSTaskGroupIndex::FrameBegin, move |_data: &FD4TaskData| {
        let read = if settings::enabled() { read() } else { None };
        let npc = read.as_ref().map(|t| t.npc_param_id);
        if npc != last {
            last = npc;
            match &read {
                Some(t) => logger::log(&format!("Target: npc {} hp {}/{} poise {:?}", t.npc_param_id, t.hp, t.max_hp, t.poise)),
                None => logger::log("Target: none"),
            }
        }
        if let Ok(mut guard) = SNAPSHOT.lock() {
            *guard = read.map(|target| (target, Instant::now()));
        }
    });
}

fn read() -> Option<Target> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let player = world_chr_man.main_player.as_ref()?;
    let handle = player.locked_on_enemy;
    if handle.is_empty() {
        return None;
    }
    let chr = chr_by_handle(world_chr_man, handle)?;
    let max_hp = chr.modules.data.max_hp;
    if max_hp <= 0 {
        return None;
    }
    let armor = &chr.modules.super_armor;
    Some(Target {
        id: chr as *const ChrIns as usize,
        npc_param_id: chr.npc_param_id,
        hp: chr.modules.data.hp.max(0),
        max_hp,
        poise: (armor.sa_durability_max > 0.0 && armor.sa_durability_max.is_finite())
            .then_some((armor.sa_durability.max(0.0), armor.sa_durability_max)),
    })
}

/// The chr of `handle` via the ChrSet's `get_chr_ins_by_handle` (vtable slot
/// 3). Not `WorldChrMan::chr_ins_by_handle`: it goes through `rva::get()`,
/// which panics on game builds newer than the pinned `fromsoftware-rs`.
fn chr_by_handle(world_chr_man: &WorldChrMan, handle: FieldInsHandle) -> Option<&ChrIns> {
    let chr_set = world_chr_man.chr_sets.get(handle.selector.container() as usize)?.as_ref()?;
    let chr_set: &ChrSet<ChrIns> = chr_set;
    type GetByHandle = extern "C" fn(&ChrSet<ChrIns>, FieldInsHandle) -> Option<std::ptr::NonNull<ChrIns>>;
    unsafe {
        let vtable = *(chr_set as *const ChrSet<ChrIns> as *const *const usize);
        let get_by_handle: GetByHandle = std::mem::transmute(*vtable.add(3));
        get_by_handle(chr_set, handle).map(|p| &*p.as_ptr())
    }
}
