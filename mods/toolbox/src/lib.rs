#![allow(non_snake_case)] // crate name is "Toolbox" to control the output DLL's filename

mod buildup;
mod map_anywhere;
mod patch;
mod speffect;
mod torrent_anywhere;
mod ui;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../Toolbox.ini");

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
        let ini_path = format!("{dir}\\Toolbox.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        logger::init(&dir, "Toolbox.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("Toolbox.ini: added {added_keys} key(s) from the template"));
        }

        ui::record_startup_state(
            config::get_bool("MapAnywhere", true),
            config::get_bool("TorrentAnywhere", true),
        );
        let ui_ini_path = ini_path.clone();
        std::thread::spawn(move || ui::install(ui_ini_path));

        // Each feature blocks its own thread (AOB retries, task registration),
        // so give each one its own. To add a feature: new module with a
        // `run()`, one line here, one section in Toolbox.ini (and a widget in
        // `ui.rs` if it should appear in the GUI).
        std::thread::spawn(map_anywhere::run);
        std::thread::spawn(torrent_anywhere::run);
        std::thread::spawn(buildup::run);

        common::reload::run(ini_path);
    });

    true
}
