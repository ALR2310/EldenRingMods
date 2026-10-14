//! Hotkey helpers shared by every mod: parsing a key name from an ini
//! (`ReloadKey`, `MenuKey`...) into a virtual-key code, and a raw-Win32
//! key-press poll that only fires while this process owns the foreground
//! window. Why not `eldenring::util::input::is_key_pressed`: `shared/docs/input.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, Ordering};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, process_id: *mut u32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
}

// Per-VK state kept between polls (one DLL = one copy, like every static in
// this crate).
const KEY_RESYNC: u8 = 0; // unknown: (re)gained focus, the next poll only records
const KEY_UP: u8 = 1;
const KEY_DOWN: u8 = 2;
static KEY_STATE: [AtomicU8; 256] = [const { AtomicU8::new(KEY_RESYNC) }; 256];

fn game_window_has_focus() -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return false;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid == unsafe { GetCurrentProcessId() }
}

/// Whether `vk` was pressed since the last call for the same key, while this
/// process's own window had focus. Poll it periodically (a frame task, or a
/// thread loop every ~100ms). Only one caller should poll a given key: the
/// state is per key, not per caller.
///
/// - Never polls while another process's window is focused, so it can't
///   consume a press meant for that window (or fire while typing elsewhere).
/// - On (re)gaining focus the first poll only records the key's state, so a
///   press made in another app before alt-tabbing back doesn't fire.
/// - Fires on the "pressed since last call" bit (catches a tap shorter than
///   the poll interval) or on an up -> down transition.
pub fn is_key_pressed(vk: i32) -> bool {
    let Some(state) = usize::try_from(vk).ok().and_then(|i| KEY_STATE.get(i)) else {
        return false;
    };
    if !game_window_has_focus() {
        state.store(KEY_RESYNC, Ordering::Relaxed);
        return false;
    }
    let raw = unsafe { GetAsyncKeyState(vk) };
    let down = raw < 0; // high bit: held right now
    let tapped = raw & 1 != 0;
    let previous = state.swap(if down { KEY_DOWN } else { KEY_UP }, Ordering::Relaxed);
    match previous {
        KEY_RESYNC => false,
        KEY_UP => down || tapped,
        _ => tapped && !down, // released and pressed again between polls
    }
}

/// Parses, in order of precedence: a raw virtual-key code in hex ("0x2D") or
/// decimal ("112"), "F1".."F24", or a single letter/digit. Returns `fallback`
/// for anything else (empty, typo, out of range).
/// https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
pub fn parse_virtual_key(name: &str, fallback: i32) -> i32 {
    let upper = name.trim().to_ascii_uppercase();

    let numeric = if let Some(hex) = upper.strip_prefix("0X") {
        i32::from_str_radix(hex, 16).ok()
    } else if upper.len() > 1 && upper.bytes().all(|b| b.is_ascii_digit()) {
        upper.parse().ok()
    } else {
        None
    };
    if let Some(vk) = numeric.filter(|vk| (1..=0xFF).contains(vk)) {
        return vk;
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<i32>().ok()).filter(|n| (1..=24).contains(n)) {
        return 0x70 + (n - 1); // VK_F1 = 0x70, VK_F2 = 0x71, ...
    }
    // VK codes for '0'-'9' and 'A'-'Z' match their ASCII value.
    match upper.as_bytes() {
        [c] if c.is_ascii_alphanumeric() => *c as i32,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FALLBACK: i32 = 0x74; // VK_F5

    #[test]
    fn parses_hex() {
        assert_eq!(parse_virtual_key("0x2D", FALLBACK), 0x2D);
    }

    #[test]
    fn parses_decimal() {
        assert_eq!(parse_virtual_key("112", FALLBACK), 112);
    }

    #[test]
    fn parses_function_keys() {
        assert_eq!(parse_virtual_key("F1", FALLBACK), 0x70);
        assert_eq!(parse_virtual_key("F9", FALLBACK), 0x78);
        assert_eq!(parse_virtual_key("f24", FALLBACK), 0x70 + 23);
    }

    #[test]
    fn parses_single_char() {
        assert_eq!(parse_virtual_key("g", FALLBACK), 'G' as i32);
        assert_eq!(parse_virtual_key("5", FALLBACK), '5' as i32); // VK codes for '0'-'9' match ASCII
        assert_eq!(parse_virtual_key("F", FALLBACK), 'F' as i32); // the letter, not a function key
    }

    #[test]
    fn falls_back_on_garbage() {
        assert_eq!(parse_virtual_key("", FALLBACK), FALLBACK);
        assert_eq!(parse_virtual_key("???", FALLBACK), FALLBACK);
        assert_eq!(parse_virtual_key("F99", FALLBACK), FALLBACK);
        assert_eq!(parse_virtual_key("0x100", FALLBACK), FALLBACK); // out of range
        assert_eq!(parse_virtual_key("0x0", FALLBACK), FALLBACK);
        assert_eq!(parse_virtual_key("256", FALLBACK), FALLBACK);
        assert_eq!(parse_virtual_key("0xZZ", FALLBACK), FALLBACK);
    }
}
