<div align="center">

# SpeedMultiplier

**Speed up or slow down the player, Torrent, summoned spirits and enemies, per kind of action.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11173-orange)](https://www.nexusmods.com/eldenring/mods/11173)

</div>

---

## About

SpeedMultiplier changes the animation speed of characters by a factor you choose, separately for
each kind of action: run faster without swinging faster, or cast faster without rolling faster.
Different speeds can also apply while an effect is active or at a given equip load.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11173).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `SpeedMultiplier.dll` in the loader's mod folder and register it.
4. Start the game. `SpeedMultiplier.toml` is created next to the DLL on first run. Every key is explained by the comments in [SpeedMultiplier.toml](SpeedMultiplier.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Speed Multiplier** entry also appears under System > Game Options. It is optional.

When the mod is updated, new keys are added to your file without touching your values, and keys that no
longer exist become comments at the end. A misspelled key or a wrong type is reported with its line
number, and the settings in use stay as they were instead of being replaced by a broken file.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11173): downloads and comments.
- [docs/animation_speed.md](docs/animation_speed.md): the speed field, other fields tried, hooked reads.
- [docs/action_groups.md](docs/action_groups.md): which animations belong to which group.
- [tae_ids](../../docs/tae_ids.md): the `aNNN` animation prefixes by weapon, spell and weapon art.
- [docs/override_conditions.md](docs/override_conditions.md): `[[Override]]` by SpEffect or equip load.
- [docs/enemy_speed.md](docs/enemy_speed.md): how enemies and bosses are told apart.
- [docs/critical_victim.md](docs/critical_victim.md): the victim of a critical hit.
