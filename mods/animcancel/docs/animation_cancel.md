# Animation cancel

Status: **Confirmed** in game on 2.7.1.0 for guard, dodge, weapon swap and jump; the guard rules
are **Inferred** from logs and param data (not every grip was measured). Conventions:
[research notes](../../../docs/research/README.md).

## Question

An animation lets another action take over only inside a cancel window that its own events open,
usually in the second half of a move. Can a mod open that window earlier, so guard, dodge, weapon
swap or jump act as soon as the button is pressed?

## How the game decides

`CSChrActionRequestModule` (`ChrIns.modules.action_request`, 0x140 bytes in fromsoftware-rs) holds
the state. The fields that matter:

| Field | Meaning |
|---|---|
| `action_requests` | buttons held now (`ChrActions` bitmask: bit 5 `sp_move`, 24 `guard`, 9 and 10 weapon change) |
| `possible_action_inputs` | which actions may be queued during this animation (set by the TAE) |
| `possible_action_cancels` | which actions may cut this animation (set by the TAE) |
| `disabled_action_inputs` | presses dropped before queueing |
| `tae_cancels` | TAE cancel flags; bit 11 `cancel_disable` blocks every cancel |
| `action_request_queue.cancel_entries[id]` | the permission the HKS reads for animation `id` |
| `action_request_queue.input_entries[id]` | the request being carried out; filled each frame, never written by the mod |

Each frame the module resets, then computes `queued = presses AND possible_action_inputs` and
`cancel_ready = queued AND possible_action_cancels AND NOT cancel_disable` (function at RVA
`0x408190` on 2.7.1.0), and fills `input_entries[id] = queued AND cancel_entries[id]`. The player
always runs in queue mode; `id` is the queue's `current_tae_id`, or `queue_tae_id_override` when
that is not negative, written as `aXXX_YYYYYY` without the `a` and `_` (`a020_032000` is
`20032000`). Cancel is recomputed from scratch every frame, so nothing stays set that needs
cleaning up.

Task order within a frame: `ChrIns_PreBehavior` (module reads the pad) → `HavokBehavior` (HKS and
TAE run) → `ChrIns_BehaviorSafe` → `ChrIns_PrePhysics`.

## What works

Each frame, at `ChrIns_PreBehaviorSafe` and at `ChrIns_BehaviorSafe`, for the animation that is
playing:

1. OR the action bits into `possible_action_inputs` and `possible_action_cancels`, and clear them
   from `disabled_action_inputs`.
2. OR them into `cancel_entries[id]`, **creating the entry if it is missing** (it only exists once
   an event wrote it).
3. OR the `tae_cancels` flags the action needs and clear `cancel_disable`.

Bits per action: guard = 24 and 2 (guard is the L1 button, bit 2; bit 24 alone queued no press);
dodge = 5, 16, 17, 18, 25; jump = 6; weapon swap = 4, 9, 10 and the queue-only bit 37. Weapon swap
also needs the `slot_switch` flag. Jump needs only bit 6 and the general action flag; bit 6 is
`ACTION_ARM_CHANGE_STYLE`, which Elden Ring uses for jumping.

Which animation is playing is decided by the **queue id**, not by `anim_queue`: while casting
during a walk, `anim_queue` shows the leg layer `a000_020010` (group Other) for a moment although
the queue id is still the spell's. The id lags a few frames when a new animation starts, which only
means the previous animation is still being cut.

`group_for_cancel` is `group_of` except that every spell animation (TAE `a400` to `a599`) is a
cast, so a long beam phase that `group_of` files under Other can still be cancelled. Critical hits
are not Attack and are never cut.

## Verification

| Try | Change | Result |
|---|---|---|
| 1 | force the bitmask at `BehaviorSafe` | `cancel_ready` has the bit early, no cancel |
| 2 | + bit 18, `tae_cancels` flags, write at `PreBehaviorSafe` too | still only cancels at the vanilla time (about 0.57 s) |
| 3 | + write `cancel_entries` and `input_entries` | strange fake commands (attack, swap, swap, roll) |
| 4 | `cancel_entries` only, create the entry | **roll and guard cancel an attack** |
| 5 | + clear `disabled_action_inputs`, bit 4 for weapon swap | swap registered early but ran late |
| 6 | + queue bit 37 for weapon swap | **weapon swap cancels an attack** |

## Rejected paths

- **Widening the module's bitmasks alone.** The module did the right thing, but the HKS did not
  switch state until the vanilla moment. The HKS reads the per-animation entry.
- **A flag in `CSChrActionFlagModule` blocking the cancel.** Logging every change of that module
  showed flags that follow the animation, none that block the cancel.
- **Writing `input_entries`.** It is the list of requests; writing it turns each press into a fake
  command.

## Guard rules

Guard (L1) only cancels while L1 really guards. With twin blades or one weapon in each hand L1 is an
attack, and letting it cut would break the combo. `EquipParamWeapon`: `enableGuard` is 1 only for
real shields; the shield animation sets (`wepmotionCategory`) are 47 greatshield, 48 small shield,
49 medium shield, 57; twin blades have `isDualBlade = 1`.

- Both hands on one weapon (`RightBothHands`, `LeftBothHands`): guards, because the other slot's
  weapon is put away, unless the weapon is a twin blade.
- One-handed (`OneHanded`): guards only with a shield in the left hand.
- Weapon unknown or no param row: guard cancels nothing.

The param row is found by the equipped ID, else by the ID without its last two digits (9080009 is row
9080000 + 9); the answer is cached by (grip, left ID, right ID). Not measured in game: both hands on
one weapon, shield plus one-handed weapon, two katanas, twin blades.

A guard that is already held when an animation begins is no new press: letting it cancel would cut an
attack started from a raised guard at once (hold guard, press attack: the attack was cut and the guard
raised again). A new animation therefore starts with guard blocked while L1 or guard (bits 2 and 24)
is held; the block ends when it is released, and pressing again counts as new.

## Open risks

- Side effects of the cut animation: invincibility frames, hitboxes, stamina, facing, FP spent when
  cutting a spell.
- Seamless Co-op: animation IDs are synchronised, so other players may see jerky motion.
  `cancel_entries.push_back` uses the game's allocator; no problem seen in testing.
