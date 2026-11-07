# Rune counter at the top right (`HUD.RuneCounter.Enabled`)

Status: **Built, not yet tried in game** (the clip place is confirmed by the log, the target place is an estimate).
Conventions: [research notes](../../../docs/research/README.md). Code: `src/rune_counter.rs`.

A request asked for the rune counter at the top right, above the minimap, so the bottom right stays clear for a
streamer's avatar.

## What the game does

- The visible counter is `CS::SoulViewParts` (movie `01_001_FE_Soul`, texts `TotalSoul/Text` and `GetSoul/Text`),
  the view at `hud + 0x7048`, built in `sub_1408D5D80` and updated by `sub_1408D6080(view, dt, model)`, called from
  `sub_1408D1F50`; the call site pattern is unique on 2.6.2.0 to 2.7.2.0. Its root clip is held by the component
  at `view + 560`.
- A first attempt moved `PlayerHUD/Gauge/Score` (`CS::ScoreViewParts`, `hud + 0x75F0`): the clip moved (the log shows
  the new place) and nothing changed on screen, because that is not the rune counter (it is a score of the
  `01_000_fe.gfx` HUD that shows in other modes).
- The movie of the counter is a separate file, and `shared::gfx::hook` takes one movie per DLL (the boss bars use the
  HUD movie), so the clip is moved at run time: its place is read with `GetDisplayInfo` and set with `sub_140D872C0`,
  both already used for the player's buildup gauge.

## What the mod does

With `HUD.RuneCounter.Enabled` on, after each update the clip is moved and scaled (`HUD.RuneCounter.Scale`, 0.7 by default)
so that its box sits where `HUD.RuneCounter.OffsetX/Y` say (from the top-right corner, pixels at 1080p; 48 and 20
by default). They are the mod's own offsets, not the minimap's: the counter does not follow the minimap, and the
drag mode (`General.DragKey`) moves it like the minimap and the target bar (`drag.rs`, `Part::Rune`).

The game's place gives the move: the box is 200 x 36 px and ends at x 1873.5, starts at y 1019 of the 1080p picture;
the clip's origin is 109 px left of the box's right edge and 20 px below its top. The clip is moved by the difference
between the origin wanted and this one, and scaled about its origin. The origin was first guessed 6 px left of the
box (right edge 194 px from the origin); with the scale at 0.7 the box ended 25 px too far right, which gave the
right 109 (`(1873.5 - O) - 0.7 * (1873.5 - O)` shows the error, three times larger than the guess's). The place is set
when it or the view changes and every 300 updates; with the key off again the clip goes back to the place and size
it had when first seen.

When the box starts above the minimap column and shares some of its width (the default), the column (the minimap and
the target panel under it) is drawn lower until it clears the box (`settings::column_offset()`); the saved offsets
and the drag mode keep the plain values.

## Open

- The origin comes from two screenshots: check the right edge of the counter against the minimap's at 0.7.
