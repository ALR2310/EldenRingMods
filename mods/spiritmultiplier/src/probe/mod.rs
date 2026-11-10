//! Diagnostics that only log, each behind a `[Logging]` key that is off by default: the summon
//! slot layout, the player's SpEffects, the characters near the player and the spirit groups.
//! No gameplay change. `WarpProbe` lives in `warp.rs`, next to the warp manager it describes.

mod effects;
mod enemies;
mod persist;
mod slots;

/// Starts the probes whose key is on, each on its own thread.
pub fn spawn() {
    std::thread::spawn(slots::run);
    std::thread::spawn(effects::run);
    std::thread::spawn(enemies::run);
    std::thread::spawn(persist::run);
}
