//! Lets guard, dodge (roll, backstep) and weapon swap cancel the animation being played.
//!
//! The game decides in `CSChrActionRequestModule`: each frame it turns the
//! pressed buttons into `queued` (press AND `possible_action_inputs`, minus
//! `disabled_action_inputs`), then fills `input_entries[id]` with
//! `queued AND cancel_entries[id]`, `id` being the animation's TAE id. The
//! HKS reads `input_entries[id]` to start the action, and `cancel_entries[id]`
//! is what the animation's own events open, a few frames at a time. So the mod
//! only opens `cancel_entries[id]` earlier; `input_entries` is the list of
//! requests and must never be written.
//!
//! Split in two: [plan] decides what may cancel (pure, tested), [open] writes
//! it into the module through the typed bitfields of `fromsoftware-rs`.
//! Written twice a frame, at `ChrIns_PreBehaviorSafe` (what the game sees this
//! frame) and `ChrIns_BehaviorSafe` (next frame).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use bitfield::{Bit, BitMut, BitRangeMut};
use eldenring::cs::{ActionQueueEntry, CSChrActionRequestModule, CSTaskGroupIndex, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::anim::{group_of, Group};
use common::logger;

use crate::config::{self, Action, Config};

/// `ChrActions` bit numbers.
const L1: usize = 2;
const ACTION: usize = 4;
const SP_MOVE: usize = 5;
/// ACTION_ARM_CHANGE_STYLE, repurposed to the jump button in Elden Ring.
const JUMP: usize = 6;
const CHANGE_WEAPON_R: usize = 9;
const CHANGE_WEAPON_L: usize = 10;
const BACKSTEP: usize = 16;
const ROLLING: usize = 17;
/// Not named in fromsoftware-rs; the game opens it together with the roll cancel.
const DODGE_18: usize = 18;
const GUARD: usize = 24;
const EMERGENCY_STEP: usize = 25;
/// Queue-only bit (not a real action): the HKS weapon swap also asks for
/// `cancel_entries[id]` bit 37. Without it the swap press was ready from
/// t=0.28 of an attack but only ran at 0.83.
const SLOT_SWITCH: usize = 37;

/// `TaeCancelFlags` bits.
const TAE_SLOT_SWITCH: usize = 3;
const TAE_ACTION_GENERAL: usize = 9;
const TAE_CANCEL_DISABLE: usize = 11;

/// Guard is L1 in game; `GUARD` alone queued no press.
const GUARD_BITS: [usize; 2] = [GUARD, L1];
const JUMP_BITS: [usize; 1] = [JUMP];
const DODGE_BITS: [usize; 5] = [SP_MOVE, ROLLING, BACKSTEP, DODGE_18, EMERGENCY_STEP];
/// ACTION too: on keyboard the swap key is the pouch/swap button.
const WEAPON_SWAP_BITS: [usize; 4] = [ACTION, CHANGE_WEAPON_R, CHANGE_WEAPON_L, SLOT_SWITCH];

/// What may cancel: action bits for the queue entries, plus the `tae_cancels`
/// flags that go with them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Allow {
    pub actions: Vec<usize>,
    pub tae: Vec<usize>,
}

impl Allow {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Whether `action` may cancel an animation of this group.
pub fn source_allowed(group: Group, action: &Action) -> bool {
    match group {
        Group::Attack => action.attack,
        Group::Skill => action.skill,
        Group::Cast => action.cast,
        Group::Item => action.item,
        _ => false,
    }
}

/// What may cancel an animation of `group`. `guard_ok`: L1 really guards with
/// what is held and was not already held when the animation began (see
/// [GuardTracker]); otherwise it may be an attack and cancels nothing.
pub fn plan(config: &Config, group: Group, guard_ok: bool) -> Allow {
    let on = |action: &Action| action.enabled && source_allowed(group, action);
    let mut allow = Allow::default();
    if guard_ok && on(&config.guard) {
        allow.actions.extend(GUARD_BITS);
    }
    if on(&config.dodge) {
        allow.actions.extend(DODGE_BITS);
    }
    if on(&config.jump) {
        allow.actions.extend(JUMP_BITS);
    }
    if on(&config.weapon_swap) {
        allow.actions.extend(WEAPON_SWAP_BITS);
        allow.tae.push(TAE_SLOT_SWITCH);
    }
    if !allow.actions.is_empty() {
        allow.tae.push(TAE_ACTION_GENERAL);
    }
    allow
}

/// The group of animation `id` for cancelling: `group_of`, except that every
/// spell animation (TAE `a400`-`a599`) is a cast, so a beam phase that
/// `group_of` files under Other can still be cancelled.
pub fn group_for_cancel(id: i32) -> Group {
    if (400..600).contains(&(id / 1_000_000)) {
        return Group::Cast;
    }
    group_of(id)
}

/// Guard (L1) held as the animation began is no new press, and letting it
/// cancel cuts the attack the player just started from a raised guard at once
/// (hold guard, press attack: the attack was cut and the guard raised again).
/// A new animation starts blocked when guard is already held; the block ends
/// when the guard is let go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GuardTracker {
    anim: i32,
    blocked: bool,
}

