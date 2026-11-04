# Keeping spirits through a rest at a grace (`KeepSpiritsOnRest`)

Status: **Confirmed** in game on 2.7.2.0: solo, and Seamless host plus a second instance. Resting at a grace
keeps the spirits. Fast travel was investigated and **rejected**. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/keep_on_rest.rs`.

## Question

Resting at a grace resets the world and removes the spirits. Which game path removes them, and can it be
blocked instead of summoning them again?

## How the game manages and removes spirits (2.7.1.0 and 2.7.2.0, from IDA)

`SummonBuddyManager+120` is a `std::map` keyed by the **id of the player who owns the spirits** (the key is
`*(PlayerIns+1408 -> +8)` through `sub_1406576E0`; it is not a map or block id). A node has the key at `+32`,
the group list at `+48` and the group count at `+56`, so in co-op each player has their own key.
Addresses below are 2.7.2.0 unless noted.

- `DisappearAll` (`sub_1404B8240`; 2.7.1.0 `sub_1404B8160`) only marks groups for recall (`+49 = 1`,
  `+52 = manager+140`, applies SpEffect 60505 or 1840 to the character) under the **local player's** key.
  It does not delete characters.
- The per-frame update `sub_1404B9390` (2.7.1.0 `sub_1404B92B0`) calls `DisappearAll` for four conditions:
  1. the player has SpEffect 202 (`sub_1403F21B0(player, 202)`);
  2. the recall flag `manager+0x28` (the Ash was pressed again);
  3. "left the stone area": `manager+0x20 < 0` and the flags `+181` and `+183` are both off;
  4. the player is in state 13 (`sub_1404B77F0()`) and HP <= 0 (`sub_140437B10`).
- The same function, second loop: for every key that still has groups it looks the owner up with
  `sub_140509480(WorldChrMan, key)` (walks `player_chr_set` at `WorldChrMan+69344`). When **no such
  player is found the whole group is destroyed** through `sub_1404BBB80` (2.7.1.0 `sub_1404BBAA0`),
  which removes the warp entry and then the character. This cleans up when **the owner leaves the
  session**, not when a block unloads.
- Third loop: a local group whose node has `+48` set (recalled) is removed when `+52` seconds run out.
- Fourth loop: a local group with the node byte `+48` (= `group+0x20`, `warp_requested` in fromsoftware-rs)
  is destroyed at once (`0x4B97D5`).
- `sub_140C98CA0` (2.7.1.0 `sub_140C987E0`) calls `DisappearAll(local)` after a series of session clean-ups.

## Finding the real path

Passive logging of the manager and groups while resting solo:

- At the rest the player gains SpEffect `202` (with `100010`, then `9608`, `9609`, `9610`, `100690`). SpEffect
  202 is a 0.1 s effect with nothing else in the row; its source is the grace's rest script.
- Blocking only the SpEffect 202 branch (redirecting `call sub_1403F21B0(player, 202)` to a function that
  returns "none") was **not enough**: no group got the `disappear` flag, yet both groups vanished together
  about 3 seconds later, and their warp entries with them. So another path deletes them.
- **Rejected**, one by one: a summon request (`manager+0x20` stayed -1, `dismiss` 0; the stone
  fields only change after the group is gone, they are recomputed from the groups); the owner-leaves loop
  (the owner id stayed `0` and the player stayed in `player_chr_set`; a test hook returning the local
  player when the lookup failed logged no failure); `DisappearAll`; the TAE 2030 vanish event on the spirit
  (`sub_1404B8440` through the TAE handler `sub_14040D190`, which sets the node byte of loop 4); a recall
  SpEffect on the spirit (the spirit's SpEffects did not change from the rest to the loss, only the
  summon-time `35, 5255, 290000`); no raw byte of any group flipped before the group disappeared.
- **Confirmed** by counting calls at the four call sites of `sub_1404BBB80`: only site 2 (the loop of
  groups carrying the `group+0x20` flag) ran, twice for the two groups. That flag is set in bulk for the
  groups of **every** owner by `sub_1404B83B0(manager)`, called from `sub_14050E0E0(WorldChrMan)`
  (2.7.2.0 `0x50E140`; 2.7.1.0 `0x50DE60`), the "reset every character" function that `sub_14050F220` runs
  during the grace's world reset (it also calls slot `+0x50` of every ChrSet in the list at
  `WorldChrMan+0x10DC8`, count at `+0x10ED8`). This matches the observation that other players' spirits
  also vanish in Seamless even though those players are not reset.

## Mechanism of the fix

`KeepSpiritsOnRest` (hot reload) redirects two `call`s with `codepatch::redirect_rel32`, both found by
patterns unique on 2.6.2.0 to 2.7.2.0:

- the `call sub_1403F21B0(player, 202)` in the update function, answering "none" while on, so
  `DisappearAll` does not run at the rest;
- the `call sub_1404B83B0` in `sub_14050E0E0` (`RESET_AOB`), doing nothing while on, so the groups are not
  flagged for removal.

With the feature off both call the originals.

### Seamless

Under Seamless the first block stopped working: only the SpEffect 202 counter rose (7 times on both machines,
the 7 frames of the 0.1 s effect) and the destruction followed. A diff of the game code in RAM against the
executable shows Seamless redirects the same `call sub_1403F21B0` (and puts an inline hook at the start of
the update function); it initialises after this mod and overwrites the mod's target. `keep_on_rest.rs`
therefore checks once a second whether the call still points at the mod's stub; if it was replaced, it
patches again and chains to the new target (Seamless's stub) and logs `patch was replaced, applied again`.
Tested host plus second instance: both players' spirits survive a rest.

## Open risks

- SpEffect 202 may be applied elsewhere (fog gates, cutscenes, arenas that forbid summoning); those no longer
  recall the spirits either. A few such situations should be tried.
- **Not established**: whether slot `+0x50` of the ChrSets removes the spirit characters in the reset (whether
  the summon ChrSet is in that list). The groups and characters survive in the solo and Seamless tests, so
  this has not shown up.
- Seamless remote copies of spirits also disappear through `sub_1404B7080` (removal on a network event of
  `sub_14050E410`, only in a session); that path is not touched.

## Diagnostics

`Logging.PersistProbe` (`src/probe/persist.rs`, off by default) logs on every change the block the player is in,
the owner key of each spirit group (flags, HP, raw flag bytes), the player's and the first spirit's SpEffects and
the manager's request fields, and counts the calls of the group-destroy and `DisappearAll` sites with counting
stubs (a `call` redirect, nothing runs in Rust inside the hook). Reading it around a rest tells which path
removed the spirits.

## Fast travel: rejected

Solo test (summon, then fast travel to the very grace being rested at) with the diagnostics on: while the
loading screen shows (`block 0xffffffff`) the whole `groups` map vanished (not even the key `0` entry,
unlike a rest), the spirit characters were gone (`chr not found` in the warp log),
`active_summon_speffect_id` became `-1` (a rest keeps it), and every `DisappearAll` and group-destroy counter
stayed 0. The whole world and the spirit manager are destroyed and rebuilt through the loading screen, so
there is no single removal command to block as there is for a rest. The only way left is to summon the Ashes
again after the load (save the active Ash SpEffects, write them into `request_summon_speffect_id` at `+0x20`
once the world is ready); spirits would reappear with the summon animation and full HP, which was judged
not worth doing.

## Related

`src/multi_spirit.rs` (`disappear_all_hook`, `groups`), `src/buddy/stone.rs`, `src/warp.rs`,
[multi_spirit.md](multi_spirit.md), [summon_anywhere.md](summon_anywhere.md), [mimic_tear.md](mimic_tear.md).
