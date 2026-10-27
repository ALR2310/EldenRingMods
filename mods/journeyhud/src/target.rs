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

use crate::{radar, settings};

// The layout the fields are read at, checked against eldenring.exe 2.6.2.0 to
// 2.7.2.0 (`docs/offsets.md`). A `fromsoftware-rs` update that moves one stops
// the build instead of reading the wrong memory.
const _: () = {
    assert!(std::mem::offset_of!(PlayerIns, locked_on_enemy) == 0x6B0);
    assert!(std::mem::offset_of!(ChrIns, modules) == 0x190);
    assert!(std::mem::offset_of!(ChrInsModuleContainer, data) == 0x00);
    assert!(std::mem::offset_of!(ChrInsModuleContainer, super_armor) == 0x40);
    // `resist` is the fifth pointer of the container, +0x20 (the container has no type for it).
    assert!(std::mem::offset_of!(ChrInsModuleContainer, time_act) == 0x18);
    assert!(std::mem::offset_of!(CSChrDataModule, hp) == 0x138);
    assert!(std::mem::offset_of!(CSChrDataModule, max_hp) == 0x13C);
    assert!(std::mem::offset_of!(CSChrSuperArmorModule, sa_durability) == 0x10);
    assert!(std::mem::offset_of!(CSChrSuperArmorModule, sa_durability_max) == 0x14);
};

/// The seven status buildups, in the order the resist module keeps them.
pub const BUILDUP: [&str; 7] = ["poison", "scarlet rot", "bleed", "death blight", "frostbite", "sleep", "madness"];

/// Where the resist module is in the module container, and where it keeps the seven values left
/// to resist and the seven maximums (`docs/buildup.md`).
const RESIST_AT: usize = 0x20;
const RESIST_LEFT: usize = 0x10;
const RESIST_MAX: usize = 0x2C;
/// A maximum this high means the enemy cannot be afflicted at all.
const IMMUNE: i32 = 999;

/// The buildup of one status on the enemy.
#[derive(Clone, Copy)]
pub struct Buildup {
    /// An index into [BUILDUP].
    pub kind: usize,
    /// How much has built up, from 0 to `max`.
    pub built: i32,
    pub max: i32,
}

/// What the panel shows for the locked-on enemy.
#[derive(Clone)]
pub struct Target {
    /// The chr's address: stable for one enemy, differs between enemies.
    pub id: usize,
    pub npc_param_id: i32,
    /// Meters from the player along the world's X and Z axes.
    pub offset: [f32; 2],
    pub hp: i32,
    pub max_hp: i32,
    /// `None` when the enemy has no poise bar (max is 0).
    pub poise: Option<(f32, f32)>,
    /// The statuses that have started to build up and can still afflict the enemy.
    pub buildup: Vec<Buildup>,
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
    let mut last_kinds: Vec<usize> = Vec::new();
    common::task::run_recurring_safe(cs_task, "Target", CSTaskGroupIndex::FrameBegin, move |_data: &FD4TaskData| {
        let read = if settings::enabled() { read() } else { None };
        radar::update(read.as_ref().map(|t| t.offset));
        let npc = read.as_ref().map(|t| t.npc_param_id);
        if npc != last {
            last = npc;
            match &read {
                Some(t) => logger::log(&format!("Target: npc {} hp {}/{} poise {:?}", t.npc_param_id, t.hp, t.max_hp, t.poise)),
                None => logger::log("Target: none"),
            }
        }
        // One line when a status starts or stops building up: how the values behave, in the log.
        let kinds: Vec<usize> = read.iter().flat_map(|t| t.buildup.iter().map(|b| b.kind)).collect();
        if kinds != last_kinds {
            if let Some(target) = &read {
                let shown: Vec<String> =
                    target.buildup.iter().map(|b| format!("{} {}/{}", BUILDUP[b.kind], b.built, b.max)).collect();
                logger::log(&format!("Buildup: {}", shown.join(", ")));
            }
            last_kinds = kinds;
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
        offset: offset_from(player, chr),
        hp: chr.modules.data.hp.max(0),
        max_hp,
        poise: (armor.sa_durability_max > 0.0 && armor.sa_durability_max.is_finite())
            .then_some((armor.sa_durability.max(0.0), armor.sa_durability_max)),
        buildup: if settings::buildup() { read_buildup(chr) } else { Vec::new() },
    })
}

/// The status buildups of `chr`, from its resist module. The module keeps what is left to resist
/// and counts down from the maximum, so what has built up is the difference.
fn read_buildup(chr: &ChrIns) -> Vec<Buildup> {
    let container = &*chr.modules as *const ChrInsModuleContainer as *const u8;
    // SAFETY: the container is the live module list of the chr; the module it points to, when
    // not null, is a `CSChrResistModule` (see `docs/buildup.md` for its layout).
    let (left, max) = unsafe {
        let module = std::ptr::read_unaligned(container.add(RESIST_AT) as *const usize);
        if module == 0 || module % std::mem::align_of::<i32>() != 0 {
            return Vec::new();
        }
        let read = |at: usize| -> [i32; 7] { std::ptr::read_unaligned((module + at) as *const [i32; 7]) };
        (read(RESIST_LEFT), read(RESIST_MAX))
    };
    buildup_of(left, max)
}

/// The statuses with a usable maximum, that are not immune and have started to build up.
fn buildup_of(left: [i32; 7], max: [i32; 7]) -> Vec<Buildup> {
    (0..7)
        .filter(|&kind| max[kind] > 0 && max[kind] < IMMUNE)
        .map(|kind| Buildup { kind, built: (max[kind] - left[kind]).clamp(0, max[kind]), max: max[kind] })
        .filter(|buildup| buildup.built > 0)
        .collect()
}

/// Where `chr` is relative to the player, in the world's X and Z.
fn offset_from(player: &PlayerIns, chr: &ChrIns) -> [f32; 2] {
    let origin = player.chr_ins.chr_ctrl.physics_model_matrix.3;
    let at = chr.chr_ctrl.physics_model_matrix.3;
    [at.0 - origin.0, at.2 - origin.2]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buildup_is_what_was_taken_off_the_maximum() {
        let found = buildup_of([300, 250, 0, 0, 0, 0, 0], [300, 300, 0, 0, 0, 0, 0]);
        // Poison has not started; scarlet rot has taken 50 of 300; the others have no maximum.
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].kind, found[0].built, found[0].max), (1, 50, 300));
    }

    #[test]
    fn immune_statuses_and_stale_values_are_left_out() {
        // Bleed is immune (a maximum of 999 or more); sleep reads above its maximum (never negative).
        let found = buildup_of([0, 0, 0, 0, 0, 400, 0], [0, 0, 999, 0, 0, 300, 0]);
        assert!(found.is_empty());
        let full = buildup_of([0, 0, 0, 0, 0, 0, 0], [100, 0, 0, 0, 0, 0, 0]);
        assert_eq!((full[0].built, full[0].max), (100, 100));
    }
}
