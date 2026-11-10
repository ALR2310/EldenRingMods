# Per-frame tasks

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/task.rs`.

## Question

How does a mod run code every frame on the game's task system, and survive a game update that the
`fromsoftware-rs` address table does not know yet?

## Mechanism

- **`CSTaskImp` comes from its singleton lookup**, polled every 500 ms by `wait_for_cs_task`. It never
  gives up and logs one error after 15 seconds, since the usual cause is a game update, not a slow start.
- **The registration function is found by AOB** (`REGISTER_TASK_PATTERN`) instead of
  `eldenring::rva::get()`. The crate's table holds one game version, and `CSTaskImp::wait_for_instance`
  treats an invalid RVA as fatal, so on a newer build the mod panicked at once.
- **A task is a fake C++ object**: a vtable pointer at offset 0 (built with `vtable-rs`) followed by the
  Rust closure, which the game never looks at. It is leaked on purpose, since a task is never unregistered.
- **`run_recurring_safe` wraps the closure in `catch_unwind`** and skips the frame when it panics. The
  vtable functions `get_runtime_class` and `destructor` are no-ops, not `unimplemented!()`: the engine
  calls them directly and a panic would unwind into its stack. This needs `panic = "unwind"`, the default.

## Groups

A task runs in the `CSTaskGroupIndex` it is registered in. `FrameBegin` is where the main player is safe
to change (`mods/autoregen`), `ChrIns_PreBehavior` runs before the behavior update
(`mods/speedmultiplier`) and `ChrIns_PostPhysics` after it. Tasks of one group run in no fixed order.

## Open risks

If the registration pattern is not found within 60 seconds the feature stays off and one error is logged.
