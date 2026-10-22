#![allow(non_snake_case)] // crate name is "WeightMultiplier" to control the output DLL's filename

mod hook;
mod menu;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../WeightMultiplier.ini");

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
        let ini_path = format!("{dir}\\WeightMultiplier.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "WeightMultiplier.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("WeightMultiplier.ini: added {added_keys} key(s) from the template"));
        }

        // Registers right away: the host only accepts menu registrations at
        // startup, before `hook::run` sleeps for `LoadDelay`.
        menu::start(hmodule, ini_path.clone());

        hook::run(ini_path);
    });

    true
}
