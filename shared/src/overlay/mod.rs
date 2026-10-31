//! Shared Dear ImGui DX12 overlay host (via `hudhook`), extracted from
//! `soulsteleport`: font loading (embedded Noto Sans + system CJK fallback),
//! warm-dark gold theme, automatic resolution scaling + `MenuScale`, cursor
//! unpin (`SetCursorPos` / `ClipCursor` hooks), direct Win32 mouse polling
//! into ImGui, DirectInput mouse blocking (`input_block`), window layout
//! persistence (`MenuX`, `MenuY`, `MenuWidth`, `MenuHeight` in 1080p units),
//! and standard header chrome (Reset button, centered title, `X` close button).
//!
//! Enabled via the `overlay` Cargo feature on `common`. Details:
//! `shared/docs/overlay.md`.

mod input_block;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::imgui::{self, Condition, Context, FontConfig, FontGlyphRanges, FontSource, Io, MouseButton};
use hudhook::mh::{MH_ApplyQueued, MhHook};
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::{BeforeWndProc, Hudhook, ImguiRenderLoop, MessageFilter, RenderContext};

pub use hudhook::imgui::Ui;

use crate::{config, logger};

/// Whether the overlay window is currently open.
pub static MENU_OPEN: AtomicBool = AtomicBool::new(false);

/// Toggles [MENU_OPEN].
pub fn toggle() {
    let was_open = MENU_OPEN.fetch_xor(true, Ordering::Relaxed);
    logger::log(if was_open { "Menu: closed" } else { "Menu: opened" });
}

/// Whether the overlay window is currently open.
pub fn is_open() -> bool {
    MENU_OPEN.load(Ordering::Relaxed)
}

/// Window metadata passed to [install].
pub struct WindowConfig {
    /// Visible title centered in the header bar (e.g. `"Toolbox"`).
    pub title: &'static str,
    /// ImGui window ID (e.g. `"##toolbox"`).
    pub id: &'static str,
    /// Default `[width, height]` at 1080p before the user resizes the window.
    pub default_size: [f32; 2],
    /// Default `[x, y]` at 1080p before the user drags the window.
    pub default_pos: [f32; 2],
}

// Latin + Latin-1 + Latin Extended-A/B + Vietnamese (Latin Extended
// Additional) + general punctuation.
static GLYPH_RANGES: [u32; 9] = [0x0020, 0x024F, 0x0300, 0x036F, 0x1E00, 0x1EFF, 0x2000, 0x206F, 0];

/// Main font, always available: Noto Sans (SIL OFL 1.1, see
/// `shared/assets/NotoSans-OFL.txt`) embedded in the DLL, subset to the ranges
/// in [GLYPH_RANGES] (~90 KB).
static EMBEDDED_FONT: &[u8] = include_bytes!("../../assets/NotoSans-Subset.ttf");

/// Optional CJK coverage for character names, merged in from the first one of
/// these found in the real Windows fonts folder (`GetWindowsDirectoryW`).
const CJK_FONT_FILES: [&str; 5] = ["msyh.ttc", "msjh.ttc", "meiryo.ttc", "msgothic.ttc", "simsun.ttc"];
const FONT_RASTER_BASE: f32 = 40.0;
const FONT_RASTER_MAX: f32 = 64.0;

const KEY_MENU_SCALE: &str = "MenuScale";
const MENU_SCALE_MIN: f32 = 0.5;
const MENU_SCALE_MAX: f32 = 3.0;

fn menu_scale() -> f32 {
    let v = config::get_double(KEY_MENU_SCALE, 1.0) as f32;
    if v.is_finite() { v.clamp(MENU_SCALE_MIN, MENU_SCALE_MAX) } else { 1.0 }
}

fn font_raster_size() -> f32 {
    (FONT_RASTER_BASE * menu_scale()).clamp(FONT_RASTER_BASE, FONT_RASTER_MAX)
}

const REFERENCE_HEIGHT: f32 = 1080.0;
const FONT_AT_1080P: f32 = 22.0;
const MIN_SCALE: f32 = 0.6;
const MAX_SCALE: f32 = 3.0;

