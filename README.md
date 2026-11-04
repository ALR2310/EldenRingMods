<div align="center">

# EldenRingMods

**Elden Ring mods written in Rust, one DLL each.**

[Tiếng Việt](README.vi.md)

![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](LICENSE)

</div>

---

## About

My Elden Ring DLL mods. Each mod is a standalone DLL: drop it into your mod loader's folder. No game
files are replaced and `regulation.bin` is never touched. Settings live in a TOML file next to the DLL,
and a reload key applies changes without restarting the game.

## Mods

| Mod | What it does | Nexus |
| --- | --- | --- |
| [**AnimCancel**](mods/animcancel) | Cuts the animation that is playing with guard, dodge, jump or a weapon swap | [11328](https://www.nexusmods.com/eldenring/mods/11328) |
| [**AutoPickup**](mods/autopickup) | Picks up materials, items, corpse loot and runes when you stand in reach | [11470](https://www.nexusmods.com/eldenring/mods/11470) |
| [**AutoRegen**](mods/autoregen) | Restores HP, FP and Stamina over time or when you land a hit | [10548](https://www.nexusmods.com/eldenring/mods/10548) |
| [**DropMultiplier**](mods/dropmultiplier) | Adjusts how often enemies drop items and how many materials you get | [11075](https://www.nexusmods.com/eldenring/mods/11075) |
| [**SpeedMultiplier**](mods/speedmultiplier) | Speeds up or slows down the player, Torrent, spirits and enemies per kind of action | [11173](https://www.nexusmods.com/eldenring/mods/11173) |

Each mod's page describes how it works and its limits, and its `CHANGELOG.md` has the release notes.

## Installation

1. Download the mod from its Nexus page above.
2. Install a loader that can load DLLs, such as
   [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases) (recommended),
   [Mod Engine 3](https://github.com/garyttierney/me3/releases) or
   [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
3. Put `<Mod>.dll` in the loader's mod folder and register it as the loader documents.
4. Start the game. `<Mod>.toml` is created next to the DLL on first run; edit it to change behavior and
   press `F5` (the default) to apply it.

Every mod also adds a settings page in the game's options when
[ERNativeUI](https://github.com/Flammrock/ERNativeUI) is installed. It is optional.

## Building and contributing

Windows x64, stable Rust with the MSVC target and the Windows SDK are needed:

```powershell
cargo build --release --workspace                 # every mod
pwsh scripts/build-mod.ps1 -Mod AutoRegen         # one mod, DLL and config copied to builds/autoregen/
```

[CONTRIBUTING.md](CONTRIBUTING.md) covers the repository layout, the shared crate, the code
conventions and how research notes are written.

## License

[MIT](LICENSE)
