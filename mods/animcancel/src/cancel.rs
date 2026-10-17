//! Lets guard, roll and weapon swap cancel the animation being played.
//!
//! How the game decides (docs/animation_cancel.md, IDA 2.7.1.0): the
//! player's `CSChrActionRequestModule` runs in queue mode. Each frame the
//! game turns the buttons pressed into `queued` (press AND
//! `possible_action_inputs`, minus `disabled_action_inputs`), then fills
//! `input_entries[id]` with `queued AND cancel_entries[id]` - `id` being the
//! TAE id of the animation, which equals its anim id. The HKS reads
//! `input_entries[id]` to start the action, and `cancel_entries[id]` is what
//! the animation's own events open, a few frames at a time (a roll becomes
//! possible ~30% into a sword attack). So the mod only has to open
//! `cancel_entries[id]` earlier; `input_entries` is the list of requests and
//! must never be written (it made every action fire at once in the first
//! test).
//!
//! Only the player. Which animation counts is decided by the queue's own id,
//! not by `CSChrTimeActModule.anim_queue`: while a spell is cast on the move
//! that queue shows the legs' layer (`a000_020010`, group Other) for a
//! stretch although the queue id stays the spell's (2026-10-12 log), so
//! going by the anim id left the spell uncancelable for that stretch. The id
//! trails the animation by a few frames at the start of a new one, which
//! only means the previous animation is still the one being cancelled.
//! Written twice a frame, at `ChrIns_PreBehaviorSafe` (what the game sees this
//! frame) and `ChrIns_BehaviorSafe` (next frame), the combination tested in
//! game.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use eldenring::cs::{ActionQueueEntry, CSChrActionRequestModule, CSTaskGroupIndex, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::anim::{group_of, Group};
use common::logger;

use crate::config::{self, Action, Config};

/// `ChrActions` bit numbers (fromsoftware-rs `action_request.rs`).
const L1: u32 = 2;
const ACTION: u32 = 4;
const SP_MOVE: u32 = 5;
const CHANGE_WEAPON_R: u32 = 9;
const CHANGE_WEAPON_L: u32 = 10;
const BACKSTEP: u32 = 16;
const ROLLING: u32 = 17;
/// Not named in fromsoftware-rs; the game opens it together with the roll
/// cancel (2026-10-12 log).
const DODGE_18: u32 = 18;
const GUARD: u32 = 24;
const EMERGENCY_STEP: u32 = 25;
/// Queue-only bit (not a real action): the HKS weapon swap also asks for
/// `cancel_entries[id]` bit 37 (`sub_140407CC0`). Without it the swap press
/// was ready from t=0.28 of an attack but only ran at 0.83.
const SLOT_SWITCH: u32 = 37;

/// `TaeCancelFlags` bits the game sets for a few frames (fromsoftware-rs).
const TAE_SLOT_SWITCH: u32 = 1 << 3;
const TAE_ACTION_GENERAL: u32 = 1 << 9;
const TAE_CANCEL_DISABLE: u32 = 1 << 11;

/// What the config allows to cancel: action bits, plus the `tae_cancels`
/// flags that go with them.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Allow {
    pub actions: u64,
    pub tae: u32,
}

fn bits(list: &[u32]) -> u64 {
    list.iter().fold(0, |mask, bit| mask | 1 << bit)
}

/// Whether `action` may cancel an animation of this group. Item is every use
/// of an item (flasks, throwables, consumables), except the whistle, see
/// `group_of`. Critical hits (the attacker's half of a throw) are never in
/// `Attack`, see `group_of`.
pub fn source_allowed(group: Group, action: &Action) -> bool {
    match group {
        Group::Attack => action.attack,
        Group::Skill => action.skill,
        Group::Cast => action.cast,
        Group::Item => action.item,
        _ => false,
    }
}

/// The mask for an animation of `group`: the actions that are enabled and may
/// cancel that group. `guard`: L1 really guards with what is held (one weapon
/// in both hands, or a shield); otherwise it may be an attack and cancels nothing.
pub fn allow_from(config: &Config, group: Group, guard: bool) -> Allow {
    let on = |action: &Action| action.enabled && source_allowed(group, action);
    let mut allow = Allow::default();
    if guard && on(&config.guard) {
        // Guard is L1 in game; ACTION_ARM_GUARD (24) alone queued no press.
        allow.actions |= bits(&[GUARD, L1]);
    }
    if on(&config.dodge) {
        allow.actions |= bits(&[SP_MOVE, ROLLING, BACKSTEP, DODGE_18, EMERGENCY_STEP]);
    }
    if on(&config.weapon_swap) {
        // ACTION (4) too: on keyboard the swap key is the pouch/swap button.
        allow.actions |= bits(&[ACTION, CHANGE_WEAPON_R, CHANGE_WEAPON_L, SLOT_SWITCH]);
        allow.tae |= TAE_SLOT_SWITCH;
    }
    if allow.actions != 0 {
        allow.tae |= TAE_ACTION_GENERAL;
    }
    allow
}

