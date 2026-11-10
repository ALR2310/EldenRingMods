<div align="center">

# RuneMultiplier

**Multiply the runes you gain from every source.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10630-orange)](https://www.nexusmods.com/eldenring/mods/10630)

</div>

---

## About

RuneMultiplier multiplies the runes you receive by a factor you choose: killing enemies, using or
selling items, and more. A factor above 1 gives more runes, below 1 fewer, and a negative one takes
runes away. Runes you spend, on level-ups and purchases, are not affected.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/10630).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `RuneMultiplier.dll` in the loader's mod folder and register it.
4. Start the game. `RuneMultiplier.toml` is created next to the DLL on first run. Every key is explained by the comments in [RuneMultiplier.toml](RuneMultiplier.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Rune
Multiplier** entry also appears under System > Game Options. It is optional.

Releases before this one used `RuneMultiplier.ini`. If there is no `RuneMultiplier.toml` yet, the
settings of the ini file are carried over the first time the mod runs; the ini file itself is left
alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/10630): downloads and comments.
- [docs/addsoul_hook.md](docs/addsoul_hook.md): the hook, why spends are not scaled, compatibility checks.
