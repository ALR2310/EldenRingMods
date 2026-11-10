# Reload hotkey

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/reload.rs`, `shared/src/input.rs`.

## Question

How does a mod re-read its config on a key press without restarting the game?

## Mechanism

`reload::run(key, banner, reload)` registers a task on `FrameBegin` that tests the key every frame:

- `key` returns the key name each frame, so the key itself can change on reload. A name (`"F5"`), a hex
  string (`"0x74"`) or a decimal number is accepted; anything else falls back to F5
  (`input::parse_virtual_key`).
- On success it logs `Reload: done`, bumps `RELOAD_GENERATION` and shows "Config reloaded" when `banner`
  says so. On error it logs the reason, **keeps the previous config** and shows "Config error - see log";
  the error banner is never optional, or a broken file would fail silently.

The `reload` closure of a mod re-reads the file, then does what follows from it (`logger::set_enabled`,
`menu::sync_from_config`). It runs on a game task, so keep it short.

`run` never returns: spawn it on its own thread from `DllMain`. Each DLL has its own copy and its own
key, so two mods with the same key each reload their own file.
