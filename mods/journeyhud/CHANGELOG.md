# JourneyHUD - Changelog

## [Unreleased]

- Added an option that moves the game's boss health bars down and draws a boss's posture and effects under them (the bars are spread when several bosses are on screen).
- The target panel now shows the effects above the buildup bars, and the buildup icons sit at the right end.
- Added an option to move the game's rune counter to the top right and resize it; it can be dragged like the minimap.
- The minimap and the target panel can now be dragged across the whole screen, and a right click on a part in the drag mode puts it back to its default place.
- The `[Layout]` section of the config is gone: the offsets are now `Minimap.OffsetX/OffsetY` and `Target.OffsetX/OffsetY`, and the rune counter has its own `[HUD.RuneCounter]` section. Existing files are migrated.
- The minimap no longer shows Torrent as a spirit.

## [1.3.0] - 2026-11-06

- Added an option to hide the minimap in Roundtable Hold.
- Fixed the minimap showing during cutscenes.

## [1.2.0] - 2026-11-03

- Added a drag mode (F7 by default) to move the minimap and the target bar with the mouse.
- Added an option to mark the hostile enemies near you as red dots on the minimap (off by default).
- Fixed the minimap being blank at the Mistwood after defeating Radahn.
- Fixed the minimap being blank at Crumbling Farum Azula.
- Fixed the option to hide the minimap in caves also affecting areas that are not caves.

## [1.1.0] - 2026-11-02

- Added support for negative values in the layout offset.
- Added compatibility with mods that override menu sprites, for example Blue Markers.
- Added compatibility with HDR output, for example ERSS-FG.
- Added an option to delay the start of the overlay, for when it conflicts with another overlay mod.
- Added a hotkey to hide and show the minimap and the target panel.

## [1.0.0] - 2026-11-01

- Initial release