const KEY_X: &str = "MenuX";
const KEY_Y: &str = "MenuY";
const KEY_W: &str = "MenuWidth";
const KEY_H: &str = "MenuHeight";

static INI_PATH: OnceLock<String> = OnceLock::new();

/// Tells the overlay which ini file to persist its layout (`MenuX`/`MenuY`/
/// `MenuWidth`/`MenuHeight`) and any [set_ini_values] calls into. Call before
/// [install].
pub fn set_ini_path(path: String) {
    let _ = INI_PATH.set(path);
}

/// Path passed to [set_ini_path], if any.
pub fn ini_path() -> Option<&'static str> {
    INI_PATH.get().map(String::as_str)
}

/// Updates `values` in the ini file registered via [set_ini_path] AND reloads
/// the in-memory `common::config` map so the change takes effect immediately.
pub fn save_ini_values(values: &[(&str, String)]) -> bool {
    let Some(path) = INI_PATH.get() else {
        return false;
    };
    let ok = config::set_values(path, values);
    if ok {
        config::load(path);
    } else {
        logger::error("Menu: saving values to ini failed");
    }
    ok
}

fn saved_layout(cfg: &WindowConfig) -> ([f32; 2], [f32; 2]) {
    let read = |key: &str, default: f32, min: f32| {
        let v = config::get_double(key, -1.0) as f32;
        if v.is_finite() && v >= min { v } else { default }
    };
    (
        [read(KEY_X, cfg.default_pos[0], 0.0), read(KEY_Y, cfg.default_pos[1], 0.0)],
        [read(KEY_W, cfg.default_size[0], 120.0), read(KEY_H, cfg.default_size[1], 80.0)],
    )
}

fn save_layout(pos: [f32; 2], size: [f32; 2]) {
    let Some(path) = INI_PATH.get() else {
        return;
    };
    let fmt = |v: f32| format!("{:.0}", v.max(0.0));
    let ok = config::set_values(
        path,
        &[(KEY_X, fmt(pos[0])), (KEY_Y, fmt(pos[1])), (KEY_W, fmt(size[0])), (KEY_H, fmt(size[1]))],
    );
    if !ok {
        logger::error("Menu: position save to ini failed");
    }
}

struct MenuRenderLoop<F> {
    cfg: WindowConfig,
    draw_body: F,
    base_style: Option<imgui::Style>,
    scale: f32,
    pos_scale: f32,
    menu_scale: f32,
    relayout: bool,
    buttons_down: [bool; 2],
    layout_seen: Option<([f32; 2], [f32; 2])>,
    layout_saved: Option<([f32; 2], [f32; 2])>,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn GetCurrentProcessId() -> u32;
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
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    (pid == unsafe { GetCurrentProcessId() }).then_some(hwnd)
}

fn feed_mouse(ctx: &mut Context, buttons_down: &mut [bool; 2]) {
    let Some(hwnd) = game_window() else {
        return;
    };
    unsafe { ClipCursor(std::ptr::null()) };

    let mut pt = Point::default();
    let mut rect = Rect::default();
    let ok = unsafe { GetCursorPos(&mut pt) != 0 && ScreenToClient(hwnd, &mut pt) != 0 && GetClientRect(hwnd, &mut rect) != 0 };
    let client_w = (rect.right - rect.left) as f32;
    let client_h = (rect.bottom - rect.top) as f32;
    let io = ctx.io_mut();
    if ok && client_w > 0.0 && client_h > 0.0 {
        let [dw, dh] = io.display_size;
        io.add_mouse_pos_event([pt.x as f32 * dw / client_w, pt.y as f32 * dh / client_h]);
    }
    for (i, (vk, button)) in [(VK_LBUTTON, MouseButton::Left), (VK_RBUTTON, MouseButton::Right)].into_iter().enumerate() {
        let down = unsafe { GetAsyncKeyState(vk) } < 0;
        if down != buttons_down[i] {
            buttons_down[i] = down;
            io.add_mouse_button_event(button, down);
        }
    }
}

fn windows_fonts_dir() -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 260];
    let len = unsafe { GetWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if len == 0 || len >= buf.len() {
        return None;
    }
    Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len])).join("Fonts"))
}

