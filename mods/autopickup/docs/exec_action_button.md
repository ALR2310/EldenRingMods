# Auto pickup through `ExecActionButton`

Status: **Confirmed** in game on 2.7.2.0 for the groups, the pickup range and pausing in
combat. The combat flag offset is **Inferred** (see below). Conventions:
[research notes](../../../docs/research/README.md).

## Question

How can a mod pick things up without the player pressing the interact button, while leaving
levers, doors, graces and conversations alone?

## Mechanism

`ExecActionButton(this, id, float, a4, ...)` is the function the game calls to ask whether
the interaction `id` (a row of `ActionButtonParam`) is triggered. It returns 0 for no and 1
for yes. The hook returns 1 for any `id` that belongs to an enabled group, without running
the original function, so the game continues as if the player had pressed the button. Every
other id runs the original function unchanged.

The groups are defined in `src/groups.rs`:

| Group | IDs | Default |
|---|---:|---|
| Materials | 112 | on |
| Items on the ground | 14 | on |
| Corpse loot and remains | 5 | on |
| Dropped runes | 1000 | on |
| Chests | 4300 | off (mimic chests open the same way) |

The ID lists were compared against the `ActionButtonParam` exports of vanilla and of a large
overhaul mod; the 98 original IDs exist in both and are identical. Where this list differs
from the one it started from:

- The source array declared 78 elements but held 76 material IDs; the two trailing zeros broke
  the sorted order a binary search needs.
- `4300` ("Search treasure chest") was moved out of Items into its own group.
- `4200-4202` ("Pillage remains") and `9532` ("Pillage corpse") moved from Items to corpse loot.
- Added `207850`, `207860-207894` (36 DLC "Acquire materials" rows, parallel to `7850` and
  `7860-7878`), plus `209210`, `209524` and `4261` ("Pick up item"). These additions rest on
  row names and table structure and have not been tried in game (**Inferred**).

The pickup range multiplies `radius`, `height` and `baseHeightOffset` of the enabled groups'
rows by `Range`. The original values are stored on the first scan (`src/range.rs`), so changing
`Range` or disabling a group always starts from the game's own numbers. The same values also
decide how far away the pickup prompt appears.

## Hook point

The core function is `sub_140A64680` on 2.7.2.0 (it uses the string
`CS::CSActionButtonManImp::ExecActionButton`): `a1` is the `CSActionButtonManImp`, `a2` the id.
Two wrapper functions next to each other forward to it:

- `A673D0` calls it directly and has 21 callers.
- `A674B0` takes the lock at `a1 + 144` first, then calls it, and has 2 callers.

A pattern taken from a wrapper matches both of them, so a hook there reaches only one. The mod
hooks the core function to cover both.

The 61-byte pattern in `src/hook.rs` is the core function's prologue and is unique on all
four reference builds:

| Build | RVA |
|---|---|
| 2.6.2.0 | `A62EE0` |
| 2.7.0.0, 2.7.1.0 | `A641C0` |
| 2.7.2.0 | `A64680` |

The first 16 bytes (seven instructions, up to `SUB RSP,0F0h`) contain no RIP-relative
operand, so they are relocated verbatim. The trampoline saves `rcx/rdx/r8/r9` and `xmm0-3`,
calls `pickup_filter(this, id)` (two atomic reads, no allocation, no lock) and returns 1 when
the result is non-zero. Otherwise it restores the registers and jumps into the relocated bytes,
which live in memory allocated at install time (see [antivirus.md](antivirus.md) for why not in
a static).

## Conflicts with other hooks

The hook overwrites the first 16 bytes of the function with a jump. If another mod hooked it
first, the full pattern no longer matches. `hook.rs::scan` then looks for the tail of the
pattern, which always sits exactly 0x10 bytes after the function start and is unique on all
four builds, and asks `common::conflict::foreign_hook_at` whether a jump leaving the
executable is present. If so it logs the conflict, shows a message box and disables the hook.
This is covered by unit tests of the jump detection; it has **not** been tried against a real
conflicting mod. A mod that hooks the wrapper functions does not conflict, because the wrappers
still call the core function.

## Probe

`Logging.PickupProbe` is off by default (`src/probe.rs`). The hook only pushes `(id, handled)`
into a fixed atomic ring buffer (no allocation, no lock, no file write). A per-frame task
reads it and logs:

- `Probe: picked <id> <group> xN`, at most once per second per id;
- `Probe: skipped <id> <group>` for an id of a disabled group or one paused by combat, once;
- `Probe: other <id>` for an interaction outside every group, once per id. This is how to find
  the id of an item the mod does not pick up.

## Combat flag

`CSSound` singleton, `+0x328` points to a struct whose byte `+4` is "in combat". The offsets
match those published by a third-party tool. The pattern
`48 8B 05 ?? ?? ?? ?? 48 85 C0 ?? ?? 83 CD 02 83 B8 ?? ?? ?? ?? 00 89 6C 24 ?? 0F 95 C1` finds the
pointer; it is unique on all four builds and on 2.7.2.0 lies in `sub_1404395D0` (RVA `439A63`),
which reads global `143D8AD68` and compares `[+0x314]` with 0.

**Inferred.** It has not been proven that `143D8AD68` is the singleton registered under the
name `CSSound`, and no writer of the `+4` byte was found (`fieldaccess 328` points to stubs of
another class, `CSSoundImp`). In game, pausing in combat behaves correctly. The flag is read
every frame in a `FrameBegin` task, not inside the hook, because singleton lookup is not safe
in a hook, and the pointer is validated before use.

## Rejected paths and failures

- **Returning to the original code through `rax`.** The first build crashed three seconds after
  entering the world with `illegal instruction at eldenring.exe+0xA64678`, `rdx=0x17D5`
  (id 6101, "Rest at site of grace", outside every group, so the hook let the original run).
  The jump back after the 16 relocated bytes used `mov rax, imm64; jmp rax`, but the prologue
  leaves `rax` equal to the entry `rsp` and the second half of the function stores registers
  through `rax` (`mov [rax+8], rbx`, `movaps [rax-38h], xmm6`). With `rax` holding a code
  address those stores overwrote the function's own code. The fix is `mov r11, imm64; jmp r11`
  (`hook.rs::jmp_via_r11`). When relocating a prologue, check that the function does not use
  the register chosen for the jump after the prologue.

## Open risks

- Returning 1 skips the original function's own conditions (`a4`, or `sub_140A656D0(a1)`
  returning 0 early). If pickup happens while a menu is open, during a cutscene or on
  horseback, add that condition here.
- Not tested together with other DLLs that offer auto pickup; disable those when testing.
