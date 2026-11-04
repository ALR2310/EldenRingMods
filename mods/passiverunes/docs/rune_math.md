# How runes are computed each interval, and the milestones

Status: **Confirmed** in game in the released versions (2.1.x). The level-up cost formula was verified in
game. Conventions: [research notes](../../../docs/research/README.md). Code: `src/rune.rs` (the task and
the game access) and `src/math.rs` (the pure parts, with a test for each case).

## Question

How are runes granted over time, what is added each interval, and how do milestones behave when frames
hitch or the player is not in the world?

## Reading and writing the runes

The mod reaches the player through `WorldChrMan`, `main_player` and `PlayerIns::player_game_data`
(`PlayerGameData`, the same struct `GameDataMan.main_player_game_data` points to), wrapped in
`with_player_game_data()`. `GameDataMan` alone is not used: its `FromStatic` goes through the address
table of fromsoftware-rs for one game version, which broke the mod on every game patch, and it
resolves already when the save data loads, **before** the player is in the world. A tick that ran
there granted "+25 runes" every 5 seconds while still at the title screen. `main_player.is_some()` is the
in-world gate, which this path gives for free ([player.md](../../../shared/docs/player.md)).

The offset of `rune_count` (`0x6C`) matches `[RCX+0x6C]` in `AddSoul_Call` of `mods/runemultiplier` and
the old C++ version of this mod, which found it through a pointer chain from `GameDataMan`: independent
reverse-engineering efforts reached the same field.

## The three parts of an interval

