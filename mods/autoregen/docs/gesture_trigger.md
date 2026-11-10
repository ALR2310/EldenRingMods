# Idle and gesture triggers

Status: **Confirmed** in game. The state machine in `src/trigger.rs` has a test per scenario below.
Conventions: [research notes](../../../docs/research/README.md).

## Question

How can `[PerTick]` tell that the player is idle, or sitting through a gesture, when the game exposes
neither?

## Idle (`Trigger = 3`)

`shared::player::main_player_action_snapshot` reads `CSChrActionRequestModule`: no movement input
(`movement_request_flags.raw_input`) and no held action (`r1`, `r2`, `l1`, `l2`, `sp_move`, `jump`,
`use_item`, `action`, `guard`, `rideon`, `rideoff`, `ladderup`, `ladderdown`). The player must stay still
for 5 seconds from the last frame that was busy.

## Gesture (`Trigger = 4`, `GestureId`)

The default list holds ten sitting gestures. `requested_gesture` is **half** of the gesture id a public
cheat table shows (Dejection 160 reads 80, Rest 184 reads 92, Sitting Sideways 186 reads 93); the factor
comes from real logs only. The list is read from the config so a new gesture needs no new DLL. The
feature was first called "Sitting" and renamed because sitting is only the default.

`requested_gesture` is valid for **one frame**, when `new_action_presses.gesture()` fires, so the result
is latched. The signal does not tell starting from cancelling: while a gesture is active, the next
gesture press, of any id, cancels it and stands up at once. Any other busy input (movement, attack, item,
guard, mounting) cancels as well.

### Confirmation (the fourth attempt)

A gesture pressed in the middle of a cast is blocked by the game, but `requested_gesture` is still
written, so it cannot count as sitting. Confirmation looks at the animation id of
`CSChrTimeActModule.anim_queue` a while later. No table of animations per gesture is needed, only whether
the animation changed since the press:

- A press in the list only records the time and the animation id at the press (the baseline).
- The gesture is active once the animation id is non-zero, differs from the baseline and has held for
  one second. Both conditions are needed: "not zero" alone fires while the cast animation has not changed
  yet, and "differs from the press" alone fires when the cast ends by itself and falls back to zero.
- A pending gesture that does not settle within 6 seconds is dropped (3 seconds was too short after a quick
  turn before the press).
- A repeated press while pending is **ignored**, without touching the baseline or the timer, so the first
  press runs its course. Overwriting the baseline made the confirmation never happen and then fire while
  standing up.

Tested in game: 5 of 5 blocked presses during a cast were rejected, 2 of 2 real sits confirmed, 2 of 2
stand-ups were immediate, and bursts of presses behaved. With nothing pending the cost is one atomic.

## Rejected paths

- `queued_action_inputs.gesture()`: only set for actions in `possible_action_inputs`, and the gesture
  wheel does not go through it, so the feature silently stopped working.
- `TaeCancelFlags::cancel_disable`: false even while casting.
- `possible_action_inputs.gesture()`: false even when the sit succeeded.
- `possible_action_cancels` (the OR of the whole bitfield): looked right over eleven tests but oscillates
  every 3 seconds between 0 and `31073500911` while standing still (an artifact of the idle animation
  cycle). Before trying another field, record several minutes without interaction and show that it is
  stable when not casting.
- Static analysis of `PlayGesture`: its gate (`GetManipulatorKind() == 1`, the pad) does not explain the
  bug, since solo play is always the pad, even while casting.
- Confirming on "not zero" only gave false positives; the animation passes through several transitional
  ids that change every half to one second, so it must hold, not be read once.

## Log

`Gesture: <id> ...` for each press and its outcome. `Tick: trigger=<n> met=<bool> (...)` only when the
pair changes, because idle flips every time the player walks and stops.
