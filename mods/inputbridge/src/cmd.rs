//! Command file `InputBridge.cmd` next to the DLL: the poller consumes it
//! (reads, then deletes) and applies one command per line:
//!
//!   down <key> | up <key> | press <key> [seconds] | release_all
//!   mouse <dx> <dy>                     one-off relative move (mouse counts)
//!   look <dx> <dy> <seconds>            move every frame for a while (camera)
//!   mdown|mup <left|right|middle>       hold / release a button
//!   mclick <left|right|middle> [secs]   press a button for a while
//!   wheel <n>                           scroll notches (120 per notch)
//!   moveto <x> <y>                      put the menu cursor at client pixel x, y (until `cursor off`)
//!   cursor off                          hand the cursor back to the real mouse
//!
//! Write it to a temp name and rename, so the poller never sees half a file.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::logger;

use crate::dinput::{
    KEYS, MOUSE_BUTTONS, MOUSE_DX_ONCE, MOUSE_DX_RATE, MOUSE_DY_ONCE, MOUSE_DY_RATE, MOUSE_DZ_ONCE,
};
use crate::gate;
use crate::keys::dik;
use crate::winput::{CURSOR_ON, CURSOR_X, CURSOR_Y};

/// Unix ms at which a timed key goes up again; 0 = not timed.
static EXPIRY: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
/// Same for the mouse buttons and for the `look` rate.
static BUTTON_EXPIRY: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
static LOOK_EXPIRY: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn set(code: usize, down: bool, hold_ms: u64) {
    EXPIRY[code].store(if down && hold_ms > 0 { now_ms() + hold_ms } else { 0 }, Ordering::Relaxed);
    KEYS[code].store(if down { 0x80 } else { 0 }, Ordering::Relaxed);
}

fn set_button(index: usize, down: bool, hold_ms: u64) {
    BUTTON_EXPIRY[index].store(if down && hold_ms > 0 { now_ms() + hold_ms } else { 0 }, Ordering::Relaxed);
    MOUSE_BUTTONS[index].store(if down { 0x80 } else { 0 }, Ordering::Relaxed);
}

fn button_index(name: &str) -> Option<usize> {
    match name {
        "left" => Some(0),
        "right" => Some(1),
        "middle" => Some(2),
        _ => None,
    }
}

fn int(tok: Option<&str>) -> Option<i32> {
    tok.and_then(|s| s.parse::<i32>().ok())
}

fn millis(tok: Option<&str>, default: f64) -> u64 {
    let secs = tok.and_then(|s| s.parse::<f64>().ok()).unwrap_or(default);
    ((secs * 1000.0) as u64).max(1)
}

fn apply(line: &str) {
    let mut it = line.split_whitespace();
    let Some(verb) = it.next() else { return };
    let ok = match verb {
        "release_all" => {
            for code in 0..256 {
                set(code, false, 0);
            }
            for b in 0..3 {
                set_button(b, false, 0);
            }
            MOUSE_DX_RATE.store(0, Ordering::Relaxed);
            MOUSE_DY_RATE.store(0, Ordering::Relaxed);
            LOOK_EXPIRY.store(0, Ordering::Relaxed);
            true
        }
        "down" | "up" | "press" => match it.next().and_then(dik) {
            Some(code) => {
                match verb {
                    "down" => set(code, true, 0),
                    "up" => set(code, false, 0),
                    _ => set(code, true, millis(it.next(), 0.1)),
                }
                true
            }
            None => false,
        },
        "mouse" => match (int(it.next()), int(it.next())) {
            (Some(dx), Some(dy)) => {
                MOUSE_DX_ONCE.fetch_add(dx, Ordering::Relaxed);
                MOUSE_DY_ONCE.fetch_add(dy, Ordering::Relaxed);
                true
            }
            _ => false,
        },
        "look" => match (int(it.next()), int(it.next())) {
            (Some(dx), Some(dy)) => {
                MOUSE_DX_RATE.store(dx, Ordering::Relaxed);
                MOUSE_DY_RATE.store(dy, Ordering::Relaxed);
                LOOK_EXPIRY.store(now_ms() + millis(it.next(), 0.5), Ordering::Relaxed);
                true
            }
            _ => false,
        },
        "mdown" | "mup" | "mclick" => match it.next().and_then(button_index) {
            Some(b) => {
                match verb {
                    "mdown" => set_button(b, true, 0),
                    "mup" => set_button(b, false, 0),
                    _ => set_button(b, true, millis(it.next(), 0.1)),
                }
                true
            }
            None => false,
        },
        "moveto" => match (int(it.next()), int(it.next())) {
            (Some(x), Some(y)) => {
                CURSOR_X.store(x, Ordering::Relaxed);
                CURSOR_Y.store(y, Ordering::Relaxed);
                CURSOR_ON.store(true, Ordering::Relaxed);
                true
            }
            _ => false,
        },
        "cursor" => match it.next() {
            Some("off") => {
                CURSOR_ON.store(false, Ordering::Relaxed);
                true
            }
            _ => false,
        },
        "wheel" => match int(it.next()) {
            Some(n) => {
                MOUSE_DZ_ONCE.fetch_add(n.saturating_mul(120), Ordering::Relaxed);
                true
            }
            None => false,
        },
        _ => false,
    };
    if ok {
        logger::log(&format!("Cmd: {line}"));
    } else {
        logger::warn(&format!("Cmd: rejected '{line}'"));
    }
}

fn expire(now: u64) {
    for (code, exp) in EXPIRY.iter().enumerate() {
        let at = exp.load(Ordering::Relaxed);
        if at != 0 && now >= at {
            set(code, false, 0);
        }
    }
    for (b, exp) in BUTTON_EXPIRY.iter().enumerate() {
        let at = exp.load(Ordering::Relaxed);
        if at != 0 && now >= at {
            set_button(b, false, 0);
        }
    }
    let look = LOOK_EXPIRY.load(Ordering::Relaxed);
    if look != 0 && now >= look {
        MOUSE_DX_RATE.store(0, Ordering::Relaxed);
        MOUSE_DY_RATE.store(0, Ordering::Relaxed);
        LOOK_EXPIRY.store(0, Ordering::Relaxed);
    }
}

pub fn run(dir: &str, gate_ready: bool) {
    let path = format!("{dir}\\InputBridge.cmd");
    loop {
        std::thread::sleep(Duration::from_millis(10));
        crate::focus::refresh();
        if gate_ready {
            gate::apply();
        }
        if let Ok(text) = fs::read_to_string(&path) {
            let _ = fs::remove_file(&path);
            text.lines().for_each(apply);
        }
        expire(now_ms());
    }
}
