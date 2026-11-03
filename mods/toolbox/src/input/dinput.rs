//! Keyboard and mouse injection into the game's DirectInput reads. The hooks themselves
//! (`GetDeviceState`, `GetDeviceData`) belong to `common::overlay` (one hook per function, and
//! the menu needs the same ones); this module registers an `InputExtension` with them.
//! Findings and open questions: `docs/background_input.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, Ordering};

use common::overlay::{self, InputExtension};

const KEYBOARD_STATE_SIZE: u32 = 256;

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
    ISOLATE.load(Ordering::Relaxed) && !super::focus::IN_FRONT.load(Ordering::Relaxed)
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

/// After `GetDeviceState`: the keyboard / mouse / other device handling the old hook did.
fn filter_state(size: u32, data: *mut c_void, mut hr: i32) -> i32 {
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

/// After `GetDeviceData` (buffered reads, any device).
fn filter_data(obj_size: u32, _data: *mut c_void, in_out: *mut u32, flags: u32, hr: i32) -> i32 {
    DATA_OBJ_SIZE.store(obj_size, Ordering::Relaxed);
    DATA_FLAGS.store(flags, Ordering::Relaxed);
    DATA.record(hr);
    if hr >= 0 && !in_out.is_null() && isolating() {
        unsafe { *in_out = 0 };
    }
    hr
}

/// Registers the filters with the overlay's hooks and installs them now (the overlay itself
/// hooks later, after `LoadDelay`).
pub fn install() {
    overlay::set_input_extension(InputExtension {
        device_state: filter_state,
        device_data: filter_data,
        unpin_cursor: super::focus::unpin_cursor,
    });
    overlay::install_input_hooks();
}