impl GuardTracker {
    const NEW: GuardTracker = GuardTracker { anim: -1, blocked: false };

    /// Feeds one tick (the animation `id` playing, whether guard is `held`)
    /// and says whether guard is blocked from cancelling it.
    fn update(&mut self, id: i32, held: bool) -> bool {
        self.blocked = if self.anim == id { self.blocked && held } else { held };
        self.anim = id;
        self.blocked
    }
}

static GUARD_TRACKER: Mutex<GuardTracker> = Mutex::new(GuardTracker::NEW);

/// Whether guard is blocked from cancelling the animation now playing (for the probe's log).
pub fn guard_blocked() -> bool {
    GUARD_TRACKER.lock().unwrap_or_else(|e| e.into_inner()).blocked
}

/// Whether the last tick opened any cancel window (for the probe's log).
static FORCED: AtomicBool = AtomicBool::new(false);

pub fn last_forced() -> bool {
    FORCED.load(Ordering::Relaxed)
}

/// TAE id the queue entries are looked up by (`queue_tae_id_override` wins).
pub fn queue_id(request: &CSChrActionRequestModule) -> i32 {
    if request.queue_tae_id_override >= 0 {
        request.queue_tae_id_override
    } else {
        request.action_request_queue.current_tae_id
    }
}

/// The local player's `ChrIns`, only once actually in game.
pub fn player<'a>() -> Option<&'a mut ChrIns> {
    common::player::main_player_chr_ins_ptr()?;
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    world_chr_man.main_player.as_mut().map(|p| &mut p.chr_ins)
}

/// Opens the cancel windows of the animation with TAE id `id` for `allow`.
fn open(request: &mut CSChrActionRequestModule, id: i32, allow: &Allow) {
    let bits = &allow.actions;
    for &bit in bits {
        // Let the presses in and keep them from being dropped before queuing.
        request.possible_action_inputs.set_bit(bit, true);
        request.possible_action_cancels.set_bit(bit, true);
        request.disabled_action_inputs.set_bit(bit, false);
    }

    // The per-animation permission the HKS reads. The entry only exists once
    // an animation event wrote it, so it is created when missing.
    let entries = &mut request.action_request_queue.cancel_entries;
    if let Some(entry) = entries.iter_mut().find(|e| e.tae_id == id) {
        for &bit in bits {
            entry.actions.set_bit(bit, true);
        }
    } else {
        // An empty action set: any bitfield value of the module, all bits cleared.
        let mut actions = request.disabled_action_inputs;
        actions.set_bit_range(63, 0, 0u64);
        for &bit in bits {
            actions.set_bit(bit, true);
        }
        entries.push_back(ActionQueueEntry { tae_id: id, actions });
    }

    for &bit in &allow.tae {
        request.tae_cancels.set_bit(bit, true);
    }
    request.tae_cancels.set_bit(TAE_CANCEL_DISABLE, false);
}

