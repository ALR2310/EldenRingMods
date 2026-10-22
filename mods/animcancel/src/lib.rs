#![allow(non_snake_case)] // crate name is "AnimCancel" to control the output DLL's filename

mod cancel;
mod config;
mod menu;
mod probe;
mod weapon;

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
        let report = config::init(&format!("{dir}\\AnimCancelV1.toml"));

        // [Logging] LogFile gates the log file entirely - see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "AnimCancel.log");
        logger::install_panic_hook();

        // Registers the in-game menu right away: the host only accepts
        // registrations in its first seconds. No-op without ERNativeUI.
        menu::start(hmodule);

        report.log("AnimCancelV1.toml");

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

        common::diag::log_environment_when_game_ready();
        probe::start();
        cancel::run();
    });

    true
}
