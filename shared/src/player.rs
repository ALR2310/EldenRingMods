//! The "is the player actually in the game world" gate. A feature touching a
//! singleton whose data isn't populated until a real player character exists
//! (`GameDataMan`'s save data, `SoloParamRepository`'s param tables) must gate
//! on this, not on the singleton's `instance()` returning `Ok`, which only
//! means the manager object exists (already at the title screen). Why, and
//! what went wrong without it: `shared/docs/player.md`.

use std::time::{Duration, Instant};

use eldenring::cs::{SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;

/// The pad-input bits read each frame off the main player's
/// `CSChrActionRequestModule`, for features that classify player input.
///
/// `r1`/`r2`/`l1`/`l2`/`new_gesture` come from `new_action_presses` (true for
/// exactly 1 frame, on the press). `requested_gesture` is only meaningful the
/// frame `new_gesture` is set. `busy` is every other way the player can be "not
/// idle": held actions plus movement input.
pub struct ActionSnapshot {
    pub r1: bool,
    pub r2: bool,
    pub l1: bool,
    pub l2: bool,
    pub new_gesture: bool,
    pub requested_gesture: i32,
    pub busy: bool,
}

/// Reads this frame's action-input snapshot off the main player's
/// `CSChrActionRequestModule`. `None` if not resolved yet.
pub fn main_player_action_snapshot() -> Option<ActionSnapshot> {
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

/// The main player's `ChrIns` address if currently resolved, `None` otherwise
/// (title screen, loading, no save loaded, ...).
pub fn main_player_chr_ins_ptr() -> Option<*const u8> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    world_chr_man
        .main_player
        .as_ref()
        .map(|p| &p.chr_ins as *const _ as *const u8)
}

/// How often to log that we're still waiting. Not a warning: picking a save
/// and loading in can take minutes. It only shows the thread didn't die.
const REMINDER_EVERY: Duration = Duration::from_secs(30);

/// Waits, forever like [`crate::task::wait_for_cs_task`], for
/// `SoloParamRepository` (the live regulation.bin) AND for the player to be in
/// the game world. `SoloParamRepository::instance_mut()` alone can return `Ok`
/// while the param tables are still empty (title/loading screen), the same
/// premature-singleton trap [main_player_chr_ins_ptr] guards against.
pub fn wait_for_solo_param_repository() -> &'static mut SoloParamRepository {
    let mut next_reminder = Instant::now() + REMINDER_EVERY;
    loop {
        if main_player_chr_ins_ptr().is_some()
            && let Ok(repo) = unsafe { SoloParamRepository::instance_mut() }
        {
            return repo;
        }
        if Instant::now() >= next_reminder {
            next_reminder += REMINDER_EVERY;
            crate::logger::log("SoloParamRepository: waiting for world");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
