//! Keeps the game from reacting to the **mouse** while the menu is open, by
//! hooking both:
//! 1. Elden Ring's `DLUID::MouseDevice::Poll` (found by AOB across all game
//!    versions) — zeros `DIMOUSESTATE2` (`+0x7E8`, 20 bytes) and the float axes
//!    (`+0x800`, 12 bytes) right after the game polls DirectInput, so this works
//!    even when a `dinput8.dll` proxy in the game folder redirects the game's
//!    DirectInput device vtable to `System32\dinput8.dll`.
//! 2. DirectInput device reads (`GetDeviceState`, `GetDeviceData`) on both the
//!    system `dinput8.dll` and any local `dinput8.dll` proxy, probing both
//!    `GUID_SysMouse` and `GUID_SysKeyboard`.
//!
//! Keyboard and gamepad still reach the game, so the player can keep moving.
//! Why window messages don't work and how the vtable is found:
//! `shared/docs/overlay.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::mh::{MH_ApplyQueued, MhHook};

use super::MENU_OPEN;
use crate::{logger, memscan};

#[repr(C)]
struct Guid(u32, u16, u16, [u8; 8]);

const IID_IDIRECTINPUT8W: Guid =
    Guid(0xBF798031, 0x483A, 0x4DA2, [0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00]);
const IID_IDIRECTINPUT8A: Guid =
    Guid(0xBF798030, 0x483A, 0x4DA2, [0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00]);
const GUID_SYS_MOUSE: Guid =
    Guid(0x6F1D2B60, 0xD5A0, 0x11CF, [0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00]);
const GUID_SYS_KEYBOARD: Guid =
    Guid(0x6F1D2B61, 0xD5A0, 0x11CF, [0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00]);
const DIRECTINPUT_VERSION: u32 = 0x0800;

const SLOT_RELEASE: usize = 2;
const SLOT_CREATE_DEVICE: usize = 3; // IDirectInput8
const SLOT_GET_DEVICE_STATE: usize = 9; // IDirectInputDevice8
const SLOT_GET_DEVICE_DATA: usize = 10;

const DIMOUSESTATE_SIZE: u32 = 16;
const DIMOUSESTATE2_SIZE: u32 = 20;

/// `DLUID::MouseDevice<DLKR::DLMultiThreadingPolicy>::Poll` (`sub_141F71280` on
/// 2.7.2.0). Reads `IDirectInputDevice8::GetDeviceState(20, this + 0x7E8)` and
/// converts `lX/lY/lZ` into floats at `this + 0x800..0x80C`.
/// Verified UNIQUE on 2.6.2.0, 2.7.0.0, 2.7.1.0, and 2.7.2.0 via
/// `scripts/ida/run.ps1 aob -Version all`.
const MOUSE_POLL_AOB: &str =
    "40 53 48 83 ec 20 48 8b d9 48 8b 89 e0 07 00 00 48 85 c9 75 08 32 c0 48 83 c4 20 5b c3 80 bb fc 07 00 00 00";
const MOUSE_STATE_OFFSET: usize = 0x7E8; // DIMOUSESTATE2 (20 bytes: lX, lY, lZ, rgbButtons[8])
const MOUSE_FLOAT_AXES_OFFSET: usize = 0x800; // 3 x f32 (12 bytes: dx, dy, dz)

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn LoadLibraryA(name: *const u8) -> *mut c_void;
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn GetSystemDirectoryW(buf: *mut u16, size: u32) -> u32;
}

type DirectInput8CreateFn =
    unsafe extern "system" fn(hinst: *mut c_void, version: u32, iid: *const Guid, out: *mut *mut c_void, outer: *mut c_void) -> i32;
type CreateDeviceFn =
    unsafe extern "system" fn(this: *mut c_void, guid: *const Guid, out: *mut *mut c_void, outer: *mut c_void) -> i32;
type ReleaseFn = unsafe extern "system" fn(this: *mut c_void) -> u32;
type GetDeviceStateFn = unsafe extern "system" fn(this: *mut c_void, size: u32, data: *mut c_void) -> i32;
type GetDeviceDataFn =
    unsafe extern "system" fn(this: *mut c_void, obj_size: u32, data: *mut c_void, in_out: *mut u32, flags: u32) -> i32;
type MousePollFn = unsafe extern "C" fn(this: *mut u8) -> u8;

