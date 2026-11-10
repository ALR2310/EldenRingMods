# Black Flame and Destined Death

Status: **Confirmed** in game: both work, and the log matched 8, 4 and 2 rows with the effects behaving
as expected. Conventions: [research notes](../../../docs/research/README.md). Code:
`src/status_effect.rs` (`BLACK_FLAME`, `DESTINED_DEATH`, `DESTINED_DEATH_MAX_HP`).

## Question

A player asked for the Black Flame or Destined Death ailments to last forever: the burn and the max HP
loss, not a new mechanic. Which rows are they, and how can they be told from the NPC burns?

## Data (a param editor export of `SpEffectParam`, 2.7.x)

Black Flame's burn is a set of 2 s DoT rows with a tick of 0.1 s and 0.1% of max HP plus 1 flat per tick,
`StateInfo 0`, `SpCategory` 10 or 20 depending on the source:

| Id | Name | Note |
|---|---|---|
| 835 | [Weapon] Black Flame Tornado - HP Burn | cat 10 |
| 855, 860 | Godslayer's Greatsword, Gargoyle's Blackblade - HP Burn | cat 20 |
| 1621000, 1623000, 1625000, 1626001 | Black Flame, Scouring Black Flame, Black Flame Ritual, Black Flame Blade (DoT Black) | cat 20 |
| 1624010 | Noble Presence (DoT Black) | cat 20 |

Left out: `29000`, `29001` to `29517` (NPC "HP Burn on Hit", inflicted by enemies), `6840` to `6851`
(Destined Death, below) and `1627001` (Black Flame's Protection, which has no `ChangeHpRate`).

## A list of ids instead of a signature

Poison and Scarlet Rot have their own `SpCategory` and `StateInfo`, so a signature works
([row_discovery.md](row_discovery.md)). Black Flame does not: its signature (`EffectEndurance 2`, tick
0.1, `ChangeHpRate 0.1`, `ChangeHpPoint 1`) is the same as Destined Death's and the NPC rows. So the mod
only looks at the 8 ids above and still checks `EffectEndurance == 2`, `StateInfo == 0` and
`ChangeHpRate != 0` before writing (an overhaul that changes the vanilla values makes a row skipped, as
for the other two ailments). Ids of the expansion or of an overhaul that are not in the list are not
edited.

## Defaults

`[BlackFlame] Duration` and `[DestinedDeath] Duration` are 10 (vanilla 2 s). A burn that lasts forever
(-1) is too strong, so the default is 10 s, a decision of the player. The percent and flat damage
default to vanilla (0.1 and 1). `[DestinedDeath] MaxHpDuration` stays -1, forever.

## Destined Death

Two groups of rows, each one ailment in the mod:

- **The burn (DoT):** `6840`, `6841`, `6850` ("DoT Main") and `6851`, with the same numbers as Black Flame
  (2 s, tick 0.1, 0.1% plus 1) and only a different `vfxId` (9011 and 9012). It is tuned like Black
  Flame, in `[DestinedDeath]` (`Duration`, `PercentDamage`, `FixedDamage`). `6840`, `6841` and `6851` have
  no name in the param editor; they are included because they sit in the same 684x to 685x block as
  `6850`, they matched in the log and they worked in the test.
- **The max HP loss:** `6860` (15 s) and `6865` (45 s), `MaxHpRate 0.9`, with no `ChangeHpRate`. The mod
  only writes `EffectEndurance` (`MaxHpDuration`, default -1 for forever) and does not touch
  `MaxHpRate`. Rows of this group only need `EffectEndurance > 0`, since 15 and 45 differ and no single
  vanilla value can be shared.

## Open risks

No row in the CSV refers to `6840` to `6865`, so what triggers them is unknown (a guess from the names:
two particular bosses). If that source is not among `6860` and `6865`, the max HP loss still follows its
original duration.
