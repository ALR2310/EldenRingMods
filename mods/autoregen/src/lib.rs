#![allow(non_snake_case)] // crate name is "AutoRegen" to control the output DLL's filename

mod heal;
mod hit_hook;
mod menu;
mod regen;
mod trigger;

use common::{config, dll_dir, logger};

const DEFAULT_INI: &str = include_str!("../AutoRegen.ini");

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
        let ini_path = format!("{dir}\\AutoRegen.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "AutoRegen.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("AutoRegen.ini: added {added_keys} key(s) from the template"));
        }

        // Registers right away: the host only accepts menu registrations at
        // startup.
        menu::start(hmodule, ini_path.clone());

        common::diag::log_environment_when_game_ready();

        // `common::reload` owns the reload key; `regen::run` only follows
        // `RELOAD_GENERATION`.
        std::thread::spawn(move || common::reload::run(ini_path));
        regen::run();
    });

    true
}
