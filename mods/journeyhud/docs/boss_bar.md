# Boss bars: posture, buildup and effects under the game's boss health bar

Status: **Built (`Target.BossBar`), position measured at a 2560x1440 frame, not yet tried in game.** Only the
position constants of step 1 below exist; the game's own boss bar is not read.
Conventions:
[research notes](../../../docs/research/README.md). Related: `design.md`, `offsets.md`, `layout_drag.md`.

## Wanted

When the locked-on enemy is a boss, draw its posture bar, buildup rows and effect icons under the game's own
boss health bar (bottom centre of the screen) and leave the mod's own health bar out, as the game already
shows the boss's health. Other enemies keep the panel under the minimap.

## What is already there

- `target::Target` holds `hp`, `poise`, `buildup`, `effects` of the locked-on enemy; `ui::draw_target` draws
  them from a rectangle given by `target_rect`. A boss variant needs only another rectangle and no health bar.
- Telling a boss from another enemy: `CSFeManImp::boss_health_displays` (+0x5BF0, 3 entries of 0x20 bytes,
  `fmg_id` at +0, handle at +8, empty = -1; confirmed on 2.6.2.0 to 2.7.2.0, `offsets.md`). The target is a
  boss when its handle equals the handle of one of the entries. `radar` already reads this for
  `Minimap.Hide.InBossFights`.
- The entries also give the bosses that are fighting without a lock-on: the same code can read the posture
  and buildup of entry 0 when nothing is locked, so the bars stay when the camera is released.

## What is missing: where the game's boss bar is

The boss bar is part of the game's Scaleform HUD movie, not a field of an engine object: no pointer gives its
rectangle, and `CSFeManImp` only keeps the handle and the name id. The usual way (a third-party posture mod
does this) is a fixed position in the 1920x1080 space of the HUD, scaled to the frame the game draws
(`ui::game_area`):

| Value in 1920x1080 space (third-party posture mod's defaults) | Meaning |
|---|---|
| Bar width 1020, height 16 | Same length as the game's boss health bar |
| Centre x 957.5, y 876 | Its first posture bar, placed under the game's bar by trial |
| 55 up per extra boss | Boss bars stack upwards (up to 3 entries) |

These numbers come from another mod's config and were tuned by eye there, with an offset-test mode to
correct them because they differ between HUD mods (a total-conversion mod moves the bar). Mods that move the
HUD will misplace ours: keep an offset.

**Measured by us** on three 2560x1600 screenshots (16:9 frame 2560x1440, bands of 80 px, scale 1.3333 to the
1920x1080 space) of Tree Sentinel, full and part health: the red fill runs from x 625 to 1934 and y 1236 to
1248 in screen pixels, that is 469 to 1450 and **y 867 to 876** in the 1080 space (centre x 960, fill about 982
wide, the ornament caps reach about 1013 to 1020). The bottom edge at 876 matches the third-party default, so
the posture bar goes right under it at y 876 and the numbers above hold at this resolution. The bar was in the
same place in all three shots (the name sits above it at the left). Only 1440p-frame was shot: 1080p is still
to check.

## Proposal

1. `Target.BossBar` (bool, default true): "when the target is a boss, draw under the game's boss bar".
2. `boss_rect(area)`: left and width from the 1920 space (`res = area height / BASE_HEIGHT`, as
   `target_rect`), top just under the game's bar; `s` is the player's size scale, as now.
3. `draw_target(boss)`: skip the health bar and its frame, start with the posture bar at the rect's top, then
   buildup rows and effects as now, left aligned or centred in the 1020 width instead of right aligned.
4. Keep the existing `TargetOffsetX/Y` drag for the correction: while `drag` is on, show the boss rect too
   (`layout_drag.md`). A bar that was dragged stays put, like the normal one.
5. Several bosses: draw the one that is locked; with no lock, entry 0 only. Stacking the others needs their
   posture read by handle (the same `chr_by_handle`), cheap, but skip it first.
6. Hide with the game's boss bar: `hud_visible` (HUD hidden in cutscenes or by the player) and the boss slot
   going empty (death, phase change) already drive `radar::in_control` / `boss_fight`.

## To measure or try in game

- Done at a 2560x1440 frame (see above). Still to shoot: 1080p and a 21:9 window, to check the 16:9 frame
  scaling of `boss_rect`.
- Does the game's bar move with the HUD size option or between bosses (a long name wraps?), with Reforged.
- Does the boss bar fade in and out (entry present before the bar is visible) and how our bars should follow.
- What the lock-on handle is on bosses with two bodies or phases (the handle changes at the phase change):
  the `Target:` log line shows `npc` changing.
- Bosses without a poise bar (max 0): the rows start with buildup, or nothing is drawn.

## As built

- Panel under the minimap, for a boss: no health or posture bar, only the buildup rows and the effects (the
  room under the game's bar is one posture row; the next boss bar's name is 55 px above).
- Posture bar under the game's bar of that boss (slot 0 is the lowest bar, each further one 55 px higher, seen in
  a two-boss screenshot), with the game's own line and left cap below it: `BarDeco` (2938x25, the whole line
  at scale 0.345, so it spans the 1013 px the game's bar does, centred on x 960, line centred on row 17.5) and
  `BarDecoL` (46x38) from the same top. Cap and line placement is from a screenshot, to be checked in game.
