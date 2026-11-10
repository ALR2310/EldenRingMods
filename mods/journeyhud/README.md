<div align="center">

# JourneyHUD

**A minimap with the world map, and the health, posture and status buildup of your target.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11543-orange)](https://www.nexusmods.com/eldenring/mods/11543)

</div>

---

## About

JourneyHUD draws a compact HUD at the top right: a round minimap on the game's own world map, and under it
the health, posture, status buildup and active effects of the enemy you are locked on to. It also shows the
time left under your own status icons. It works offline and with Seamless Co-op, and needs the DX12 renderer.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11543).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `JourneyHUD.dll` in the loader's mod folder and register it.
4. Start the game. `JourneyHUD.toml` is created next to the DLL on first run. Every key is explained by the comments in [JourneyHUD.toml](JourneyHUD.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Journey HUD**
entry also appears under System > Game Options. It is optional.

Another mod that hooks the same DX12 swap chain can clash with the overlay: if the game exits at start-up,
raise `LoadDelay` in `JourneyHUD.toml`.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11543): downloads and comments.
- [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt): licences of the code and ideas it adapts.
- [docs/design.md](docs/design.md): the HUD design, layout and textures.
- [docs/offsets.md](docs/offsets.md): offsets and checks per executable build.
- [docs/radar.md](docs/radar.md): the minimap dots.
- [docs/map.md](docs/map.md): the world map, graces, points, beacons and the DLC.
- [docs/buildup.md](docs/buildup.md): the status buildup bars.
- [docs/effects.md](docs/effects.md): active effects, the state icon row and the player buildup gauge.
- [docs/layout_drag.md](docs/layout_drag.md): dragging the minimap and the target bar.
- [docs/hdr.md](docs/hdr.md): the overlay on an HDR display.
- [docs/player_marker.md](docs/player_marker.md): a replaced player arrow (Blue Markers).
- [docs/questpath.md](docs/questpath.md): the start-up crash with QuestPath.
- [docs/heap_corruption.md](docs/heap_corruption.md): an unexplained heap-corruption crash.
