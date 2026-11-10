# Editing a Scaleform movie in memory

Status: **Confirmed** in game on 2.7.2.0 by `mods/spiritmultiplier` (HP bar rows added to the HUD movie, with
another mod hooking the same file opener). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/gfx/mod.rs`, `shared/src/gfx/hook.rs`.

## Question

How can a mod change a menu movie (`.gfx`) without shipping the file, and without fighting other mods that
touch the same loader?

## Pieces

| Function | Use |
|---|---|
| `gfx::extend_numbered_placements(movie, path, prefix, count)` | Walks the instance names `path` (from the stage down) to a sprite and makes it hold `prefix0 ..= prefix{count-1}` by copying its last numbered placement. Returns `Ok(None)` when the rows already exist. |
| `gfx::hook::install(file_name, edit)` | When the game opens the movie whose path ends in `file_name`, replaces its bytes by `edit(original)` when that is `Some`. One movie per DLL. |

## Movie edit (`gfx`)

The `.gfx` is the uncompressed SWF layout: a header, then tags. The parser reads the tag list, finds the named
sprite through its `PlaceObject2` tags, and inserts new `PlaceObject2` tags (same character id and scale, each
a further step in the position difference of the last two rows, depth above the highest existing) before the
sprite's first `ShowFrame`, so the rows exist in frame 1. Errors (`GfxError`) mean "leave the movie alone":
not a `.gfx`, truncated, path not found, fewer than two numbered rows to copy, an unsupported placement.

## Hook (`gfx::hook`)

- The game's file opener is reached only through a function-pointer slot in the executable's data, so the
  slot is repointed to a detour (`memscan::find_data_pointers`) instead of hooking the function's entry. A
  third-party UI library hooks the entry and overwrites its first 14 bytes; a pattern that began at the entry
  stopped matching once that library had loaded (log `file opener pattern not found`), so `OPENER_AOB`
  starts at the 17th byte (`OPENER_OFFSET`) and both approaches coexist.
- The detour calls the original, then, if the path ends in the target name, swaps the in-memory file object's
  byte pointer and length (`+0x18`, `+0x20`). It first checks the object is readable and unused (`refcount`
  non-zero, cursor 0, valid flag 1, length 12 bytes to 64 MiB). Otherwise it logs
  `left unchanged, unexpected file object` and leaves the movie as is.
- The edited bytes are built once (under a mutex), leaked for the process lifetime because Scaleform can keep
  the pointer as long as the movie lives, and reused on every open.
- A panic in the edit is caught and never unwinds into the game.
- Install waits for the pattern (up to 60 s), because the module may still be unpacking when a mod loads.
  `false` means nothing was changed.

## Open risks

- The opener pattern was checked unique on 2.6.2 to 2.7.2 by an IDA script, not by a run on each build.
- Only the movie loaded through that opener is covered; a movie loaded another way is not.
