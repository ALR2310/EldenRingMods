#![allow(non_snake_case)] // crate name is "JourneyHud" to control the output DLL's filename

mod config;
mod map;
mod menu;
mod radar;
mod radar_ui;
mod settings;
mod target;
mod textures;
mod ui;

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
        let report = config::init(&format!("{dir}\\JourneyHud.toml"));

        // `LogFile` gates the log file entirely, see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "JourneyHud.log");
        logger::install_panic_hook();
        report.log("JourneyHud.toml");

        settings::load(&config::get());
        // Early: the menu host only takes registrations in the first seconds.
        menu::start(hmodule);
        std::thread::spawn(|| {
            common::reload::run_with(
                || config::get().general.reload_key.clone(),
                || config::get().general.reload_banner,
                || {
                    config::reload()?;
                    logger::set_enabled(config::get().logging.log_file);
                    settings::load(&config::get());
                    menu::sync_from_config();
                    Ok(())
                },
            )
        });
        ui::install();
        common::diag::log_environment_when_game_ready();
        target::run();
    });

    true
}
