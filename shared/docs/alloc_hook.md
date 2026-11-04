# The game's heap allocator

Status: **Confirmed** in released mods of the old repository (used by the announcement banner); not yet
re-tested in game from this one. Conventions: [research notes](../../docs/research/README.md). Code:
`shared/src/alloc_hook.rs`.

## Question

Some engine strings (`DLString`) must be allocated with the game's own runtime heap, or the game frees them
with the wrong allocator. How does a mod reach that allocator without `eldenring::rva::get()`, which only has
a table for some builds?

## Mechanism

`runtime_heap_allocator()` returns `Option<&'static DLAllocator>`:

1. A pattern (`48 83 ec 28 e8 ?? ?? ?? ?? 48 8b 05 ?? ?? ?? ?? 48 85 c0 75 0c e8 ...`) finds the **getter** of
   the allocator global. Inside it a `mov rax,[rip+disp32]` at offset 9 loads the global; the address of that
   global is `instruction end + disp32` (3-byte opcode, 4-byte displacement, 7-byte instruction).
2. The address of the global (not its value) is cached in a `CachedAddr`. The allocator may not exist yet, so a
   null instance is **not** cached: the value is read again on the next call.
3. A non-null pointer is returned as a `'static` reference, because once the game has set it the allocator
   lives for the rest of the process.

Failures are logged (`AllocHook: getter pattern not found`, `AllocHook: instance still null`) and return
`None`; callers do nothing then (see [announce.md](announce.md)).

## Open risks

- The pattern must match exactly once on every supported build (check it with
  `scripts/ida/run.ps1 aob -Version all`). A game update that changes the getter makes the lookup
  return `None`, never a wrong pointer.
