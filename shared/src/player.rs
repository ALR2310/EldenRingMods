//! The "is the player in the game world" gate. A singleton's `instance()` returning `Ok` only means
//! the manager exists (already at the title screen); features that need real player data must
//! wait for this instead.

use std::time::Duration;

use eldenring::cs::{SoloParamRepository, WorldChrMan};
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
