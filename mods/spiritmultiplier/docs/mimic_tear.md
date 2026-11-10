# Mimic Tear: the Blob stage and the count

Status: **Confirmed** in game on 2.7.2.0 for `Amount` (10 gives 10 Mimic Tears) and for `InstantMimic`
(the Mimic appears at once, follows, fights and has an HP bar on the HUD). The fade-in is left as in
vanilla. Many Mimic Tears can crash the game on the GPU side, see the end. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/chain.rs`, `src/mimic.rs`.

## Question

Why does `Amount = 10` give only 5 Mimic Tears, and how can the Blob stage be skipped?

## Why only 5

The SpEffect `207010` -> BuddyParam chain of the Mimic Tear has two elements: a Blob (appears briefly and
disappears) and the real Mimic. The log (`slots changed`) shows 10 spirits alternating NPC `133201000`
(`[Spirit Summon] Mimic Tear - Blob`) and `100000010` (`[Spirit Summon] Mimic Tear`); about 7 seconds
later the 5 Blobs vanish and 5 Mimics remain. `Amount` counts chain elements, so 10 elements are 5 Blobs
and 5 Mimics. `Multiplier` multiplies the original length (2), so it was already right: 5 gives 5 + 5.

## Fix in `chain.rs`

`TRANSIENT_NPCS` (only `133201000`) gives the set of BuddyParam IDs holding that NPC, built once from
`BuddyParam` through `shared::params`. In `Amount` mode the element count is
`Amount * original chain length / real elements`, rounded up: 10 Mimics need 20 elements. Chains without
a transient element are unchanged. The exported `NpcParam` data has exactly one "Blob" row.

## `InstantMimic`: dropping the Blob

Method (`mimic.rs`, `chain.rs`):

- Group `BuddyParam` rows by `triggerSpEffectId`. The row holding the Blob NPC takes the spawn fields of
  the non-Blob row of the same group: NPC and think, ridden, offset, doping lv 0 to 10, init,
  `generateAnimId`, **and the three flag bytes (+36 to +38)**. **The row ID stays**: the game recognises
  "this is a Mimic Tear" by that ID and copies the player's build.
- The chain shrinks because the Blob became a Mimic, so one link is one Mimic: `[Blob, Mimic]` becomes
  one link (`Amount = 10` gives 10, `Multiplier = 5` gives 5). It shrinks only when the Blob is at the head
  of the chain (the head is a game node the mod does not write) and only after the Blob row was rewritten
  (`mimic::blob_replaced`).
- Everything is captured once and applied from the snapshot, so F5 works both ways.
- The fade-in is **kept**: `NpcParam` Mimic slot `spEffectID24` = 16316 "Spirit Summon - Fade In" is a
  visual only (vfx 53202, 3 s, no damage or buff).

The animation on the Blob row also matters: the most common non-Blob animation (1830) is set on both
the Blob row and the real Mimic row (the Mimic's own 60500, the Blob's 1831).

### Rejected attempts

- **Blob row takes only NPC, think, offset, doping and animation fields**: two NPC `100000010` slots, one
  stood still forever and one rose and followed, with a single HP bar. The three flag bytes were missing:
  the Blob row's `pcFollowType` is a kind that does not follow the player, so the Mimic copy on that row
  stood still. Copying the flags fixed it.
- **Chain `[Blob, Mimic]` with the Blob row copied only partly**: same result; reverted.

### HP bar on the HUD

With `InstantMimic` two Mimics fought but the HUD showed one bar; with `Multiplier = 1` (one Mimic, from
the Blob row) **no bar at all**. Params do not explain it: after the copy the `20700000` row differs from
`20700001` only in ID. Cause in game code (2.7.2.0): the HUD buddy list builder `sub_140771ED0` skips a
character for which `sub_1404B7A10` (RVA 0x4B7A10, "this character is a Mimic Blob": the character's
group has an entry with BuddyParam id **20700000**, flag bit 4 at chr +457) returns 1. The ID `20700000`
is also used by the buddy spawn routine `sub_1404BBEB0` to raise a "Mimic Tear requested" flag; only two
places in the executable hold that constant.

Fix (`mimic.rs`): redirect the `call sub_1404B7A10` inside `sub_140771ED0` to a stub returning `al = 0`
while `InstantMimic` is applied (every character from the Blob row is now a real Mimic), otherwise it
jumps to the original. Pattern `84 C0 74 ?? 48 8B D6 49 8B CE E8 ?? ?? ?? ?? 84 C0 75 ?? 83 FF 05`, `call`
at +10, unique on 2.6.2, 2.7.0, 2.7.1 and 2.7.2. The same call is what [buddy_hud.md](buddy_hud.md) hooks
to record the spirits that get a slot.

## What the reference "10x" mod does (decompiled)

For every `BuddyParam` row (by index):

- A row with `generateAnimId` (dword 22, +0x58) = 1831 and the Blob row ID (20700000) is the Blob. With its
  `drop_mimic_blob` option it **copies the next row's 156 bytes (index +1, same `triggerSpEffectId`) over
  the Blob row**, keeping dword +0x9C; the row ID stays.
- A `fix_fields` step (on by default) on **every** row sets `appearOnAroundSekihi = 0`,
  `disablePCTargetShare = 0`, `pcFollowType = 0`, and any `generateAnimId > 0` other than 1830 to 1830
  (the Mimic too: 60500 -> 1830). That is what makes the Mimic appear at once.
- Fade: `NpcParam` row 100000010, the field holding 16316, is set to -1 (this mod keeps the fade).
- Chain: rebuilt with a loop cycling the original IDs, head unchanged, real entry count = the number of
  non-Blob entries (at least 1); the same as `chain.rs`.

## Many Mimic Tears can crash the game (not the mod)

Reproduced with `Multiplier = 100` (chain cut at `MaxSpirits = 60` = 30 Blobs + 30 Mimics) in Seamless
Co-op: the game exits about 7 seconds after the summon, as the Blobs start to vanish. There is no WER
dump and no `Application Error` event; the vectored handler of the mods' crash module logged:

- **With Seamless** (2 of 2 runs, same signature): `access violation, read 0x0, at nvwgf2umx.dll+0x7E3868`,
  stack `D3D12Core.dll+0x22A38D < 0x2395D4 < eldenring.exe+0x19E4344 < 0x19EC3ED < 0x1C4AC70 < 0x1C4A793`.
  `eldenring.exe+0x19EC310` is the render-target setter (it loops over an array of graphics resource
  pointers) calling D3D12; a pointer to a broken or NULL resource.
- **Without Seamless**: the same settings ran fine when the Blobs vanished (30 Mimics left); a different
  crash appeared later: `write 0x0` in a `memmove` (`eldenring.exe+0x251E7FB`, a copy loop stepping by
  `sub_1411B1700`).
- **Two-player Seamless, 2.7.2.0** (`Multiplier = 25`, `MaxSpirits = 100`, both logs): the same fault within
  one second, stack `...+0x19E5A34 < 0x19EDADD < 0x1C4C360`. The timing differed: not when the Blobs
  vanished (the first summon of 25 + 25 went fine) but **right after the second summon**, while the new
  characters were being created (the host had 25 Blobs + 34 Mimics at the crash: 25 old Mimics, 25 new
  Blobs and 9 new Mimics; states `NetworkInitializing` / `ReadyForActivation`).

**Inferred**: each Mimic copies the player's model and equipment, so many of them exhaust graphics or
memory resources; a failed allocation is ignored by the engine and a NULL is used later. VRAM and RAM
were not measured. `Amount = 10` ran fine many times and `Amount = 30` ran once. With Seamless a count
near 30 can make the game exit and the mod cannot fix it. Each Mimic takes 2 slots when it first
appears, so `MaxSpirits = 60` gives at most 30 real Mimics. `InstantMimic` halves the characters per
summon (no Blobs) and may help; it was not tested for this.

## Reports that did not reproduce

- "Cannot summon again after dismissing": a summon made **before** the game finished clearing the old
  spirits (60 spirits take about 5 to 6 seconds) is lost; waiting a few seconds works.
- "`Amount` or `Multiplier` does not exceed the original count": not reproduced; likely an edited config
  without pressing `ReloadKey`, or no restart after changing `MaxSpirits`.
