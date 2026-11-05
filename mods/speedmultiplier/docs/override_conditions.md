# `[[Override]]`: different speeds under a condition

Status: **Confirmed** in game. Conventions: [research notes](../../../docs/research/README.md). Code:
`src/config.rs` (`effective_speed`), `src/speed.rs` (reads the SpEffects and the weight class). The file
format, template merging and migration are in [toml_config](../../../shared/docs/toml_config.md).

## Question

Players want a different speed while a buff is active (a SpEffect) or at a given equip load. That is a
list of rules, so what shape should it take in the config, and what does the game say about the two
conditions?

## Shape

- `[[Override]]` is a TOML array of tables: write as many blocks as needed. The name says what it does
  (replaces values when it matches); `Rule`, `Condition` and `Profile` were rejected.
- The condition sits inside the block, with no sub-table. Keys inside an override use the same names as
  the base tables: `Player.Roll = 1.5`, `Torrent.Run = 2`, or a sub-table `[Override.Player]`. A key left
  out keeps the base value.
- An override needs `SpEffect`, `EquipLoad` or both (both must match). None is an error with a line,
  like a wrong id or an unknown key.

| Condition | Values | Matches when |
|---|---|---|
| `SpEffect` | one id or a list, numbers or strings; an empty list is an error | the player has **any** of the ids ("all of them" was considered and dropped) |
| `EquipLoad` | `Light`, `Medium`, `Heavy`, `Overweight` (any case) or a list | the current load class is in the list |

The mod only changes animation speed, and a SpEffect already covers buffs, debuffs and talismans, so
conditions on HP, weapon or time of day would add nothing.

## Stacking

Every matching override is applied from top to bottom and the **later block wins** on a key both set. Two
independent buffs, one speeding up rolls and one speeding up attacks, therefore work together; there is
no "first match wins". `All` goes through the same steps. Torrent follows the **player's** SpEffects.
The log line `Speed: overrides #1, #2` (or `none`) appears when the set of active overrides changes.

## What the game says (measured in game)

- **Weight class**: `ChrCtrl.weight_type` is 1, 2, 3, 4 in order. 0 means the data is not loaded yet
  (just after entering the game). The roll animation follows it (`027110` at 2, `027120` at 3, `027130`
  at 4) and `max_equip_load` changes independently (swap a talisman).

| `weight_type` | Class |
|---|---|
| 1 | Light |
| 2 | Medium |
| 3 | Heavy |
| 4 | Overweight |

  A class that is unknown (0 or outside 1 to 4) matches no override that has `EquipLoad`. The load is the
  total as the game sees it, so a mod that changes the carried weight changes the class too. A continuous
  load by percentage is not offered.
- **SpEffect ids** to look for: Golden Vow (the spell) is `1660000`, `1660001` and `1660002`, about 80
  seconds; the weapon-art version is `1730` and the item version `20503170`.

## Finding an id

`EffectProbe` in `[Logging]` writes the player's SpEffects to the log whenever they change
(`Probe: effects +[added] -[lost] now [...]`), and `SpeedProbe` writes `Probe: weight_type=N
max_equip_load=X`. Both only log; see `src/probe.rs`.
