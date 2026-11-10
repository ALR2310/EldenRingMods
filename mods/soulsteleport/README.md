<div align="center">

# SoulsTeleport

**Teleport to exactly where a partner stands in Seamless Co-op.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11119-orange)](https://www.nexusmods.com/eldenring/mods/11119)

</div>

---

## About

SoulsTeleport is a standalone mod for **Seamless Co-op**, which it needs. Press the menu key, pick a player of
the session from the list (with their role: Host, Co-op, Hunter...) and you are teleported to exactly where they
stand, through a loading screen, however far. The position is asked from that player over Steam, so **the player
you go to must have the mod too**.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11119).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `SoulsTeleport.dll` in the loader's mod folder and register it.
4. Start the game. `SoulsTeleport.toml` is created next to the DLL on first run. Every key is explained by the comments in [SoulsTeleport.toml](SoulsTeleport.toml). The menu opens with `Tab` by default; `Scale` needs a restart.

Releases before this one used `SoulsTeleport.ini`. If there is no `SoulsTeleport.toml` yet, the settings of the
ini file are carried over the first time the mod runs; the ini file itself is left alone.

Use it offline with EAC disabled, like Seamless Co-op itself.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11119): downloads and comments.
- [docs/warp_mechanism.md](docs/warp_mechanism.md): how the warp works.
- [docs/position_exchange.md](docs/position_exchange.md): getting a partner's position over Steam.
- [docs/roles_and_invaders.md](docs/roles_and_invaders.md): roles and blocking teleport between hostile players.
- [docs/menu_ui.md](docs/menu_ui.md): the menu, mouse hooks and display.
