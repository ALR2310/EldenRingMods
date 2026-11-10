<div align="center">

# WeightMultiplier

**Multiply your equip load by a factor, or fix it to one number.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10549-orange)](https://www.nexusmods.com/eldenring/mods/10549)

</div>

---

## About

WeightMultiplier changes your total equip load by a factor you choose, or sets it to one number
whatever you wear. One hook covers both the number on the screen and the real behavior, roll type
and run speed. A factor below 1 makes you lighter, above 1 heavier. The mod used to be called
ReductionWeight.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/10549).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `WeightMultiplier.dll` in the loader's mod folder and register it.
4. Start the game. `WeightMultiplier.toml` is created next to the DLL on first run. Every key is explained by the comments in [WeightMultiplier.toml](WeightMultiplier.toml); edit it and press `F5` (the default) to apply a change without restarting the game. The mod waits a few seconds after the game starts before it applies itself (`LoadDelay`).

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Weight
Multiplier** entry also appears under System > Game Options. It is optional.

Releases before this one used `WeightMultiplier.ini`. If there is no `WeightMultiplier.toml` yet,
the settings of the ini file are carried over the first time the mod runs; the ini file itself is
left alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/10549): downloads and comments.
- [docs/hook_point.md](docs/hook_point.md): where the hook is, how it was found, what happens when another mod hooks the same place.
