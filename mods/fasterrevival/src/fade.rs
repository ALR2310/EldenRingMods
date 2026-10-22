//! Skips the pause after "YOU DIED": sets
//! `MenuCommonParam[0].soloPlayDeath_ToFadeOutTime` (vanilla 3.8s) to 0, once.
//! The same effect as ImAxel0's FasterRespawn, which patches the jump that
//! decides whether that wait comes from this param; writing the param needs no
//! code patch. Why this field: `docs/respawn_fade.md`.

use eldenring::cs::{MenuCommonParam, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use common::logger;

#[derive(Default)]
pub struct FadeState {
    done: bool,
}

/// Writes the param once. Only call it while the main player exists:
/// `SoloParamRepository` reads before that can panic (see `common::player`).
pub fn apply(state: &mut FadeState) {
    if state.done {
        return;
    }
    let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else { return };
    // Whatever happens below, don't retry every frame.
    state.done = true;
    if let Err(reason) = common::params::check::<MenuCommonParam>(repo) {
        logger::error(&format!("Fade: failed ({reason})"));
        return;
    }
    let Some(row) = repo.get_row_by_index_mut::<MenuCommonParam>(0) else {
        logger::error("Fade: failed (MenuCommonParam has no row)");
        return;
    };
    let vanilla = row.solo_play_death_to_fade_out_time();
    row.set_solo_play_death_to_fade_out_time(0.0);
    logger::log(&format!("Fade: delay 0s (was {vanilla}s)"));
}
