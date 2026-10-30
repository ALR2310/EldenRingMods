# RiseArcher - Changelog

## [Unreleased]

- Fixed the reload key working outside the game window.
- Rewritten as a Rust DLL - patches EquipParamWeapon/Bullet live in memory via fromsoftware-rs instead of shipping a modified regulation.bin, so it no longer conflicts with other regulation.bin mods.
- Every multiplier/value is now configurable via RiseArcher.ini instead of fixed at build time.
- Added in-game settings menu support (requires the ERNativeUI mod).

## [1.16.1]

- Static regulation.bin edit via Smithbox Mass Edit - bow/crossbow/ballista/arrow/bolt overhaul.
