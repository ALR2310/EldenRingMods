#![allow(non_snake_case)] // crate name is "SoulsTeleport" to control the output DLL's filename

mod config;
mod net;
mod party;
mod steam;
mod ui;
mod warp;
mod wire;

use shared::{dll_dir, logger};

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
        let name = shared::dll_stem(hmodule);
        let path = format!("{dir}\\{name}.toml");
        config::adopt_ini(&path, &format!("{dir}\\{name}.ini"));
        let report = config::init(&path);

        // `LogFile` gates the log file entirely, see shared::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, &format!("{name}.log"));
        logger::install_panic_hook();
        report.log(&format!("{name}.toml"));

        ui::install();

        shared::diag::log_environment_when_game_ready();
        warp::run();
    });

    true
}
