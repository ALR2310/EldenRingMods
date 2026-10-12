#![allow(non_snake_case)] // crate name is "AnimCancel" to control the output DLL's filename

mod probe;

use common::{config, dll_dir, logger};

// AnimCancel.ini is embedded verbatim into the binary at compile time via
// include_str! - the single source of truth for the default config.
const DEFAULT_INI: &str = include_str!("../AnimCancel.ini");

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
        let ini_path = format!("{dir}\\AnimCancel.ini");
        let migrated = config::load_or_create_default(&ini_path, DEFAULT_INI);

        logger::init(&dir, "AnimCancel.log");
        logger::install_panic_hook();
        logger::log("Activating AnimCancel...");
        if migrated > 0 {
            logger::log(&format!(
                "AnimCancel.ini updated: added {migrated} new key(s) from a newer default template."
            ));
        }

        common::diag::log_environment_when_game_ready();
        probe::run();
    });

    true
}