/// One tick of one stage.
fn apply() {
    let config = config::get();
    let Some(chr) = player() else {
        return;
    };
    let request = &mut chr.modules.action_request;
    let id = queue_id(request);
    let held = request.action_requests.bit(GUARD) || request.action_requests.bit(L1);
    let blocked = GUARD_TRACKER.lock().unwrap_or_else(|e| e.into_inner()).update(id, held);
    if id < 0 {
        FORCED.store(false, Ordering::Relaxed);
        return;
    }
    let guard_ok = crate::weapon::guard_button_guards() && !blocked;
    let allow = plan(&config, group_for_cancel(id), guard_ok);
    FORCED.store(!allow.is_empty(), Ordering::Relaxed);
    if !allow.is_empty() {
        open(request, id, &allow);
    }
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();
    for stage in [CSTaskGroupIndex::ChrIns_PreBehaviorSafe, CSTaskGroupIndex::ChrIns_BehaviorSafe] {
        common::task::run_recurring_safe(cs_task, "Cancel", stage, move |_data: &eldenring::fd4::FD4TaskData| apply());
    }
    logger::log("Cancel: active");
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Action = Action { enabled: true, attack: true, skill: true, cast: true, item: true };
    const OFF: Action = Action { enabled: false, ..ALL };

    fn config(dodge: Action) -> Config {
        Config { dodge, ..Config::default() }
    }

    fn only(guard: bool, dodge: bool, weapon_swap: bool) -> Config {
        let pick = |on: bool| if on { ALL } else { OFF };
        Config { guard: pick(guard), dodge: pick(dodge), weapon_swap: pick(weapon_swap), ..Config::default() }
    }

    #[test]
    fn dodge_off_cancels_nothing() {
        assert!(plan(&config(OFF), Group::Attack, true).is_empty());
    }

    #[test]
    fn dodge_opens_the_roll_and_step_bits() {
        let allow = plan(&config(ALL), Group::Attack, true);
        for bit in DODGE_BITS {
            assert!(allow.actions.contains(&bit), "bit {bit}");
        }
        assert!(!allow.actions.contains(&CHANGE_WEAPON_R));
    }

    #[test]
    fn weapon_swap_needs_the_queue_bit_and_the_slot_flag() {
        let allow = plan(&only(false, false, true), Group::Attack, true);
        assert!(allow.actions.contains(&SLOT_SWITCH));
        assert!(allow.tae.contains(&TAE_SLOT_SWITCH));
        // The others don't get the slot flag.
        assert!(!plan(&only(false, true, false), Group::Attack, true).tae.contains(&TAE_SLOT_SWITCH));
    }

    #[test]
    fn any_allowed_action_sets_the_general_flag() {
        assert!(plan(&only(false, true, false), Group::Attack, true).tae.contains(&TAE_ACTION_GENERAL));
        assert!(plan(&only(false, false, false), Group::Attack, true).tae.is_empty());
    }

    #[test]
    fn only_enabled_animation_groups_are_cancelled() {
        let dodge = Action { cast: false, ..ALL };
        assert!(plan(&config(dodge), Group::Cast, true).is_empty());
        assert!(!plan(&config(dodge), Group::Attack, true).is_empty());
        // Never: a critical hit, rolls, walking.
        for group in [Group::Critical, Group::Roll, Group::Walk, Group::Run, Group::Other] {
            assert!(plan(&config(ALL), group, true).is_empty(), "{group:?}");
        }
    }

    #[test]
    fn jump_opens_only_the_jump_bit() {
        let jump = Config { jump: ALL, ..Config::default() };
        let allow = plan(&jump, Group::Attack, true);
        assert_eq!(allow.actions, vec![JUMP]);
        assert!(allow.tae.contains(&TAE_ACTION_GENERAL));
        assert!(!allow.tae.contains(&TAE_SLOT_SWITCH));
        // Off, or an animation group it may not cancel: nothing.
        assert!(plan(&Config { jump: OFF, ..Config::default() }, Group::Attack, true).is_empty());
        let no_cast = Config { jump: Action { cast: false, ..ALL }, ..Config::default() };
        assert!(plan(&no_cast, Group::Cast, true).is_empty());
        // The others never open the jump bit.
        assert!(!plan(&only(true, true, true), Group::Attack, true).actions.contains(&JUMP));
    }

    #[test]
    fn guard_includes_l1() {
        let allow = plan(&only(true, false, false), Group::Attack, true);
        assert!(allow.actions.contains(&GUARD) && allow.actions.contains(&L1));
        assert!(allow.tae.contains(&TAE_ACTION_GENERAL));
    }

    /// L1 not guarding (dual wield, twin blades, or held before the attack):
    /// guard cancels nothing, the rest still does.
    #[test]
    fn guard_cancels_nothing_when_l1_does_not_guard() {
        let config = only(true, true, true);
        let with = plan(&config, Group::Attack, true);
        let without = plan(&config, Group::Attack, false);
        assert!(with.actions.contains(&GUARD) && with.actions.contains(&L1));
        assert!(!without.actions.contains(&GUARD) && !without.actions.contains(&L1));
        assert!(without.actions.contains(&ROLLING));
    }

    #[test]
    fn guard_held_before_the_attack_does_not_cancel_it() {
        let mut tracker = GuardTracker::NEW;
        // Guard held, attack starts: blocked while the guard stays held.
        assert!(tracker.update(100, true));
        assert!(tracker.update(100, true));
        // Let go: no longer blocked; a new press during the attack cancels.
        assert!(!tracker.update(100, false));
        assert!(!tracker.update(100, true));
        // Attack started without guard: a guard pressed later cancels it.
        assert!(!tracker.update(200, false));
        assert!(!tracker.update(200, true));
        // Held across into the next animation: blocked again.
        assert!(tracker.update(300, true));
    }

    #[test]
    fn every_spell_animation_is_a_cast() {
        assert_eq!(group_for_cancel(405_045_100), Group::Cast);
        assert_eq!(group_for_cancel(451_045_110), Group::Cast); // Placidusax's Ruin beam
        assert_eq!(group_for_cancel(885_040_000), Group::Skill);
        assert_eq!(group_for_cancel(20_032_000), Group::Attack);
        assert_eq!(group_for_cancel(22_100), Group::Walk);
    }
}
