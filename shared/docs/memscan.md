# Memory scanning

Status: **Confirmed** in released mods and in unit tests. Conventions:
[research notes](../../docs/research/README.md). Code: `shared/src/memscan.rs`.

## Question

How does a mod find game code and data by pattern, safely, on several builds?

## Pieces

| Function | Use |
|---|---|
| `find_pattern_in_module(pattern)` | First match of an IDA-style AOB (`"48 8B 05 ?? ?? ?? ?? 48 85 C0"`, `?` or `??` wildcard) in the executable sections of the main module. |
| `wait_for_pattern_in_module(pattern, interval, timeout)` | Same, retried until the module has unpacked. |
| `find_jumps_to(target)` | Every `jmp rel32` in executable sections that lands on `target`: the tail jump of a small wrapper into a function. |
| `read_safe::<T>(address)` | Reads a `T` with `ReadProcessMemory`, `None` when unreadable: the way to walk a pointer chain that is not checked on every build. |
| `CachedAddr` | Caches one resolved address (a hit is kept, a miss is retried by the next caller) behind a lock. |
| `find_data_pointers(value)` | Every 8-byte-aligned slot in the non-executable sections holding `value`: where a function address sits in a vtable or call table. |

A pattern must match exactly once on every supported build (see the research notes); a pattern that matches
twice is unresolved.

## `find_data_pointers` and guard pages

Scanning a data section by its size crashed the game: a section can contain guard or no-access pages that
fault when touched. The scan walks it page by page, asks `conflict::readable_len` for how much is committed
and readable at the current address, reads only that span and skips to the next page otherwise. A test
checks that a pointer stored in the test binary's own data is found and a missing value is not.

## `find_jumps_to`

Only the displacement of an `E9` is compared (`base + i + 5 + rel == target`), so a data byte `E9` that
happens to land on the target would be reported too. Callers treat the result as candidates and check the
bytes before them (the hook of the spirit HUD wrapper does).
