//! The edits to the game's HUD movie (`01_000_fe.gfx`). `shared::gfx::hook` allows one hook per
//! DLL, so one function runs every edit, each on the result of the one before.

use shared::gfx::hook;
use shared::logger;

use crate::{boss_list, enemy_tags, settings};

const MOVIE: &str = "01_000_fe.gfx";

fn edit(movie: &[u8]) -> Option<Vec<u8>> {
    let mut edited: Option<Vec<u8>> = None;
    for step in [boss_list::edit, enemy_tags::edit] {
        if let Some(next) = step(edited.as_deref().unwrap_or(movie)) {
            edited = Some(next);
        }
    }
    edited
}

/// Hooks the movie open (on a thread of its own: it waits for the game's code). Only when an
/// option that edits it is on, read when the mod starts.
pub fn install() {
    if !settings::boss_bar() && settings::enemy_hp_bar() {
        return;
    }
    std::thread::spawn(|| {
        if hook::install(MOVIE, edit) {
            logger::log("HudMovie: hooked");
        }
    });
}
