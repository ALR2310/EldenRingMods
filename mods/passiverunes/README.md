<div align="center">

# PassiveRunes

**Gain (or lose) runes over time just from playing.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10528-orange)](https://www.nexusmods.com/eldenring/mods/10528)

</div>

---

## About

PassiveRunes gives you runes at a steady pace while you play, with a bonus at play-time milestones
from 30 minutes up to 24 hours. Every value can be negative, which takes runes away instead. It
reads and writes your runes directly, with no code patch.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/10528).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `PassiveRunes.dll` in the loader's mod folder and register it.
4. Start the game. `PassiveRunes.toml` is created next to the DLL on first run. Every key is explained by the comments in [PassiveRunes.toml](PassiveRunes.toml); edit it and press `F5` (the default) to apply a change without restarting the game. The milestone list is read once at start-up.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Passive
Runes** entry also appears under System > Game Options. It is optional.

Releases before this one used `PassiveRunes.ini`. If there is no `PassiveRunes.toml` yet, the
settings of the ini file are carried over the first time the mod runs; the ini file itself is left
alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/10528): downloads and comments.
- [docs/rune_math.md](docs/rune_math.md): the three parts of each reward, negative values and the milestones.
