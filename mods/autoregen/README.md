<div align="center">

# AutoRegen

**Restores HP, FP and Stamina automatically, over time or when you land a hit.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10548-orange)](https://www.nexusmods.com/eldenring/mods/10548)

</div>

---

## About

AutoRegen restores HP, FP and Stamina in two independent ways that can be on together: over time
(optionally only out of combat, in combat, idle or while sitting) and when you land a hit. Every
amount can be changed.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/10548).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `AutoRegen.dll` in the loader's mod folder and register it.
4. Start the game. `AutoRegen.toml` is created next to the DLL on first run. Every key is explained by the comments in [AutoRegen.toml](AutoRegen.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Auto Regen** entry also appears under System > Game Options. It is optional.

Releases before this one used `AutoRegen.ini`. If there is no `AutoRegen.toml` yet, the settings of the
ini file are carried over the first time the mod runs; the ini file itself is left alone. A misspelled
key or a wrong type is reported with its line number and the settings in use stay as they were.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/10548): downloads and comments.
- [docs/per_hit.md](docs/per_hit.md): the hit hook, classifying hits, excluding weapon arts, past failures.
- [docs/gesture_trigger.md](docs/gesture_trigger.md): idle and gesture conditions, what was tried.
