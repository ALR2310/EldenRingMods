//! Keyboard injection by hooking the DirectInput device reads
//! (`GetDeviceState`, `GetDeviceData`). Same vtable trick as
//! `soulsteleport/src/input_block.rs`. Findings and open questions:
//! `docs/background_input.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering};

use hudhook::mh::{MH_ApplyQueued, MH_Initialize, MhHook};

use common::logger;

#[repr(C)]
struct Guid(u32, u16, u16, [u8; 8]);

const IID_IDIRECTINPUT8W: Guid =
    Guid(0xBF798031, 0x483A, 0x4DA2, [0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00]);
const IID_IDIRECTINPUT8A: Guid =
    Guid(0xBF798030, 0x483A, 0x4DA2, [0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00]);
const GUID_SYS_KEYBOARD: Guid =
    Guid(0x6F1D2B61, 0xD5A0, 0x11CF, [0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00]);
const DIRECTINPUT_VERSION: u32 = 0x0800;

const SLOT_RELEASE: usize = 2;
const SLOT_CREATE_DEVICE: usize = 3; // IDirectInput8
const SLOT_GET_DEVICE_STATE: usize = 9; // IDirectInputDevice8
const SLOT_GET_DEVICE_DATA: usize = 10;

const KEYBOARD_STATE_SIZE: u32 = 256;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn LoadLibraryA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

type DirectInput8CreateFn =
    unsafe extern "system" fn(hinst: *mut c_void, version: u32, iid: *const Guid, out: *mut *mut c_void, outer: *mut c_void) -> i32;
type CreateDeviceFn =
    unsafe extern "system" fn(this: *mut c_void, guid: *const Guid, out: *mut *mut c_void, outer: *mut c_void) -> i32;
type ReleaseFn = unsafe extern "system" fn(this: *mut c_void) -> u32;
type GetDeviceStateFn = unsafe extern "system" fn(this: *mut c_void, size: u32, data: *mut c_void) -> i32;
type GetDeviceDataFn =
    unsafe extern "system" fn(this: *mut c_void, obj_size: u32, data: *mut c_void, in_out: *mut u32, flags: u32) -> i32;

/// Desired keyboard state per scan code: 0x80 = down. Written by `cmd`, read
/// by the state hook (atomics only, no locks in the hook).
pub static KEYS: [AtomicU8; 256] = [const { AtomicU8::new(0) }; 256];

/// Synthetic mouse, added to every `DIMOUSESTATE`/`DIMOUSESTATE2` read (layout
/// `lX, lY, lZ` as i32 then one byte per button). `*_ONCE` is consumed by the
/// next read; `*_RATE` is added on every read until `cmd` clears it.
pub static MOUSE_DX_ONCE: AtomicI32 = AtomicI32::new(0);
pub static MOUSE_DY_ONCE: AtomicI32 = AtomicI32::new(0);
pub static MOUSE_DZ_ONCE: AtomicI32 = AtomicI32::new(0);
pub static MOUSE_DX_RATE: AtomicI32 = AtomicI32::new(0);
pub static MOUSE_DY_RATE: AtomicI32 = AtomicI32::new(0);
/// Left, right, middle: 0x80 = down.
pub static MOUSE_BUTTONS: [AtomicU8; 3] = [const { AtomicU8::new(0) }; 3];

/// Hand the game a keyboard even when it reports the device as lost.
pub static BACKGROUND: AtomicBool = AtomicBool::new(true);

/// While the game window is not really in front, drop the real keyboard and
/// mouse: with the foreground gate removed, DirectInput would otherwise feed
/// the game everything the user types in other apps.
pub static ISOLATE: AtomicBool = AtomicBool::new(true);

fn isolating() -> bool {
    ISOLATE.load(Ordering::Relaxed) && !crate::focus::IN_FRONT.load(Ordering::Relaxed)
}

