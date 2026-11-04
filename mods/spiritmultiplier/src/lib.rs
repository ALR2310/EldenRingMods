#![allow(non_snake_case)] // crate name is "SpiritMultiplier" to control the output DLL's filename

mod activate_limit;
mod band;
mod buddy;
mod chain;
mod config;
mod conflicts;
mod ghost_color;
mod keep_on_rest;
mod menu;
mod mimic;
mod multi_spirit;
mod regen;
mod think;
mod warp;

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
        conflicts::init(&name);

        // The menu host accepts registrations only in its first seconds. The mod reads every setting
        // but `MaxSpirits` live.
        menu::start(hmodule);

        report.log(&format!("{name}.toml"));

        // The band comes first: without it, chains longer than 10 are harmless, since the engine
        // drops the extra spirits. `MaxSpirits` sizes the summon ChrSet, so it is read once here and
        // applies from the next game start.
        let max_spirits = config::get().spirit.max_spirits.max(0) as u32;
        if !band::install(max_spirits) {
            logger::error("Band: patch not installed, spirits stay capped at 10 per cast");
        }

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
        std::thread::spawn(keep_on_rest::run);
        std::thread::spawn(activate_limit::run);
        std::thread::spawn(ghost_color::run);
        std::thread::spawn(mimic::run);
        std::thread::spawn(regen::run);
        std::thread::spawn(multi_spirit::run);
        std::thread::spawn(buddy::hud::run);
        std::thread::spawn(warp::run);
        std::thread::spawn(think::run);
        std::thread::spawn(buddy::stone::run);

        shared::diag::log_environment_when_game_ready();
        chain::run();
    });

    true
}
