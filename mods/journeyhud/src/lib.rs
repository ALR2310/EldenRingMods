#![allow(non_snake_case)] // crate name is "JourneyHud" to control the output DLL's filename

mod menu;
mod settings;
mod target;
mod textures;
mod ui;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../JourneyHud.ini");

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
        let ini_path = format!("{dir}\\JourneyHud.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "JourneyHud.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("JourneyHud.ini: added {added_keys} key(s) from the template"));
        }

        settings::load();
        // Early: the menu host only takes registrations in the first seconds.
        menu::start(hmodule, ini_path);
        ui::install();
        common::diag::log_environment_when_game_ready();
        target::run();
    });

    true
}
