//! The "is the player actually in the game world" gate. A feature touching a
//! singleton whose data isn't populated until a real player character exists
//! (`GameDataMan`'s save data, `SoloParamRepository`'s param tables) must gate
//! on this, not on the singleton's `instance()` returning `Ok`, which only
//! means the manager object exists (already at the title screen).

use std::time::Duration;

use eldenring::cs::{SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;

/// The main player's `ChrIns` address if currently resolved, `None` otherwise
/// (title screen, loading, no save loaded, ...).
pub fn main_player_chr_ins_ptr() -> Option<*const u8> {
    // SAFETY: called from game tasks; `instance` returns `Err` while the singleton does not exist.
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    world_chr_man.main_player.as_ref().map(|p| &p.chr_ins as *const _ as *const u8)
}

/// Waits, forever like [`crate::task::wait_for_cs_task`], for
/// `SoloParamRepository` (the live regulation.bin) AND for the player to be in
/// the game world. `SoloParamRepository::instance_mut()` alone can return `Ok`
/// while the param tables are still empty (title/loading screen), the same
/// premature-singleton trap [main_player_chr_ins_ptr] guards against.
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
