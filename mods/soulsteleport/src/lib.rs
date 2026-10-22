#![allow(non_snake_case)] // crate name is "SoulsTeleport" to control the output DLL's filename

mod input_block;
mod net;
mod party;
mod steam;
mod ui;
mod warp;
mod wire;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../SoulsTeleport.ini");

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
        let ini_path = format!("{dir}\\SoulsTeleport.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "SoulsTeleport.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("SoulsTeleport.ini: added {added_keys} key(s) from the template"));
        }

        ui::set_ini_path(ini_path.clone());
        ui::install();

        common::diag::log_environment_when_game_ready();
        warp::run();
    });

    true
}
