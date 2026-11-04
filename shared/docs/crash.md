# Crash logging

Status: **Confirmed** in game: it logged the faulting addresses and stacks used in the research notes (for
example [heap corruption](../../mods/journeyhud/docs/heap_corruption.md), the QuestPath clash). Unit tests cover
the exception names and a fake access violation with an unreadable stack. Conventions:
[research notes](../../docs/research/README.md). Code: `shared/src/crash.rs`, installed by
`logger::install_panic_hook`.

## Question

The game swallows some crashes, so a mod's log ends with no hint. How can a mod leave the faulting address, the
registers and a stack in its own log, without interfering with the game or with other mods?

## Mechanism

`crash::install()` (called once through `logger::install_panic_hook`) adds a **vectored exception handler** that
only logs and returns `EXCEPTION_CONTINUE_SEARCH`; it never handles anything. These are first-chance exceptions,
so the **last `Exception:` line before the log ends is the crash**. A top-level exception filter does not work:
the game's CRT replaces it.

Logged exceptions: access violation (with read, write or execute and the target address), stack overflow,
illegal instruction, integer divide by zero, privileged instruction, fast fail and heap corruption
(`0xC0000374`). Other codes (C++ exceptions, breakpoints) are ignored. At most 16 distinct `(code, address)`
pairs are logged per process, and a re-entrant exception inside the handler is skipped.

Each entry has:

- `Exception: <name> at <module>+0xRVA (thread N)`;
- `code:` the 16 bytes at the faulting address;
- two lines of registers, with `module+0xRVA` shown for values that point into a loaded module;
- the **stack**, in one of two ways: unwinding with `RtlLookupFunctionEntry` / `RtlVirtualUnwind` over a copy of
  the `CONTEXT` (up to 12 frames, stopping when `rsp` leaves the thread's stack bounds read from the TEB;
  functions without unwind data fall back to reading the return address at `rsp`), or, when it gives fewer than 2 frames,
  a scan of the top 0x400 bytes of the stack for values that are inside a module and right after a `call`
  (`E8 rel32`, `FF D0..D7`, `FF 10..17`, `FF 15 rel32`, `FF 50..57 disp8`);
- for a **stack overflow** the innermost frames are all the same loop, so the unwinder is skipped and an
  `outermost:` line scans the first 0x1000 bytes of the stack from the top, which shows who entered the loop.

All memory reads go through `ReadProcessMemory`, so an unreadable address makes the read fail instead of
faulting inside the handler.

## Limits

- Offsets are RVAs of the module named in the log; resolve them against the matching executable build.
- A copy of the handler lives in every DLL that links `shared`, so several mods log the same crash; each writes
  to its own log.
- A crash on a thread whose stack bounds cannot be read gives only the registers.
