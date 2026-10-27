#![allow(non_snake_case)] // crate name is "InputBridge" to control the output DLL's filename

mod cmd;
mod dinput;
mod focus;
mod gate;
mod keys;

use std::sync::atomic::Ordering;
use std::time::Duration;

use common::{config, dll_dir, logger};

const DEFAULT_INI: &str = include_str!("../InputBridge.ini");

/// One line every 5 s, only when a counter moved: which DirectInput paths the
/// game really uses, and what they return while the window is not focused.
fn report_stats() {
    let mut last = (0u32, 0u32, 0u32, 0u32);
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let kb = &dinput::KEYBOARD;
        let now = (
            kb.calls.load(Ordering::Relaxed),
            dinput::MOUSE.calls.load(Ordering::Relaxed),
            dinput::OTHER.calls.load(Ordering::Relaxed),
            dinput::DATA.calls.load(Ordering::Relaxed),
        );
        if now == last {
            continue;
        }
        last = now;
        logger::log(&format!(
            "DInput: fg={} gate={} kb {}/{} fail (hr {:#x}), mouse {}/{}, other[{}] {}/{}, data {}/{} (obj {}, flags {:#x})",
            focus::game_is_really_foreground(),
            gate::state(),
            now.0,
            kb.fails.load(Ordering::Relaxed),
            kb.last_hr.load(Ordering::Relaxed),
            now.1,
            dinput::MOUSE.fails.load(Ordering::Relaxed),
            dinput::OTHER_SIZE.load(Ordering::Relaxed),
            now.2,
            dinput::OTHER.fails.load(Ordering::Relaxed),
            now.3,
            dinput::DATA.fails.load(Ordering::Relaxed),
            dinput::DATA_OBJ_SIZE.load(Ordering::Relaxed),
            dinput::DATA_FLAGS.load(Ordering::Relaxed),
        ));
    }
}

/// # Safety
/// This is exposed this way so the library loader can call it. Do not call it
/// yourself.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: u64, reason: u32) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason != DLL_PROCESS_ATTACH {
        return true;
    }

    std::thread::spawn(move || {
        let dir = dll_dir(hmodule);
        let ini_path = format!("{dir}\\InputBridge.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        logger::init(&dir, "InputBridge.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("InputBridge.ini: added {added_keys} key(s) from the template"));
        }

        dinput::BACKGROUND.store(config::get_bool("BackgroundKeyboard", true), Ordering::Relaxed);
        dinput::ISOLATE.store(config::get_bool("IsolateInput", true), Ordering::Relaxed);
        focus::ENABLED.store(config::get_bool("SpoofForeground", true), Ordering::Relaxed);
        dinput::install();
        let gate_ready = dinput::BACKGROUND.load(Ordering::Relaxed) && gate::resolve();
        if focus::ENABLED.load(Ordering::Relaxed) {
            focus::install();
        }
        std::thread::spawn(report_stats);
        cmd::run(&dir, gate_ready);
    });

    true
}
