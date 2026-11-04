# The `AddSoul_Call` hook

Status: **Confirmed** in game since the C++ version and in the released Rust versions (1.0.0 to 1.0.4),
including 1.17 and 1.17.1. The anchor pattern was checked in IDA and is unique on 2.6.2.0, 2.7.0.0 and
2.7.1.0. Conventions: [research notes](../../../docs/research/README.md). Code: `src/hook.rs`.

## Question

Where can every rune gain, whatever its source, be scaled in one place, without also scaling what the
player spends?

## Why `AddSoul_Call`

It is the game's lowest-level function that adds runes, shared by **every source** (kills, picking up or
using items, selling). Disassembled from the real executable (headless Ghidra without full analysis, since
the 87 MB file is too slow):

```
MOV R9D,[RCX+0x6C]        ; R9D = current runes
LEA R8D,[R9+RDX*1]        ; sum = current + amount (RDX = amount)
CMP R8D,0x3B9AC9FF        ; clamp to the cap of 999,999,999
...
MOV [RCX+0x6C],EAX        ; write the new runes
RET
```

`[RCX+0x6C]` is the same offset (`0x6C`) that the C++ version of PassiveRunes found independently through
a pointer chain from `GameDataMan`, and it matches `GameDataMan::main_player_game_data.rune_count` in
fromsoftware-rs: two separate reverse-engineering efforts reached the same field.

## Finding `AddSoul_Call` without a pattern of its own

The function has no byte pattern distinctive enough, so the mod uses an anchor (`ANCHOR_PATTERN` =
`F3 0F 59 C1 F3 0F 2C F8 48 8B 8B`): the game's reward scaling step for enemy kills (`mulss xmm0,xmm1;
cvttss2si edi,xmm0; ...`). The `call` (`E8 rel32`) at `anchor + 0x11` (4 + 4 + 7 + 2 = 17 bytes) is
resolved to an absolute address. If the byte there is not `0xE8` the layout is not what was expected and
nothing is patched.

Checked in IDA:

| Build | Anchor (RVA) | `call` at `anchor + 0x11` | `AddSoul_Call` |
|---|---|---|---|
| 2.6.2.0 | `0x651883` | `0x651894` | `0x25E100` |
| 2.7.0.0 | `0x6526D3` | `0x6526E4` | `0x25E0E0` |
| 2.7.1.0 | `0x6526D3` | `0x6526E4` | `0x25E0E0` |

The pattern is unique on all three. The first 12 bytes of `AddSoul_Call` are identical on all three
(`44 8B 49 6C`, `45 33 DB`, `44 89 5C 24 10`). The RVAs differ between builds, so only the AOB is used.

## The patch

The **first 12 bytes** of `AddSoul_Call` (`mov r9d,[rcx+0x6c]`, `xor r11d,r11d`, `mov [rsp+0x10],r11d`,
none of which reads `RDX`) are overwritten with `mov rax,<stub>; jmp rax` (10 + 2 = 12 bytes, an exact
fit, no NOP padding). The stub, in `alloc_executable` memory, scales `EDX` (the amount), replays the 3
original instructions (copied from the game's memory when the hook is installed, never typed by hand) and
returns with `mov r11,<return>; jmp r11` to just after those 12 bytes.

- **Fixed-point Q20 integer math, no XMM.** `AddSoul_Call` has many callers, so no XMM register can be
  assumed dead at all of them. The factor is `FIXED_Q20 = round(Multiplier * 2^20)`, read by the stub
  through a pointer, so a reload only updates that variable. The error is at most **1 rune** (`SAR` rounds
  down), checked on about 100 combinations of amount (0 to 999,999,999) and multiplier (0 to 100).
- **Only when `amount > 0`:** the stub has `test eax,eax; jle skip`. See the rejected paths.
- **Absolute jumps in both directions** remove the distance limit of `E9 rel32`, so the stub can be
  anywhere. The C++ version had to scan the pages near the target for an allocation within 32-bit reach.
- The encoding was checked byte by byte: the bytes of the multiply code were loaded into Ghidra as x86-64
  and disassembled back, matching the intent.

### Why there is no try/catch around the assembly

It cannot be wrapped: `try/catch`, SEH and `catch_unwind` depend on unwinding through call frames that
have a handler table, and code glued into the middle of a function with a `jmp` has none of that. The
hook is safe because it **dereferences no game pointer**: it touches only CPU registers (`rax`, `rdx`,
`r8`, `r10`, `r11`) and reads one static variable that is valid for the life of the DLL. A hook that must
read `WorldChrMan` or the player pointer (null while loading), like the hit hook of `mods/autoregen`, needs
protection, and gets it with `catch_unwind` around the **Rust function** it calls, not around the assembly.

## Rejected paths

1. **A second hook at the enemy-kill reward step** (the anchor above). It scaled enemy kills but **not the
   runes from items** (Golden Rune and the like). Adding a hook on `AddSoul_Call` to cover items made
   kills count **twice** (64 runes became 256 instead of 128), because enemy kills go through
   `AddSoul_Call` as well. The first hook was dropped; that point is now only the anchor.
2. **Scaling every `amount`** (1.0.0): `AddSoul_Call` is a general "current += amount", so **spends**
   (level-ups, purchases, a negative `amount`) were scaled too: prices went up and the leftover runes
   dropped to 0 after a level-up (reported by two players). Fixed in 1.0.1: the stub skips the multiply
   when `amount <= 0`. A public cheat table entry that served as the starting point only scaled the
   enemy-kill reward and never hooked `AddSoul_Call` directly.

A consequence of `amount > 0`: `Multiplier` can still be below 1 or negative (runes received are
reduced or subtracted), since only a **gain** is multiplied.

## Where the first idea came from

A public cheat table has a "Rune Multiplier" entry (not encrypted, unlike most entries of that table, which
are paid features hidden behind a decode function) that scans for the same pattern, and a third-party DLL
does the same, patching the original 8 bytes into a jump to its own stub. Both **replace `xmm0`** with a
constant before multiplying by `xmm1`. This mod's first patch (since dropped) kept the game's own
`mulss xmm0,xmm1` and multiplied the result again, an improvement that stacked wrongly with the
`AddSoul_Call` hook.

## Open risks

- The anchor and the `call` at `anchor + 0x11` are derived from the executables listed above. A patch that
  changes the layout makes the mod log `anchor not found` or `AddSoul_Call not resolved` and stay off for
  that session, patching nothing.
- Builds other than 2.6.2.0, 2.7.0.0 and 2.7.1.0 have not been checked.
