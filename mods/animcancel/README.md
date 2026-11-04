# AnimCancel

[Tiếng Việt](README.vi.md)

Cut the animation that is playing with another action, without waiting for it to end.

## Summary

In Elden Ring an animation only lets another action take over inside a "cancel window" that it
opens itself, usually in the second half of a move. AnimCancel opens that window earlier, so
guard, dodge, jump and weapon swap act as soon as you press them.

Four actions can cut an animation, each switched on or off on its own:

| Action | Cuts the animation with |
|---|---|
| **Guard** | raising the guard (L1) |
| **Dodge** | rolling, backstepping |
| **Jump** | jumping |
| **Weapon swap** | switching weapons |

For each action you choose which animations it may cut:

| Animation | Examples |
|---|---|
| **Attack** | normal attacks |
| **Skill** | weapon arts |
| **Cast** | sorceries, incantations |
| **Item** | drinking a flask, using an item |

Guard only cuts when L1 really guards, so it does not break your combos:

- **Cuts:** one weapon in both hands, or one hand with a shield in the left.
- **Does not cut:** one weapon in each hand (dual wielding), twin blades, or a guard that was
  already held before the attack began.

## Installation

1. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
2. Put `AnimCancel.dll` in the loader's mod folder and register it.
3. Start the game. `AnimCancel.toml` is created next to the DLL on first run.

## Configuration

Settings live in `AnimCancel.toml` next to the DLL, named after the DLL. Every key, its default and
its meaning is explained by the comments in [AnimCancel.toml](AnimCancel.toml), the template that is
embedded in the DLL. Edit the file and press the reload key (`F5` by default) to apply it without
restarting the game. A settings file from 1.2.0 (`AnimCancelV1.toml`) is copied over the first time.

### In-game menu (optional)

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, an **Anim Cancel**
entry appears under System > Game Options: one page per action and a switch for the log file.
Changes are written to the config file, keeping its comments. Without ERNativeUI the mod works the
same from the file.

## How it works

Every frame, if the animation that is playing belongs to an enabled group, the mod opens its cancel
permission in the action queue of `CSChrActionRequestModule`, the place the game's behavior scripts
read, instead of waiting for the animation to open it. No pattern or offset is needed: it only uses
the structs of [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs). The game's mechanism
and the results of each attempt: [docs/animation_cancel.md](docs/animation_cancel.md).

## Compatibility and limits

- Applies to the local player only; Seamless Co-op has not been checked.
- Guarding in the air cannot be done with a DLL, see [docs/air_guard.md](docs/air_guard.md).

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [docs/animation_cancel.md](docs/animation_cancel.md): how cancelling works and what was tried.
- [docs/air_guard.md](docs/air_guard.md): why guarding in the air was dropped.
- [docs/antivirus.md](docs/antivirus.md): antivirus detections and what changed them.
