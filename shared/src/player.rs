//! The "is the player in the game world" gate. A singleton's `instance()` returning `Ok` only means
//! the manager exists (already at the title screen); features that need real player data must
//! wait for this instead.

use std::time::Duration;

use eldenring::cs::{ChrIns, ChrLoadStatus, ChrSet, SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;

/// The main player's `ChrIns` address, `None` at the title screen or while loading.
pub fn main_player_chr_ins_ptr() -> Option<*const u8> {
    // SAFETY: called from game tasks; `instance` returns `Err` while the singleton does not exist.
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    world_chr_man.main_player.as_ref().map(|p| &p.chr_ins as *const _ as *const u8)
}

/// Waits for `SoloParamRepository` and for the player to be in the world: `instance_mut()` alone
/// can be `Ok` while the param tables are still empty.
pub fn wait_for_solo_param_repository() -> &'static mut SoloParamRepository {
    let mut logged = false;
    loop {
        if main_player_chr_ins_ptr().is_some()
            // SAFETY: called from a worker that waits for the world; `instance_mut` returns `Err` while the singleton does not exist.
            && let Ok(repo) = unsafe { SoloParamRepository::instance_mut() }
        {
            return repo;
        }
        // Logged once: picking a save and loading in can take minutes.
        if !logged {
            logged = true;
            crate::logger::log("SoloParamRepository: waiting for world");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Calls `f` on every character of `set` that is safe to touch. Only `Active` and
/// `ReadyForActivation` entries are dereferenced: any other entry can be stale.
pub fn for_each_active_chr(set: &ChrSet<ChrIns>, mut f: impl FnMut(&mut ChrIns)) {
    for slot in 0..set.capacity {
        // SAFETY: `slot` is below the set's capacity, so the entry is inside its allocation.
        let entry = unsafe { set.entries.add(slot as usize).as_ref() };
        let Some(mut chr) = entry.chr_ins else {
            continue;
        };
        if !matches!(
            entry.chr_load_status,
            ChrLoadStatus::Active | ChrLoadStatus::ReadyForActivation
        ) {
            continue;
        }
        // SAFETY: the entry is active, so its character is alive for this frame.
        f(unsafe { chr.as_mut() });
    }
}

/// The pad input read each frame from the main player's `CSChrActionRequestModule`.
/// `r1`, `r2`, `l1`, `l2` and `new_gesture` are true for exactly one frame, on the press, and
/// `requested_gesture` only means something on that frame. `busy` is every other way the player is
/// not idle: held actions and movement input.
pub struct ActionSnapshot {
    pub r1: bool,
    pub r2: bool,
    pub l1: bool,
    pub l2: bool,
    pub new_gesture: bool,
    pub requested_gesture: i32,
    pub busy: bool,
}

/// This frame's input of the main player, `None` until the player is resolved.
pub fn main_player_action_snapshot() -> Option<ActionSnapshot> {
    // SAFETY: called from game tasks; `instance` returns `Err` while the singleton does not exist.
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let main_player = world_chr_man.main_player.as_ref()?;
    let action_request = &main_player.chr_ins.modules.action_request;
    let new_presses = &action_request.new_action_presses;
    let held = &action_request.action_requests;
    let busy = action_request.movement_request_flags.raw_input()
        || held.r1()
        || held.r2()
        || held.l1()
        || held.l2()
        || held.sp_move()
        || held.jump()
        || held.use_item()
        || held.action()
        || held.guard()
        || held.rideon()
        || held.rideoff()
        || held.ladderup()
        || held.ladderdown();
    Some(ActionSnapshot {
        r1: new_presses.r1(),
        r2: new_presses.r2(),
        l1: new_presses.l1(),
        l2: new_presses.l2(),
        new_gesture: new_presses.gesture(),
        requested_gesture: action_request.requested_gesture,
        busy,
    })
}
