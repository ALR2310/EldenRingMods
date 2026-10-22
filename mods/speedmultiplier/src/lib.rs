#![allow(non_snake_case)] // crate name is "SpeedMultiplier" to control the output DLL's filename

mod config;
mod enemy;
mod menu;
mod probe;
mod seamless;
mod speed;

use common::{dll_dir, logger};

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
        let report = config::init(&format!("{dir}\\SpeedMultiplier.toml"));

        // `LogFile` gates the log file entirely, see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "SpeedMultiplier.log");
        logger::install_panic_hook();

        // Registers right away: the host only accepts menu registrations at
        // startup.
        menu::start(hmodule);

        report.log("SpeedMultiplier.toml");

        std::thread::spawn(|| {
            common::reload::run_with(
                || config::get().general.reload_key.clone(),
                || config::get().general.reload_banner,
                || {
                    config::reload()?;
                    logger::set_enabled(config::get().logging.log_file);
                    menu::sync_from_config();
                    Ok(())
                },
            )
        });
        std::thread::spawn(speed::run);

        common::diag::log_environment_when_game_ready();
        probe::run();
    });

    true
}
