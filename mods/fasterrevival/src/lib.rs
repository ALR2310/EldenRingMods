#![allow(non_snake_case)] // crate name is "FasterRevival" to control the output DLL's filename

mod death;
mod fade;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../FasterRevival.ini");

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
        let ini_path = format!("{dir}\\FasterRevival.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        logger::init(&dir, "FasterRevival.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("FasterRevival.ini: added {added_keys} key(s) from the template"));
        }

        common::diag::log_environment_when_game_ready();
        death::run();
    });

    true
}
