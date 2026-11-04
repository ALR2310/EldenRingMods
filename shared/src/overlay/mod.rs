//! What a mod needs to draw its own Dear ImGui overlay (through `hudhook`) and to hold the
//! mouse while it does: the cursor and mouse-block hooks. While [MENU_OPEN] is set the game gets
//! no mouse and its cursor is free. Enabled by the `overlay` Cargo feature; details:
//! `shared/docs/overlay.md`.

mod input_block;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hudhook::imgui::{Context, MouseButton};
use hudhook::mh::{MH_ApplyQueued, MhHook};

/// MinHook (`MhHook`...), for a mod that hooks more than the overlay does.
pub use hudhook::mh;

use crate::logger;

/// Whether the game must ignore the mouse because an overlay window has it.
pub static MENU_OPEN: AtomicBool = AtomicBool::new(false);

/// Whether the overlay window is currently open.
pub fn is_open() -> bool {
    MENU_OPEN.load(Ordering::Relaxed)
}

#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn GetModuleHandleA(name: *const u8) -> *mut std::ffi::c_void;
    fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetWindowThreadProcessId(hwnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn ScreenToClient(hwnd: *mut std::ffi::c_void, point: *mut Point) -> i32;
    fn GetClientRect(hwnd: *mut std::ffi::c_void, rect: *mut Rect) -> i32;
    fn ClipCursor(rect: *const Rect) -> i32;
    fn GetAsyncKeyState(vk: i32) -> i16;
}

const VK_LBUTTON: i32 = 0x01;
const VK_RBUTTON: i32 = 0x02;

fn game_window() -> Option<*mut std::ffi::c_void> {
    // SAFETY: plain Win32 queries with valid out pointers.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        (pid == GetCurrentProcessId()).then_some(hwnd)
    }
}

/// Whether the game must not see the window message `umsg`: a mouse message or raw input while
/// [MENU_OPEN] is set (relative raw-input events would overwrite the absolute cursor position).
/// For a mod's own `before_wnd_proc`.
pub fn blocks_window_message(umsg: u32) -> bool {
    const WM_INPUT: u32 = 0x00FF;
    const WM_MOUSEFIRST: u32 = 0x0200;
    const WM_MOUSELAST: u32 = 0x020E;
    let mouse_msg = umsg == WM_INPUT || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&umsg);
    mouse_msg && is_open()
}

/// Feeds the cursor position (in `display_size` units) and the left and right buttons to ImGui,
/// and frees the cursor from the game's clip rectangle. For a mod that draws with its own
/// `ImguiRenderLoop` while it holds [MENU_OPEN] (and [install_input_hooks]); `buttons_down` is
/// the state of the last call.
pub fn feed_mouse(ctx: &mut Context, buttons_down: &mut [bool; 2]) {
    let Some(hwnd) = game_window() else {
        return;
    };
    let mut pt = Point::default();
    let mut rect = Rect::default();
    // SAFETY: `hwnd` is the game's window and the out pointers are valid.
    let ok = unsafe {
        ClipCursor(std::ptr::null());
        GetCursorPos(&mut pt) != 0
            && ScreenToClient(hwnd, &mut pt) != 0
            && GetClientRect(hwnd, &mut rect) != 0
    };
    let client_w = (rect.right - rect.left) as f32;
    let client_h = (rect.bottom - rect.top) as f32;
    let io = ctx.io_mut();
    if ok && client_w > 0.0 && client_h > 0.0 {
        let [dw, dh] = io.display_size;
        io.add_mouse_pos_event([pt.x as f32 * dw / client_w, pt.y as f32 * dh / client_h]);
    }
    for (i, (vk, button)) in
        [(VK_LBUTTON, MouseButton::Left), (VK_RBUTTON, MouseButton::Right)].into_iter().enumerate()
    {
        // SAFETY: a plain key-state query.
        let down = unsafe { GetAsyncKeyState(vk) } < 0;
        if down != buttons_down[i] {
            buttons_down[i] = down;
            io.add_mouse_button_event(button, down);
        }
    }
}

type SetCursorPosFn = unsafe extern "system" fn(x: i32, y: i32) -> i32;
type ClipCursorFn = unsafe extern "system" fn(rect: *const Rect) -> i32;

static SET_CURSOR_POS_ORIG: AtomicUsize = AtomicUsize::new(0);
static CLIP_CURSOR_ORIG: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn set_cursor_pos_hook(x: i32, y: i32) -> i32 {
    if is_open() {
        return 1;
    }
    let orig = SET_CURSOR_POS_ORIG.load(Ordering::Relaxed);
    // SAFETY: `orig` is the trampoline MinHook made for `SetCursorPos`.
    unsafe { std::mem::transmute::<usize, SetCursorPosFn>(orig)(x, y) }
}

unsafe extern "system" fn clip_cursor_hook(rect: *const Rect) -> i32 {
    let rect = if is_open() { std::ptr::null() } else { rect };
    let orig = CLIP_CURSOR_ORIG.load(Ordering::Relaxed);
    // SAFETY: `orig` is the trampoline MinHook made for `ClipCursor`.
    unsafe { std::mem::transmute::<usize, ClipCursorFn>(orig)(rect) }
}

fn hook_user32(name: &str, detour: *mut std::ffi::c_void, orig_slot: &AtomicUsize) -> bool {
    // SAFETY: `user32.dll` is loaded in the game; the name is NUL-terminated.
    let user32 = unsafe { GetModuleHandleA(c"user32.dll".as_ptr() as *const u8) };
    let c_name = format!("{name}\0");
    // SAFETY: `user32` is checked below before the result is used.
    let target = unsafe { GetProcAddress(user32, c_name.as_ptr()) };
    if user32.is_null() || target.is_null() {
        return false;
    }
    // SAFETY: `target` is a function of user32 and `detour` has its signature.
    match unsafe { MhHook::new(target, detour) } {
        Ok(hook) => {
            orig_slot.store(hook.trampoline() as usize, Ordering::Relaxed);
            // SAFETY: the hook was just created and is applied by `MH_ApplyQueued`.
            unsafe { hook.queue_enable() }.is_ok()
        }
        Err(_) => false,
    }
}

/// Installs the cursor and input hooks (`ClipCursor`, `SetCursorPos`, the game's mouse poll,
/// DirectInput reads) once; later calls do nothing.
pub fn install_input_hooks() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // Already initialized (by `Hudhook::builder`, or by an earlier call) is fine.
        // SAFETY: MinHook initialization has no preconditions.
        let _ = unsafe { hudhook::mh::MH_Initialize() };
        install_cursor_hooks();
        input_block::install();
    });
}

fn install_cursor_hooks() {
    let set_pos = hook_user32(
        "SetCursorPos",
        set_cursor_pos_hook as *mut std::ffi::c_void,
        &SET_CURSOR_POS_ORIG,
    );
    let clip =
        hook_user32("ClipCursor", clip_cursor_hook as *mut std::ffi::c_void, &CLIP_CURSOR_ORIG);
    // SAFETY: applies the hooks queued above.
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if set_pos && clip && applied {
        logger::log("Overlay: cursor hooks applied");
    } else {
        logger::error(&format!(
            "Overlay: cursor hooks incomplete (SetCursorPos {set_pos}, ClipCursor {clip}, applied {applied})"
        ));
    }
}
