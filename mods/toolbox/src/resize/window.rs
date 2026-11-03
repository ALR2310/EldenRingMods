//! Makes the game's windowed-mode window resizable by dragging its edges:
//! adds `WS_THICKFRAME` back (the game creates its window without it) and
//! subclasses the window procedure for the resize cursor, the minimum size and
//! the aspect lock. The border is re-applied from a 1s watchdog because the
//! game drops it whenever the screen mode or resolution changes. Why this
//! works and what was tried: `docs/resize_approach.md`; the size math is in
//! `geometry.rs`.

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use common::{config, logger};

use crate::geometry::{self, Limits, Rect};

type Hwnd = *mut c_void;

#[repr(C)]
#[allow(dead_code)] // only written, read by Windows
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[allow(dead_code)] // only written, read by Windows
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track_size: Point,
    max_track_size: Point,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(callback: unsafe extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
    fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
    fn IsWindow(hwnd: Hwnd) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn IsIconic(hwnd: Hwnd) -> i32;
    fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
    fn GetClassNameW(hwnd: Hwnd, buf: *mut u16, len: i32) -> i32;
    fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
    fn SetWindowPos(hwnd: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn CallWindowProcW(prev: isize, hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn DefWindowProcW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn AdjustWindowRectExForDpi(rect: *mut Rect, style: u32, menu: i32, ex_style: u32, dpi: u32) -> i32;
    fn GetDpiForWindow(hwnd: Hwnd) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
}

const GWLP_WNDPROC: i32 = -4;
const GWL_STYLE: i32 = -16;
const GWL_EXSTYLE: i32 = -20;

const WS_CAPTION: u32 = 0x00C0_0000;
const WS_THICKFRAME: u32 = 0x0004_0000;

const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_FRAMECHANGED: u32 = 0x0020;

const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

const WM_SETCURSOR: u32 = 0x0020;
const WM_GETMINMAXINFO: u32 = 0x0024;
const WM_SIZING: u32 = 0x0214;
const WM_ENTERSIZEMOVE: u32 = 0x0231;

// Hit-test codes of the resize border, HTLEFT..=HTBOTTOMRIGHT.
const HT_SIZE_FIRST: u32 = 10;
const HT_SIZE_LAST: u32 = 17;

/// The game's own window procedure, called for everything we don't handle.
static ORIG_WNDPROC: AtomicIsize = AtomicIsize::new(0);
/// Whether the resize border is currently on because we added it - the
/// subclassed messages only do anything while it is.
static FRAME_ADDED: AtomicBool = AtomicBool::new(false);
/// Set once the player starts dragging the border - from then on the
/// startup `Width`/`Height` is never re-applied over their size.
static USER_RESIZED: AtomicBool = AtomicBool::new(false);

/// How long after hooking the startup size keeps being re-applied: the game
/// may still set its own resolution on the window right after creating it.
const START_SIZE_WINDOW: Duration = Duration::from_secs(20);

/// Hooks the game window once it exists, then keeps its resize border in
/// sync with the ini for the rest of the process's life. Never returns.
pub fn run() {
    // Physical pixels for this thread's own calls (`Width`/`Height`,
    // `GetClientRect`/`SetWindowPos`), whatever DPI mode the game runs in -
    // a DPI-unaware caller gets scaled coordinates (a 480x300 request came out
    // 720x450 at 150% in the first outside test).
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let mut hooked: Hwnd = null_mut();
    let mut hooked_at = Instant::now();
    loop {
        if hooked.is_null() || unsafe { IsWindow(hooked) } == 0 {
            hooked = null_mut();
            if let Some(hwnd) = find_game_window() {
                hook(hwnd);
                hooked = hwnd;
                hooked_at = Instant::now();
            }
        }
        if !hooked.is_null() {
            sync_frame(hooked);
            if hooked_at.elapsed() < START_SIZE_WINDOW && !USER_RESIZED.load(Ordering::SeqCst) {
                apply_start_size(hooked);
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

unsafe extern "system" fn find_callback(hwnd: Hwnd, out: isize) -> i32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid != unsafe { GetCurrentProcessId() } || unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) }.max(0) as usize;
    // Class (and title) is "ELDEN RING™"; the process also owns hidden
    // "IME"/"DIEmWin" helper windows.
    if String::from_utf16_lossy(&buf[..len]).starts_with("ELDEN RING") {
        unsafe { *(out as *mut Hwnd) = hwnd };
        return 0;
    }
    1
}

fn find_game_window() -> Option<Hwnd> {
    let mut found: Hwnd = null_mut();
    unsafe { EnumWindows(find_callback, &mut found as *mut Hwnd as isize) };
    (!found.is_null()).then_some(found)
}

fn hook(hwnd: Hwnd) {
    // Store the current procedure before swapping, so a message arriving
    // between the swap and the store below never sees 0.
    ORIG_WNDPROC.store(unsafe { GetWindowLongPtrW(hwnd, GWLP_WNDPROC) }, Ordering::SeqCst);
    let prev = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc as *const () as isize) };
    if prev == 0 {
        logger::error("Window: subclass failed (SetWindowLongPtr returned 0), resizing unavailable");
        return;
    }
    ORIG_WNDPROC.store(prev, Ordering::SeqCst);
    FRAME_ADDED.store(false, Ordering::SeqCst);
    logger::log(&format!("Window: hooked (DPI {})", unsafe { GetDpiForWindow(hwnd) }));
}

/// Adds or removes `WS_THICKFRAME` to match `Resizable`, windowed mode only
/// (fullscreen/borderless windows have no caption and are left alone).
fn sync_frame(hwnd: Hwnd) {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    let windowed = style & WS_CAPTION == WS_CAPTION;
    let has_frame = style & WS_THICKFRAME != 0;
    let want = windowed && config::get_bool("Resizable", true);

    if !windowed {
        FRAME_ADDED.store(false, Ordering::SeqCst);
        return;
    }
    let new_style = if want && !has_frame {
        FRAME_ADDED.store(true, Ordering::SeqCst);
        logger::log("Resize border: added");
        style | WS_THICKFRAME
    } else if !want && has_frame && FRAME_ADDED.load(Ordering::SeqCst) {
        FRAME_ADDED.store(false, Ordering::SeqCst);
        logger::log("Resize border: removed");
        style & !WS_THICKFRAME
    } else {
        if want {
            // Border already there (e.g. the window was re-hooked) - make sure
            // the message handling is on for it.
            FRAME_ADDED.store(true, Ordering::SeqCst);
        }
        return;
    };
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
        SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// Outer window size minus client size for the window's current style and
/// DPI - computed rather than read from the live rects, which are 0x0 while
/// the window is minimized.
fn frame_size(hwnd: Hwnd) -> (i32, i32) {
    let mut rect = Rect::default();
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        AdjustWindowRectExForDpi(&mut rect, style, 0, ex_style, GetDpiForWindow(hwnd));
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

/// Resizes the window so its client area is the ini's startup size, keeping
/// its position. No-op when already that size, minimized, or not windowed.
fn apply_start_size(hwnd: Hwnd) {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    if style & WS_CAPTION != WS_CAPTION || unsafe { IsIconic(hwnd) } != 0 {
        return;
    }
    let mut client = Rect::default();
    let mut window = Rect::default();
    unsafe {
        GetClientRect(hwnd, &mut client);
        GetWindowRect(hwnd, &mut window);
    }
    let current = (client.right - client.left, client.bottom - client.top);
    let (width, height) = (config::get_int("Width", 0), config::get_int("Height", 0));
    let Some((w, h)) = geometry::start_client_size(width, height, current, &Limits::from_config()) else {
        return;
    };
    if current == (w, h) {
        return;
    }
    // Frame measured from the live rects (same DPI context as the resize
    // call below) rather than `frame_size`, which uses the window's DPI.
    let frame_w = (window.right - window.left) - current.0;
    let frame_h = (window.bottom - window.top) - current.1;
    unsafe {
        SetWindowPos(hwnd, null_mut(), 0, 0, w + frame_w, h + frame_h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
    logger::log(&format!("Window: set to {w}x{h} (was {}x{})", current.0, current.1));
}

unsafe extern "system" fn wnd_proc(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
    let orig = ORIG_WNDPROC.load(Ordering::SeqCst);
    if FRAME_ADDED.load(Ordering::SeqCst) {
        match msg {
            WM_ENTERSIZEMOVE => USER_RESIZED.store(true, Ordering::SeqCst),
            WM_SETCURSOR => {
                let hit = (lparam & 0xFFFF) as u32;
                if (HT_SIZE_FIRST..=HT_SIZE_LAST).contains(&hit) {
                    return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
                }
            }
            WM_GETMINMAXINFO => {
                let result = unsafe { CallWindowProcW(orig, hwnd, msg, wparam, lparam) };
                let (frame_w, frame_h) = frame_size(hwnd);
                let (min_w, min_h) = Limits::from_config().min;
                let info = unsafe { &mut *(lparam as *mut MinMaxInfo) };
                info.min_track_size = Point { x: min_w + frame_w, y: min_h + frame_h };
                return result;
            }
            WM_SIZING => {
                unsafe { CallWindowProcW(orig, hwnd, msg, wparam, lparam) };
                geometry::constrain(wparam, unsafe { &mut *(lparam as *mut Rect) }, frame_size(hwnd), &Limits::from_config());
                return 1;
            }
            _ => {}
        }
    }
    unsafe { CallWindowProcW(orig, hwnd, msg, wparam, lparam) }
}