fn find_cjk_font() -> Option<(&'static str, Vec<u8>)> {
    let dir = windows_fonts_dir()?;
    CJK_FONT_FILES.iter().find_map(|name| std::fs::read(dir.join(name)).ok().map(|b| (*name, b)))
}

fn scale_for(display_size: [f32; 2]) -> f32 {
    if display_size[1] <= 0.0 {
        return 1.0;
    }
    (display_size[1] / REFERENCE_HEIGHT).clamp(MIN_SCALE, MAX_SCALE)
}

impl<F: FnMut(&Ui) + Send + Sync + 'static> ImguiRenderLoop for MenuRenderLoop<F> {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        ctx.set_ini_filename(None);

        let main = FontSource::TtfData {
            data: EMBEDDED_FONT,
            size_pixels: font_raster_size(),
            config: Some(FontConfig {
                glyph_ranges: FontGlyphRanges::from_slice(&GLYPH_RANGES),
                ..FontConfig::default()
            }),
        };
        let cjk = find_cjk_font();
        match &cjk {
            Some((name, bytes)) => {
                ctx.fonts().add_font(&[
                    main,
                    FontSource::TtfData {
                        data: bytes,
                        size_pixels: font_raster_size(),
                        config: Some(FontConfig {
                            glyph_ranges: FontGlyphRanges::chinese_simplified_common(),
                            ..FontConfig::default()
                        }),
                    },
                ]);
                logger::log(&format!("Menu: font Noto Sans + CJK ({name})"));
            }
            None => {
                ctx.fonts().add_font(&[main]);
                logger::log("Menu: font Noto Sans (no system CJK font)");
            }
        }
        self.menu_scale = menu_scale();
        if self.menu_scale != 1.0 {
            logger::log(&format!("Menu: MenuScale {:.2}", self.menu_scale));
        }
        apply_theme(ctx.style_mut());
        self.base_style = Some(*ctx.style());
        logger::log("Menu: ImGui initialized");
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        let open = MENU_OPEN.load(Ordering::Relaxed);
        ctx.io_mut().mouse_draw_cursor = open;
        if open {
            feed_mouse(ctx, &mut self.buttons_down);
        } else if self.buttons_down != [false; 2] {
            self.buttons_down = [false; 2];
            ctx.io_mut().add_mouse_button_event(MouseButton::Left, false);
            ctx.io_mut().add_mouse_button_event(MouseButton::Right, false);
        }

        self.pos_scale = scale_for(ctx.io().display_size);
        let menu_scale = if self.menu_scale > 0.0 { self.menu_scale } else { 1.0 };
        let scale = self.pos_scale * menu_scale;
        if (scale - self.scale).abs() > 0.01 {
            if let Some(base) = self.base_style {
                let mut style = base;
                style.scale_all_sizes(scale);
                style.mouse_cursor_scale = base.mouse_cursor_scale * scale;
                *ctx.style_mut() = style;
            }
            ctx.io_mut().font_global_scale = FONT_AT_1080P * scale / font_raster_size();
            if self.scale != 0.0 {
                logger::log(&format!("Menu: UI scale {:.2} -> {scale:.2}", self.scale));
            }
            self.scale = scale;
            self.relayout = true;
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        if !MENU_OPEN.load(Ordering::Relaxed) {
            return;
        }

        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        let pos_scale = if self.pos_scale > 0.0 { self.pos_scale } else { 1.0 };
        let cond = if std::mem::take(&mut self.relayout) { Condition::Always } else { Condition::FirstUseEver };
        let (pos, size) = saved_layout(&self.cfg);
        let mut keep_open = true;
        let mut reset_layout = false;
        let mut layout = None;
        let title = self.cfg.title;
        let draw_body = &mut self.draw_body;
        ui.window(self.cfg.id)
            .title_bar(false)
            .size(size.map(|v| v * scale), cond)
            .position(pos.map(|v| v * pos_scale), cond)
            .collapsible(false)
            .build(|| {
                layout = Some((ui.window_pos().map(|v| v / pos_scale), ui.window_size().map(|v| v / scale)));
                draw_header(ui, title, &mut keep_open, &mut reset_layout);
                draw_body(ui);
            });

        if reset_layout {
            if let Some(path) = INI_PATH.get() {
                let reset = |k| (k, "-1".to_string());
                config::set_values(path, &[reset(KEY_X), reset(KEY_Y), reset(KEY_W), reset(KEY_H)]);
            }
            self.relayout = true;
            self.layout_saved = None;
            self.layout_seen = None;
            layout = None;
        }

        if let Some(now) = layout {
            if self.layout_saved.is_none() {
                self.layout_saved = Some(now);
            }
            self.layout_seen = Some(now);
        }
        let mouse_held = self.buttons_down[0];
        if !mouse_held && let Some(seen) = self.layout_seen {
            let changed = self.layout_saved.is_none_or(|(p, s)| {
                (p[0] - seen.0[0]).abs() >= 1.0
                    || (p[1] - seen.0[1]).abs() >= 1.0
                    || (s[0] - seen.1[0]).abs() >= 1.0
                    || (s[1] - seen.1[1]).abs() >= 1.0
            });
            if changed {
                save_layout(seen.0, seen.1);
                self.layout_saved = Some(seen);
            }
        }

        if !keep_open {
            MENU_OPEN.store(false, Ordering::Relaxed);
        }
    }

    fn before_wnd_proc(&self, _hwnd: HWND, umsg: u32, _wparam: WPARAM, _lparam: LPARAM) -> BeforeWndProc {
        const WM_INPUT: u32 = 0x00FF;
        const WM_MOUSEFIRST: u32 = 0x0200;
        const WM_MOUSELAST: u32 = 0x020E;
        let mouse_msg = umsg == WM_INPUT || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&umsg);
        if mouse_msg && MENU_OPEN.load(Ordering::Relaxed) { BeforeWndProc::Break } else { BeforeWndProc::Continue }
    }

    fn message_filter(&self, _io: &Io) -> MessageFilter {
        if MENU_OPEN.load(Ordering::Relaxed) { MessageFilter::InputAll } else { MessageFilter::empty() }
    }
}

