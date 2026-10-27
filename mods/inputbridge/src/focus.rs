//! Makes the game believe its window is in the foreground while it is not.
//! The game compares `GetForegroundWindow()` with its own window in several
//! places (`0x2671EB0` has four callers), so the user32 call is hooked to
//! return the game's window. A game that thinks it is focused also grabs the
//! cursor, so `ClipCursor` and `SetCursorPos` are neutralized while the window
//! is not really in front. `docs/background_input.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::time::Duration;

use hudhook::mh::{MH_ApplyQueued, MH_Initialize, MhHook};

use common::logger;

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(cb: unsafe extern "system" fn(*mut c_void, isize) -> i32, lparam: isize) -> i32;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
    fn IsWindowVisible(hwnd: *mut c_void) -> i32;
    /// Unlike `GetWindowTextW` it never sends `WM_GETTEXT`: sent to the game's own
    /// window early in startup, that message crashed its window procedure.
    fn InternalGetWindowText(hwnd: *mut c_void, text: *mut u16, max: i32) -> i32;
    fn GetForegroundWindow() -> *mut c_void;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

/// The game's main window (0 until found).
static GAME_HWND: AtomicIsize = AtomicIsize::new(0);
/// Spoof on/off (config).
pub static ENABLED: AtomicBool = AtomicBool::new(true);
/// The game window is really in front (refreshed by `refresh`, read by the
/// input hooks, which must not call into user32 themselves).
pub static IN_FRONT: AtomicBool = AtomicBool::new(true);

/// The game's main window, null until found.
pub fn game_hwnd() -> *mut c_void {
    GAME_HWND.load(Ordering::Relaxed) as *mut c_void
}

pub fn refresh() {
    IN_FRONT.store(game_is_really_foreground(), Ordering::Relaxed);
}

static ORIG_GET_FOREGROUND: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLIP_CURSOR: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_CURSOR_POS: AtomicUsize = AtomicUsize::new(0);
static ORIG_XINPUT_GET_STATE: AtomicUsize = AtomicUsize::new(0);

type GetForegroundFn = unsafe extern "system" fn() -> *mut c_void;
type ClipCursorFn = unsafe extern "system" fn(*const c_void) -> i32;
type SetCursorPosFn = unsafe extern "system" fn(i32, i32) -> i32;
type XInputGetStateFn = unsafe extern "system" fn(u32, *mut u8) -> u32;

unsafe extern "system" fn find_cb(hwnd: *mut c_void, _: isize) -> i32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut buf = [0u16; 64];
        let n = InternalGetWindowText(hwnd, buf.as_mut_ptr(), 64).max(0) as usize;
        if String::from_utf16_lossy(&buf[..n]).contains("ELDEN RING") {
            GAME_HWND.store(hwnd as isize, Ordering::Relaxed);
            return 0;
        }
    }
    1
}

fn find_game_window(timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        unsafe { EnumWindows(find_cb, 0) };
        if GAME_HWND.load(Ordering::Relaxed) != 0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/// The real foreground window, bypassing our own hook.
fn real_foreground() -> *mut c_void {
    let orig = ORIG_GET_FOREGROUND.load(Ordering::Relaxed);
    if orig == 0 {
        return unsafe { GetForegroundWindow() };
    }
    unsafe { std::mem::transmute::<usize, GetForegroundFn>(orig)() }
}

pub fn game_is_really_foreground() -> bool {
    let mut pid = 0u32;
    unsafe {
        let hwnd = real_foreground();
        !hwnd.is_null() && GetWindowThreadProcessId(hwnd, &mut pid) != 0 && pid == GetCurrentProcessId()
    }
}

unsafe extern "system" fn get_foreground_hook() -> *mut c_void {
    let real = real_foreground();
    let game = GAME_HWND.load(Ordering::Relaxed);
    if ENABLED.load(Ordering::Relaxed) && game != 0 {
        return game as *mut c_void;
    }
    real
}

unsafe extern "system" fn clip_cursor_hook(rect: *const c_void) -> i32 {
    let orig = unsafe { std::mem::transmute::<usize, ClipCursorFn>(ORIG_CLIP_CURSOR.load(Ordering::Relaxed)) };
    if ENABLED.load(Ordering::Relaxed) && !game_is_really_foreground() {
        return unsafe { orig(std::ptr::null()) };
    }
    unsafe { orig(rect) }
}

unsafe extern "system" fn set_cursor_pos_hook(x: i32, y: i32) -> i32 {
    let orig = unsafe { std::mem::transmute::<usize, SetCursorPosFn>(ORIG_SET_CURSOR_POS.load(Ordering::Relaxed)) };
    if ENABLED.load(Ordering::Relaxed) && !game_is_really_foreground() {
        return 1;
    }
    unsafe { orig(x, y) }
}

/// A pad plugged in while the game is in the background must not drive the
/// game: neutralize `XINPUT_GAMEPAD` (12 bytes after the packet number).
unsafe extern "system" fn xinput_get_state_hook(index: u32, state: *mut u8) -> u32 {
    let orig = unsafe { std::mem::transmute::<usize, XInputGetStateFn>(ORIG_XINPUT_GET_STATE.load(Ordering::Relaxed)) };
    let ret = unsafe { orig(index, state) };
    if ret == 0 && !state.is_null() && crate::dinput::ISOLATE.load(Ordering::Relaxed) && !IN_FRONT.load(Ordering::Relaxed) {
        unsafe { std::ptr::write_bytes(state.add(4), 0, 12) };
    }
    ret
}

fn hook(module: &[u8], name: &[u8], detour: *mut c_void, orig: &AtomicUsize) -> bool {
    unsafe {
        let m = GetModuleHandleA(module.as_ptr());
        if m.is_null() {
            return false;
        }
        let target = GetProcAddress(m, name.as_ptr());
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

/// Waits for the game window (up to 60 s), then hooks the three calls.
pub fn install() {
    if !find_game_window(Duration::from_secs(60)) {
        logger::error("Focus: game window not found");
        return;
    }
    let _ = unsafe { MH_Initialize() };
    let ok = hook(b"user32.dll\0", b"GetForegroundWindow\0", get_foreground_hook as *mut c_void, &ORIG_GET_FOREGROUND)
        && hook(b"user32.dll\0", b"ClipCursor\0", clip_cursor_hook as *mut c_void, &ORIG_CLIP_CURSOR)
        && hook(b"user32.dll\0", b"SetCursorPos\0", set_cursor_pos_hook as *mut c_void, &ORIG_SET_CURSOR_POS)
        && hook(b"xinput1_4.dll\0", b"XInputGetState\0", xinput_get_state_hook as *mut c_void, &ORIG_XINPUT_GET_STATE);
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if ok && applied {
        logger::log("Focus: hooks applied");
    } else {
        logger::error(&format!("Focus: hooks incomplete (created {ok}, applied {applied})"));
    }
}
