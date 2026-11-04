# EldenRingMods

[Tiếng Việt](README.vi.md)

A collection of Elden Ring mods written in Rust. Each mod is a standalone DLL: drop it into
your mod loader's folder, no game files are replaced and `regulation.bin` is never touched.
Mods read and change game state at runtime and are configured through a TOML file next to the
DLL.

> The mods are being moved into this repository one at a time. The list below grows as each
> one lands.

## Mods

| Mod | What it does |
| --- | --- |
| [AutoPickup](mods/autopickup) | Picks up materials, items, corpse loot and runes when you stand in reach |
| [AnimCancel](mods/animcancel) | Cuts the animation that is playing with guard, dodge, jump or a weapon swap |
| [SpeedMultiplier](mods/speedmultiplier) | Speeds up or slows down the player, Torrent, spirits and enemies per kind of action |

## Installation

1. Install a loader that can load DLLs, such as
   [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases),
   [Mod Engine 3](https://github.com/garyttierney/me3/releases) or
   [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
2. Put `<Mod>.dll` in the loader's mod folder and register it as the loader documents.
3. Start the game. `<Mod>.toml` is created next to the DLL on first run; edit it to change
   behavior.

## Building from source

Requirements: Windows x64, stable Rust with the MSVC target, Windows SDK.

```powershell
cargo build --release -p <crate>
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the repository layout and code conventions.

## License

[MIT](LICENSE)
