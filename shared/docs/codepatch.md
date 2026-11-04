# Code patching

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/codepatch.rs`.

## Question

How does a mod redirect game code, and which choices keep the game stable and antivirus engines quiet?

## Pieces

| Function | Use |
|---|---|
| `absolute_jmp(target)` | `mov rax, target; jmp rax` (12 bytes): a jump that reaches any address. Clobbers `rax`. |
| `alloc_executable(len)` | RWX memory for stubs and relocated code. |
| `overwrite_bytes(addr, bytes)` | Patches code in place and flushes the instruction cache. |
| `rel32_target(site)` | Where the `call rel32` or `jmp rel32` at `site` goes. |
| `redirect_rel32(site, stub)` | Points an existing `call`/`jmp rel32` to a stub allocated within reach (about 1.8 GB), changing only the 4 displacement bytes. |

## Entry hook

The pattern used by `mods/autopickup` and `mods/autoregen`:

1. Find the function by an AOB pattern of its prologue.
2. Copy the first instructions (`RELOCATE_LEN` bytes, ending on an instruction boundary) to an
   `alloc_executable` buffer and append a jump back to `entry + RELOCATE_LEN`.
3. Overwrite the entry with `absolute_jmp(trampoline)`, padded with NOPs.
4. The trampoline saves the argument registers, calls Rust, restores them and jumps to the buffer.

## Rules

- **Relocate only position-independent instructions.** No RIP-relative operand, and no read of an
  argument register that the copy changes.
- **Size the buffer for its jump back.** One that was too short let the CPU run into the data after it
  and crashed on the first hit.
- **Keep the scratch register free.** `rax` is free at the entry of most functions, but a prologue such as
  `mov rax, rsp` makes it the frame, and a jump through `rax` then writes into the function's own code.
  Jump back through `r11` there (`mov r11, imm64; jmp r11`, as in `mods/autopickup/src/hook.rs`). A
  prologue that starts with `mov r11, rsp` is the opposite case (`mods/autoregen`).
- **Hook at the entry of a function**, never in the middle. A hijack of a call site inside the function
  crashed on entering an area, probably for lack of unwind data for the hijacked range.
- **Stubs and relocated code live in `alloc_executable` memory**, never in a `static` made executable
  later: a DLL that jumps into its own data section draws antivirus detections
  (`mods/autopickup/docs/antivirus.md`).
- **Export `DllMain` only.** Reference the trampoline's Rust callbacks through `sym` in `global_asm!` and
  keep addresses in statics, so no helper symbol is exported.
- **Install once the game is running**, so other mods that scan the same code have finished, and check
  for a foreign hook first ([conflict.md](conflict.md)).
- **A hook must not allocate, lock or panic.** Pass settings as atomics and leave logging to a task.

## Redirecting a call

`redirect_rel32` suits a site that is a `call`: other mods match the opcode and the bytes around it, not
the displacement, so their signatures still find the site. The stub is entered exactly as the old
destination would be and must end in a `ret` or a tail jump (`mods/speedmultiplier/src/seamless.rs`).