// --- Theme & shared widgets ------------------------------------------------

pub const GOLD: [f32; 4] = [0.84, 0.72, 0.47, 1.00];
pub const GOLD_DIM: [f32; 4] = [0.62, 0.54, 0.38, 1.00];
pub const TEXT: [f32; 4] = [0.93, 0.90, 0.83, 1.00];
pub const TEXT_MUTED: [f32; 4] = [0.60, 0.57, 0.52, 1.00];
pub const ERROR_TEXT: [f32; 4] = [0.90, 0.48, 0.40, 1.00];

pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

fn apply_theme(style: &mut imgui::Style) {
    use imgui::StyleColor as C;

    style.window_padding = [18.0, 16.0];
    style.window_rounding = 10.0;
    style.window_border_size = 1.0;
    style.window_title_align = [0.5, 0.5];
    style.frame_padding = [14.0, 9.0];
    style.frame_rounding = 6.0;
    style.frame_border_size = 1.0;
    style.item_spacing = [10.0, 9.0];
    style.popup_rounding = 6.0;
    style.popup_border_size = 1.0;
    style.scrollbar_size = 12.0;
    style.scrollbar_rounding = 6.0;
    style.grab_rounding = 6.0;

    style[C::Text] = TEXT;
    style[C::TextDisabled] = TEXT_MUTED;
    style[C::WindowBg] = rgba(16, 14, 12, 0.93);
    style[C::PopupBg] = rgba(22, 19, 16, 0.97);
    style[C::Border] = rgba(158, 132, 86, 0.55);
    style[C::BorderShadow] = [0.0, 0.0, 0.0, 0.0];

    style[C::TitleBg] = rgba(28, 24, 19, 0.96);
    style[C::TitleBgActive] = rgba(40, 33, 24, 0.98);
    style[C::TitleBgCollapsed] = rgba(28, 24, 19, 0.80);

    style[C::FrameBg] = rgba(34, 29, 23, 0.90);
    style[C::FrameBgHovered] = rgba(58, 48, 34, 0.90);
    style[C::FrameBgActive] = rgba(72, 59, 40, 0.95);

    style[C::Button] = rgba(38, 32, 25, 0.95);
    style[C::ButtonHovered] = rgba(86, 69, 42, 0.95);
    style[C::ButtonActive] = rgba(120, 95, 55, 1.00);

    style[C::Header] = rgba(60, 50, 35, 0.80);
    style[C::HeaderHovered] = rgba(86, 69, 42, 0.90);
    style[C::HeaderActive] = rgba(120, 95, 55, 1.00);

    style[C::Separator] = rgba(158, 132, 86, 0.40);
    style[C::SeparatorHovered] = rgba(200, 168, 110, 0.70);
    style[C::SeparatorActive] = rgba(214, 184, 120, 1.00);

    style[C::ScrollbarBg] = rgba(16, 14, 12, 0.40);
    style[C::ScrollbarGrab] = rgba(90, 76, 54, 0.80);
    style[C::ScrollbarGrabHovered] = rgba(120, 100, 68, 0.90);
    style[C::ScrollbarGrabActive] = rgba(158, 132, 86, 1.00);

    style[C::ResizeGrip] = rgba(158, 132, 86, 0.25);
    style[C::ResizeGripHovered] = rgba(200, 168, 110, 0.60);
    style[C::ResizeGripActive] = rgba(214, 184, 120, 0.90);

    style[C::CheckMark] = GOLD;
    style[C::SliderGrab] = GOLD_DIM;
    style[C::SliderGrabActive] = GOLD;
    style[C::NavHighlight] = GOLD;
}

