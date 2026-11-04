#![allow(non_snake_case)] // crate name is "RiseArcher" to control the output DLL's filename

mod bullet;
mod config;
mod menu;
mod weapon;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository};
use fromsoftware_shared::FromStatic;

use shared::{dll_dir, logger};

/// Set by a reload or a menu save; the tick applies it once the player is in the world.
static APPLY_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_apply() {
    APPLY_REQUESTED.store(true, Ordering::Relaxed);
}

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
                    request_apply();
                    Ok(())
                },
            )
        });

        shared::diag::log_environment_when_game_ready();
        run();
    });

    true
}

/// Waits for the player to be in the world, applies every buff, then applies again on every
/// [request_apply] from the cached original values. The param rows are loaded once at start-up, so
/// the buff needs no per-frame task; the one below only notices requests.
fn run() {
    let repo = shared::player::wait_for_solo_param_repository();

    let Some((weapon_changed, bullet_changed)) = apply_with_retry(repo, Duration::from_secs(30))
    else {
        logger::error("Apply: kept panicking for 30s, disabled for this session");
        return;
    };
    logger::log(&format!("Applied: {weapon_changed} weapon, {bullet_changed} bullet row(s)"));

    let cs_task = shared::task::wait_for_cs_task();
    shared::task::run_recurring_safe(
        cs_task,
        "RiseArcher",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            if !APPLY_REQUESTED.swap(false, Ordering::Relaxed) {
                return;
            }
            // SAFETY: called from a game task; `instance_mut` returns `Err` while the singleton does not exist.
            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                logger::warn("Apply: SoloParamRepository unavailable, change skipped");
                return;
            };
            let weapon_changed = weapon::apply(repo);
            let bullet_changed = bullet::apply(repo);
            logger::log(&format!(
                "Applied: {weapon_changed} weapon, {bullet_changed} bullet row(s) (changed)"
            ));
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

/// Runs both `apply`s, retrying for up to `timeout` if either panics: a param whose res cap has not
/// finished loading panics instead of returning an `Err`. A panic happens before any row is
/// mutated, so a retry is safe.
fn apply_with_retry(repo: &mut SoloParamRepository, timeout: Duration) -> Option<(usize, usize)> {
    let deadline = Instant::now() + timeout;
    loop {
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (weapon::apply(repo), bullet::apply(repo))
        }));
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
