//! Game-side reader of the locked-on enemy. Runs as a frame task and copies
//! what the panel shows into [SNAPSHOT], so the render thread never touches
//! live game objects. Details: `docs/design.md`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use eldenring::cs::{
    CSChrDataModule, CSChrSuperArmorModule, CSTaskGroupIndex, ChrIns, ChrInsModuleContainer, ChrSet, FieldInsHandle,
    PlayerIns, SpecialEffectEntry, WorldChrMan,
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
    // The effect list: `special_effect` of the chr, entries at 0x78 bytes with the time left at
    // +0x44, the float after `removal_timer` (`docs/effects.md`).
    assert!(std::mem::offset_of!(ChrIns, special_effect) == 0x178);
    assert!(std::mem::offset_of!(SpecialEffectEntry, removal_timer) == 0x40);
    assert!(std::mem::size_of::<SpecialEffectEntry>() == 0x78);
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

/// One active SpEffect the panel shows: one that has an icon, or the proc of a status buildup.
#[derive(Clone, Copy, PartialEq)]
pub struct Effect {
    /// The `SpEffectParam` row.
    pub param_id: i32,
    /// `SpEffectParam.iconId`: the sprite `MENU_StatusIcon_<icon>`; 0 or less for a proc.
    pub icon: i32,
    /// The status whose buildup proc this is (an index into [BUILDUP]), drawn with that status'
    /// icon; the params give these effects no icon.
    pub kind: Option<usize>,
    /// Seconds left; only meaningful when not [permanent](Effect::permanent).
    pub left: f32,
    /// The effect has no end (`effectEndurance` below zero): its cell shows no time.
    pub permanent: bool,
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
    /// The effects it has that show an icon, in the order the game keeps them.
    pub effects: Vec<Effect>,
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

/// The player's own effects and when they were read, kept like [SNAPSHOT].
static OWN: Mutex<Option<(Vec<Effect>, Instant)>> = Mutex::new(None);

/// The player's effects (the ones the game shows no timer for), `[]` when stale.
pub fn own_effects_now() -> Vec<Effect> {
    let Ok(guard) = OWN.lock() else { return Vec::new() };
    match guard.as_ref() {
        Some((effects, at)) if at.elapsed() < STALE_AFTER => effects.clone(),
        _ => Vec::new(),
    }
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    let mut last = None;
    let mut last_kinds: Vec<usize> = Vec::new();
    let mut last_effects: Vec<i32> = Vec::new();
    let mut last_own: Vec<i32> = Vec::new();
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
        // One line when the set of effects changes: which ones show up, for `docs/effects.md`.
        let effects: Vec<i32> = read.iter().flat_map(|t| t.effects.iter().map(|e| e.param_id)).collect();
        if effects != last_effects {
            if let Some(target) = &read {
                logger::log(&format!("Effects: enemy {}", describe(&target.effects)));
            }
            last_effects = effects;
        }
        let own = own_effects();
        let ids: Vec<i32> = own.iter().map(|e| e.param_id).collect();
        if ids != last_own {
            logger::log(&format!("Effects: player {}", describe(&own)));
            last_own = ids;
        }
        if let Ok(mut guard) = OWN.lock() {
            *guard = Some((shown_effects(own, MAX_EFFECTS), Instant::now()));
        }
        if let Ok(mut guard) = SNAPSHOT.lock() {
            *guard = read.map(|target| (target, Instant::now()));
        }
    });
}

/// At most `max` of `effects`, in the same order: when there are more, the ones with the
/// most time left (the permanent ones first) are dropped.
pub fn shown_effects(mut effects: Vec<Effect>, max: usize) -> Vec<Effect> {
    if effects.len() > max {
        let mut by_time: Vec<usize> = (0..effects.len()).collect();
        let time = |e: &Effect| if e.permanent { f32::INFINITY } else { e.left };
        by_time.sort_by(|&a, &b| time(&effects[a]).total_cmp(&time(&effects[b])));
        let mut keep = vec![false; effects.len()];
        by_time.iter().take(max).for_each(|&i| keep[i] = true);
        let mut at = 0;
        effects.retain(|_| {
            at += 1;
            keep[at - 1]
        });
    }
    effects
}

/// `id icon time` for each effect, for the log.
fn describe(effects: &[Effect]) -> String {
    if effects.is_empty() {
        return "none".to_string();
    }
    let shown: Vec<String> = effects
        .iter()
        .take(40)
        .map(|e| if e.permanent { format!("{} icon {} permanent", e.param_id, e.icon) } else { format!("{} icon {} {:.0}s", e.param_id, e.icon, e.left) })
        .collect();
    shown.join(", ")
}

