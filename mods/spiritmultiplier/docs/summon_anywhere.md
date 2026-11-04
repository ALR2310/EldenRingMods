# NoRestResummon and SummonAnywhere (BuddyStoneParam)

Status: **Confirmed** in game on vanilla, Reforged and Seamless Co-op. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/buddy/stone.rs`.

## Question

How can spirits be summoned again without resting, and outside the summoning pool areas, when the game
checks a lock, a stone object and a range?

## NoRestResummon

A param edit only: `BuddyStoneParam.summonedEventFlagId` (`+0xC`) = 0. The event flag is raised on a
summon at a pool and cleared only by resting, which is the "once per rest" lock. The original value is
captured once and applied from the snapshot (F5 works both ways). Log:
`304 summoning pool row(s), 302 with a once-per-rest flag`.

It cannot remove Reforged's own rule (after the Spirit-Severing Blade the Ash slot stays grey until a
rest): see [open_issues.md](open_issues.md).

**Interaction with SummonAnywhere** (tested in game): vanilla's lock (the Ash greyed after a recall or death
until a rest) exists only while `SummonAnywhere` is off. With it on, the stateInfo 373 stub forces
`al = 1` and overrides the lock, so `NoRestResummon = false` did nothing. Fix: with
`NoRestResummon = false` and Anywhere on, a tick reads the original `summonedEventFlagId` of the current
stone from `CSEventFlagMan`; if the flag is raised and no spirit is out, `ANYWHERE` is set to 0 so the game
answers for itself (greyed). The lock is per stone, so another stone (another map cell) unlocks again.

## SummonAnywhere

The param edits (`activateRange +0x1C = 65535`, `overwriteReturnRange +0x1E = -1`,
`overwriteActivateRegionEntityId +0x20 = 0`, `eliminateTargetEntityId +0x8 = 0`) were **not enough**: in the
Shadowlands and outside a pool it still failed. Three more things need a stone object loaded near the
player:

1. `GetBuddyState` (`sub_1404B72F0`) allows an Ash only while the player carries a SpEffect with
   **stateInfo 373** (`sub_1404FA370(player+0x178, 0x175)`, `0x1404B7368`); the HUD function
   `sub_1404B7610` (tail call of the same check) does the same. The pool area puts that effect on the
   player.
2. The current stone `+0x38` is set only by **the stone's own ESD script** (ESD command 122 ->
   `sub_1404B8050`) and **cleared every frame** by `sub_1404B6E80` (from `sub_140EAFDE0`). With no stone
   loaded `+0x38 = 0`, so `GetBuddyState` = -1 (greyed).
3. `sub_1404BD870` (in `Update`, `0x1404B8B2E`) recomputes `+0xB5` (`is_within_activation_range`) and
   `+0xB7` (`is_within_warn_range`) each frame by finding the stone object `+0x3C` (`sub_1405EEFD0`); with
   the stone gone the flags are false and the "left the area" branch dismisses everything.

Patches (only while `SummonAnywhere` is on, through the `ANYWHERE` flag the stubs read; all redirect a
`call`, see [seamless_conflict.md](seamless_conflict.md)):

- stateInfo 373: the `call` form (AOB `BA 75 01 00 00 48 8B 88 78 01 00 00 E8 ...`, `0x1404B735C`) and the
  tail `jmp` form (`0x1404B7655`) answer `al = 1`. The stub in `GetBuddyState` (rbx = manager) writes
  `FALLBACK_STONE` into `+0x38` when it is 0, right before it is read; every summon path asks
  `GetBuddyState` first. While spirits are out it prefers the active stone `+0x3C`, so `+0x38 == +0x3C` and
  the state stays 2 ("this group is out").
- In-range: a stub for `call sub_1404BD870` (AOB `41 0F 28 CA 49 8B CF E8 ...`, `0x1404B8B1B`).
- `FALLBACK_STONE` is the nearest real stone seen last, or the first BuddyStoneParam row without a
  `dopingSpEffectId` (for example `10000100`). A BuddyStoneParam row ID is the stone's entity id (param 134 is
  looked up with it directly, `sub_140D28320`).
- If the three patches cannot be installed it falls back to the param edits only (the error is logged).

### Reforged fix

The first in-range stub forced `+0xB5 = 1, +0xB7 = 0` **every frame, even with no spirit out**. The routine
that runs before it, `sub_1404B8EB0`, attaches the "entered the summoning range" SpEffects
(`GameSystemCommonParam.onBuddySummon_inActivateRange_spEffectId_pc` = 9540 on the player, `_buddy` = 9541 on
the spirit) on the **0 to 1 edge** of `+0xB5`; with the flag forced there was never an edge, so none were
attached. Reforged relies on them for the Spirit Ring and for allowing the Ash and the Blade. Fix: set
`+0xB5 = 1` only when a spirit is out (`+0xB4 player_has_alive_summon`) **and** the game computed no range
(`+0xB5 == 0 && +0xB7 == 0`); otherwise leave the game's values. Found by splitting `SummonAnywhere` into
three test switches (Params, State, Range) and enabling one at a time; Range was the cause.

## Observations

- Limgrave: `stones: current=` follows where the player stands (`1045380100`, `1046380100`, ...; the two
  middle number pairs are the map cell `m60_XX_YY`), sometimes `0` (no stone).
- Each stone has its own `dopingSpEffectId`, so the spirits' buff can differ with the stone chosen.
- Final test: summoning works in the Lands Between and in the Shadowlands outside a pool; after summoning at
  a pool and riding Torrent far away the spirits stay; sending back and summoning again works.
- `+0x3C` returns to 0 in `sub_1404B8EB0` when no spirit remains.
- **Rejected**: writing `+0x38` from the FrameBegin tick (the clear runs after it); an inline patch of the
  clearing instruction `mov [rsi+38h],0` (conflicts with Seamless).
