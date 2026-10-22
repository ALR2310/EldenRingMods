#![allow(non_snake_case)] // crate name is "RiseArcher" to control the output DLL's filename

mod bullet;
mod menu;
mod weapon;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use common::{config, dll_dir, logger};

// The default config, embedded verbatim: the single source of truth for keys
// and defaults.
const DEFAULT_INI: &str = include_str!("../RiseArcher.ini");

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
        let ini_path = format!("{dir}\\RiseArcher.ini");
        let added_keys = config::load_or_create_default(&ini_path, DEFAULT_INI);

        // `LogFile` gates the log file entirely, see common::logger.
        logger::init(&dir, "RiseArcher.log");
        logger::install_panic_hook();
        if added_keys > 0 {
            logger::log(&format!("RiseArcher.ini: added {added_keys} key(s) from the template"));
        }

        // The menu registers right away (the host only accepts menu
        // registrations at startup); its changes reach `run` through
        // `RELOAD_GENERATION`, which `common::reload` bumps (it owns the reload key).
        menu::start(hmodule, ini_path.clone());
        std::thread::spawn(move || common::reload::run(ini_path));

        common::diag::log_environment_when_game_ready();
        run();
    });

    true
}

/// Waits for the player to be in the world, applies every buff, then follows
/// `RELOAD_GENERATION` for the rest of the DLL's life, reapplying from the
/// cached original values. Regulation rows are loaded once at startup, so the
/// buff itself needs no per-frame task; the one below only notices reloads.
fn run() {
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    let repo = common::player::wait_for_solo_param_repository();

    let Some((weapon_changed, bullet_changed)) = apply_with_retry(repo, Duration::from_secs(30)) else {
        logger::error("Apply: kept panicking for 30s, disabled for this session");
        return;
    };
    logger::log(&format!("Applied: {weapon_changed} weapon, {bullet_changed} bullet row(s)"));

    let cs_task = common::task::wait_for_cs_task();
    let _handle = common::task::run_recurring_safe(
        cs_task,
        "RiseArcher",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation == last_seen_generation {
                return;
            }
            last_seen_generation = generation;
            menu::sync_from_config();

            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                logger::warn("Reload: SoloParamRepository unavailable, skipped");
                return;
            };
            let weapon_changed = weapon::apply(repo);
            let bullet_changed = bullet::apply(repo);
            logger::log(&format!("Reload: {weapon_changed} weapon, {bullet_changed} bullet row(s)"));
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

/// Runs both `apply`s, retrying for up to `timeout` if either panics: a param
/// whose res cap hasn't finished loading panics instead of returning an
/// `Err`. A panic happens before any row is mutated, so a retry is safe.
fn apply_with_retry(repo: &mut SoloParamRepository, timeout: Duration) -> Option<(usize, usize)> {
    let deadline = Instant::now() + timeout;
    loop {
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (weapon::apply(repo), bullet::apply(repo))));
        match attempt {
            Ok(counts) => return Some(counts),
            Err(_) => logger::warn("Apply: panicked, retrying"),
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
