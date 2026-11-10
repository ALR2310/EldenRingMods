<div align="center">

# AnimCancel

**Cut the animation that is playing with another action, without waiting for it to end.**

[Tiếng Việt](README.vi.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11328-orange)](https://www.nexusmods.com/eldenring/mods/11328)

</div>

---

## About

In Elden Ring an animation only lets another action take over at a point it chooses, usually late in
the move. AnimCancel opens that point earlier, so guard, dodge, jump and weapon swap act as soon as
you press them. You choose which actions may cut which animations.

Not checked with Seamless Co-op.

## Installation

1. Download the mod from its [Nexus page](https://www.nexusmods.com/eldenring/mods/11328).
2. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
3. Put `AnimCancel.dll` in the loader's mod folder and register it.
4. Start the game. `AnimCancel.toml` is created next to the DLL on first run. Every key is explained by the comments in [AnimCancel.toml](AnimCancel.toml); edit it and press `F5` (the default) to apply a change without restarting the game.

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, the **Anim Cancel** entry also appears under System > Game Options. It is optional.

A settings file from 1.2.0 (`AnimCancelV1.toml`) is copied over the first time.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [Nexus page](https://www.nexusmods.com/eldenring/mods/11328): downloads and comments.
- [docs/animation_cancel.md](docs/animation_cancel.md): how cancelling works and what was tried.
- [docs/air_guard.md](docs/air_guard.md): why guarding in the air was dropped.
- [docs/antivirus.md](docs/antivirus.md): antivirus detections and what changed them.
