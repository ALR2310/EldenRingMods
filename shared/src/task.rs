//! `CSTaskImp` acquisition and a panic-safe wrapper for recurring per-frame
//! tasks, both resolved without `fromsoftware-rs`'s version-gated
//! `eldenring::rva::get()`. Why: `shared/docs/task.md`.

use std::time::{Duration, Instant};

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::FromStatic;

use crate::logger;
use crate::memscan::CachedAddr;
use crate::task_hook;

// `CSTaskImp` never moves once constructed. Cached so a mod registering
// several tasks (`common::reload` + its own feature) waits once and logs
// "found" once instead of every thread polling on its own.
static CS_TASK: CachedAddr = CachedAddr::new();

const WARN_AFTER: Duration = Duration::from_secs(15);

/// Polls for `CSTaskImp` (the game's per-frame task scheduler) via its
/// name-based singleton lookup, retrying every 500ms. Logs one error if it
/// takes longer than `WARN_AFTER` (usually the game needs a mod update, not
/// just a slow start). Never gives up.
pub fn wait_for_cs_task() -> &'static CSTaskImp {
    let addr = CS_TASK.get_or_resolve(|| Some(poll_cs_task() as *const CSTaskImp as usize));
    // `poll_cs_task` never gives up, so the cache is always filled here.
    unsafe { &*(addr.expect("poll_cs_task never returns None") as *const CSTaskImp) }
}

fn poll_cs_task() -> &'static CSTaskImp {
    let start = Instant::now();
    let mut warned = false;
    loop {
        if let Ok(instance) = unsafe { CSTaskImp::instance() } {
            logger::log("CSTaskImp found.");
            return instance;
        }
        if start.elapsed() < WARN_AFTER {
            logger::log("CSTaskImp not ready yet, retrying in 500ms...");
        } else if !warned {
            warned = true;
            logger::error(&format!(
                "CSTaskImp not found after {}s - game may need a mod update, check Nexus.",
                WARN_AFTER.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Registers `f` as a recurring task via [task_hook::run_recurring], but
/// catches a panic `f` raises in a frame instead of letting it unwind into the
/// game's own call stack: `f` is just skipped for that frame (`tag` names the
/// feature in the log). Returns `false` if the registration itself failed
/// (AOB not found). Needs `panic = "unwind"`: `catch_unwind` catches nothing
/// once a panic aborts the process.
pub fn run_recurring_safe<F>(cs_task: &'static CSTaskImp, tag: &'static str, group: CSTaskGroupIndex, mut f: F) -> bool
where
    F: FnMut(&FD4TaskData) + 'static + Send,
{
    let installed = task_hook::run_recurring(cs_task, group, move |data: &FD4TaskData| {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(data))).is_err() {
            logger::error(&format!("{tag}: tick panicked, skipped this frame."));
        }
    });
    if installed {
        logger::log(&format!("{tag}: task registered."));
    }
    installed
}
