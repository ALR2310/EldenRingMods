# Finding the Poison and Scarlet Rot rows

Status: **Confirmed** in game: the player judged the matched rows right without further changes.
Conventions: [research notes](../../../docs/research/README.md). Code: `src/status_effect.rs`. The
main lesson: filter directly on the `SpEffectParam` **running in the game**, never on conclusions
from a static CSV export, which were wrong at least three times for this mod.

## Question

Which fields of `SpEffectParam` set the duration of the damage-over-time (DoT) effects that the
**player** inflicts, and how to edit exactly those rows without touching the ones of enemies, swamps
or bosses?

## Rejected: editing one shared row, `505`

An analysis of the `SpEffectParam` CSV (a param editor export filtered on "Scarlet Rot" and
"Poison") reached a conclusion that was **wrong at its key point**:

- The 90 s seen on rows `106xxx` and `107xxx` ("Poison/Scarlet Rot - Low/Medium/High -
  Addition/Innate +0..+25") is not the DoT duration but that of a separate buff that raises the
  build-up per hit, for example from a talisman.
- The real DoT effect sits in one shared row, id `505` ("Poison/Scarlet Rot (Cycled)",
  `EffectEndurance = 900`); the `106xxx` and `107xxx` families both have
  `cycleOccurrenceSpEffectId = 505`.
- `-1` is the convention for "permanent" in `EffectEndurance` (row `90010`, "NPC: Disable Scarlet Rot",
  ships with `-1`).
- `effectTarget*` only says which kinds of entity may receive the effect; it does not fix a side: it
  applies to anyone whose build-up bar fills.

The plan was `set_effect_endurance(-1.0)` on row `505` once at start-up. It was **never tested in game**.

It was dropped when following the chain to the damage column (`behaviorId = 2105`, to
`BehaviorParam_PC` (refType Bullet, refId 1005), to `Bullet` 1005, `atkId_Bullet = 2`, to `AtkParam` id
`2` "Blood Loss - Bullet") showed that the last `AtkParam` row is **shared with the Bleed effect**. That
undermined all the earlier reasoning, which rested only on reading the CSV, so writing that field was
ruled out.

## The way that works: filter the live data

The mod walks the real `SpEffectParam` in game and adds one field condition at a time, printing the
matching row count and the relevant fields to compare with a param editor by eye, until the region is
exact. The final signature:

| Condition | Scarlet Rot | Poison |
|---|---|---|
| `SpCategory` | `10005` | `10004` |
| `StateInfo` | `5` | `2` |
| `ChangeHpRate != 0` | yes | yes |
| `EffectEndurance == 90` | yes | yes |

- **`StateInfo` differs between the two ailments** and cannot be shared. Assuming `5` for both made
  Poison match **0 rows**; looking the value up in a param editor gave `2`. So each `Ailment` keeps its
  own `state_info`.
- `ChangeHpRate != 0` keeps only the rows that really cost health over time, dropping the ones that
  only raise build-up.
- **`EffectEndurance == 90` is both the vanilla duration and a filter** against groups that match the
  other three conditions but are not "inflicted by the player":
  - `180`: the swamp and poison-lake hazards (`TargetEnemy = false`) and "NPC: Scarlet Rot" and "NPC:
    Poison" (enemies inflicting it);
  - `300`: a boss's own rot effect;
  - `30`: "NPC: Fast Poison" and a few unnamed rows outside `CycleOccurrenceSpEffectId` (one-shot, not
    part of the DoT cycle).

The result is **175 Scarlet Rot rows and 203 Poison rows**, checked by hand against a param editor for
every id (one Poison row, id `3750`, has no name there; that is not a filtering error).

## Fields written

| Field | Meaning | Vanilla Scarlet Rot | Vanilla Poison |
|---|---|---|---|
| `EffectEndurance` | Duration in seconds, `-1` for infinite | 90 | 90 |
| `ChangeHpRate` | Percent of max HP lost per tick | `0.18` (`0.33` on some rows) | `0.07` |
| `ChangeHpPoint` | Flat damage added per tick | `15` | `7` |

An early assumption that the vanilla `FixedDamage` was 0 was **wrong**; the values above, read from a
param editor, are the template defaults.

## Why the list of rows is cached after the first scan

The filter includes `EffectEndurance == 90`, so after the first write a new scan would not match the
rows just edited: a reload would look as if it did nothing although the first apply worked. Each
`Ailment` keeps the `(id, index)` of its rows from the first scan (`matched_rows`) and every later
apply reuses them. The mod writes **absolute values**, never a factor, so a reload does not compound and
needs no snapshot of the originals. The same kind of bug was met in the drop-rate mod.

## Open risks

- A regulation of an overhaul that changes the vanilla `EffectEndurance` (other than 90) matches no
  rows.
- The first scan walks rows by index through `shared::params` (`shared/docs/params.md`), so a regulation
  whose header disagrees with the runtime row count, such as Convergence's, no longer panics.
