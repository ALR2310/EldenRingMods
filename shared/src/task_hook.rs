//! Registers a per-frame task without `fromsoftware-rs`'s version-gated
//! `eldenring::rva::get()`: the game's task-registration function is found by
//! AOB and called with a fake native task object. Why and how:
//! `shared/docs/task.md`.

use std::ffi::c_void;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, CSTaskImp};
use eldenring::fd4::FD4TaskData;
use vtable_rs::VPtr;

use crate::logger;
use crate::memscan::{self, CachedAddr};

// Resolved once per process and shared by every task a mod registers.
static REGISTER_TASK_ADDR: CachedAddr = CachedAddr::new();

const REGISTER_TASK_PATTERN: &str = "48 89 5c 24 08 57 48 83 ec 40 48 8d 4c 24 20 49 8b d8 8b fa e8 ?? ?? ?? ?? 48 8b 0d ?? ?? ?? ?? 48 85 c9 75 2e 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 4c 8b c8 4c 8d 05 ?? ?? ?? ?? ba b4 00 00 00 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 48 8b 0d";

#[vtable_rs::vtable]
trait TaskVmt {
    fn get_runtime_class(&self) -> usize;
    fn destructor(&mut self);
    fn execute(&mut self, data: *const c_void);
}

/// The game only reads a vtable pointer at offset 0; the closure after it is
/// opaque to the game and only reached through `execute`.
#[repr(C)]
struct Task {
    vftable: VPtr<dyn TaskVmt, Self>,
    closure: Box<dyn FnMut(&FD4TaskData) + Send>,
}

impl TaskVmt for Task {
    // Harmless no-ops, not `unimplemented!()`: a panic here would unwind into
    // the game's own frames if it ever calls them directly (see the docs).
    extern "C" fn get_runtime_class(&self) -> usize {
        0
    }

    extern "C" fn destructor(&mut self) {}

    extern "C" fn execute(&mut self, data: *const c_void) {
        (self.closure)(unsafe { &*(data as *const FD4TaskData) });
    }
}

/// Registers `closure` on `group`, like `CSTaskImp::run_recurring` but with the
/// registration function found by AOB. Returns `false` (logged, closure never
/// installed) if the pattern isn't found within 60s; treat it like any other
/// AOB feature failing to find its anchor.
///
/// The task is never unregistered: it lives for the DLL's lifetime, so its
/// allocation is leaked on purpose.
pub fn run_recurring<F>(cs_task: &'static CSTaskImp, group: CSTaskGroupIndex, closure: F) -> bool
where
    F: FnMut(&FD4TaskData) + 'static + Send,
{
    let Some(register_task_addr) = REGISTER_TASK_ADDR.get_or_resolve(|| {
        let addr = memscan::wait_for_pattern_in_module(
            REGISTER_TASK_PATTERN,
            Duration::from_millis(500),
            Duration::from_secs(60),
        )?;
        logger::log("register_task AOB found.");
        Some(addr as usize)
    }) else {
        logger::error("Could not locate the game's task-registration function (AOB pattern not found) - task NOT installed.");
        return false;
    };

    let task: &'static Task = Box::leak(Box::new(Task {
        vftable: Default::default(),
        closure: Box::new(closure),
    }));

    let register_task: extern "C" fn(&'static CSTaskImp, CSTaskGroupIndex, &'static Task) =
        unsafe { std::mem::transmute(register_task_addr) };
    register_task(cs_task, group, task);
    true
}
