//! In-place code patches shared by the "Anywhere" features: find an AOB,
//! overwrite bytes, log the outcome.

use std::time::Duration;

use common::{codepatch, logger, memscan};

const SCAN_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);

/// Waits for `pattern` in the game module and overwrites `patched` at
/// `offset` bytes past the match. Returns whether the patch was applied.
pub fn apply(feature: &str, name: &str, pattern: &str, offset: usize, patched: &[u8]) -> bool {
    match memscan::wait_for_pattern_in_module(pattern, SCAN_RETRY_INTERVAL, SCAN_TIMEOUT) {
        Some(addr) if unsafe { codepatch::overwrite_bytes(addr.add(offset), patched) } => {
            logger::log(&format!("{feature}: {name} patched at {addr:p}."));
            true
        }
        _ => {
            logger::error(&format!("{feature}: {name} pattern not found/patch failed."));
            false
        }
    }
}

/// Parks the calling feature thread for good (the task it registered keeps
/// running on the game's own schedule).
pub fn park() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
