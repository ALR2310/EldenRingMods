# Patching the live params for bows, crossbows, ballistas, arrows and bolts

Status: **Confirmed** in game: the first release applied exactly 99 `EquipParamWeapon` rows and 352
`Bullet` rows. Conventions: [research notes](../../../docs/research/README.md). Code: `src/weapon.rs`,
`src/bullet.rs`, `src/lib.rs`.

## Question

How can the ranged weapons and their ammunition be buffed without shipping a modified `regulation.bin`,
and how do the changes survive a reload and a start-up that is too early?

## Two ways that coexist

1. **The Rust DLL (recommended).** It patches `SoloParamRepository` live through fromsoftware-rs and never
   touches `regulation.bin` on disk, so it does not conflict with another mod that does; every multiplier
   comes from `RiseArcher.toml`.
2. **A static edit of `regulation.bin` through a param editor.** `RiseArcher.MASSEDIT` is the Mass Edit
   script of the original release, to paste into the param editor. It is useful when the changes have to be
   merged by hand with one particular `regulation.bin` mod, or for someone who does not want a DLL.

**They are independent and do not synchronise.** The DLL's default multipliers were ported from the values
hard-coded in the script, so they match today, but changing one (for example in the config) does not update
the other. Change the default multipliers in both if they should match.

The DLL uses fromsoftware-rs (Rust) rather than a C++ library as first planned, because
`SoloParamRepository` already has the `EQUIP_PARAM_WEAPON_ST` and `BULLET_PARAM_ST` accessors for every field
in use, so no offset has to be found.

## `EquipParamWeapon` (`weapon.rs`)

The same filters as the script, run once over every row instead of dozens of Mass Edit commands per field:

| Kind | Condition |
|---|---|
| Bow, Greatbow | `weaponCategory == 10` |
| Crossbow | `weaponCategory == 11` and `wepType == 55` |
| Ballista | `weaponCategory == 11` and `wepType == 56` (crossbow and ballista share a category, told apart by `wepType`) |
| Arrow | `weaponCategory == 13` |
| Bolt | `weaponCategory == 14` |

`sortId == 9999999` is FromSoftware's convention for a duplicate row that is only for NPCs, hidden from the
player's menu; it is always excluded, like `!prop sortId 9999999` on every original Mass Edit command.
`enum WeaponKind` classifies once and serves both the filter and the choice of multipliers.

Fields written (script name, then the Rust accessor): `attackBase*` (Physics, Magic, Fire, Thunder, Dark,
`u16`), `correct*` (Strength, Agility, Magic, Faith, Luck, `f32`), `gemMountType`, `weight` (`f32`),
`sellValue` (`i32`) and `maxArrowQuantity` (`u8`).

## `Bullet` (`bullet.rs`)

`Bullet` has no field to filter on like `weaponCategory` (confirmed while analysing the original CSV), so
the set of rows to edit **has to be data**. What is regular is the layout of ids inside each "family" of
ammunition: a base id plus fixed offsets for the Ash of War variants (for example `base + 11` Mighty Shot,
`base + 50` the speed of Rain of Arrows, `base + 51` its range). The mod encodes that structure once (a list
of base ids combined with 4 offset tables: Arrow, Great Arrow, Radahn's Spear, Bolt) instead of about 1000
hand-written Mass Edit lines. Today there are 63 base ids (32 Arrow, 6 Great Arrow, 1 Radahn's Spear, 24
Bolt).

Each offset has a role: `Speed` (`initVellocity`, `maxVellocity`), and only Rain of Arrows is split into 3
sub-bullets because they need separate tuning: the range (`dist`, where damage falloff starts, not how far it
flies), the count (`numShoot`) and the spread (`shootAngleYMaxRandom`, `shootAngleXMaxRandom`).

- **`numShoot` is assigned, not multiplied.** The vanilla value is always 1, so "count" is a more honest key
  than a multiplier of a value that is always 1.
- Great Arrows cannot fire Mighty Shot, Barrage or Enchanted Shot, so their offset table jumps from the plain
  bullet straight to Rain of Arrows.
- `apply` counts the ids it did not find and warns when there are any: a sign that the game updated and the
  base id lists need checking (export `Bullet.csv` from a param editor and compare the ids).

## A reload does not compound

`weapon::apply` and `bullet::apply` once scaled **the value currently read from the row**, so a second call
compounded (`* 1.5` twice gave `* 2.25`). The **original** values of each row (`weapon::Baseline`,
`bullet::Baseline`, keyed by row index and by row id) are now saved on the first `apply`, and every later
one, reloads included, scales from that baseline. Fields assigned absolutely need none: `max_arrow_quantity`,
`num_shoot` and `gem_mount_type` (always `2`).

## Start-up

`SoloParamRepository::instance_mut()` returns `Ok` as soon as the object exists, which is **earlier** than the
moment the resource file of each param (`EquipParamWeapon`, `Bullet`) finishes loading. Calling `rows_mut`
then reaches `get_param_file_mut`'s `.expect("Expected param holder to have exactly one res cap")` and
panics, which on a thread spawned with `std::thread::spawn` ends the thread silently: no crash and no log
line. The first test showed it as a log that stopped right after "SoloParamRepository ready", with nothing
applied.

- `shared::player::wait_for_solo_param_repository` also waits for `WorldChrMan::main_player`, meaning the
  player is in the world and the params are loaded ([player.md](../../../shared/docs/player.md)). It waits
  forever, as `wait_for_cs_task` does, so there is no early give-up that would also drop the reload task.
- A second layer, `apply_with_retry` in `lib.rs`: the first `apply` runs under `catch_unwind` and is retried
  for up to 30 seconds if it still panics. Retrying is safe because the panic always comes before any row is
  edited.
- The `Mutex` that keeps the original values recovers from poisoning, so a retry does not panic again at the
  lock.

## Rows are walked by index

Both files use `shared::params` (`for_each_row_mut` and `row_ids_or_reason`) instead of the `rows()` and
`get_mut(id)` of fromsoftware-rs, which go through a lookup table that can be one entry shorter than the
header on a regulation such as Convergence's and panic ([params.md](../../../shared/docs/params.md)).

## Open risks

- **Integer rounding:** `u16` fields (damage) and `u8` (`max_arrow_quantity`) use `.round()` before the cast.
  The difference from a param editor's Mass Edit, which may round or truncate, has not been checked.
- **The bullet id lists belong to the vanilla regulation:** an overhaul that changes the id layout makes some
  ids "not found" (a warning, no crash).
