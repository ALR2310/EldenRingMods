# SpeedMultiplier

[Tiếng Việt](README.vi.md)

Speed up or slow down the player, Torrent, summoned spirits and enemies, per kind of action.

## Summary

SpeedMultiplier changes the animation speed of characters by a factor you choose, separately for each
kind of action: run faster without swinging faster, or cast faster without rolling faster. It can also
apply different speeds while the player has an effect on them or carries a given equip load.

| Who | What can be set |
|---|---|
| **Player** | one factor for every action, or per group: walk, run, sneak, jump, roll, ladder, attack, critical hit, weapon skill, cast, item, other |
| **Torrent** | one factor for everything, or walk, run, jump, other |
| **Summoned spirits** | one factor |
| **Enemies and bosses** | one factor (not animals, NPCs or merchants) |

An overall factor other than 1 overrides the other keys of its group. Every speed is kept between 0.1
and 10.

**Conditional overrides:** `[[Override]]` blocks set other speeds while the player has a SpEffect, is at
an equip load class (Light, Medium, Heavy, Overweight), or both. Several matching blocks stack, and the
one lower in the file wins on a key they both set.

## Installation

1. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
2. Put `SpeedMultiplier.dll` in the loader's mod folder and register it.
3. Start the game. `SpeedMultiplier.toml` is created next to the DLL on first run.

## Configuration

Settings live in `SpeedMultiplier.toml` next to the DLL, named after the DLL. Every key, its default and
its meaning is explained by the comments in [SpeedMultiplier.toml](SpeedMultiplier.toml), the template
embedded in the DLL. Press the reload key (`F5` by default) to apply a change without restarting the
game.

When the mod is updated, new keys are added to your file without touching your values, and keys that no
longer exist become comments at the end. A misspelled key or a wrong type is reported with its line
number, and the settings in use stay as they were instead of being replaced by a broken file.

### In-game menu (optional)

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, a **Speed
Multiplier** entry appears under System > Game Options with a slider for each speed (0.1 to 10) and a
switch for the log file. Changes are written to the config file, keeping its comments. `[[Override]]`
blocks are edited in the file only. Without ERNativeUI the mod works the same from the file.

## How it works

Every frame the mod writes the `animation_speed` field of each character, by the group of the animation
that is playing, and only when the value changes because the game does not reset it. The field, the
others that were tried and Seamless Co-op's hook of the game's read of it: [docs/animation_speed.md](docs/animation_speed.md).
The groups and their animation ranges: [docs/action_groups.md](docs/action_groups.md).

## Compatibility and limits

- The game has **one** animation speed per character, so the upper body cannot be sped up alone:
  drinking while running follows the movement group, drinking while standing the item group.
- Seamless Co-op hooks the game's read of the speed; the mod detects it and bypasses the hook for the
  player and Torrent. Speeds of spirits and enemies do not apply under Seamless.
- In co-op the pictures are not synchronised between machines: each machine only speeds up its own
  character.
- Critical hits and grabs are paired with the victim's animation. The victim follows the player's speed
  ([docs/critical_victim.md](docs/critical_victim.md)); a high `Critical` value can still look out of
  step.
- The beam of Placidusax's Ruin runs on its own clock and does not follow the cast speed.
- The walking animation when overweight is outside the walk range. Equip load is matched by class, not
  by percentage.
- The overall factor also affects being hit, riding and resting at a grace.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [docs/animation_speed.md](docs/animation_speed.md): the speed field, other fields tried, hooked reads.
- [docs/action_groups.md](docs/action_groups.md): which animations belong to which group.
- [docs/enemy_speed.md](docs/enemy_speed.md): how enemies and bosses are told apart.
- [docs/critical_victim.md](docs/critical_victim.md): the victim of a critical hit.
