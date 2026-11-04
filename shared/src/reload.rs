//! Watches the reload hotkey and reloads the config when it is pressed, independently of any
//! feature module. [RELOAD_GENERATION] counts reloads for features that must react to one.
//! This must be the only poller of the reload key in a process.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::CSTaskGroupIndex;

use crate::input::{self, parse_virtual_key};
use crate::logger;

const VK_F5: i32 = 0x74;

/// Bumped on every successful reload.
pub static RELOAD_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Watches the reload key on the game's `FrameBegin` task group for the rest of the process's
/// life. Meant to run on its own worker thread spawned from `DllMain`; never returns.
///
/// `reload_key` gives the current key name (read every frame, so the key itself can change on
/// reload) and `reload` re-reads the config. An `Err` is logged and shown in game as
/// "Config error - see log", and leaves [RELOAD_GENERATION] alone. `banner`, read after a
/// successful reload, says whether to show "Config reloaded".
pub fn run<K, B, R>(reload_key: K, banner: B, mut reload: R)
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
                    logger::error(&format!(
                        "Reload: config not reloaded, the previous one stays in use:\n{err}"
                    ));
                    crate::announce::show_announcement("Config error - see log");
                }
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
