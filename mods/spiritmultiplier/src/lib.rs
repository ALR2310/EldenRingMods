#![allow(non_snake_case)] // crate name is "SpiritMultiplier" to control the output DLL's filename

mod activate_limit;
mod band;
mod buddy_stone;
mod chain;
mod conflicts;
mod effect_probe;
mod enemy_probe;
mod ghost_color;
mod menu;
mod mimic;
mod multi_spirit;
mod probe;
mod regen;
mod think_override;
mod warp;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../SpiritMultiplier.ini");

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
        let ini_path = format!("{dir}\\SpiritMultiplier.ini");
        let added_keys = config::load_or_create_default_with_renames(&ini_path, DEFAULT_INI, &[("Regen", "RegenValue")]);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "SpiritMultiplier.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("SpiritMultiplier.ini: added {added_keys} key(s) from the template"));
        }

        // Registers right away: the host only accepts menu registrations at
        // startup. Every setting but MaxSpirits is read live by the mod.
        menu::start(hmodule, ini_path.clone());

        // Band first: without it, chains longer than 10 are harmless (the engine
        // drops the extra spirits). `MaxSpirits` sizes the summon ChrSet, so it is
        // read once here and applies from the next game start.
        let max_spirits = config::get_int("MaxSpirits", band::DEFAULT_MAX_SPIRITS as i32).max(0) as u32;
        if !band::install(max_spirits) {
            logger::error("Band: patch not installed, spirits stay capped at 10 per cast");
        }

        std::thread::spawn(move || common::reload::run(ini_path));
        std::thread::spawn(probe::run);
        std::thread::spawn(enemy_probe::run);
        std::thread::spawn(effect_probe::run);
        std::thread::spawn(activate_limit::run);
        std::thread::spawn(ghost_color::run);
        std::thread::spawn(mimic::run);
        std::thread::spawn(regen::run);
        std::thread::spawn(multi_spirit::run);
        std::thread::spawn(warp::run);
        let think_dir = dir.clone();
        std::thread::spawn(move || think_override::run(think_dir));
        std::thread::spawn(buddy_stone::run);

        common::diag::log_environment_when_game_ready();
        chain::run();
    });

    true
}

