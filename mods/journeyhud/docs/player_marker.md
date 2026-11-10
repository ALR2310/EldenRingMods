# A clearer player arrow, and compatibility with Blue Markers

Status: **Confirmed** in game with ModEngine 2 and me3 that JourneyHUD picks up an overriding asset. UXM was
not tried. Conventions: [research notes](../../../docs/research/README.md). Code:
`src/map/sprites.rs::read_menu_file`.

## Question

A user asked for the player arrow to be easier to see and wanted to use the Blue Markers mod (Nexus 11179).
The default arrow blends into the grace icons and the map. How can JourneyHUD show the replaced arrow?

## What Blue Markers is

One file: `menu/hi/01_common.tpf.dcx` (about 67 MB compressed), a replacement for the game's common menu
sprite sheet (the same file JourneyHUD reads its sprites from, see [map.md](map.md)). Unpacked and compared
sprite by sprite: only the `SB_MapCursor` sheet changes, 63 sprites (the arrows `MENU_MAP_Player_01/02`,
enemies, friends, host, memo, markers...); `SB_MapCursor_02` and `_03_dlc` are identical. The `Player_01`
arrow has the same frame and size and only changes from yellow to cyan. No `01_common.sblytbnd.dcx` ships
with it, so the sprite coordinates stay.

## Why JourneyHUD did not show the blue arrow

`sprites.rs` read `/menu/hi/01_common.tpf.dcx` through `archive::open(game_dir, ...)`, that is only from the
game's own `Data0..3` and `DLC` archives. A loose file that a mod loader serves from the mod folder was not
considered, so the arrow stayed yellow.

## How the loaders serve an override

Both work when the program opens a file, so the DLL needs no profile or mod folder lookup:

- **ModEngine 2** hooks `CreateFileW`; for a path under the game folder it tries each `mods` entry of
  `config_eldenring.toml` and opens the file with the same path there.
- **me3** separates `[[packages]]` (assets) from `[[natives]]` (DLLs); since 0.8.1 it installs the
  `CreateFileW` hook before loading natives so a file looked up inside the game folder can come from a package.

So there is no public VFS API to call, and parsing `config_eldenring.toml` or a `.me3` file is wrong: ME2 uses
the `MODENGINE_CONFIG` environment variable and me3 hands a compiled package list to its host and may use a
profile anywhere.

## Fix

`read_menu_file` first tries `std::fs::read(<game_dir>/menu/hi/01_common.tpf.dcx)`, for the texture and for
the layout. With UXM that is the real file; with ME2 or me3 the `CreateFileW` hook returns the winning file in
package order. If the file is not there it falls back to `archive::open` as before. JourneyHUD therefore uses
the asset the game uses, whatever the loader or the mod folder name.

A cheaper option that does not depend on another mod is an option for the player arrow itself (larger,
tinted or outlined), drawn in `radar/ui.rs` (`Icon::Player`).

## Open risks

UXM and the case without an override were not tried again after the change.
