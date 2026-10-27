//! The same synthetic input, for what reads it through Win32 instead of
//! DirectInput: `GetAsyncKeyState` (hotkeys of other mods, such as
//! `common::input::is_key_pressed`) and `GetCursorPos` (the game's menu cursor,
//! which imports it with `ScreenToClient`). While the game window is not really
//! in front the real keyboard and mouse are hidden from both, like in `dinput`.
//! `docs/background_input.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicUsize, Ordering};

use hudhook::mh::{MH_ApplyQueued, MH_Initialize, MhHook};

use common::logger;

use crate::dinput::{ISOLATE, KEYS, MOUSE_BUTTONS};
use crate::focus::{self, IN_FRONT};

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
    fn ClientToScreen(hwnd: *mut c_void, point: *mut Point) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

const MAPVK_VK_TO_VSC_EX: u32 = 4;
const VK_LBUTTON: i32 = 0x01;
const VK_RBUTTON: i32 = 0x02;
const VK_MBUTTON: i32 = 0x04;

/// Virtual-key code to DirectInput scan code (0 = none), built once at start.
static VK_TO_DIK: [AtomicU8; 256] = [const { AtomicU8::new(0) }; 256];
/// Whether the key was already reported down (the "pressed since the last call" bit is set once per press).
static REPORTED: [AtomicBool; 256] = [const { AtomicBool::new(false) }; 256];

/// The cursor the game sees while the window is in the background, in client
/// pixels. `CURSOR_ON` turns it on (`moveto`) and off (`cursor off`).
pub static CURSOR_ON: AtomicBool = AtomicBool::new(false);
pub static CURSOR_X: AtomicI32 = AtomicI32::new(0);
pub static CURSOR_Y: AtomicI32 = AtomicI32::new(0);

static ORIG_GET_ASYNC_KEY_STATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_CURSOR_POS: AtomicUsize = AtomicUsize::new(0);

type GetAsyncKeyStateFn = unsafe extern "system" fn(i32) -> i16;
type GetCursorPosFn = unsafe extern "system" fn(*mut Point) -> i32;

fn isolating() -> bool {
    ISOLATE.load(Ordering::Relaxed) && !IN_FRONT.load(Ordering::Relaxed)
}

/// Scan code from the extended virtual-key mapping: `0xE0xx` is the DirectInput code `0x80 | xx`.
fn dik_of(vsc_ex: u32) -> u8 {
    match vsc_ex >> 8 {
        0 => vsc_ex as u8,
        0xE0 => (vsc_ex as u8) | 0x80,
        _ => 0,
    }
}

fn build_table() {
    for vk in 1..=0xFEu32 {
        let vsc = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC_EX) };
        VK_TO_DIK[vk as usize].store(dik_of(vsc), Ordering::Relaxed);
    }
}

/// Whether the agent holds the key or button `vk`.
fn synthetic_down(vk: i32) -> bool {
    let button = match vk {
        VK_LBUTTON => Some(0),
        VK_RBUTTON => Some(1),
        VK_MBUTTON => Some(2),
        _ => None,
    };
    if let Some(i) = button {
        return MOUSE_BUTTONS[i].load(Ordering::Relaxed) != 0;
    }
    let dik = usize::try_from(vk).ok().and_then(|i| VK_TO_DIK.get(i)).map_or(0, |d| d.load(Ordering::Relaxed));
    dik != 0 && KEYS[dik as usize].load(Ordering::Relaxed) != 0
}

unsafe extern "system" fn get_async_key_state_hook(vk: i32) -> i16 {
    let orig = unsafe { std::mem::transmute::<usize, GetAsyncKeyStateFn>(ORIG_GET_ASYNC_KEY_STATE.load(Ordering::Relaxed)) };
    let real = unsafe { orig(vk) };
    let mut state = if isolating() { 0 } else { real };
    let slot = usize::try_from(vk).ok().and_then(|i| REPORTED.get(i));
    if synthetic_down(vk) {
        state |= i16::MIN; // high bit: down now
        if slot.is_some_and(|r| !r.swap(true, Ordering::Relaxed)) {
            state |= 1; // low bit: pressed since the last call
        }
    } else if let Some(r) = slot {
        r.store(false, Ordering::Relaxed);
    }
    state
}

unsafe extern "system" fn get_cursor_pos_hook(point: *mut Point) -> i32 {
    if CURSOR_ON.load(Ordering::Relaxed) && !IN_FRONT.load(Ordering::Relaxed) && !point.is_null() {
        let hwnd = focus::game_hwnd();
        let mut p = Point { x: CURSOR_X.load(Ordering::Relaxed), y: CURSOR_Y.load(Ordering::Relaxed) };
        if !hwnd.is_null() && unsafe { ClientToScreen(hwnd, &mut p) } != 0 {
            unsafe { point.write(p) };
            return 1;
        }
    }
    let orig = unsafe { std::mem::transmute::<usize, GetCursorPosFn>(ORIG_GET_CURSOR_POS.load(Ordering::Relaxed)) };
    unsafe { orig(point) }
}

fn hook(name: &[u8], detour: *mut c_void, orig: &AtomicUsize) -> bool {
    unsafe {
        let module = GetModuleHandleA(b"user32.dll\0".as_ptr());
        let target = if module.is_null() { std::ptr::null_mut() } else { GetProcAddress(module, name.as_ptr()) };
        if target.is_null() {
            return false;
        }
        match MhHook::new(target, detour) {
            Ok(h) => {
                orig.store(h.trampoline() as usize, Ordering::Relaxed);
                h.queue_enable().is_ok()
            }
            Err(_) => false,
        }
    }
}

pub fn install() {
    build_table();
    let _ = unsafe { MH_Initialize() };
    let keys = hook(b"GetAsyncKeyState\0", get_async_key_state_hook as *mut c_void, &ORIG_GET_ASYNC_KEY_STATE);
    let cursor = hook(b"GetCursorPos\0", get_cursor_pos_hook as *mut c_void, &ORIG_GET_CURSOR_POS);
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if (keys || cursor) && applied {
        logger::log(&format!("Win32 input: hooks applied (keys {keys}, cursor {cursor})"));
    } else {
        logger::error(&format!("Win32 input: hooks incomplete (keys {keys}, cursor {cursor}, applied {applied})"));
    }
}

#[cfg(test)]
mod tests {
    use super::dik_of;

    #[test]
    fn extended_scan_codes_get_the_high_bit() {
        assert_eq!(dik_of(0x0D), 0x0D); // = / +
        assert_eq!(dik_of(0xE04B), 0xCB); // left arrow
        assert_eq!(dik_of(0xE135), 0); // pause-like prefixes are dropped
    }
}
