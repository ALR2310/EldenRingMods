# AutoPickup

[Tiếng Việt](README.vi.md)

Picks things up when you stand within reach, without pressing the interact button.

## Summary

In Elden Ring every plant, ground item and corpse needs you to stand in the right spot and press
a button. AutoPickup presses it for you. Levers, doors, graces and conversations are left alone.

Five groups, each with its own switch and reach:

| Group | Covers |
|---|---|
| **Materials** | plants, mushrooms and other crafting materials |
| **Items** | items lying on the ground |
| **CorpseLoot** | loot on corpses and remains |
| **LostRunes** | the runes you dropped when you died |
| **Chests** | treasure chests (off by default, because a mimic opens the same way) |

You can also pause auto pickup while in combat, widen the pickup reach (which is also the reach
of the pickup prompt) per group, and turn off the pickup animation.

## Installation

1. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
2. Put `AutoPickup.dll` in the loader's mod folder and register it.
3. Start the game. `AutoPickup.toml` is created next to the DLL on first run.

## Configuration

Settings live in `AutoPickup.toml` next to the DLL. Every key, its default and its meaning is
explained by the comments in [AutoPickup.toml](AutoPickup.toml), the template that is embedded
in the DLL. Edit the file and press the reload key (`F5` by default) to apply it without
restarting the game.

### In-game menu (optional)

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, an **Auto
Pickup** entry appears under System > Game Options with all the settings above. Without
ERNativeUI the mod works the same from the TOML file.

## How it works

The mod hooks the game function that asks "is this interaction triggered"
(`CSActionButtonManImp::ExecActionButton`). For an interaction in an enabled group the hook
answers yes, so the game picks the item up as if you had pressed the button; every other
interaction runs as before. The reach is the radius, height and height offset of the
`ActionButtonParam` row, multiplied by `Range` and always computed from the game's own values.
See [docs/exec_action_button.md](docs/exec_action_button.md) and
[docs/pickup_animation.md](docs/pickup_animation.md).

## Compatibility and limits

- Does not work together with Seamless Co-op.
- The hook is located by a byte pattern. A game update can break it; the mod then logs the
  problem and keeps the game running without pickup.
- Do not use it with another DLL that also auto-picks up items; turn that feature off in the
  other one.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [docs/exec_action_button.md](docs/exec_action_button.md): mechanism, hook point, risks.
- [docs/pickup_animation.md](docs/pickup_animation.md): how the animation is switched off.