/// The group of animation `id` for cancelling: `group_of`, except that every
/// spell animation (TAE `a400`-`a599`) is a cast. `group_of` files the beam
/// phase of Placidusax's Ruin (`a451_045110`, ~9 s) under Other, because
/// SpeedMultiplier must not speed up an animation the beam is timed to; that
/// is no reason not to cancel it (2026-10-12 log: rolls pressed all through
/// the beam did nothing).
pub fn group_for_cancel(id: i32) -> Group {
    if (400..600).contains(&(id / 1_000_000)) {
        return Group::Cast;
    }
    group_of(id)
}

/// The ChrActions / TaeCancelFlags bitfield structs keep their value in a
/// private field, so read and write it through a pointer (they are plain
/// `u64` / `u32` newtypes).
pub fn read<T, U: Copy>(value: &T) -> U {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<U>());
    unsafe { (value as *const T as *const U).read() }
}

fn write<T, U>(value: &mut T, new: U) {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<U>());
    unsafe { (value as *mut T as *mut U).write(new) }
}

/// TAE id the queue entries are looked up by (`queue_tae_id_override` wins).
pub fn queue_id(request: &CSChrActionRequestModule) -> i32 {
    if request.queue_tae_id_override >= 0 {
        request.queue_tae_id_override
    } else {
        request.action_request_queue.current_tae_id
    }
}

/// The local player's `ChrIns`, only once actually in game (see
/// `common::player::main_player_chr_ins_ptr`).
pub fn player<'a>() -> Option<&'a mut ChrIns> {
    common::player::main_player_chr_ins_ptr()?;
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    world_chr_man.main_player.as_mut().map(|p| &mut p.chr_ins)
}

/// What the last tick did, for the probe's log: [NOT_FORCED] (nothing to do:
/// no action allowed, or the animation isn't one that may be cancelled) or
/// [FORCED].
static LAST: AtomicU8 = AtomicU8::new(NOT_FORCED);
pub const NOT_FORCED: u8 = 0;
pub const FORCED: u8 = 1;

pub fn last_state() -> u8 {
    LAST.load(Ordering::Relaxed)
}

/// Opens the cancel windows of the animation with TAE id `id` for `allow`.
fn open_windows(request: &mut CSChrActionRequestModule, id: i32, allow: Allow) {
    let mask = allow.actions;

    // Let the presses in and keep them from being dropped before queuing.
    let inputs: u64 = read(&request.possible_action_inputs);
    write(&mut request.possible_action_inputs, inputs | mask);
    let cancels: u64 = read(&request.possible_action_cancels);
    write(&mut request.possible_action_cancels, cancels | mask);
    let disabled: u64 = read(&request.disabled_action_inputs);
    write(&mut request.disabled_action_inputs, disabled & !mask);

    // The per-animation permission the HKS reads. The entry only exists once
    // an animation event wrote it, so it is created when missing.
    let cancel_entries = &mut request.action_request_queue.cancel_entries;
    if let Some(entry) = cancel_entries.iter_mut().find(|e| e.tae_id == id) {
        let actions: u64 = read(&entry.actions);
        write(&mut entry.actions, actions | mask);
    } else {
        // SAFETY: plain integers; an all-zero entry is valid.
        let mut entry: ActionQueueEntry = unsafe { std::mem::zeroed() };
        entry.tae_id = id;
        write(&mut entry.actions, mask);
        cancel_entries.push_back(entry);
    }

    let tae: u32 = read(&request.tae_cancels);
    write(&mut request.tae_cancels, (tae | allow.tae) & !TAE_CANCEL_DISABLE);
}

