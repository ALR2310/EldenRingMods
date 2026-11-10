<div align="center">

# RiseArcher

**A buff for bows, crossbows, ballistas, arrows and bolts.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-5807-orange)](https://www.nexusmods.com/eldenring/mods/5807)

</div>

---

## About

RiseArcher makes ranged weapons worth using: more damage and scaling, an Ash of War on every bow,
lighter weapons, a higher sell value, more ammunition to carry, and a Rain of Arrows that is faster,
farther and fires more arrows. Every multiplier can be changed. It edits the game's data while it
runs and never replaces `regulation.bin`, so it does not conflict with other mods that do.

`RiseArcher.MASSEDIT` is the Mass Edit script of the original release, for editing `regulation.bin`
directly in a param editor instead of using the DLL.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/5807).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `RiseArcher.dll` in the loader's mod folder and register it.
4. Start the game. `RiseArcher.toml` is created next to the DLL on first run. Every key is explained by the comments in [RiseArcher.toml](RiseArcher.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Rise
Archer** entry also appears under System > Game Options. It is optional.

Releases before this one used `RiseArcher.ini`. If there is no `RiseArcher.toml` yet, the settings
of the ini file are carried over the first time the mod runs; the ini file itself is left alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/5807): downloads and comments.
- [docs/param_patching.md](docs/param_patching.md): which rows are patched, how the originals are kept, start-up.