/// Draws `text` horizontally centered in the current content region.
pub fn centered_text(ui: &Ui, color: [f32; 4], text: &str) {
    let avail = ui.content_region_avail()[0];
    let width = ui.calc_text_size(text)[0];
    let x = ui.cursor_pos()[0] + ((avail - width) * 0.5).max(0.0);
    ui.set_cursor_pos([x, ui.cursor_pos()[1]]);
    ui.text_colored(color, text);
}

fn draw_header(ui: &Ui, title: &str, keep_open: &mut bool, reset_layout: &mut bool) {
    let pad = ui.clone_style().frame_padding;
    let header_y = ui.cursor_pos()[1];
    let header_x = ui.cursor_pos()[0];
    ui.set_cursor_pos([header_x, header_y - pad[1] * 0.5]);
    if ui.small_button("Reset") {
        *reset_layout = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset the menu's position and size");
    }
    ui.same_line();
    ui.set_cursor_pos([header_x, header_y]);
    centered_text(ui, GOLD, title);
    let close_w = ui.calc_text_size("X")[0] + pad[0] * 2.0;
    ui.same_line_with_pos(ui.window_content_region_max()[0] - close_w);
    ui.set_cursor_pos([ui.cursor_pos()[0], header_y - pad[1] * 0.5]);
    if ui.small_button("X") {
        *keep_open = false;
    }
    ui.separator();
    ui.spacing();
}

// --- Cursor unpin ----------------------------------------------------------

type SetCursorPosFn = unsafe extern "system" fn(x: i32, y: i32) -> i32;
type ClipCursorFn = unsafe extern "system" fn(rect: *const Rect) -> i32;

static SET_CURSOR_POS_ORIG: AtomicUsize = AtomicUsize::new(0);
static CLIP_CURSOR_ORIG: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn set_cursor_pos_hook(x: i32, y: i32) -> i32 {
    if MENU_OPEN.load(Ordering::Relaxed) {
        return 1;
    }
    let orig = SET_CURSOR_POS_ORIG.load(Ordering::Relaxed);
    unsafe { std::mem::transmute::<usize, SetCursorPosFn>(orig)(x, y) }
}

