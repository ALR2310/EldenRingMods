# Seamless Co-op: redirect a `call` instead of rewriting bytes

Status: **Confirmed** with Seamless Co-op 2.0.1 and offline. This is the rule for every code patch of this
mod. Conventions: [research notes](../../../docs/research/README.md). Code:
`shared::codepatch::{rel32_target, redirect_rel32}`.

## Question

Why did the game stop at start-up with Seamless Co-op installed, and how must code be patched so that it
does not?

## The incident

Some early versions of the mod exited at once with Seamless Co-op 2.0.1: `Fatal Error: No such pattern
E8 ? ? ? ? 83 F8 02 41 0F 44 FC ... signatures.cpp 1426`. That site is the HP cost calculation
(`0x1403C0C2F`: `call GetBuddyState; cmp eax,2; cmovz edi,r12d`) which the MultiSpirit cost patch had
replaced with an inline `jmp` (`E8` became `E9`, plus NOPs). Seamless finds its patch sites by byte
signature and **terminates the game** when one does not match. It was not noticed earlier because every
test had been offline.

The Seamless DLL is not readable enough to list its signatures: strings are almost all encrypted, and
the signatures are decrypted only when used. So assume that **any byte around a patch site may be part of one of its signatures**.

## Rule

Write only the four displacement bytes of an existing `call` or `jmp rel32` so that it points to a nearby
stub; keep the opcode and every surrounding byte (Seamless's signatures wildcard exactly the
`E8 ? ? ? ?` part). If another mod already redirected the same instruction, `rel32_target` returns its
stub, so the two mods chain. A stub is "a called function": it is entered with `rsp % 16 == 8`, does
`sub rsp,28h/38h` before every inner call, and ends with `ret` or a tail `jmp`.

Applied in `multi_spirit.rs` (UI x3, CanUseItem, FP/HP x2, recall, DoSummon, DisappearAll) and
`buddy/stone.rs` (stateInfo call and tail, in-range). Two older inline patches (keeping the stone, 8 bytes;
clearing `+0x38`, 7 bytes) were dropped and their logic moved into stubs. `band.rs` was unchanged: it had
worked with Seamless from the start.

## Verification

With Seamless 2.0.1 loaded the game is no longer terminated, every `was` line of the log points at the
game's original function; MultiSpirit summons several Ashes and sends them back one by one (an upgraded Ash
`258001`, a 15-spirit Ash `240000`); SummonAnywhere uses the fallback stone `10000100`. Offline: all 12
patches install. With Reforged loaded: 12 patches, no error.

## `MaxSpirits > 60` made Seamless stop with "No such pattern"

Status: **Fixed** (`band.rs`, `seamless_loaded`); the failure was reproduced, the fix was not re-tested in
game.

Seamless showed `Fatal Error: No such pattern "BA 00 05 00 00 41 B8 08 00 00 00 FF 50 50 48 89 46 18"`
when installed with this mod and `MaxSpirits > 60`. That pattern is the start of `CAPACITY_INIT_AOB` (the
init of the spirit ChrSet): Seamless scans it to raise the capacity to 1000, while the capacity patch of
this mod overwrote `BA 00 05 00 00` (the allocation size), so Seamless could not find it. The capacity
patch broke the rule above.

Fix: with `MaxSpirits > 60` wait up to 10 seconds (on a separate thread, not in `DllMain`) to see whether
`ersc.dll` loads. If it does, skip the capacity patch and let Seamless raise it (the band is still
`MaxSpirits` wide, `band_layout` splits by the real capacity); if not, patch as before. The patch only has
to finish before the world is created, so waiting a few seconds is fine. **Remaining**: if `ersc.dll` loads
after 10 seconds the old failure still happens.

## Other effects of Seamless

- Spirits lose the ghost colour (a Seamless hook): [open_issues.md](open_issues.md).
- Seamless also redirects `call sub_1403F21B0` and the update function that
  [persist_spirits.md](persist_spirits.md) patches; the mod checks and re-applies there.

## Report that could not be reproduced: cannot summon again with Seamless

Status: **Not reproduced, not caused by the mod** (checked). Investigation stopped; if more reports of the
same appear, see "Next".

A user reported that after sending spirits back or after they died they could not summon again, with any
character, only with Seamless installed (they went back to an older "resummon for Seamless" mod). On the
author's machine, even with the same ME3 setup and a DLSS frame generation mod, summoning again always
worked; their log held no error from this mod.

Outside hints: the Seamless author answered other spirit mods that "SpEffect `2147300000`" is the fix; a
third-party mod rewrites a row `503045` to `2147300000` in the internal param table of `ersc.dll`; the
user reported that forcing `2147300000` with a memory editor about 1 second after sending spirits back
let them summon again.

Checked with diagnostic logging (player SpEffects on every change and the rows `503045` and
`2147300000`), with and without Seamless:

- `SpEffectParam 2147300000`: **no such row** even with Seamless, so it is not a real game effect but a code
  Seamless handles itself.
- `503045`: an empty vanilla effect with infinite duration; appears about 1 s after entering the game
  (also **without** Seamless) and disappears on a map reload. It does not change on summon, send-back or
  death, so it is not a Seamless marker.
- Around a summon only `232000` (the Ash's SpEffect), `6518201` and `9540` appear.

**Inferred**: the block lives in Seamless code and cannot be seen from SpEffects; applying `2147300000` with
`apply_speffect` most likely does nothing (no param row). Decompiling `ersc.dll` (packed or virtualised)
was not worth it, and a fix could not be verified without a reproduction.

**Next, if reports continue**: look in a code diff of the game in RAM against the executable for Seamless
patches on the summon path (DoSummon, the update function, the spawn routine, the stone and `stateInfo`
checks), then use hardware breakpoints or logging to see which flag Seamless reads at the summon press,
instead of reading its code.
