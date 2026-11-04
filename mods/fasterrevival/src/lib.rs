#![allow(non_snake_case)] // crate name is "FasterRevival" to control the output DLL's filename

mod config;
mod death;
mod fade;

use shared::{dll_dir, logger};

/// # Safety
/// Called by the loader only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: u64, reason: u32) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason != DLL_PROCESS_ATTACH {
        return true;
    }

    std::thread::spawn(move || {
        let dir = dll_dir(hmodule);
        let name = shared::dll_stem(hmodule);
        let report = config::init(&format!("{dir}\\{name}.toml"));

        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, &format!("{name}.log"));
        logger::install_panic_hook();

        report.log(&format!("{name}.toml"));

        shared::diag::log_environment_when_game_ready();
        death::run();
    });

    true
}
