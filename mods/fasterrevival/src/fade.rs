//! Skips the pause after "YOU DIED": sets `MenuCommonParam[0].soloPlayDeath_ToFadeOutTime`
//! (vanilla 3.8 s) to 0, once. Why this field: `docs/respawn_fade.md`.

use eldenring::cs::{MenuCommonParam, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use shared::logger;

#[derive(Default)]
pub struct FadeState {
    done: bool,
}

/// Writes the param once. Only call it while the main player exists: reading
/// `SoloParamRepository` before that can panic (`shared/docs/player.md`).
pub fn apply(state: &mut FadeState) {
    if state.done {
        return;
    }
    // SAFETY: called from a game task; `instance_mut` returns `Err` while the singleton does not exist.
    let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else { return };
    // Whatever happens below, do not retry every frame.
    state.done = true;
    if let Err(reason) = shared::params::check::<MenuCommonParam>(repo) {
        logger::error(&format!("Fade: {reason}"));
        return;
    }
    let Some(row) = repo.get_row_by_index_mut::<MenuCommonParam>(0) else {
        logger::error("Fade: MenuCommonParam has no row");
        return;
    };
    let vanilla = row.solo_play_death_to_fade_out_time();
    row.set_solo_play_death_to_fade_out_time(0.0);
    logger::log(&format!("Fade: delay 0s (was {vanilla}s)"));
}
