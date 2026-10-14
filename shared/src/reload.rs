//! Watches a hotkey (`ReloadKey` by convention) and reloads the config on
//! press, independent of any feature module. [RELOAD_GENERATION] counts
//! reloads for features that must react to the reload edge. This must be the
//! only poller of the reload key in a process. Details: `shared/docs/reload.md`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::CSTaskGroupIndex;
use eldenring::util::input;

use crate::config;
use crate::input::parse_virtual_key;
use crate::logger;

const VK_F5: i32 = 0x74;

/// Bumped on every successful reload.
pub static RELOAD_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Watches `ReloadKey` on the game's `FrameBegin` task group for the rest of
/// the process's life: on a press it reloads `ini_path` into the shared config
/// map, bumps [RELOAD_GENERATION] and, if `ReloadBanner` is on, shows a
/// "Config reloaded" banner. Meant to run on its own worker thread spawned
/// from `DllMain`; never returns.
pub fn run(ini_path: String) {
    run_with(
        || config::get_string("ReloadKey", "F5"),
        || config::get_bool("ReloadBanner", false),
        move || {
            config::load(&ini_path);
            Ok(())
        },
    );
}

/// [run] for a config that isn't the ini map ([`crate::toml_config`]).
/// `reload_key` gives the current key name (re-read every frame), `reload`
/// re-reads the config; an `Err` is logged and shown as "Config error - see
/// log" (always, or a failed reload would be silent in game) and leaves
/// [RELOAD_GENERATION] alone. `banner`, read after the reload, says whether
/// to show "Config reloaded".
pub fn run_with<K, B, R>(reload_key: K, banner: B, mut reload: R)
where
    K: Fn() -> String + Send + 'static,
    B: Fn() -> bool + Send + 'static,
    R: FnMut() -> Result<(), String> + Send + 'static,
{
    let cs_task = crate::task::wait_for_cs_task();

    crate::task::run_recurring_safe(
        cs_task,
        "Reload",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let key = parse_virtual_key(&reload_key(), VK_F5);
            if !input::is_key_pressed(key) {
                return;
            }
            match reload() {
                Ok(()) => {
                    RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
                    logger::log("Config reloaded");
                    if banner() {
                        crate::announce::show_announcement("Config reloaded");
                    }
                }
                Err(err) => {
                    logger::error(&format!("Reload: config not reloaded, the previous one stays in use:\n{err}"));
                    crate::announce::show_announcement("Config error - see log");
                }
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
