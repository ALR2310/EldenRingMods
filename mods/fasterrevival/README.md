<div align="center">

# FasterRevival

**Respawn sooner after dying: no waiting on the death animation or on "YOU DIED".**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11160-orange)](https://www.nexusmods.com/eldenring/mods/11160)

</div>

---

## About

When you die, Elden Ring makes you watch the whole fall animation, wait for the "YOU DIED" screen,
and then wait again before you respawn. FasterRevival cuts both waits: a normal death respawns in
about 4 to 5 seconds instead of 12 to 14. It is always on and has nothing to configure. It does not
edit animation files, so it does not conflict with animation mods.

Falling off the world is unchanged, and it has not been tested while riding Torrent.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11160).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `FasterRevival.dll` in the loader's mod folder and register it.
4. Start the game. `FasterRevival.toml` is created next to the DLL on first run; it only holds the log switch and a diagnostic option, explained by its comments.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11160): downloads and comments.
- [docs/death_flow.md](docs/death_flow.md): how the game kills the player, and how the mod moves that earlier.
- [docs/respawn_fade.md](docs/respawn_fade.md): the pause after "YOU DIED".
