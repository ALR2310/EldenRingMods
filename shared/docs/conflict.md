# Detecting another mod's hook

Status: **Inferred**: the byte shapes are unit-tested, and the code shipped in a released mod. Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/conflict.rs`.

## Question

If another mod has already patched the code we want to hook, how does the mod find out and tell the
player, instead of crashing or failing silently?

## Mechanism

`foreign_hook_at(site)` reads the first 13 bytes at a patch site and recognises the shapes mods hook with:
`E9 rel32`, `FF 25` through a pointer, `mov r64, imm64; jmp r64` (12 bytes) and the `41 FF E0+r` form
(13). It is a foreign hook when the jump leaves the game executable; a jump into the game is the game's
own code. Only hooks installed **before** ours are seen.

The owner is the module that contains the destination. A destination in memory the other DLL allocated
belongs to no module, so the first 128 bytes of the stub are searched for a pointer or an
`E8`/`E9 rel32` into a non-game, non-system module. That is a guess and is reported as one.

`report(mod_name, feature, log_file, hook)` writes an ERROR line naming the DLL and queues the feature
for **one message box per mod**. Conflicts reported within two seconds share the box, and the box is shown
on its own thread so a modal dialog never stalls the game or holds the loader lock.

## Use

Scan for the full pattern, then for the pattern minus the replaced prologue: a tail that matches while the
full pattern does not means the entry was patched (`scan` in `mods/autopickup/src/hook.rs`). Call
`foreign_hook_at` on that entry and, if it returns a hook, `report` it and leave the feature off.

## Open risks

A mod that installs its hook after ours is not seen.
