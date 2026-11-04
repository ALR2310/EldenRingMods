# Logging

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/logger.rs`.

## Mechanism

- **Off by default.** `set_enabled(config.logging.log_file)` after loading and after each reload. While off,
  no file is created; the first line after switching on truncates the log of the previous run.
- `init(dir, file_name)` only records the path. Build `file_name` from `shared::dll_stem` rather than
  spelling `<Mod>.log` out (see "Antivirus detections" in [CONTRIBUTING.md](../../CONTRIBUTING.md)).
- Levels: `log` (INFO), `warn` (recovered or retrying) and `error` (a feature failed and stays off).
- `install_panic_hook()` logs thread, location and message of any panic. Without it a panic on a worker
  thread ends the thread and the log just stops.
- A poisoned lock is still used: logging must never panic, least of all from the panic hook.

## Style

One short line, `Component: state`, for example `Hook: installed` or `Config: PerTick.HP: 0.5 -> 1`. Log a
change, not a state that repeats every frame.

## In a hook

Do not log from a game hook: it takes a lock and writes a file, and file I/O inside hit resolution can
disturb tightly timed animations. Count in atomics and let a task write the line (`take_restored` in
`mods/autoregen/src/hit_hook.rs`).
