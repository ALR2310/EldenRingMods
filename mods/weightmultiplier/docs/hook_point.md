# The hook point of the equip load calculation

Status: **Confirmed** in game in the released versions (2.0.1 on). The anchor pattern was checked in IDA
and is unique on 2.6.2.0, 2.7.0.0 and 2.7.1.0. The conflict report was never run against a real second
weight mod. Conventions: [research notes](../../../docs/research/README.md). Code: `src/hook.rs`.

## Question

Where can the equip load be scaled or fixed so that both the number on the screen and the real
behavior (roll type, run speed) follow, with a single factor?

## What the mod does

It patches **one CPU instruction** in `eldenring.exe` (an inline hook: a jump to a stub allocated at run
time, then a jump back) and reads and writes no game struct. The patched instruction runs **exactly once**
after the game has added up the weight of every item, so scaling or overwriting there gives one clean
number.

## Finding the point (three attempts)

The loop that sums the 5 slots looks like this:

```asm
addss xmm6, dword ptr [rax + 0xc]   ; add this slot's weight
inc ebx                              ; <-- a third-party mod hooks here (sets a fixed value every pass)
cmp ebx, 5
jl <loop start>
lea r11, [rsp+0x70]
movaps xmm0, xmm6                    ; <-- this mod hooks here (runs exactly once)
mov rbx, [r11+0x10]
```

1. **Wrong, dropped.** Inferred from the decompile of a third-party weight DLL: hook 5 bytes before the
   matching AOB (guessed to be `addss xmm6,[rax+0xc]`). It built and ran without error but **did
   nothing**: `addss` runs conditionally and is not a fixed 5 bytes from the AOB, so the hook landed on
   the middle of another instruction, harmless and without a crash.
2. **Right for the UI, wrong for gameplay.** Found with a memory scanner: pin the address of the
   current equip load and "find out what writes to this address", which gives `movss [rsi+1C],xmm0`
   (`rsi` is the equip-load struct). Hooking there made the **UI show the reduced number** but left the
   **roll type and run speed unchanged**: it only affects the display path, not the gameplay one.
3. **Right (in use).** With that third-party DLL installed (set to a weight of 0) the player confirmed that
   the roll type really changed, and the DLL's log printed the address it patches, which matches the AOB
   position of attempt 1 **exactly** (not 5 bytes earlier). The AOB was right; only the guessed offset
   was wrong.

That mod hooks `inc ebx` and overwrites `xmm6` with a fixed value **on every pass** of the loop; only the
last write survives, so the total is always the fixed number. That is fine for "set to 0" but **useless
for a percentage**: multiplying `xmm6` there compounds with each pass (the first slots are reduced more
than the last). `movaps xmm0,xmm6` runs once after the total is complete, so multiplying there is a single
factor.

## AOB, patch and stub

- **Anchor:** `FF C3 83 FB ?? 7C ?? 4C 8D 5C 24 ??` (`inc ebx; cmp ebx,N; jl ..; lea r11,[rsp+..]`).
- **Patch point:** anchor + 12 bytes (past `inc`, `cmp`, `jl` at 7 and `lea r11,[rsp+0x70]` at 5), landing
  on `movaps xmm0,xmm6` (3) and `mov rbx,[r11+0x10]` (4): **7 bytes**.
- **The jump.** Seven bytes leave no room for an absolute jump (`mov reg,imm64; jmp reg`, about 12 bytes)
  as in `mods/runemultiplier` and `mods/autoregen`. It uses the relative 5-byte `E9 rel32` and a stub in
  executable memory near the target, found with `VirtualAlloc` at explicit addresses (backward and
  forward in steps of `0x10000`): `shared::codepatch::install_jmp_hook`.
- **The stub:**
  ```asm
  mov   rcx, &WEIGHT_VALUE
  mulss xmm6, dword ptr [rcx]        ; mode 0 (Multiplier); mode 1 uses movss
  movaps xmm0, xmm6                  ; original instruction
  mov   rbx, qword ptr [r11+0x10]    ; original instruction
  jmp   <target + 7>
  ```
- **Two modes at the same point:** `Mode = 0` uses `mulss xmm6,[rcx]` (multiply) and `Mode = 1` uses
  `movss xmm6,[rcx]` (overwrite the total with `FixedValue`). One static, `WEIGHT_VALUE`, means a factor or
  a fixed number depending on the mode, since only one mode is active at a time.
- **Hot reload.** Changing `Multiplier` or `FixedValue` only rewrites `WEIGHT_VALUE`; the stub reads it from
  a fixed address on every run. Changing **`Mode`** changes the opcode, which is baked into running code,
  so the mod patches exactly those 4 bytes again with `codepatch::overwrite_bytes`
  (`STUB_MODE_INSTR_ADDR`). The risk is the one of the first install (overwriting code the CPU may be
  running), repeated only on request. The player accepted this trade-off when choosing to support
  changing `Mode` at run time. If the patch fails the mod keeps the old `Mode` and logs a warning.

## Another mod hooked the same place

When the anchor is not found the mod also scans `FOREIGN_PATCH_PATTERN` (`E9 ?? ?? ?? ?? 7C ?? 4C 8D 5C
24 ??`), which is `inc ebx; cmp ebx,5` (exactly 5 bytes) replaced by `jmp rel32`, where DLLs of that kind
hook. If it matches, the log says another mod hooked the code, with the name of the DLL that holds the
destination of the `jmp` (or "unknown" if it jumps into a stub allocated outside every module), and
`shared::conflict` shows one message box ([conflict.md](../../../shared/docs/conflict.md)). This could not
be tested in game, since it needs another weight mod to reproduce.

A report of "anchor pattern not found" had an unclear cause: either another weight-reducing DLL hooking
over `inc ebx`, or a different game version. No conclusion was recorded.

## Compatibility check (IDA)

| Build | Anchor (RVA) | Patch point (anchor + 12) | The two instructions there |
|---|---|---|---|
| 2.6.2.0 | `0x247C9E` | `0x247CAA` | `movaps xmm0,xmm6`, `mov rbx,[r11+10h]` |
| 2.7.0.0 | `0x247C9E` | `0x247CAA` | the same |
| 2.7.1.0 | `0x247C9E` | `0x247CAA` | the same |

The pattern is unique on all three and at the **same RVA** (unlike `mods/runemultiplier`, where the RVA
differs between builds). The mod still uses only the AOB, never the RVA.

## When testing

- **Check both sides:** the number on the UI **and** the real behavior (roll, run). The lesson of attempt
  2: a hook can be right for half of it, the UI, and wrong for the other half, without a crash or an error.
- This is a live code patch: if the game build changed, it can crash. Test somewhere safe, not in the
  middle of a boss fight.
- Look in the log for `Hook: anchor pattern not found`.
