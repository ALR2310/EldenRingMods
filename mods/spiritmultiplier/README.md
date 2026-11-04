<div align="center">

# SpiritMultiplier

**More spirits per Ash, several Spirit Ashes at once, summon anywhere.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11168-orange)](https://www.nexusmods.com/eldenring/mods/11168)

</div>

---

## About

SpiritMultiplier improves summoning with Spirit Ashes: more spirits per Ash, different Ashes out at the
same time, summoning again without resting, summoning anywhere, spirits that regenerate and keep their
place when you rest, and more spirit HP bars on the HUD. It works offline and with Seamless Co-op.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11168).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `SpiritMultiplier.dll` in the loader's mod folder and register it.
4. Start the game. `SpiritMultiplier.toml` is created next to the DLL on first run. Every key is explained by the comments in [SpiritMultiplier.toml](SpiritMultiplier.toml); edit it and press `F5` (the default) to apply a change without restarting the game. `MaxSpirits` and `HubHpBars` are read once and need a restart.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Spirit
Multiplier** entry also appears under System > Game Options. It is optional.

Releases before this one used `SpiritMultiplier.ini`. If there is no `SpiritMultiplier.toml` yet, the
settings of the ini file are carried over the first time the mod runs; the ini file itself is left alone.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11168): downloads and comments.
- [docs/slots_and_band.md](docs/slots_and_band.md): the 10 spirit cap and how it is lifted.
- [docs/activate_limit.md](docs/activate_limit.md): enemies vanishing with many spirits.
- [docs/multi_spirit.md](docs/multi_spirit.md): several Ashes at once and `CloneSpirit`.
- [docs/summon_anywhere.md](docs/summon_anywhere.md): `NoRestResummon` and `SummonAnywhere`.
- [docs/persist_spirits.md](docs/persist_spirits.md): keeping spirits when resting at a grace.
- [docs/ghost_color.md](docs/ghost_color.md): removing the ghostly tint.
- [docs/mimic_tear.md](docs/mimic_tear.md): the Mimic Tear Blob stage and counts.
- [docs/think_override.md](docs/think_override.md): spirit AI options.
- [docs/buddy_hud.md](docs/buddy_hud.md): more HP bars on the HUD.
- [docs/warp_research.md](docs/warp_research.md): teleporting spirits back to the player.
- [docs/seamless_conflict.md](docs/seamless_conflict.md): rules for patching next to Seamless Co-op.
- [docs/open_issues.md](docs/open_issues.md): Reforged, Seamless and Convergence issues still open.
