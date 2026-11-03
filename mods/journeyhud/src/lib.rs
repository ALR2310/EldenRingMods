#![allow(non_snake_case)] // crate name is "JourneyHUD" to control the output DLL's filename

mod config;
mod drag;
mod enemies;
mod map;
mod menu;
mod radar;
mod radar_ui;
mod settings;
mod state_icons;
mod target;
mod text;
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
        let report = config::init(&format!("{dir}\\JourneyHUD.toml"));

        // `LogFile` gates the log file entirely, see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "JourneyHUD.log");
        logger::install_panic_hook();
        report.log("JourneyHUD.toml");

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
                    ui::apply_hdr();
                    menu::sync_from_config();
                    Ok(())
                },
            )
        });
        state_icons::install();
        state_icons::install_gauge();
        // The overlay hooks the game's DX12 swap chain; other mods and overlays hook it too, so
        // `LoadDelay` can hold this back (its own thread: the rest of the mod does not wait).
        match config::get().general.load_delay {
            0 => ui::install(),
            ms => {
                logger::log(&format!("Overlay: waiting {ms} ms before hooking"));
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(u64::from(ms)));
                    ui::install();
                });
            }
        }
        common::diag::log_environment_when_game_ready();
        target::run();
    });

    true
}