unsafe extern "system" fn clip_cursor_hook(rect: *const Rect) -> i32 {
    let rect = if MENU_OPEN.load(Ordering::Relaxed) { std::ptr::null() } else { rect };
    let orig = CLIP_CURSOR_ORIG.load(Ordering::Relaxed);
    unsafe { std::mem::transmute::<usize, ClipCursorFn>(orig)(rect) }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut std::ffi::c_void;
    fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
    fn CreateMutexA(attrs: *const std::ffi::c_void, initial_owner: i32, name: *const u8) -> *mut std::ffi::c_void;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    fn ReleaseMutex(handle: *mut std::ffi::c_void) -> i32;
}

/// Process-wide named mutex so multiple DLLs that hook DX12 via `hudhook` do
/// not call `MH_CreateHook` + `MH_ApplyQueued` concurrently (which causes the
/// second DLL's trampoline to be built from the unhooked prologue before the
/// first DLL's `MH_ApplyQueued` lands, overwriting the first DLL's hook).
struct ProcessHookLock(*mut std::ffi::c_void);

impl ProcessHookLock {
    fn acquire() -> Self {
        let pid = unsafe { GetCurrentProcessId() };
        let name = format!("Local\\EldenRingMod_OverlayInit_{pid}\0");
        let handle = unsafe { CreateMutexA(std::ptr::null(), 0, name.as_ptr()) };
        if !handle.is_null() {
            unsafe { WaitForSingleObject(handle, 10_000) };
        }
        Self(handle)
    }
}

impl Drop for ProcessHookLock {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ReleaseMutex(self.0) };
        }
    }
}

fn hook_user32(name: &str, detour: *mut std::ffi::c_void, orig_slot: &AtomicUsize) -> bool {
    let user32 = unsafe { GetModuleHandleA(c"user32.dll".as_ptr() as *const u8) };
    let c_name = format!("{name}\0");
    let target = unsafe { GetProcAddress(user32, c_name.as_ptr()) };
    if user32.is_null() || target.is_null() {
        return false;
    }
    match unsafe { MhHook::new(target, detour) } {
        Ok(hook) => {
            orig_slot.store(hook.trampoline() as usize, Ordering::Relaxed);
            unsafe { hook.queue_enable() }.is_ok()
        }
        Err(_) => false,
    }
}

fn install_cursor_hooks() {
    let set_pos = hook_user32("SetCursorPos", set_cursor_pos_hook as *mut std::ffi::c_void, &SET_CURSOR_POS_ORIG);
    let clip = hook_user32("ClipCursor", clip_cursor_hook as *mut std::ffi::c_void, &CLIP_CURSOR_ORIG);
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if set_pos && clip && applied {
        logger::log("Menu: cursor hooks applied");
    } else {
        logger::error(&format!(
            "Menu: cursor hooks incomplete (SetCursorPos {set_pos}, ClipCursor {clip}, applied {applied})"
        ));
    }
}

/// Installs the DX12 ImGui overlay and cursor/mouse-block hooks. Call once
/// from `DllMain`'s worker thread.
pub fn install<F>(cfg: WindowConfig, draw_body: F)
where
    F: FnMut(&Ui) + Send + Sync + 'static,
{
    // Wait for CSTaskImp + 800 ms so standalone HUD overlays (e.g. JourneyHUD)
    // that hook DX12 right at CSTaskImp readiness finish `MH_CreateHook` +
    // `MH_ApplyQueued` before we inspect the DX12 vtable prologues, while still
    // installing ~1.5 s before the game creates its DX12 SwapChain.
    let _ = crate::task::wait_for_cs_task();
    std::thread::sleep(std::time::Duration::from_millis(800));
    let _guard = ProcessHookLock::acquire();

    let loop_state = MenuRenderLoop {
        cfg,
        draw_body,
        base_style: None,
        scale: 0.0,
        pos_scale: 0.0,
        menu_scale: 0.0,
        relayout: false,
        buttons_down: [false; 2],
        layout_seen: None,
        layout_saved: None,
    };
    match Hudhook::builder().with::<ImguiDx12Hooks>(loop_state).build().apply() {
        Ok(()) => {
            logger::log("Menu: DX12 hooks applied");
            install_cursor_hooks();
            input_block::install();
        }
        Err(e) => logger::error(&format!("Menu: DX12 hooks failed ({e:?}), menu unavailable")),
    }
}
