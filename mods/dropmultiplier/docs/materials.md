# Material amounts

Status: **Confirmed** in game. Conventions: [research notes](../../../docs/research/README.md).
Code: `src/materials.rs`.

## Question

How can the number of materials from a pickup or a drop be multiplied without touching weapons, key items
or the materials that balance the game?

## Why it exists

A player asked for the multiplier to cover crafting materials and the things gathered in the open world
as well. No DLL mod did that. The third-party mods that do replace `regulation.bin` and edit item by item,
so they conflict with overhauls and do not cover the expansion.

## Config design

Three keys in `[Materials]`: `Crafting`, `Upgrade` and `Unique`. Three, because that separates the items
that affect balance the most (Scadutree Fragments, Sacred Tears, Smithing Stones on corpses, all in
`Unique`) from harmless farming.

Rejected: two keys by source (`Farmable` and `Unique`) that do not separate crafting from upgrade
materials; one key for all materials; the names `Repeatable` and `OneTime`. "Unique" is clear because the
`[Materials]` section already limits the context, and `Multiplier` in a key name is easily confused with
the drop rate.

## Classifying a slot

Based on the vanilla 1.17 exports of `ItemLotParam_map` and `EquipParamGoods`. Each slot of a lot row
falls into exactly one group, or none:

1. **Untouched:** the slot is not Goods (`lotItemCategory0N != 1`), or its `EquipParamGoods.goodsType` is
   not 2 (crafting material, 106 items) or 14 (upgrade material: Smithing and Somber Stones, Gloveworts,
   Golden Seeds, Sacred Tears, Scadutree Fragments, Revered Spirit Ash). Weapons, key items,
   consumables (Flask, Kukri and Golden Rune are all `goodsType 0`) and odd rows such as
   `ItemLotParam_map` id 2 (a random Crimson Tears flask roll with no flag and a 60% empty result) stay
   out. "No flag" does not mean "gathering point": filter by `goodsType` first.
2. **`Unique`:** the row's `getItemFlagId` or the slot's `getItemFlagId0N` is non-zero, so it can be
   obtained once.
3. **`Crafting` or `Upgrade`:** the rest, which come back after resting at a grace, split by `goodsType`.

Gathering points are the `ItemLotParam_map` rows without a flag, ids `9965xx` to `9993xx` (base game) and
`463xxxx` (expansion); `997200`, for one, gives Rowa Fruit as 1, 2, 3 or 5. Farmable Smithing Stones come
from respawning enemies (miners, golems, soldiers, in `ItemLotParam_enemy`); there is no vein that
respawns. A simulation on the vanilla CSVs gives 1602 crafting, 1046 upgrade and 1659 unique slots.

`goodsType` is read from `EquipParamGoods` at run time, with no hard-coded ids, so an overhaul classifies
its own items correctly. Lookup by id goes through `shared::params::row_ids`, not `repo.get()`, which
binary-searches the lookup table that panics on some regulations ([params.md](../../../shared/docs/params.md)).

## Formula and order

- `lotItemNum0N` (`u8`) is multiplied, rounded, with a minimum of 1 and a maximum of 255; a slot with a
  count of 0 is kept.
- The original counts and the group of every slot are captured once; every apply and reload recomputes
  from that snapshot, the same baseline pattern as [drop_rate.md](drop_rate.md).
- It applies to `ItemLotParam_map` (gathering points and items placed in the world) and
  `ItemLotParam_enemy` (drops from enemies and animals), after the drop rate, at start-up and on reload.
  The two features write different columns (`lotItemBasePoint` and `lotItemNum`), so they are independent.

## Not done

Multiplying gathered goods such as Erdleaf Flower and Trina's Lily was suggested and dropped: the person
asking did not follow up, and their `goodsType` was never checked.
