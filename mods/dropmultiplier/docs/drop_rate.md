# Enemy drop rates

Status: **Confirmed** in game on 2.7.1.0. Conventions: [research notes](../../../docs/research/README.md).
Code: `src/drop_rate.rs` (a test for each case of `scale_row`) and `src/lot.rs`.

## Question

How does the game choose what an enemy drops, and how can the chance be multiplied or fixed without
breaking the table?

## Mechanism

An `ItemLotParam_enemy` row has up to 8 slots, each with a weight (`lot_item_base_point0N`). The chance
of slot N is its weight over the **sum of the weights of its own row**:

`lot_item_base_point0N / sum(lot_item_base_point01..08)`

"Nothing drops" is just another slot (`lot_item_id0N == 0`) in the same sum. This matches the "% chance to
occur" column of a CSV exported from a param editor: `985 / (985 + 15) = 98.5%`, and after changing the
second weight from 15 to 30, `30 / (985 + 30) = 2.96%` and `985 / (985 + 30) = 97.04%`. The first slot
loses chance because it was not multiplied while the sum grew, which is not a bug.

`cumulate_lot_point0N` is left alone. It is 0 in every slot of every row of that export, and the editor
computes the chance from `lot_item_base_point` only. Despite the field name and what older Souls games
did, it is not what the Elden Ring lottery reads; writing a non-vanilla value there would rely on behavior
no game build has run.

## The two modes (`Mode`)

One at a time, never both.

- **`Mode = 0`, `Multiplier`:** multiplies the weight of every slot that holds a real item
  (`lot_item_id0N != 0`). Multiplying the "nothing" slot as well would change nothing, since numerator and
  denominator scale together, so it is left alone.
- **`Mode = 1`, `Percentage`:** forces the combined chance of "a real item drops" in each row to the chosen
  percent, keeping the relative shares of the items of that row.

### The algebra of `Percentage`

`itemSum` is the sum of the weights of the real slots, `otherSum` the weight of the "nothing" slots, which
stay as they are. Solving `target = itemSum_new / (itemSum_new + otherSum)`:

`itemSum_new = target * otherSum / (1 - target)`

Each real weight is then multiplied by `itemSum_new / itemSum`, so the group lands on `target` and every
item keeps its share. Edge cases:

- **`target >= 100%`:** the formula divides by zero, so the weights of the "nothing" slots are set to 0
  instead. A drop is guaranteed and the real weights stay.
- **A row without a "nothing" slot (`otherSum == 0`):** already drops every time and cannot go below 100%,
  so a `target` below 100% skips it: there is nowhere to put the removed chance.
- **A row without a real item (`itemSum == 0`):** nothing to put a chance on; unchanged.

A consequence: this mode can turn an item that always dropped (100%) into one that drops by chance.

## Reloading does not compound

A reload needs the game's **original** weights, captured once before any edit, so each reload recomputes
from them: multiplying by 2 and reloading with 2 must stay at 2, not become 4. The snapshot is keyed by
**row index**, not by id ([params.md](../../../shared/docs/params.md)), and guarded by a `Mutex` because
the first capture runs on the module's thread and later reloads on whichever thread the game's
`FrameBegin` group uses. A poisoned lock is still used so one panic does not break every later reload.

## Open risks

Only the base rate is changed: the player's Discovery stat still applies on top, as in vanilla.
