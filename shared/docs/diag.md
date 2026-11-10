# Environment dump and module lookups

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/diag.rs`.

## Question

What does a bug report need to show about the game build and the other mods, without leaking the user's
folders?

## Mechanism

- `log_environment_when_game_ready()` waits for `CSTaskImp` (by then every mod loader has loaded its
  DLLs) and logs the game executable and every loaded module that is not a system one, with version, base
  and size. Call it before a feature starts, so a "mod never started" report still has the environment.
- Logs are posted publicly, so a path outside the game folder is never printed in full, only the file
  name. Modules shipped with the game or its platform are counted but hidden (`HIDDEN_MODULES`).
- `is_module_loaded(name)` is one `GetModuleHandleW` call. `module_containing(addr)` names the module whose
  image holds an address, for example to say which DLL a foreign `jmp` points into. `loaded_modules()` lists
  everything and reads each file's version resource, so use the other two for a single question.
