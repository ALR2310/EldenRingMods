# AutoRegen

[Tiếng Việt](README.vi.md)

Restores HP, FP and Stamina automatically, over time or when you land a hit.

## Summary

AutoRegen restores HP, FP and Stamina in two independent ways that can be on together: **over time**
(every interval, under a condition you choose) and **on hit**. Every number can be changed, and a
change applies as soon as you press the reload key.

### Over time (`[PerTick]`)

Each interval the mod restores an amount of HP, FP and Stamina while the chosen condition holds:

| Condition | Restores while |
|---|---|
| **Always** | at all times |
| **Out of combat** | you have not dealt or taken a hit in the last 15 seconds |
| **In combat** | you dealt or took a hit in the last 15 seconds |
| **Idle** | you have not moved or held any action for 5 seconds |
| **During a gesture** | you are sitting (or doing another gesture of your list) |

The amount is **points**, a **percent of the maximum** or a **percent of what is missing** (fast when
low, slower as it fills). A **cap** keeps a tick from restoring above a chosen percent.

### On hit (`[PerHit]`)

Every hit you land restores an amount, as **points**, a **percent of the maximum** or a **percent of the
damage dealt**. You choose which hits count (melee, spells and ranged, or both) and can exclude weapon
arts.

## Installation

1. Install a mod loader that can load DLLs (see the [repository README](../../README.md)).
2. Put `AutoRegen.dll` in the loader's mod folder and register it.
3. Start the game. `AutoRegen.toml` is created next to the DLL on first run.

## Configuration

Settings live in `AutoRegen.toml` next to the DLL, named after the DLL. Every key, its default and its
meaning is explained by the comments in [AutoRegen.toml](AutoRegen.toml), the template embedded in the
DLL. Press the reload key (`F5` by default) to apply a change without restarting the game.

Releases before this one used `AutoRegen.ini`. If there is no `AutoRegen.toml` yet, the settings of the
ini file are carried over the first time the mod runs; the ini file itself is left alone. A misspelled
key or a wrong type is reported with its line number and the settings in use stay as they were.

### In-game menu (optional)

If [ERNativeUI](https://github.com/Flammrock/ERNativeUI) is loaded before the mod, an **Auto Regen**
entry appears under System > Game Options with a **Per Tick** page, a **Per Hit** page and a switch for
the log file. Amounts and the interval are text rows where a number is typed (decimals work). The
gesture list is edited in the file only. Without ERNativeUI the mod works the same from the file.

## How it works

- **Over time:** a task on the game's `FrameBegin` group reads and writes HP, FP and Stamina through
  `CSChrDataModule` of [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs).
- **On hit:** a hook at the start of the game's hit-resolution function only observes who hit whom and
  for how much; the mod writes the restored amount itself. Details:
  [docs/per_hit.md](docs/per_hit.md).
- **Gestures:** counted as sitting only when the animation really changed and held, so a gesture blocked
  in the middle of a cast does not restore anything. Details: [docs/gesture_trigger.md](docs/gesture_trigger.md).

## Compatibility and limits

- "In combat" is an approximation: standing still to dodge a boss does not count as combat.
- A spell with a melee motion (the Carian Greatsword, for one) counts as a melee hit.
- Excluding weapon arts follows the latest button pressed, with a check of the animation for stance arts.
  The arts of a few weapons with their own move set can still be counted as normal attacks.
- The hit hook is verified on one game build (2.7.1.0). If it is not found, only restore-on-hit is off;
  restoring over time keeps working.

## Documentation

- [CHANGELOG.md](CHANGELOG.md): release notes.
- [docs/per_hit.md](docs/per_hit.md): the hit hook, classifying hits, excluding weapon arts, past failures.
- [docs/gesture_trigger.md](docs/gesture_trigger.md): idle and gesture conditions, what was tried.
