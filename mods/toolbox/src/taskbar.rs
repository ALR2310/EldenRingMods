//! Replaces the game's icon on its taskbar button (and title bar / Alt-Tab) with a blank or the
//! default Windows one (`[Taskbar] Icon`). Done with `WM_SETICON` plus the window class icons;
//! `"Game"` restores what the window had. The game or Windows may put the icon back (new window,
//! Explorer restart), so it is checked every couple of seconds.

use std::ffi::c_void;
use std::time::Duration;

use common::logger;

use crate::config;
use crate::resize::find_game_window;

type Hwnd = *mut c_void;

#[repr(C)]
struct IconInfo {
    is_icon: i32,
    x_hotspot: u32,
    y_hotspot: u32,
    mask: isize,
    color: isize,
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateBitmap(width: i32, height: i32, planes: u32, bits_per_pixel: u32, bits: *const c_void) -> isize;
    fn DeleteObject(object: isize) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SendMessageTimeoutW(
        hwnd: Hwnd,
        msg: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout_ms: u32,
        result: *mut usize,
    ) -> isize;
    fn CreateIconIndirect(info: *const IconInfo) -> isize;
    fn LoadIconW(instance: *mut c_void, name: *const u16) -> isize;
    fn GetClassLongPtrW(hwnd: Hwnd, index: i32) -> usize;
    fn SetClassLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> usize;
}

const WM_SETICON: u32 = 0x0080;
const WM_GETICON: u32 = 0x007F;
const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;
const SMTO_ABORTIFHUNG: u32 = 0x0002;
const GCLP_HICON: i32 = -14;
const GCLP_HICONSM: i32 = -34;
const IDI_APPLICATION: usize = 32512;

/// A fully transparent 32x32 icon with a 32-bit colour bitmap (every pixel ARGB 0) and an
/// all-ones mask (keep what is behind), so it is transparent whether Windows reads the alpha
/// channel or the mask. A 1-bit icon was not honoured by the taskbar.
fn blank_icon() -> isize {
    let mask = [0xFFu8; 128];
    let pixels = [0u8; 32 * 32 * 4];
    unsafe {
        let mask_bitmap = CreateBitmap(32, 32, 1, 1, mask.as_ptr() as *const c_void);
        let color_bitmap = CreateBitmap(32, 32, 1, 32, pixels.as_ptr() as *const c_void);
        let info = IconInfo { is_icon: 1, x_hotspot: 0, y_hotspot: 0, mask: mask_bitmap, color: color_bitmap };
        let icon = CreateIconIndirect(&info);
        DeleteObject(mask_bitmap);
        DeleteObject(color_bitmap);
        icon
    }
}

fn default_icon() -> isize {
    unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION as *const u16) }
}

fn send(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> usize {
    let mut result = 0usize;
    // A timeout so a busy game window never blocks this thread.
    unsafe { SendMessageTimeoutW(hwnd, msg, wparam, lparam, SMTO_ABORTIFHUNG, 1000, &mut result) };
    result
}

/// The icons a window shows: `(big, small, class big, class small)`.
#[derive(Clone, Copy, PartialEq)]
struct Icons(isize, isize, isize, isize);

fn read(hwnd: Hwnd) -> Icons {
    unsafe {
        Icons(
            send(hwnd, WM_GETICON, ICON_BIG, 0) as isize,
            send(hwnd, WM_GETICON, ICON_SMALL, 0) as isize,
            GetClassLongPtrW(hwnd, GCLP_HICON) as isize,
            GetClassLongPtrW(hwnd, GCLP_HICONSM) as isize,
        )
    }
}

fn write(hwnd: Hwnd, icons: Icons) {
    send(hwnd, WM_SETICON, ICON_BIG, icons.0);
    send(hwnd, WM_SETICON, ICON_SMALL, icons.1);
    unsafe {
        SetClassLongPtrW(hwnd, GCLP_HICON, icons.2);
        SetClassLongPtrW(hwnd, GCLP_HICONSM, icons.3);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Game,
    Blank,
    Default,
}

fn parse(text: &str) -> Mode {
    match text.trim().to_ascii_lowercase().as_str() {
        "blank" => Mode::Blank,
        "default" => Mode::Default,
        "game" | "" => Mode::Game,
        other => {
            logger::warn(&format!("Taskbar: Icon = \"{other}\" is not Game, Blank or Default, keeping the game's icon"));
            Mode::Game
        }
    }
}

pub fn run() {
    let mut blank = 0isize;
    let mut window: Hwnd = std::ptr::null_mut();
    // What the window had before we changed it, to give it back.
    let mut original: Option<Icons> = None;
    let mut last_mode = Mode::Game;
    let mut rewrites = 0u32;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        let mode = parse(&config::get().taskbar.icon);
        let Some(hwnd) = find_game_window() else { continue };
        if hwnd != window {
            // A new window (or the first): it has its own icons, nothing to restore on the old.
            window = hwnd;
            original = None;
        }
        if mode == Mode::Game {
            if let Some(icons) = original.take() {
                write(hwnd, icons);
                logger::log("Taskbar: game icon restored");
            }
            last_mode = mode;
            continue;
        }
        let icon = match mode {
            Mode::Blank => {
                if blank == 0 {
                    blank = blank_icon();
                }
                blank
            }
            _ => default_icon(),
        };
        if icon == 0 {
            logger::warn("Taskbar: icon not created");
            continue;
        }
        let wanted = Icons(icon, icon, icon, icon);
        let current = read(hwnd);
        if current != wanted {
            if original.is_none() {
                original = Some(current);
            }
            write(hwnd, wanted);
            rewrites += 1;
            // The first few only: how often something puts the icon back (and whether it stuck).
            if rewrites <= 5 {
                let after = read(hwnd);
                logger::log(&format!(
                    "Taskbar: icon written (#{rewrites}), was {:#x}/{:#x}, now {:#x}/{:#x}, wanted {icon:#x}",
                    current.0, current.1, after.0, after.1
                ));
            }
        }
        if mode != last_mode {
            logger::log(if mode == Mode::Blank { "Taskbar: icon blanked" } else { "Taskbar: default icon set" });
        }
        last_mode = mode;
    }
}
