//! Per-frame tasks without `eldenring::rva::get()`: `CSTaskImp` comes from its singleton lookup and
//! the game's task-registration function is found by AOB.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp};
use eldenring::fd4::FD4TaskData;
use fromsoftware_shared::FromStatic;
use vtable_rs::VPtr;

use crate::logger;
use crate::memscan::{self, CachedAddr};

// `CSTaskImp` never moves once constructed; cached so several tasks wait and log once.
static CS_TASK: CachedAddr = CachedAddr::new();

static REGISTER_TASK_ADDR: CachedAddr = CachedAddr::new();

const WARN_AFTER: Duration = Duration::from_secs(15);

const REGISTER_TASK_PATTERN: &str = "48 89 5c 24 08 57 48 83 ec 40 48 8d 4c 24 20 49 8b d8 8b fa e8 ?? ?? ?? ?? 48 8b 0d ?? ?? ?? ?? 48 85 c9 75 2e 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 4c 8b c8 4c 8d 05 ?? ?? ?? ?? ba b4 00 00 00 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 48 8b 0d";

#[vtable_rs::vtable]
trait TaskVmt {
    fn get_runtime_class(&self) -> usize;
    fn destructor(&mut self);
    fn execute(&mut self, data: *const c_void);
}

/// The game only reads a vtable pointer at offset 0; the closure after it is opaque to it.
#[repr(C)]
struct Task {
    vftable: VPtr<dyn TaskVmt, Self>,
    closure: Box<dyn FnMut(&FD4TaskData) + Send>,
}

impl TaskVmt for Task {
    // No-ops, not `unimplemented!()`: a panic would unwind into the game's own frames.
    extern "C" fn get_runtime_class(&self) -> usize {
        0
    }

    extern "C" fn destructor(&mut self) {}

    extern "C" fn execute(&mut self, data: *const c_void) {
        // SAFETY: the game passes a valid `FD4TaskData` pointer for the duration of the call.
        (self.closure)(unsafe { &*(data as *const FD4TaskData) });
    }
}

/// Polls for `CSTaskImp` every 500 ms and never gives up; logs one error after `WARN_AFTER`
/// (usually the game needs a mod update, not just a slow start).
pub fn wait_for_cs_task() -> &'static CSTaskImp {
    let addr = CS_TASK.get_or_resolve(|| Some(poll_cs_task() as *const CSTaskImp as usize));
    // SAFETY: the cached address came from a `&'static CSTaskImp`.
    unsafe { &*(addr.expect("poll_cs_task never returns None") as *const CSTaskImp) }
}

fn poll_cs_task() -> &'static CSTaskImp {
    let start = Instant::now();
    let mut warned = false;
    let mut waiting_logged = false;
    loop {
        // SAFETY: `instance` returns `Err` until the game has created the singleton.
        if let Ok(instance) = unsafe { CSTaskImp::instance() } {
            logger::log("CSTaskImp: ready");
            return instance;
        }
        if start.elapsed() < WARN_AFTER {
            if !waiting_logged {
                waiting_logged = true;
                logger::log("CSTaskImp: waiting");
            }
        } else if !warned {
            warned = true;
            logger::error(&format!(
                "CSTaskImp: not found after {}s - game may need a mod update, check Nexus.",
                WARN_AFTER.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Like `CSTaskImp::run_recurring` but with the registration function found by AOB; `false`
/// (logged) if the pattern isn't found within 60 s. The task is never unregistered, so its
/// allocation is leaked on purpose.
fn register<F>(cs_task: &'static CSTaskImp, group: CSTaskGroupIndex, closure: F) -> bool
where
    F: FnMut(&FD4TaskData) + 'static + Send,
{
    let Some(register_task_addr) = REGISTER_TASK_ADDR.get_or_resolve(|| {
        let addr = memscan::wait_for_pattern_in_module(
            REGISTER_TASK_PATTERN,
            Duration::from_millis(500),
            Duration::from_secs(60),
        )?;
        logger::log("register_task: ready");
        Some(addr as usize)
    }) else {
        logger::error(
            "register_task: pattern not found - game may need a mod update, check Nexus.",
        );
        return false;
    };

    let task: &'static Task =
        Box::leak(Box::new(Task { vftable: Default::default(), closure: Box::new(closure) }));

    // SAFETY: `register_task_addr` was found by `REGISTER_TASK_PATTERN` and has this signature.
    let register_task: extern "C" fn(&'static CSTaskImp, CSTaskGroupIndex, &'static Task) =
        unsafe { std::mem::transmute(register_task_addr) };
    register_task(cs_task, group, task);
    true
}

/// Registers `f` as a recurring task, skipping a frame in which it panics instead of unwinding
/// into the game's stack (`tag` names the feature in the log). `false` if registration failed.
/// Needs `panic = "unwind"`.
pub fn run_recurring_safe<F>(
    cs_task: &'static CSTaskImp,
    tag: &'static str,
    group: CSTaskGroupIndex,
    mut f: F,
) -> bool
where
    F: FnMut(&FD4TaskData) + 'static + Send,
{
    let installed = register(cs_task, group, move |data: &FD4TaskData| {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(data))).is_err() {
            logger::error(&format!("{tag}: tick panicked, frame skipped"));
        }
    });
    if installed {
        logger::log(&format!("{tag}: task registered"));
    }
    installed
}
