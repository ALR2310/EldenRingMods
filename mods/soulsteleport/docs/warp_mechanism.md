# The warp: teleporting to an arbitrary position

Status: **Confirmed** in game on 2.7.0.0 offline and with Seamless Co-op without dropping the session: across the
overworld and underground (`m18_00`, `m12_02`). Conventions: [research notes](../../../docs/research/README.md).
Code: `src/warp.rs`.

## Question

How can the player be moved to exactly where another player stands, however far, with a loading screen and
without breaking the co-op session?

## Rejected

A chat mod's `/warpto` only warps to the other player's last grace (Lua event `Lua_Warp_1` = 4085, id = grace -
1000). Considered and dropped:

- **Writing `CSChrPhysicsModule.position` directly**: only safe when the destination area is loaded; in Seamless
  Co-op players are often far apart, so the character falls into terrain that does not exist yet.
- **Warping to the grace nearest to the other player and then setting the position**: the nearest grace can still
  be outside the loaded area (a large empty region, another level, a block without a grace).

## Mechanism: replay the game's own sequence

Static analysis of `eldenring.exe` (starting from the `GameMan` fields of fromsoftware-rs `cs/game_man.rs`) found
the sequence the game uses when a multiplayer session ends, to return the player to where they stand
(`sub_1405F36E0` / `sub_1405F34A0` on 2.7.1.0):

1. `RequestMoveMap` (`sub_14067BA20`) writes `GameMan+0x14` = the target block, normalising an overworld block
   (area 50..88).
2. `SetCustomSpawn` (`sub_14067B970`) writes `GameMan+0xC90` the position in the block (w = 1.0), `+0xCA0` the
   orientation and raises the flag `+0xCB0`.
3. `GameMan+0x10` (`warp_requested`) = 1: the game shows the loading screen and changes map.

After loading, the spawn-point chooser (`sub_140AFE280`) sees the flag `+0xCB0` and takes `+0xC90` plus the origin
of the target block as the spawn position; `sub_14067A680` clears the flag.

**Step 3 deliberately writes `warp_requested` directly** instead of calling the game's trigger
(`sub_1405F89C0`): that function also calls into the session manager while online (`sub_1409F9A50`) and could
drop a Seamless Co-op session. Confirmed in game that the direct write does not drop the session.

The position comes straight from `PlayerIns.block_position` and `current_block_id` (fromsoftware-rs), already in
block coordinates, so no conversion from world coordinates is needed.

## Details

- `SetCustomSpawn` reads two vectors with `movaps`, so the `Vec4` struct must be `#[repr(align(16))]` (otherwise a
  crash).
- The orientation is built as the game builds `multiplay_join_orientation` (`sub_1406FC370`): `(0, yaw, 0, 0)`.
- An early version offset the position by +1 m on x so the two characters would not overlap; dropped, because the
  game pushes two overlapping characters apart by itself.
- **`warp::warp_pending`** (`GameMan.warp_requested` is on) stops the warper from warping while it has a move-map
  pending, and tells the answering side that it is changing map.

## AOB instead of RVA, no version lock

The first test on 2.7.0.0 failed at start-up: a panic `Unsupported game version 2.7.0.0` in `rva.rs`.
`GameMan::instance()` of fromsoftware-rs goes through a per-version RVA table (`rva::get()`) that only has 2.7.1.0,
unlike `WorldChrMan` and `CSMenuManImp` (singletons found by name). Now:

- `RequestMoveMap` and `SetCustomSpawn` are found by AOB with `shared::memscan::find_pattern_in_module`. The
  patterns deliberately include the `GameMan` offsets the functions use (`+0xB28`, `+0xC90/+0xCA0/+0xCB0`), so if
  a later build changes the layout the AOB misses and **the mod switches itself off instead of writing to the
  wrong place**.
- The `GameMan` pointer is taken from the `mov rax, [rip+disp32]` inside `SetCustomSpawn` (no RVA at all).
- `warp_requested` is written straight to `GameMan+0x10` (the game's setter `sub_14067BCF0` is
  `mov [rax+10h], cl`).

### Compatibility check (IDA)

Both patterns are unique on all three builds checked:

| Build | `RequestMoveMap` | `SetCustomSpawn` |
|---|---|---|
| 2.6.2.0 | `0x67ABD0` | `0x67AB20` |
| 2.7.0.0 | `0x67BA20` | `0x67B970` |
| 2.7.1.0 | `0x67BA20` | `0x67B970` |

## Tried in game

Offline on 2.7.0.0: every warp showed a loading screen and put the character in the saved place. Blocks tried:
`0x3C2A2400` (overworld `m60_42_36`, Limgrave), `0x12000000` (`m18_00`, the opening dungeon), `0x0C020000`
(`m12_02`, Siofra, underground, coordinates as large as `(1459, -813, 1547)`). `RequestMoveMap` returned the block
unchanged in all three (the overworld normalisation changes nothing for a block taken from `current_block_id`).

## Limits

Teleport is not blocked when the other player is riding Torrent in the air, in a boss fight or dying. Warping
while oneself is riding Torrent was not tried.