// Up to 4 DirectInput vtable targets (system + proxy, W/A, mouse/keyboard).
static STATE_TARGETS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
static STATE_ORIGS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
static DATA_ORIGS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
static MOUSE_POLL_ORIG: AtomicUsize = AtomicUsize::new(0);
/// Device objects seen reading a mouse-format state (see [state_hook]).
static MOUSE_DEVICES: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

fn remember_mouse(this: *mut c_void) {
    let this = this as usize;
    if MOUSE_DEVICES.iter().any(|d| d.load(Ordering::Relaxed) == this) {
        return;
    }
    for slot in &MOUSE_DEVICES {
        if slot.compare_exchange(0, this, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            return;
        }
    }
}

fn is_mouse(this: *mut c_void) -> bool {
    MOUSE_DEVICES.iter().any(|d| d.load(Ordering::Relaxed) == this as usize)
}

unsafe fn vtable_fn(obj: *mut c_void, slot: usize) -> usize {
    unsafe { *(*(obj as *const *const usize)).add(slot) }
}

fn menu_open() -> bool {
    MENU_OPEN.load(Ordering::Relaxed)
}

unsafe extern "C" fn mouse_poll_hook(this: *mut u8) -> u8 {
    let orig = unsafe { std::mem::transmute::<usize, MousePollFn>(MOUSE_POLL_ORIG.load(Ordering::Relaxed)) };
    let ok = unsafe { orig(this) };
    if menu_open() && !this.is_null() {
        unsafe {
            std::ptr::write_bytes(this.add(MOUSE_STATE_OFFSET), 0, DIMOUSESTATE2_SIZE as usize);
            std::ptr::write_bytes(this.add(MOUSE_FLOAT_AXES_OFFSET), 0, 12);
        }
    }
    ok
}

macro_rules! state_hook {
    ($name:ident, $i:expr) => {
        unsafe extern "system" fn $name(this: *mut c_void, size: u32, data: *mut c_void) -> i32 {
            let orig = unsafe { std::mem::transmute::<usize, GetDeviceStateFn>(STATE_ORIGS[$i].load(Ordering::Relaxed)) };
            let hr = unsafe { orig(this, size, data) };
            if matches!(size, DIMOUSESTATE_SIZE | DIMOUSESTATE2_SIZE) {
                remember_mouse(this);
                if hr >= 0 && menu_open() && !data.is_null() {
                    unsafe { std::ptr::write_bytes(data as *mut u8, 0, size as usize) };
                }
            }
            hr
        }
    };
}
state_hook!(get_device_state_hook_0, 0);
state_hook!(get_device_state_hook_1, 1);
state_hook!(get_device_state_hook_2, 2);
state_hook!(get_device_state_hook_3, 3);

macro_rules! data_hook {
    ($name:ident, $i:expr) => {
        unsafe extern "system" fn $name(
            this: *mut c_void,
            obj_size: u32,
            data: *mut c_void,
            in_out: *mut u32,
            flags: u32,
        ) -> i32 {
            let orig = unsafe { std::mem::transmute::<usize, GetDeviceDataFn>(DATA_ORIGS[$i].load(Ordering::Relaxed)) };
            let hr = unsafe { orig(this, obj_size, data, in_out, flags) };
            if hr >= 0 && menu_open() && !in_out.is_null() && is_mouse(this) {
                unsafe { *in_out = 0 };
            }
            hr
        }
    };
}
data_hook!(get_device_data_hook_0, 0);
data_hook!(get_device_data_hook_1, 1);
data_hook!(get_device_data_hook_2, 2);
data_hook!(get_device_data_hook_3, 3);

fn hook(target: usize, detour: *mut c_void, orig: &AtomicUsize) -> bool {
    if target == 0 {
        return false;
    }
    match unsafe { MhHook::new(target as *mut c_void, detour) } {
        Ok(h) => {
            orig.store(h.trampoline() as usize, Ordering::Relaxed);
            unsafe { h.queue_enable() }.is_ok()
        }
        Err(_) => false,
    }
}

