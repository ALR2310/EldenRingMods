#![allow(non_snake_case)] // crate name is "DropMultiplier" to control the output DLL's filename

mod drop_rate;
mod lot;
mod materials;
mod menu;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../DropMultiplier.ini");

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
        let ini_path = format!("{dir}\\DropMultiplier.ini");
        // `ChancePercent` was renamed `Percentage` (2026-09-28): carry the value over.
        let added_keys =
            config::load_or_create_default_with_renames(&ini_path, DEFAULT_INI, &[("ChancePercent", "Percentage")]);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "DropMultiplier.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("DropMultiplier.ini: added {added_keys} key(s) from the template"));
        }

        // Registers right away: the host only accepts menu registrations at
        // startup. The menu follows reloads through `drop_rate::run`.
        menu::start(hmodule, ini_path.clone());

        // `common::reload` owns the reload key; `drop_rate::run` only follows
        // `RELOAD_GENERATION`.
        std::thread::spawn(move || common::reload::run(ini_path));

        common::diag::log_environment_when_game_ready();
        drop_rate::run();
    });

    true
}
