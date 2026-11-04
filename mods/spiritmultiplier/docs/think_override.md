# Spirit AI: `ImproveSenses`, `ImproveAggression`, `ImproveFollow`

Status: **Confirmed** in game: the three bundles together improve spirit behaviour (the groups were not
tested one by one); the shared-row restriction below was reproduced and confirmed fixed alongside a
third-party Ashes overhaul. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/think/mod.rs`, `src/think/fields.rs`.

## Question

Which `NpcThinkParam` fields make spirits notice enemies, engage together and keep up with the player, and
how can they be tuned live without shipping a regulation?

## Mechanism

- Target rows: every `npcThinkParamId` and `npcThinkParamId_ridden` of `BuddyParam`, addressed by index
  (never through the runtime lookup table).
- The first time, the whole 228 bytes of each row are saved; every apply starts from that snapshot and
  writes the chosen fields (the baseline pattern of [ghost_color.md](ghost_color.md)). Spirits already out
  keep the old values: summon them again.
- `think/fields.rs` was generated from `NPC_THINK_PARAM_ST` of fromsoftware-rs: 85 numeric fields and 12
  single-bit flags (`enableNaviFlg_*`, `isNoAvoidHugeEnemy`, ...; value 0 or 1); the test `offsets_match_struct`
  checks the offsets against the library's setters.

### Built-in bundles (on by default, hot reload)

The values come from the regulation of a third-party spirit overhaul (compared with the base game, the
Lone Wolf row) and were tried in game.

| Field | Overhaul | Base |
|---|---|---|
| `TeamAttackEffectivity` | 100 | 34 (first spirit: 50) |
| `BattleStartDist` | 15 | 2 |
| `eye_dist` / `searchEye_dist` | 15 | 2 / 0 |
| `nose_dist` | 5 | 0 |
| `searchTarget_LV1/2_forgetTime` | 5.0 | 1 |

- `ImproveSenses`: sight, smell, hearing, how long a target is remembered, sight in combat.
- `ImproveAggression`: `TeamAttackEffectivity`, `BattleStartDist`, calling for help (Platoon), `isGuard_Act`,
  `thinkAttr_doAdmirer`, jumping.
- `ImproveFollow`: return to the owner when farther than about 30 m (base 9999) and permission to use
  ladders, holes, navmesh walls and edges. Lava is left out on purpose.

Each entry has a mode: `Max` only raises the original, `Min` only lowers it, `Set` always writes, so that
regulations that already tuned these fields (for example the overhaul gives healers their own values) are not
broken. Not in the bundles:
`isBuddyAI`, `disableDark` (already 1), `enableNaviFlg_Lava`, the `goalAction_*` group (decided against) and
`fallDamageDump` (no effect).

- Navigation group of the overhaul: `enableNaviFlg_Ladder/Hole/InSideWall/Lava/Edge_Ordinary` = 1 (base 0):
  permission to path over ladders, holes, navmesh walls, lava and edges. **Hypothesis**: lets spirits follow
  over ledges instead of getting stuck; lava = 1 could let them walk into lava.
- Retreat group: `maxBackhomeDist` / `backhomeDist` 30 (base 9999 / 9979), `backhomeBattleDist` 60 (base 10),
  `BackHome_LookTargetTime/Dist` 15, `backToHomeStuckAct` 1 (base 0), `BackHomeLifeOnHitEneWal` 0.1 (base 5.0).
  **Hypothesis**: "home" is the owner; by default a spirit chases enemies almost without limit, with 30 m it
  is pulled back when it goes too far.
- The tooltip of `TeamAttackEffectivity` in a param editor says a higher value means fewer attackers at once,
  opposite to the overhaul author's description. Which one is right was not tested.

## What the overhaul's `NpcThinkParam` shows (200 rows)

- Almost every field is constant over the 200 rows (`BattleStartDist` 15, the sight values, the Navigation
  and Retreat groups). Only a few vary: the `goalAction_*` group, `TeamAttackEffectivity` (100 in 179 rows;
  0, 30, 50, 60, 80 in a few, usually healers and support), `useFall_onNormalCaution` (0 or 2, split),
  `actTypeOnFailedPath`, `eye_BeginDist`.
- `goalAction_*` (ToCaution, ToCautionImportant, ToSearchLv1, ToSearchLv2, ToDisappear): 116 melee rows =
  3/3/3/3/3 (run to the source). 41 sorcerer and archer rows (Noble Sorcerer, Rennala, Page, Oracle Envoy,
  the Archers, even Spirit Jellyfish) = 2/3/1/1/2: approach or only look, do not charge. The rest are small
  variants (healers with ToDisappear 0, fire knights, ...). Setting 3 everywhere would send sorcerers into
  melee, which is why the group is not in the bundles.

## Rejected: fall damage

`NpcParam.fallDamageDump = 100` did **not** make spirits survive a fall (tried under Convergence). The field
is 0 on every `[Spirit Summon]`, 100 on dummies, Bonfire, Maliketh and Gurranq, 90 on dragons and Giant
Crow; **inferred** to be a percent fall-damage reduction, but spirits still died with 100: either it does
not apply to spirits or that fall is a hard-coded kill. Untried: a SpEffect with `fall_damage_rate` (f32) put into the `spEffectID0..31` slots of the spirit's NpcParam.
`HitMtrlParam.disable_fall_damage` is per ground material and cannot be used for spirits only. Falling out
of the map belongs to [warp_research.md](warp_research.md).

## Shared rows with enemies (a fix)

A user running the mod together with a third-party Ashes overhaul reported that enemies and bosses chased a
short distance, then teleported back to their spawn and stopped chasing (some bosses sank into the floor and
could not be killed). Reproduced with both mods; with one of them it was normal; with `ImproveFollow = false`
it stopped.

- A spirit does not only use AI rows of its own: `BuddyParam.npcThinkParamId` may point at an enemy's row.
  Vanilla has 98 rows, 7 with `isBuddyAI = 0` and enemy names (Demi-Human 1 to 5 `141000000` and so on, Land
  Squirt `144400000`, Miranda Sprout `144810000`); the other 91 have `isBuddyAI = 1`. In the whole vanilla
  `NpcThinkParam`, 10 more `isBuddyAI = 1` rows are not referenced by `BuddyParam` (IDs `100000000`,
  `1500000xx`, `199999999`, unnamed).
- In the overhaul: 640 think rows used by spirits, 429 with `isBuddyAI = 1` and **210 with `isBuddyAI = 0`**
  shared with enemies. The enemies' rows there have a typical `maxBackhomeDist` of 85 (a leash); `ImproveFollow`
  (`Min 30`) lowered it to 30, so enemies gave up the chase early.
- `ImproveSenses` and `ImproveAggression` also wrote to those rows (sight, `BattleStartDist`, ...), changing
  the shared enemies' behaviour too; vanilla had always been affected on the 7 rows above.
- **Fix**: `apply` writes the bundles only to rows with `isBuddyAI = 1` (read from the original snapshot);
  other rows are skipped and logged as `skipped N shared with enemies`. Cost: the 7 vanilla spirits above (and
  spirits borrowing an enemy row in another mod) no longer get the AI improvements.
- Confirmed in game that the enemies are no longer pulled back.
