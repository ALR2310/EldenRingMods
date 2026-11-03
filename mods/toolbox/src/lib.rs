#![allow(non_snake_case)] // crate name is "Toolbox" to control the output DLL's filename

mod buildup;
mod config;
mod input;
mod map_anywhere;
mod patch;
mod resize;
mod speffect;
mod torrent_anywhere;
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
        let report = config::init(&format!("{dir}\\Toolbox.toml"));

        // `LogFile` gates the log file entirely, see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "Toolbox.log");
        logger::install_panic_hook();
        report.log("Toolbox.toml");

        let startup = config::get();
        ui::record_startup_state(startup.map_anywhere.enabled, startup.torrent_anywhere.enabled);
        // Its own thread: it waits `LoadDelay` before hooking, the features below do not.
        std::thread::spawn(ui::install);

        // Each feature blocks its own thread (AOB retries, task registration),
        // so give each one its own. To add a feature: new module with a
        // `run()`, one line here, one section in Toolbox.toml (and a widget in
        // `ui.rs` if it should appear in the GUI).
        std::thread::spawn(map_anywhere::run);
        std::thread::spawn(torrent_anywhere::run);
        std::thread::spawn(buildup::run);
        std::thread::spawn(resize::run);
        if startup.input_bridge.enabled {
            let dir = dir.clone();
            std::thread::spawn(move || input::run(&dir));
        }

        common::reload::run_with(
            || config::get().general.reload_key.clone(),
            || config::get().general.reload_banner,
            || {
                config::reload()?;
                logger::set_enabled(config::get().logging.log_file);
                Ok(())
            },
        );
    });

    true
}
