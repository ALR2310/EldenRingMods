# Disabling the pickup animation (`NoPickupAnimation`)

Status: **Confirmed** in game on 2.7.2.0. Conventions:
[research notes](../../../docs/research/README.md).

## Question

Auto pickup of materials and corpse loot plays no animation, but a ground item does, and it
plays even while mounted. Which game data controls it, and can it be switched off without
losing the item?

## Mechanism

`GameSystemCommonParam` (one row) has three `msbEventGeomTreasureInfo_*` groups for treasure
placed on the map:

| Kind | `actionButtonParamId` | `itemGetAnimId` |
|---|---:|---:|
| shine (glowing item on the ground) | 4250 "Pick up item" | 60070 |
| corpse (remains) | 4200 "Pillage remains" | 60070 |
| box (chest) | 4300 "Search treasure chest" | 60090 |

With `NoPickupAnimation` on, the mod writes the nonexistent animation id `999999` to
`itemGetAnimId` of the shine, corpse and box groups and to
`PlayerCommonParam.animeID_DropItemPick`. Turning the option off restores the original values.
The original values are stored on the first write and logged:
`Animation: drop item pick <id>, shine <id>, corpse <id>, chest <id>`. The code is in
`src/animation.rs`.

Materials are different: their `ObjActParam` row (button 7800, row 90017) has
`playerAnimId = 0`, which is why they have no animation. The Items group has no `ObjActParam`
row at all, so its animation comes from the treasure path above, not from `ObjActParam`.

## Verification

- **Confirmed:** with the option on, ground items, remains and chests are picked up without
  the animation and still reach the inventory.
- **Rejected:** `PlayerCommonParam.animeID_DropItemPick` alone. It is 60070 on vanilla 2.7.2.0;
  writing 999999 did not stop the ground-item animation, so that field does not control it.
  The shine entry in `GameSystemCommonParam` is the one that matters.
- The `ObjActParam` fields `player_anim_id`, `action_button_param_id` and `event_kick_timing`
  were investigated as an alternative. They are not needed with the treasure entries above.

## Open risks

- The animation of picking up while on horseback is the same data; no separate horse field
  exists in `PlayerCommonParam`.
- If an item were only handed over at `event_kick_timing` inside the animation, removing the
  animation could keep it out of the inventory. This did not happen in testing.
- The default is `NoPickupAnimation = true`, so a player who updates from a version without the
  option gets the animation disabled when the key is added from the template.