/// Reads the device vtable's GetDeviceState/GetDeviceData for one DirectInput
/// flavor (W or A) and device GUID (`GUID_SysMouse` or `GUID_SysKeyboard`).
fn dinput_device_fns(create: DirectInput8CreateFn, iid: &Guid, dev_guid: &Guid) -> Option<(usize, usize)> {
    unsafe {
        let hinst = GetModuleHandleA(std::ptr::null());
        let mut di: *mut c_void = std::ptr::null_mut();
        if create(hinst, DIRECTINPUT_VERSION, iid, &mut di, std::ptr::null_mut()) < 0 || di.is_null() {
            return None;
        }
        let create_device = std::mem::transmute::<usize, CreateDeviceFn>(vtable_fn(di, SLOT_CREATE_DEVICE));
        let mut dev: *mut c_void = std::ptr::null_mut();
        let fns = if create_device(di, dev_guid, &mut dev, std::ptr::null_mut()) >= 0 && !dev.is_null() {
            let fns = (vtable_fn(dev, SLOT_GET_DEVICE_STATE), vtable_fn(dev, SLOT_GET_DEVICE_DATA));
            std::mem::transmute::<usize, ReleaseFn>(vtable_fn(dev, SLOT_RELEASE))(dev);
            Some(fns)
        } else {
            None
        };
        std::mem::transmute::<usize, ReleaseFn>(vtable_fn(di, SLOT_RELEASE))(di);
        fns
    }
}

fn load_system_dinput8() -> *mut c_void {
    let mut buf = [0u16; 260];
    let len = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if len == 0 || len >= buf.len() {
        return std::ptr::null_mut();
    }
    let dir = String::from_utf16_lossy(&buf[..len]);
    let full: Vec<u16> = format!("{dir}\\dinput8.dll\0").encode_utf16().collect();
    unsafe { LoadLibraryW(full.as_ptr()) }
}

fn install_dinput() -> usize {
    let state_detours = [
        get_device_state_hook_0 as *mut c_void,
        get_device_state_hook_1 as *mut c_void,
        get_device_state_hook_2 as *mut c_void,
        get_device_state_hook_3 as *mut c_void,
    ];
    let data_detours = [
        get_device_data_hook_0 as *mut c_void,
        get_device_data_hook_1 as *mut c_void,
        get_device_data_hook_2 as *mut c_void,
        get_device_data_hook_3 as *mut c_void,
    ];

    // Probe the real System32 dinput8.dll first (in case the game folder has a
    // dinput8.dll proxy that unloads itself after forwarding to System32), then
    // also probe the default `dinput8.dll` module.
    let sys_mod = load_system_dinput8();
    let local_mod = unsafe { LoadLibraryA(c"dinput8.dll".as_ptr() as *const u8) };

    let mut hooked = 0;
    for module in [sys_mod, local_mod] {
        if module.is_null() {
            continue;
        }
        let create = unsafe { GetProcAddress(module, c"DirectInput8Create".as_ptr() as *const u8) };
        if create.is_null() {
            continue;
        }
        let create = unsafe { std::mem::transmute::<*mut c_void, DirectInput8CreateFn>(create) };

        for iid in [&IID_IDIRECTINPUT8W, &IID_IDIRECTINPUT8A] {
            for dev_guid in [&GUID_SYS_MOUSE, &GUID_SYS_KEYBOARD] {
                if hooked >= STATE_TARGETS.len() {
                    break;
                }
                let Some((state_fn, data_fn)) = dinput_device_fns(create, iid, dev_guid) else {
                    continue;
                };
                if STATE_TARGETS.iter().any(|t| t.load(Ordering::Relaxed) == state_fn) {
                    continue;
                }
                let i = hooked;
                STATE_TARGETS[i].store(state_fn, Ordering::Relaxed);
                if hook(state_fn, state_detours[i], &STATE_ORIGS[i])
                    && hook(data_fn, data_detours[i], &DATA_ORIGS[i])
                {
                    hooked += 1;
                }
            }
        }
    }
    hooked
}

fn install_mouse_poll_hook() -> bool {
    let Some(target) = memscan::find_pattern_in_module(MOUSE_POLL_AOB) else {
        logger::warn("Menu: DLUID::MouseDevice::Poll AOB not found");
        return false;
    };
    hook(target as usize, mouse_poll_hook as *mut c_void, &MOUSE_POLL_ORIG)
}

/// Installs the input hooks. MinHook must already be initialized (done by
/// `Hudhook::builder()` in `overlay::install`).
pub fn install() {
    let poll = install_mouse_poll_hook();
    let dinput = install_dinput();
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if applied && (poll || dinput > 0) {
        logger::log(&format!(
            "Menu: mouse block hooks applied (MousePoll {poll}, DirectInput {dinput})"
        ));
    } else {
        logger::error(&format!(
            "Menu: mouse block hooks incomplete (MousePoll {poll}, DirectInput {dinput}, applied {applied})"
        ));
    }
}
