<div align="center">

# InfiniteAilment

**Set how long Poison, Scarlet Rot, Black Flame and Destined Death last, and how hard they hit.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11087-orange)](https://www.nexusmods.com/eldenring/mods/11087)

</div>

---

## About

Poison and Scarlet Rot last only 90 seconds in Elden Ring, and Black Flame and Destined Death burn
for a couple of seconds. InfiniteAilment lets you set the duration, even infinite, and the damage
per tick of these four effects when **you** inflict them on enemies. It edits the game's data while
it runs and never replaces `regulation.bin`, so it also works with large overhauls such as
Convergence.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11087).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `InfiniteAilment.dll` in the loader's mod folder and register it.
4. Start the game. `InfiniteAilment.toml` is created next to the DLL on first run. Every key, with its vanilla value, is explained by the comments in [InfiniteAilment.toml](InfiniteAilment.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Infinite
Ailment** entry also appears under System > Game Options. It is optional.

Releases before this one used `InfiniteAilment.ini`. If there is no `InfiniteAilment.toml` yet, the
settings of the ini file are carried over the first time the mod runs; the ini file itself is left
alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11087): downloads and comments.
- [docs/row_discovery.md](docs/row_discovery.md): how the Poison and Scarlet Rot rows were found.
- [docs/black_flame.md](docs/black_flame.md): Black Flame and Destined Death.