/// One tick of one stage.
fn apply() {
    let config = config::get();
    let Some(chr) = player() else {
        return;
    };
    let request = &mut chr.modules.action_request;
    let id = queue_id(request);
    let allow = if id < 0 { Allow::default() } else { allow_from(&config, group_for_cancel(id), crate::weapon::guard_button_guards()) };
    if allow.actions == 0 {
        LAST.store(NOT_FORCED, Ordering::Relaxed);
        return;
    }
    open_windows(request, id, allow);
    LAST.store(FORCED, Ordering::Relaxed);
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    for (tag, stage) in [
        ("Cancel", CSTaskGroupIndex::ChrIns_PreBehaviorSafe),
        ("Cancel", CSTaskGroupIndex::ChrIns_BehaviorSafe),
    ] {
        common::task::run_recurring_safe(cs_task, tag, stage, move |_data: &eldenring::fd4::FD4TaskData| apply());
    }
    logger::log("Cancel: animation cancels active.");
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Action = Action { enabled: true, attack: true, skill: true, cast: true, item: true };
    const OFF: Action = Action { enabled: false, ..ALL };

    fn config(guard: Action, dodge: Action, weapon_swap: Action) -> Config {
        Config { guard, dodge, weapon_swap, ..Config::default() }
    }

    fn only(guard: bool, dodge: bool, weapon_swap: bool) -> Config {
        let pick = |on: bool| if on { ALL } else { OFF };
        config(pick(guard), pick(dodge), pick(weapon_swap))
    }

    #[test]
    fn nothing_allowed_means_no_bits() {
        assert_eq!(allow_from(&only(false, false, false), Group::Attack, true), Allow::default());
    }

    #[test]
    fn guard_includes_l1() {
        let allow = allow_from(&only(true, false, false), Group::Attack, true);
        assert_eq!(allow.actions, 1 << GUARD | 1 << L1);
        assert_eq!(allow.tae, TAE_ACTION_GENERAL);
    }

    /// L1 not guarding (dual wield, twin blades): guard cancels nothing, the rest still does.
    #[test]
    fn guard_cancels_nothing_when_l1_attacks() {
        let config = only(true, true, true);
        let with = allow_from(&config, Group::Attack, true);
        let without = allow_from(&config, Group::Attack, false);
        assert!(with.actions & (1 << GUARD | 1 << L1) != 0);
        assert_eq!(without.actions & (1 << GUARD | 1 << L1), 0);
        assert!(without.actions & 1 << ROLLING != 0);
    }

    #[test]
    fn dodge_includes_the_roll_and_step_bits() {
        let allow = allow_from(&only(false, true, false), Group::Attack, true);
        for bit in [SP_MOVE, ROLLING, BACKSTEP, DODGE_18] {
            assert!(allow.actions & 1 << bit != 0, "bit {bit}");
        }
        assert_eq!(allow.actions & 1 << CHANGE_WEAPON_R, 0);
    }

    #[test]
    fn weapon_swap_needs_the_queue_bit_and_the_slot_flag() {
        let allow = allow_from(&only(false, false, true), Group::Attack, true);
        assert!(allow.actions & 1 << SLOT_SWITCH != 0);
        assert!(allow.tae & TAE_SLOT_SWITCH != 0);
        // The others don't get the slot flag.
        assert_eq!(allow_from(&only(true, true, false), Group::Attack, true).tae & TAE_SLOT_SWITCH, 0);
    }

    #[test]
    fn only_enabled_animation_groups_are_cancelled() {
        let action = Action { attack: true, skill: false, cast: false, item: false, ..ALL };
        assert!(source_allowed(Group::Attack, &action));
        assert!(!source_allowed(Group::Skill, &action));
        assert!(!source_allowed(Group::Cast, &action));
        assert!(!source_allowed(Group::Item, &action));
        let items = Action { item: true, ..action };
        assert!(source_allowed(Group::Item, &items));
        // Never: a critical hit, rolls, walking.
        for group in [Group::Critical, Group::Roll, Group::Walk, Group::Run, Group::Other] {
            assert!(!source_allowed(group, &ALL), "{group:?}");
        }
    }

    /// The reported case: guard must not cut a spell, but still cuts the rest,
    /// while dodge and weapon swap cut spells too.
    #[test]
    fn each_action_has_its_own_animation_set() {
        let no_cast = Action { cast: false, ..ALL };
        let config = config(no_cast, ALL, ALL);
        let guard = 1 << GUARD;
        assert_eq!(allow_from(&config, Group::Cast, true).actions & guard, 0);
        assert!(allow_from(&config, Group::Cast, true).actions & 1 << ROLLING != 0);
        assert!(allow_from(&config, Group::Cast, true).actions & 1 << SLOT_SWITCH != 0);
        for group in [Group::Attack, Group::Skill, Group::Item] {
            assert!(allow_from(&config, group, true).actions & guard != 0, "{group:?}");
        }
        // An action that is off cancels nothing, whatever its set says.
        let off = self::config(OFF, ALL, ALL);
        assert_eq!(allow_from(&off, Group::Attack, true).actions & guard, 0);
    }

    #[test]
    fn every_spell_animation_is_a_cast() {
        assert_eq!(group_for_cancel(405_045_100), Group::Cast); // a405_045100
        assert_eq!(group_for_cancel(451_045_100), Group::Cast); // Placidusax's Ruin, wind-up
        assert_eq!(group_for_cancel(451_045_110), Group::Cast); // ... and its beam
        assert_eq!(group_of(451_045_110), Group::Other); // (group_of leaves the beam out)
        assert_eq!(group_for_cancel(885_040_000), Group::Skill); // Carian Sovereignty
        assert_eq!(group_for_cancel(20_032_000), Group::Attack);
        assert_eq!(group_for_cancel(22_100), Group::Walk);
    }

    #[test]
    fn group_of_sorts_an_attack_anim_into_attack() {
        assert_eq!(group_of(20_032_000), Group::Attack); // a020_032000
        assert_eq!(group_of(23_030_000), Group::Attack); // a023_030000
        assert_eq!(group_of(23_031_710), Group::Critical); // backstab
    }
}
