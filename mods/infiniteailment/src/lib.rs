#![allow(non_snake_case)] // crate name is "InfiniteAilment" to control the output DLL's filename

mod menu;
mod status_effect;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../InfiniteAilment.ini");

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
        let ini_path = format!("{dir}\\InfiniteAilment.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        logger::init(&dir, "InfiniteAilment.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("InfiniteAilment.ini: added {added_keys} key(s) from the template"));
        }

        // Registers right away: the host only accepts menu registrations at
        // startup.
        menu::start(hmodule, ini_path.clone());

        // `common::reload` owns the reload key; `status_effect::run` only
        // follows `RELOAD_GENERATION`.
        std::thread::spawn(move || common::reload::run(ini_path));

        common::diag::log_environment_when_game_ready();
        status_effect::run();
    });

    true
}