/// Per device class: calls, failed calls, last failing HRESULT.
pub struct Stat {
    pub calls: AtomicU32,
    pub fails: AtomicU32,
    pub last_hr: AtomicI32,
}

impl Stat {
    const fn new() -> Self {
        Self { calls: AtomicU32::new(0), fails: AtomicU32::new(0), last_hr: AtomicI32::new(0) }
    }

    fn record(&self, hr: i32) {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if hr < 0 {
            self.fails.fetch_add(1, Ordering::Relaxed);
            self.last_hr.store(hr, Ordering::Relaxed);
        }
    }
}

pub static KEYBOARD: Stat = Stat::new();
pub static MOUSE: Stat = Stat::new();
pub static OTHER: Stat = Stat::new();
/// Buffered reads (`GetDeviceData`), any device.
pub static DATA: Stat = Stat::new();
/// Size seen by the state hook outside keyboard/mouse (joystick formats).
pub static OTHER_SIZE: AtomicU32 = AtomicU32::new(0);
/// `obj_size` / `flags` of the last buffered read.
pub static DATA_OBJ_SIZE: AtomicU32 = AtomicU32::new(0);
pub static DATA_FLAGS: AtomicU32 = AtomicU32::new(0);

static STATE_TARGETS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static STATE_ORIGS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static DATA_ORIGS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

unsafe fn vtable_fn(obj: *mut c_void, slot: usize) -> usize {
    unsafe { *(*(obj as *const *const usize)).add(slot) }
}

/// Adds the synthetic mouse to a mouse state buffer (at least 16 bytes).
unsafe fn add_mouse(buf: *mut u8) {
    unsafe {
        let axes = buf as *mut i32;
        let dx = MOUSE_DX_ONCE.swap(0, Ordering::Relaxed) + MOUSE_DX_RATE.load(Ordering::Relaxed);
        let dy = MOUSE_DY_ONCE.swap(0, Ordering::Relaxed) + MOUSE_DY_RATE.load(Ordering::Relaxed);
        let dz = MOUSE_DZ_ONCE.swap(0, Ordering::Relaxed);
        axes.write_unaligned(axes.read_unaligned().wrapping_add(dx));
        axes.add(1).write_unaligned(axes.add(1).read_unaligned().wrapping_add(dy));
        axes.add(2).write_unaligned(axes.add(2).read_unaligned().wrapping_add(dz));
        for (i, b) in MOUSE_BUTTONS.iter().enumerate() {
            if b.load(Ordering::Relaxed) != 0 {
                *buf.add(12 + i) = 0x80;
            }
        }
    }
}

macro_rules! state_hook {
    ($name:ident, $i:expr) => {
        unsafe extern "system" fn $name(this: *mut c_void, size: u32, data: *mut c_void) -> i32 {
            let orig = unsafe { std::mem::transmute::<usize, GetDeviceStateFn>(STATE_ORIGS[$i].load(Ordering::Relaxed)) };
            let mut hr = unsafe { orig(this, size, data) };
            match size {
                KEYBOARD_STATE_SIZE => {
                    KEYBOARD.record(hr);
                    if !data.is_null() {
                        let buf = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, 256) };
                        if hr < 0 && BACKGROUND.load(Ordering::Relaxed) {
                            buf.fill(0);
                            hr = 0;
                        } else if hr >= 0 && isolating() {
                            buf.fill(0);
                        }
                        if hr >= 0 {
                            for (i, key) in KEYS.iter().enumerate() {
                                if key.load(Ordering::Relaxed) != 0 {
                                    buf[i] = 0x80;
                                }
                            }
                        }
                    }
                }
                16 | 20 => {
                    MOUSE.record(hr);
                    if !data.is_null() {
                        if hr < 0 && BACKGROUND.load(Ordering::Relaxed) {
                            unsafe { std::ptr::write_bytes(data as *mut u8, 0, size as usize) };
                            hr = 0;
                        } else if hr >= 0 && isolating() {
                            unsafe { std::ptr::write_bytes(data as *mut u8, 0, size as usize) };
                        }
                        if hr >= 0 {
                            unsafe { add_mouse(data as *mut u8) };
                        }
                    }
                }
                _ => {
                    OTHER_SIZE.store(size, Ordering::Relaxed);
                    OTHER.record(hr);
                }
            }
            hr
        }
    };
}
state_hook!(get_device_state_hook_0, 0);
state_hook!(get_device_state_hook_1, 1);

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
            DATA_OBJ_SIZE.store(obj_size, Ordering::Relaxed);
            DATA_FLAGS.store(flags, Ordering::Relaxed);
            DATA.record(hr);
            if hr >= 0 && !in_out.is_null() && isolating() {
                unsafe { *in_out = 0 };
            }
            hr
        }
    };
}
data_hook!(get_device_data_hook_0, 0);
data_hook!(get_device_data_hook_1, 1);

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

