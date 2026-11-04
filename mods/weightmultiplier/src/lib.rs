#![allow(non_snake_case)] // crate name is "WeightMultiplier" to control the output DLL's filename

mod config;
mod hook;
mod menu;

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

        // The menu host accepts registrations only in its first seconds, before `hook::run`
        // sleeps for `LoadDelay`.
        menu::start(hmodule);

        report.log(&format!("{name}.toml"));

        hook::run(&name);
    });

    true
}
