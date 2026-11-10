# Spirit slots, the summon band and the spirit cap

Status: **Confirmed** on 2.7.1.0 offline (100 spirits) and in two-player Seamless Co-op. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/band.rs`, `src/chain.rs`.

## Question

Why does an Ash never summon more than 10 spirits however long its summon chain is made, and how can
the cap be raised without reallocating a live game structure?

## Mechanism

- Spirits live in `WorldChrMan.summon_buddy_chr_set`, a `ChrSet<ChrIns>` that also holds Torrent. The
  game builds it with a capacity of **80** (the init routine does `alloc(0x500)`, `capacity = 0x50`,
  three imm32). Slot 0 is Torrent, slots 1 to 19 are other players' Torrents, and the local player's
  band starts at **slot 20**.
- The cap sits in the "spawn one spirit" routine: `band_start = 10 * (player_index + 2)`,
  `cursor %= 10`, walk 10 slots for an empty entry, return null when none is free (the spirit is
  dropped). The next cursor is `(slot - 20 + 1) % 10`. The cursor is `SummonBuddyManager.last_buddy_slot`
  (`+0xBC`).
- Offsets on 2.7.1.0: `WorldChrMan.summon_buddy_chr_set = +0x10F90`, `summon_buddy_manager = +0x1E538`.
- Dismissing and despawning does not walk a hard-coded band: 30 spirits at slots 20 to 49 were cleaned
  up completely, including slots 30 and above.

## What the mod patches (`band.rs`, installed once at start-up)

1. **Slot search** (unique AOB `0F B6 C2 83 C0 02 8D 04 80 44 8D 14 00 B8 67 66 66 66 41 F7 E9`): the
   whole search loop is replaced by a stub that walks a band of `MaxSpirits` slots for player index 0,
   checks `slot < capacity`, and leaves through the same two exits as vanilla.
2. **Next cursor** (AOB `83 E8 14 78 22 8D 48 01 B8 67 66 66 66 F7 E9 C1 FA 02`): returns
   `slot - 19` without wrapping; the stub turns that back into a position inside the caller's own band.
3. **Capacity** (only when `MaxSpirits > 60`): the three imm32 of the init routine are raised to
   `20 + MaxSpirits`, so the game builds the larger array itself. It must run before the world is
   created, which is why `MaxSpirits` is read once and needs a restart. If this patch fails the band
   falls back to 60. Slot indices are encoded in `FieldInsHandle` with a 20-bit mask, so large capacities
   are representable.

`MaxSpirits` is clamped to `60..=1000`; at 60 the game's 80-slot array stays as in vanilla. At
`MaxSpirits = 100` offline, 100 wolves spawned in slots 20 to 119, loaded over about 2 seconds,
dismissed cleanly and did not crash. FPS drops with the count. A further engine ceiling (the
character heap) was not searched for.

`chain.rs` lengthens the SpEffect to BuddyParam chain in
`WorldChrMan.summon_buddy_manager.trigger_speffect_to_buddy_map` according to `Multiplier` or
`Amount`, capped at the band length:

- The game's head node is left alone; only its `next` is redirected to a leaked extension that cycles
  through the original IDs (never freed, the game may still hold the old chain).
- The original chain is snapshotted once per SpEffect ID and every rebuild starts from that snapshot,
  otherwise `Multiplier` would compound on every tick.
- The check is repeated on a periodic tick (cheap and idempotent), which also picks up a hot reload.
  Resetting the settings to vanilla returns the vanilla length.

## Seamless Co-op

- Seamless raises the capacity to **1000** on every machine. The routine that creates a spirit received
  over the network does not check `slot < capacity` and writes `entries[slot]` directly, returning the
  existing character if the slot is taken. So every player's band must fit in every machine's capacity
  and bands must not overlap.
- The stub splits bands by the real capacity (`band_layout`, unit-tested): index 0 gets
  `[20, 20 + MaxSpirits)`; indices 1 to 5 get `part = (capacity - 20) / 6`, start at
  `20 + index * part` and are `min(MaxSpirits, part)` long. Capacity 80 gives `part = 10`, exactly the
  vanilla bands. Capacity 1000 gives starts `20, 183, 346, 509, 672, 835`.
- Two players with the mod and `Amount = 30`: the joining player summoned all 30 at slot 183 onward,
  both machines saw all 60 spirits, and the dismissal matched.
- **Limits**: a partner without the mod still uses the vanilla bands (slots 30 to 79), which overlap
  the host's widened band; with `MaxSpirits > 163` under Seamless, band 0 overlaps band 1; all machines
  are assumed to share one capacity.
- **Inferred, not confirmed**: right after one side reloads an area, only part of the spirits
  sometimes sync (11 of 30, 13 of 30). It looks like engine or Seamless network sync.

## Diagnostics

`Logging.SlotProbe` (`src/probe/slots.rs`, off by default) logs the summon ChrSet's capacity, the length histogram of
the vanilla chains (`{1: 58, 2: 18, 3: 8, 4: 4, 5: 6}`), the slot layout whenever it changes, `last_buddy_slot`
and the stone fields. It is what located the cap.

## Rejected paths

- Reallocating the live `entries` array of the ChrSet: high risk (the engine may cache pointers and
  indices), and unnecessary because 80 slots are enough for 60.
- Spawning NPCs outside the buddy system: loses the "follow the player" AI and the despawn with the
  Ash.

## Open risks

- A third-party mod that hooks the same slot-reserve routine cannot coexist with `MaxSpirits`.
- The 60 spirit default is the largest count that keeps the vanilla array; see
  [activate_limit.md](activate_limit.md) for why large counts make enemies vanish.