/// The effects of the player, `[]` when out of the world: the ones with an icon (the game's row
/// shows them, the times go under it) and the procs of the status buildups (which have no icon of
/// the game's: the mod draws them after its row, `ui::draw_icon_times`).
fn own_effects() -> Vec<Effect> {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else { return Vec::new() };
    world_chr_man
        .main_player
        .as_ref()
        .map_or_else(Vec::new, |player| read_effects(&player.chr_ins))
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
        effects: shown_effects(read_effects(chr), MAX_EFFECTS),
    })
}

/// The status a buildup proc effect belongs to, by the `stateInfo` of its `SpEffectParam` row
/// (an index into [BUILDUP]): the game sets it on every effect that afflicts a status, whatever
/// the row (an enemy's hit, a swamp, an item), where a run of row ids would miss most of them.
/// The effects that cure a status have other values and no time. Statuses whose effect is instant
/// (blood loss, madness, death blight) only show when a row lasts longer than a second.
fn proc_kind(state_info: u16) -> Option<usize> {
    match state_info {
        2 => Some(0),
        5 => Some(1),
        6 => Some(2),
        116 => Some(3),
        260 => Some(4),
        436 => Some(5),
        437 => Some(6),
        _ => None,
    }
}

/// The most effects kept in the snapshot: two rows of six. The panel shows fewer when its cells do not fit.
const MAX_EFFECTS: usize = 12;

/// The effects of `chr` that show an icon (`iconId` above 0) or are the proc of a status buildup,
/// and either have no end or last longer than a second, with the time they have left. Rows with a tenth of a second exist only to re-apply a buff in cycles
/// (Golden Vow's "Cycled" rows) and keep their icon: they would be a second, empty cell.
fn read_effects(chr: &ChrIns) -> Vec<Effect> {
    chr.special_effect
        .entries()
        .filter_map(|entry| {
            // SAFETY: `param_data` is the live param row of the entry, `SpEffectParam`'s `iconId` is
            // its first field.
            let row = unsafe { entry.param_data?.as_ref() };
            let (icon, state_info) = (row.icon_id(), row.state_info());
            // SAFETY: the entry is 0x78 bytes (asserted above); +0x44 is the time left.
            let left = unsafe { std::ptr::read_unaligned((entry as *const SpecialEffectEntry as *const u8).add(0x44) as *const f32) };
            let permanent = entry.duration < 0.0;
            // An effect with an icon is drawn with it; the proc of a status has none.
            let kind = if icon > 0 { None } else { proc_kind(state_info) };
            ((icon > 0 || kind.is_some()) && (permanent || entry.duration > 1.0)).then_some(Effect {
                param_id: entry.param_id,
                icon: icon as i32,
                kind,
                left: if left.is_finite() { left } else { 0.0 },
                permanent,
            })
        })
        .collect()
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
    fn effect(param_id: i32, left: f32, permanent: bool) -> Effect {
        Effect { param_id, icon: 1, kind: None, left, permanent }
    }

    #[test]
    fn state_info_tells_which_status_an_effect_afflicts() {
        assert_eq!((proc_kind(2), proc_kind(5), proc_kind(6), proc_kind(116)), (Some(0), Some(1), Some(2), Some(3)));
        assert_eq!((proc_kind(260), proc_kind(436), proc_kind(437)), (Some(4), Some(5), Some(6)));
        // The cures (10, 11, 12...) and the game's own marker (275) are not afflictions.
        assert_eq!((proc_kind(0), proc_kind(10), proc_kind(11), proc_kind(275)), (None, None, None, None));
    }

    #[test]
    fn more_than_twelve_effects_drop_the_ones_with_most_time_left() {
        let mut all: Vec<Effect> = (0..12).map(|i| effect(i, 10.0 + i as f32, false)).collect();
        all.push(effect(100, 0.0, true));
        all.push(effect(101, 3.0, false));
        let shown = shown_effects(all, MAX_EFFECTS);
        let ids: Vec<i32> = shown.iter().map(|e| e.param_id).collect();
        // The permanent one and the one with 21 s left (the longest timed) go; the order stays.
        assert_eq!(shown.len(), MAX_EFFECTS);
        assert!(!ids.contains(&100) && !ids.contains(&11));
        assert_eq!(ids, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 101]);
    }

    #[test]
    fn up_to_twelve_effects_are_all_shown() {
        let all: Vec<Effect> = (0..12).map(|i| effect(i, 5.0, false)).collect();
        assert_eq!(shown_effects(all, MAX_EFFECTS).len(), 12);
    }

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