Each interval (`[Passive] Interval`, in milliseconds) the mod adds a total of **three independent parts
that stack**; setting a part to `0` turns that one off. The result is always clamped to
`0..=999,999,999` (the game's real cap) and never goes below 0. With `Enabled = false` the mod does
nothing, neither the interval nor the milestones.

### 1. A fixed amount (`Amount`)

`Amount` runes per interval. This is the original behavior of the mod.

### 2. A percent of the next level-up cost (`Percent`)

Requested because a fixed amount quickly means nothing at a high level. Each interval adds
`floor(level_up_cost(level) * Percent / 100)` runes, **at least 1** when `Percent != 0`.

- **It stacks with the fixed amount**, not replaces it. The first idea was to choose one (`Percent > 0`
  ignores `Amount`); combining them won: each tick gets `Amount` plus the percent. They share one
  `Interval` clock, which keeps the timing easy to work out (for `Percent = 1` and `Interval = 10000`,
  100 ticks, about 16.7 minutes per level from this part alone).
- **It always uses the current level**, not the runes held: collecting runes without leveling up keeps
  earning the percent of the next level on every tick.
- The level is `PlayerGameData.level`, next to `rune_count`.
- **The level-up cost has no param in the game.** It is a formula hard-coded in the executable that the
  community worked out: `l = level + 81`, `x = max(0, (l - 92) * 0.02)`, `cost = floor((x + 0.1) * l^2 +
  1)` (`level_up_cost()`). Level 1 to 2 gives 673, matching the level-up screen. **Verified in game**:
  level 257 to 258 gives 573,505 runes, exactly what the screen shows.

### 3. Compound interest on the runes held (`Interest`)

A player's idea: a third mode, a percent of the runes held. A `Mode = 1/2/3` layout was rejected because
it contradicts the "stack" decision above; a third key that also stacks replaced it, each part off at `0`.

- **Exponential growth.** Simulated with `Interval = 10000` and only the interest: `Interest = 1`
  doubles every ~70 ticks (~11.6 minutes), reaching the cap in ~25 minutes from 229 million runes and in
  ~3.2 hours from 10,000; `Interest = 0.1` from 1,000 runes reaches the cap in ~38 hours. So the template
  suggests about `0.05` to `0.2`. A first version had an `Interest.Max` cap on the runes per tick; it was
  **dropped the same day** on the player's decision: no limit, let the interest run free and choose a small
  `Interest` for a slow one.
- **The fraction is carried** (`InterestState::carry`): rounding down every tick would make `0.1%` of 1,000
  to 1,999 runes always pay 1 (1,000 ticks to double instead of ~694) and pay nothing below 1,000, which is
  not real compound interest. The fraction is kept for the next tick and **reset** when the runes held are
  lower than after the previous grant (`last_held`: spent or lost on death), when `Interest` is turned off,
  or when its sign flips on a hot reload.
- **No minimum**, unlike `Percent` (minimum 1): holding 0 runes earns 0. Losing the runes on death loses
  the principal, a natural risk and reward. It is computed on the runes held, not on dropped runes.

## Negative values: taking runes over time

`Amount = -50` takes 50 runes each interval, and so do `Percent`, `Interest` and the bonuses in
`Milestones`. Before, every negative value was clamped to 0 (off).

- All arithmetic is `i64`, and `apply_runes(i64)` clamps to `0..=999,999,999`.
- A negative `Percent` is `-(floor(cost * |p| / 100))`, still with a minimum of 1 rune (-1 this time) when
  `p != 0`, symmetric with the positive side.
- A negative `Interest` is compound decay (it takes `|p|%` of the runes held each tick). The fraction
  rounds toward 0 (`trunc`) so it is right in both directions.
- The parts still stack: `Amount = -50` with `Percent = 0.25` takes 50 and adds 0.25% of the level-up cost.
- `Interval` is clamped to `>= 0` (a negative interval means nothing). A milestone with a **negative
  number of seconds** is skipped (it used to be parsed and paid the moment the player entered the world).
- **Values are clamped to the ranges the template states:** `Amount` to `-999999999..999999999`, `Percent`
  and `Interest` to `-100..100`. Before, `Percent` and `Interest` were not clamped (500 still ran).

## Milestones (`Milestones`)

A string of `seconds:bonus` pairs separated by commas. An entry that does not parse or has negative
seconds is skipped; an empty string gives an empty list (it turns the milestones off without turning off
the interval). The list is sorted by time, and it is read **once** at start-up: a milestone already granted
stays granted across a reload, and only future ones see new amounts.

- **Threshold crossing, not an exact match.** The C++ version compared `elapsed == atSeconds`; a tick out
  of step (a hitch, or an `Interval` that does not divide the time) lost the bonus for good. The Rust
  version compares `session_elapsed >= milestone.at`, so a milestone cannot be missed.
- **It only moves to the next milestone when the grant succeeds.** An earlier loop advanced
  `next_milestone` even when the add failed (before the player was in the game), losing the bonus; now a
  failure stops and the next tick tries that same milestone again.
- **The session and interval clocks run only in the world.** They used to count from the moment the task
  registered, so time at the title screen and on loading screens counted toward milestones (a real log
  showed `session 90s` on the first tick before entering the game). Out of the world a tick returns at once
  and adds to no clock. They **pause, not reset**, when the player leaves the world, because resetting on a
  loading screen would erase the progress of the milestones at every teleport. So leaving to the title
  screen and loading another save keeps counting from the old number, without the time spent outside.
- A milestone is still a fixed number; a percent is not supported.

## Defaults

By the player's decision the default is, every 10 seconds, 50 fixed runes plus 0.25% of the level-up cost
(about 400 ticks, 67 minutes per level from the percent alone). For players of older releases: the config
loader only **adds** missing keys, so an old file keeps its old `Amount`, but `Percent` was a new key and
was added with `0.25`, so 2.1 **turned the percent part on** even for them, stacking with their old
`Amount`. Accepted: it is the main feature of that release.

## Log

With `LogFile` on, a tick logs `Tick: {amount} (fixed, percent, interest), held {held}, level {level}` and
a milestone logs `Milestone: {bonus} at {seconds}s, held, level`. `apply_runes()` returns
`Option<Granted { held, level }>` to get both values in one access to `PlayerGameData`.

## Open risks

A reload does not re-read `Milestones`; restart the game to change them.
