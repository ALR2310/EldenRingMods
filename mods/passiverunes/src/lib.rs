#![allow(non_snake_case)] // crate name is "PassiveRunes" to control the output DLL's filename

mod config;
mod math;
mod menu;
mod rune;

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
        let path = format!("{dir}\\{name}.toml");
        config::adopt_ini(&path, &format!("{dir}\\{name}.ini"));
        let report = config::init(&path);

        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, &format!("{name}.log"));
        logger::install_panic_hook();

        // The menu host accepts registrations only in its first seconds.
        menu::start(hmodule);

        report.log(&format!("{name}.toml"));

        std::thread::spawn(|| {
            shared::reload::run(
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

        shared::diag::log_environment_when_game_ready();
        rune::run();
    });

    true
}
