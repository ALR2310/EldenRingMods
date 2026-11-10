# The player and its characters

Status: **Confirmed** in released mods of the old repository (not yet re-tested in game from this one). Conventions: [research notes](../../docs/research/README.md).
Code: `shared/src/player.rs`.

## Question

When is it safe to read or change the player, and how are the characters of a set walked safely?

## Mechanism

- **`main_player_chr_ins_ptr()` is the gate.** A singleton's `instance()` returning `Ok` only says the
  manager exists, which it does at the title screen. Features that need real data (`GameDataMan` save data,
  the param tables of `SoloParamRepository`) must wait until the main player exists. `SoloParamRepository`
  can be `Ok` while its tables are still empty.
- **`wait_for_solo_param_repository()`** waits for both, logging once because picking a save and loading
  in can take minutes.
- **`for_each_active_chr(set, f)`** dereferences only `Active` and `ReadyForActivation` entries; any other
  entry can be stale.
- **`main_player_action_snapshot()`** reads the pad input of `CSChrActionRequestModule`. `r1`, `r2`, `l1`,
  `l2` and `new_gesture` are true for one frame, on the press, and `requested_gesture` means something only
  on that frame. `busy` is every other way the player is not idle: held actions and movement input.

## Use

Check the gate at the start of every tick and return when it is `None`. Write to the player only from a
game task (`FrameBegin` for stats), never from a worker thread.
