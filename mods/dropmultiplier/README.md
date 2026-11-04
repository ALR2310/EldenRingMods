<div align="center">

# DropMultiplier

**Adjust how often enemies drop items and how many materials you get.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11075-orange)](https://www.nexusmods.com/eldenring/mods/11075)

</div>

---

## About

DropMultiplier adjusts two independent things: how often enemies drop their items, and how many
materials you get from each pickup, so you spend less time farming. Drop chances can be multiplied
or set to a fixed value, and crafting, upgrade and once-only materials each have their own
multiplier. It edits the game's data while it runs and never replaces `regulation.bin`, so it also
works with large overhauls such as Convergence.

The fixed chance can turn a drop that was guaranteed into one that depends on chance.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11075).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `DropMultiplier.dll` in the loader's mod folder and register it.
4. Start the game. `DropMultiplier.toml` is created next to the DLL on first run. Every key is explained by the comments in [DropMultiplier.toml](DropMultiplier.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Drop Multiplier** entry also appears under System > Game Options. It is optional.

Releases before this one used `DropMultiplier.ini`. If there is no `DropMultiplier.toml` yet, the settings of the ini file are carried over the first time the mod runs; the ini file itself is left alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11075): downloads and comments.
- [docs/drop_rate.md](docs/drop_rate.md): the game's lottery, the two modes, the fixed-chance formula.
- [docs/materials.md](docs/materials.md): how materials are classified and scaled.
- [shared/docs/params.md](../../shared/docs/params.md): reading param rows safely, and the Convergence panic.