- With no lock-on, the lowest boss bar's boss is the target, so its bars show without targeting it
  (`target::read`); the minimap then marks it as the locked enemy.

Measured again on a screenshot with the bars drawn: the game's line runs from x 622 to 1938 (screen pixels at
the 1440p frame), so 466 to 1453 in the 1080 space (987 wide); its left cap is about 25 px wide and 17.5 px
high there, so the texture scale is 0.46, and the cap ends about 3 px inside the bar.

The right end was short: the bar measured at less than full health stops before its end. At full health the line
ends at x 1952 (screen pixels), so the bar runs from 466 to 1463 in the 1080 space (997 wide).

## Spreading the game's bars apart (`src/boss_list.rs`)

The game's boss bars are `BossList/Item_0_0` to `Item_2_0` of `menu/01_000_fe.gfx` (found in IDA on 2.7.2.0 by
the string `BossList/Item_%d_0`, used in the HUD constructor `sub_1408C6C80`; the movie parsed from the
extracted file): `BossList` is placed on the stage at (947, 908), and the items sit at y -35, -90 and -145
(55 px apart, the lowest first); each holds `HP`, `Name`, `Damage` and its caps. The three views the game
binds to them are 1904-byte objects at `hud + 105000 + 1904 * k`.

The gap is changed in the movie, the way the spirit bars' rows are added (`shared/docs/gfx.md`):
`gfx::shift_placements` moves `Item_1_0` and `Item_2_0` by 55 and 110 px further up, so the pitch is 110 px.
The edit is made as the game opens the movie, so it needs the hook installed before the HUD loads and does not
change when `Target.BossBar` is switched at run time (the key is read at start). `boss_list::spread()` tells
the drawing whether the edit was made; the posture bars and the effect icons use the pitch accordingly (55 px
when not). Checked on the extracted movie (the unit tests and an ignored test that writes the edited movie);
**not yet tried in game**. Not tried: a runtime move of the clips (the placement code of the player's gauge
could do it, but needs the root clip of each item, which the view does not keep).

## Lowering the bars (part of `Target.BossBar`)

A request asked for the boss bars at the height of the flask's name (the bottom of the screen). The same movie
edit moves all three items down by 114 px, so the lowest bar's bottom edge goes from y 876 to 990 of the 1080
picture (`boss_list::DROP_PX`); `boss_rect` adds `boss_list::drop_px()` to the place it expects, so the posture
bar and the effects follow. It is on with `BossBar` and needs a restart like the spread. Tried in game with
two bosses: the posture and the effect icons sit under each bar, the effects of the lowest end short of the
rune counter.

The rune counter request is in [rune_counter.md](rune_counter.md).
