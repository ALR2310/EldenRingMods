//! `[Startup] StartMinimized`: the game window goes to the taskbar as soon as it exists, so it
//! does not jump in front of the window the player is working in; Alt-Tab or its taskbar icon
//! brings it back. The game restores its window when it switches the window mode (about 15 s
//! after the mods load), so for the first `WATCH` the window is minimized again each time it
//! comes back by itself. Restored by the player within `REPEAT` of our minimizing means they want
//! the game: stop for this run.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use common::logger;

use crate::config;
use crate::resize::find_game_window;

type Hwnd = *mut c_void;

#[link(name = "user32")]
unsafe extern "system" {
    fn IsIconic(hwnd: Hwnd) -> i32;
    fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
}

/// Minimize without activating (and without taking the foreground from the window in front).
const SW_SHOWMINNOACTIVE: i32 = 7;

/// How long after the mod loads the game is kept minimized.
const WATCH: Duration = Duration::from_secs(90);
/// Restored this soon after we minimized it means the player asked for it.
const REPEAT: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq)]
enum Verdict {
    Minimize,
    /// The player wants the game: stop for this run.
    GiveUp,
    Leave,
}

/// What to do when the game window has just been restored.
fn verdict(in_watch: bool, gave_up: bool, since_minimized: Option<Duration>) -> Verdict {
    if gave_up || !in_watch {
        return Verdict::Leave;
    }
    match since_minimized {
        Some(elapsed) if elapsed < REPEAT => Verdict::GiveUp,
        _ => Verdict::Minimize,
    }
}

pub fn run() {
    let started = Instant::now();
    // The window was minimized at the last look (true until it is first seen, so the first
    // look counts as a restore).
    let mut was_iconic = true;
    let mut minimized_at: Option<Instant> = None;
    let mut gave_up = false;
    loop {
        std::thread::sleep(Duration::from_millis(50));
        if !config::get().startup.start_minimized {
            continue;
        }
        let Some(game) = find_game_window() else { continue };
        let iconic = unsafe { IsIconic(game) } != 0;
        let restored = was_iconic && !iconic;
        was_iconic = iconic;
        if !restored {
            continue;
        }
        match verdict(started.elapsed() < WATCH, gave_up, minimized_at.map(|at| at.elapsed())) {
            Verdict::Leave => {}
            Verdict::GiveUp => {
                gave_up = true;
                logger::log("Startup: the game was restored right after minimizing, leaving it");
            }
            Verdict::Minimize => {
                unsafe { ShowWindow(game, SW_SHOWMINNOACTIVE) };
                minimized_at = Some(Instant::now());
                was_iconic = true;
                logger::log(&format!("Startup: the game window minimized at {:.1}s", started.elapsed().as_secs_f32()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_minimized_during_the_watch_only() {
        assert_eq!(verdict(true, false, None), Verdict::Minimize);
        assert_eq!(verdict(false, false, None), Verdict::Leave);
    }

    #[test]
    fn restoring_it_at_once_is_the_player_asking_for_it() {
        assert_eq!(verdict(true, false, Some(Duration::from_secs(2))), Verdict::GiveUp);
        // The window-mode switch comes back much later and is minimized again.
        assert_eq!(verdict(true, false, Some(Duration::from_secs(14))), Verdict::Minimize);
        assert_eq!(verdict(true, true, None), Verdict::Leave);
    }
}
