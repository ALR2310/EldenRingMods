# GhostColor: removing the ghostly tint of spirits

Status: **Confirmed** in game on vanilla and on ELDEN RING Reforged 2.3.5.3; hot reload works both ways
and applies at once to spirits already out. Conventions: [research notes](../../../docs/research/README.md).
Code: `src/ghost_color.rs`. Open: see [open_issues.md](open_issues.md).

## Question

Where does the tint of a summoned spirit come from, and how can it be removed on vanilla and on a
regulation mod?

## Parameter chain of the colour

`BuddyParam.npcParamId` -> `NpcParam` -> SpEffect (32 `spEffectID` slots) -> `SpEffectParam.vfxId..vfxId7`
-> `SpEffectVfxParam.phantomParamOverwriteType/Id` -> `PhantomParam` (the real RGBA).

- **Vanilla**: NpcParam slot 26 (`spEffectID26`, offset `0x210`) = `295000` "[Spirit Summon] Color"
  (`stateInfo 432`, vfx `57000`), or `295200` for the Puppet (vfx `57100`). Vfx `57000` has
  `phantomParamOverwriteType 2`, `Id 200`, so PhantomParam 200 is the ghost colour. The Puppet also has
  `295201` to `295204` (slot 24), which only attach decorative SFX and do not overwrite the phantom.
- **Reforged**: no `295000`. The colour comes from `spEffectID4`, a SpEffect of each Ash (`200100` to
  `292100`) -> vfx `70000` (109 spirits) / `70010` (7) / `70020` (5) -> PhantomParam `2000` / `2010` /
  `420`. Those SpEffects also carry stat adjustments and `cycleOccurrenceSpEffectId` chains, so removing
  the SpEffect would unbalance Reforged. 4 spirits have no colour path at all.

## Mechanism

Spirit = NpcParam row that a `BuddyParam` row spawns (`npcParamId` / `npcParamId_ridden`, never row
names, which a param mod can get wrong) -> every SpEffect in its 32 slots -> every SpEffectVfx with
`phantomParamOverwriteType != 0`. `GhostColor = false` sets `phantomParamOverwriteType = 0` on those vfx
rows and leaves the SpEffect alone. Original values are captured once, before the first write, and every
later change is applied from that snapshot, so F5 works both ways. IDs come from `shared::params`, rows
are addressed by index and never through the runtime lookup table (`get_mut` / `rows_mut`), which can hit
the wrong row on a modded regulation such as Convergence.

Ratio rule: a vfx is switched off only when **the NpcParam rows of spirits using it outnumber the rows
that are not spirits**. Vfx `54183` (1 spirit, 16 others) is SpEffect `14495` -> PhantomParam `240`
(`isVisibleDeadChr`) of the Mausoleum Knight, and switching it off would remove the enemy's look. A rule
of "no ordinary NPC uses it" is too strict: it would exclude `57000` as well.

Typical log lines:

```text
vanilla:  106 spirit NpcParam row(s) from 107 BuddyParam NPC id(s); tint vfx 57000 (99/40), 57100 (7/4); left alone 54183 (1/16)
Reforged: 125 spirit NpcParam row(s) from 126 BuddyParam NPC id(s); tint vfx [70000 (109/0), 70010 (7/0), 70020 (5/0)]
```

## Rejected paths

- Stripping SpEffects 295000 to 295999 from every spirit each frame: costly and only affects spirits
  summoned later.
- Writing `NpcParam.spEffectID26 = -1`: right only for vanilla, no effect on Reforged.
- Wrong guesses on the way: `298091` is only a numeric SpEffect, not a colour; `stateInfo 432` is not a
  colour marker; the colour does not come from `GameSystemCommonParam`.
- Row names shown by a param editor follow the base game and can be wrong in a modpack, so always start
  from `BuddyParam`.

## Reference: spirit behaviour flags

Every spirit has `297000` "Follow & Warp to Player" (slot 28). The follow style is in slot 29: `297100`
WALK_FOLLOW (Putrid Corpse), `297101` REAR_FOLLOW (23 ranged, magic and support spirits), `297102`
FLY_FOLLOW (Winged Misbegotten, Jellyfish, Warhawk, Stormhawk), `297103` NO_FOLLOW (Latenna); no flag
means the default follow. `297200` INTERCEPT_LONGRANGE (slot 24) is an attack-style flag. Changing them
was not tried in game.