/// Reads GetDeviceState/GetDeviceData of one DirectInput flavor (W or A) off
/// a throwaway keyboard device.
fn dinput_device_fns(create: DirectInput8CreateFn, iid: &Guid) -> Option<(usize, usize)> {
    unsafe {
        let hinst = GetModuleHandleA(std::ptr::null());
        let mut di: *mut c_void = std::ptr::null_mut();
        if create(hinst, DIRECTINPUT_VERSION, iid, &mut di, std::ptr::null_mut()) < 0 || di.is_null() {
            return None;
        }
        let create_device = std::mem::transmute::<usize, CreateDeviceFn>(vtable_fn(di, SLOT_CREATE_DEVICE));
        let mut dev: *mut c_void = std::ptr::null_mut();
        let fns = if create_device(di, &GUID_SYS_KEYBOARD, &mut dev, std::ptr::null_mut()) >= 0 && !dev.is_null() {
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

/// Number of DirectInput flavors hooked (0 = failed).
fn install_dinput() -> usize {
    let module = unsafe { LoadLibraryA(c"dinput8.dll".as_ptr() as *const u8) };
    if module.is_null() {
        return 0;
    }
    let create = unsafe { GetProcAddress(module, c"DirectInput8Create".as_ptr() as *const u8) };
    if create.is_null() {
        return 0;
    }
    let create = unsafe { std::mem::transmute::<*mut c_void, DirectInput8CreateFn>(create) };

    let state_detours = [get_device_state_hook_0 as *mut c_void, get_device_state_hook_1 as *mut c_void];
    let data_detours = [get_device_data_hook_0 as *mut c_void, get_device_data_hook_1 as *mut c_void];
    let mut hooked = 0;
    for iid in [&IID_IDIRECTINPUT8W, &IID_IDIRECTINPUT8A] {
        let Some((state_fn, data_fn)) = dinput_device_fns(create, iid) else {
            continue;
        };
        // Same implementation for both flavors: already hooked.
        if STATE_TARGETS.iter().any(|t| t.load(Ordering::Relaxed) == state_fn) {
            continue;
        }
        let i = hooked;
        STATE_TARGETS[i].store(state_fn, Ordering::Relaxed);
        if hook(state_fn, state_detours[i], &STATE_ORIGS[i]) && hook(data_fn, data_detours[i], &DATA_ORIGS[i]) {
            hooked += 1;
        }
    }
    hooked
}

pub fn install() {
    // Each DLL links its own MinHook instance; initialize ours.
    let _ = unsafe { MH_Initialize() };
    let dinput = install_dinput();
    let applied = unsafe { MH_ApplyQueued() }.ok().is_ok();
    if applied && dinput > 0 {
        logger::log(&format!("DInput: hooks applied ({dinput})"));
    } else {
        logger::error(&format!("DInput: hooks incomplete (flavors {dinput}, applied {applied})"));
    }
}
