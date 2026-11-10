<div align="center">

# AutoPickup

**Picks things up when you stand within reach, without pressing the interact button.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11470-orange)](https://www.nexusmods.com/eldenring/mods/11470)

</div>

---

## About

In Elden Ring every plant, ground item and corpse needs you to stand in the right spot and press a
button. AutoPickup presses it for you: materials, items, corpse loot and the runes you dropped.
Levers, doors, graces and conversations are left alone. Each group can be switched on or off and
given its own reach.

Does not work with Seamless Co-op, or together with another DLL that also picks items up.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11470).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `AutoPickup.dll` in the loader's mod folder and register it.
4. Start the game. `AutoPickup.toml` is created next to the DLL on first run. Every key is explained by the comments in [AutoPickup.toml](AutoPickup.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Auto Pickup** entry also appears under System > Game Options. It is optional.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11470): downloads and comments.
- [docs/exec_action_button.md](docs/exec_action_button.md): mechanism, hook point, risks.
- [docs/pickup_animation.md](docs/pickup_animation.md): how the animation is switched off.
