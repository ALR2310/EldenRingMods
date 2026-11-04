//! Input bridge (was the `InputBridge` mod): feeds the game keyboard, mouse and focus from the
//! outside so an agent can play while the window is in the background. `docs/background_input.md`.
//!
//! Commands arrive through `Toolbox.cmd` next to the DLL (`cmd.rs`), the DirectInput reads are
//! filtered by `dinput.rs` (through the overlay's hooks), `focus.rs` spoofs the foreground window
//! and `gate.rs` clears the game's own focus gate.

mod cmd;
mod dinput;
mod focus;
mod gate;
mod keys;
mod winput;

use std::sync::atomic::Ordering;
use std::time::Duration;

use shared::logger;

/// One line every 5 s, only when a counter moved: which DirectInput paths the
/// game really uses, and what they return while the window is not focused.
fn report_stats() {
    let mut last = (0u32, 0u32, 0u32, 0u32);
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let kb = &dinput::KEYBOARD;
        let now = (
            kb.calls.load(Ordering::Relaxed),
            dinput::MOUSE.calls.load(Ordering::Relaxed),
            dinput::OTHER.calls.load(Ordering::Relaxed),
            dinput::DATA.calls.load(Ordering::Relaxed),
        );
        if now == last {
            continue;
        }
        last = now;
        logger::log(&format!(
            "DInput: fg={} gate={} kb {}/{} fail (hr {:#x}), mouse {}/{}, other[{}] {}/{}, data {}/{} (obj {}, flags {:#x})",
            focus::game_is_really_foreground(),
            gate::state(),
            now.0,
            kb.fails.load(Ordering::Relaxed),
            kb.last_hr.load(Ordering::Relaxed),
            now.1,
            dinput::MOUSE.fails.load(Ordering::Relaxed),
            dinput::OTHER_SIZE.load(Ordering::Relaxed),
            now.2,
            dinput::OTHER.fails.load(Ordering::Relaxed),
            now.3,
            dinput::DATA.fails.load(Ordering::Relaxed),
            dinput::DATA_OBJ_SIZE.load(Ordering::Relaxed),
            dinput::DATA_FLAGS.load(Ordering::Relaxed),
        ));
    }
}

/// Applies `[InputBridge]` and starts everything; blocks (the command loop), so call it on its
/// own thread.
pub fn run(dir: &str) {
    let config = crate::config::get();
    let input = &config.input_bridge;
    dinput::BACKGROUND.store(input.background_keyboard, Ordering::Relaxed);
    dinput::ISOLATE.store(input.isolate_input, Ordering::Relaxed);
    focus::ENABLED.store(input.spoof_foreground, Ordering::Relaxed);
    dinput::install();
    let gate_ready = input.background_keyboard && gate::resolve();
    if input.spoof_foreground {
        focus::install();
    }
    winput::install();
    std::thread::spawn(report_stats);
    cmd::run(dir, gate_ready);
}
