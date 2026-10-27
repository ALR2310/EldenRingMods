//! The game's own focus gate on its input pollers. `docs/background_input.md`
//! has the analysis; in short: a singleton holds "window active" (+0x88D) and
//! one "foreground only" flag per device (+0x88E pad, +0x88F keyboard, +0x890
//! mouse). A poller skips reading when its flag is set and the window is not
//! active, so clearing the three flags removes the gate.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{logger, memscan};

/// `mov rax, [rip+singleton]; test rax,rax; jnz +5; call getter; movzx eax,
/// byte [rax+0x88E]; test al,al; jz ...` in the pad poller. UNIQUE on
/// 2.6.2.0, 2.7.0.0, 2.7.1.0 and 2.7.2.0 (`scripts/ida/run.ps1 aob`).
const PATTERN: &str = "48 8b 05 ?? ?? ?? ?? 48 85 c0 75 05 e8 ?? ?? ?? ?? 0f b6 80 8e 08 00 00 84 c0 74 20";

const OFF_ACTIVE: usize = 0x88D;
const OFF_FOREGROUND_ONLY: [usize; 3] = [0x88E, 0x88F, 0x890];

/// Address of the global that holds the singleton pointer (0 = not resolved).
static SLOT: AtomicUsize = AtomicUsize::new(0);

pub fn resolve() -> bool {
    let Some(hit) = memscan::wait_for_pattern_in_module(PATTERN, Duration::from_millis(500), Duration::from_secs(60))
    else {
        logger::error("Gate: pattern not found");
        return false;
    };
    // rip-relative operand: disp32 at +3, next instruction at +7.
    let slot = unsafe {
        let disp = (hit.add(3) as *const i32).read_unaligned();
        hit.add(7).offset(disp as isize) as usize
    };
    SLOT.store(slot, Ordering::Relaxed);
    logger::log("Gate: resolved");
    true
}

fn singleton() -> Option<usize> {
    let slot = SLOT.load(Ordering::Relaxed);
    if slot == 0 {
        return None;
    }
    let obj = unsafe { (slot as *const usize).read_volatile() };
    (obj != 0).then_some(obj)
}

/// Clears the "foreground only" flags (only writes when one is set).
pub fn apply() {
    let Some(obj) = singleton() else { return };
    for off in OFF_FOREGROUND_ONLY {
        let p = (obj + off) as *mut u8;
        unsafe {
            if p.read_volatile() != 0 {
                p.write_volatile(0);
            }
        }
    }
}

/// `active/pad/keyboard/mouse` flag bytes for the log, `-` before resolve.
pub fn state() -> String {
    let Some(obj) = singleton() else { return "-".to_string() };
    let b = |off: usize| unsafe { ((obj + off) as *const u8).read_volatile() };
    format!("{}/{}/{}/{}", b(OFF_ACTIVE), b(OFF_FOREGROUND_ONLY[0]), b(OFF_FOREGROUND_ONLY[1]), b(OFF_FOREGROUND_ONLY[2]))
}
