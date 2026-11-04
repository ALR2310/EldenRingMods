# The world map under the minimap

Status: **Confirmed** in game: reading the archives and decoding the tiles (an image of Limgrave on 2.7.2.0),
the map under the minimap, beacons (overworld and DLC), the Land of Shadow origin, the Mistwood block fix.
Graces, points of interest and the Map for Goblins markers were written and checked offline; the rest of the
open points are at the end. Conventions: [research notes](../../../docs/research/README.md). Code:
`src/map/`. The dots drawn above it: [radar.md](radar.md).

The map is the game's own textures, read from the install folder at run time: the DLL ships **no image** of the
game.

## Mechanism

`src/map/` has four files (`reader.rs`, `archive.rs`, `formats.rs`, `tiles.rs`) adapted from an MIT-licensed
minimap mod (source noted at the top of each file, licence in `THIRD-PARTY-NOTICES.txt`); `world.rs` and
`layer.rs` follow its ideas. The five public RSA keys in `src/map/keys/` are per-archive keys the modding
community has published (through UXM and SoulsFormats), not anyone's secret.

1. **Archives**: `Data0..3` and `DLC` (`.bhd` index and `.bdt` data). The index is RSA-encrypted (256-byte
   blocks, public-exponent power); files are found by path hash (`hash = hash * 0x85 + byte` over the lowercase
   path); the data has AES-128 encrypted ranges.
2. **Files used**: `/menu/71_maptile.mtmskbnd.dcx` (map piece masks per cell) and `/menu/71_maptile.tpfbhd` +
   `.tpfbdt` (tiles `MENU_MapTile_M00_L0_XX_YY_<flags>.tpf.dcx`, each 256x256, 1 pixel = 1 m, M00 = overworld,
   M01 = underground).
3. **Decoding**: DCX of kind KRAK with the game's own Oodle (`oo2core_6_win64.dll` in the game folder, loaded with
   `libloading`), then TPF, then DDS BC7 or BC1 (`texture2ddecoder`).
4. **A 3x3 cell window** (768x768 RGBA) around the player is built on a background thread and stitched into one
   texture; the render thread only uploads it.
5. **Drawing**: a fan of 72 wedges cuts the texture into a disc; every rim vertex has its own UV, rotated by the
   camera (facing is up).
6. **Coordinates**: an overworld tile `m60_XX_ZZ_LL` is `256 << LL` metres wide with the position relative to the
   tile centre; map cell (i, j) is `m60_{i+28}_{j+24}`. Dungeons convert through `WorldMapLegacyConvParam`
   (nearest reference point); underground uses overworld coordinates with its own map (M01).

Verified offline: an ignored developer test used to read the archives of a real install and write the window
around cell (13, 11) as a PNG (about 2 seconds on 2.7.2.0, a correct map of Limgrave with its icons and
hand-drawn lines); two small unit tests check the overworld conversion.

## Differences from the original minimap mod

- The game build is not locked: if the archives or the parameter table cannot be read the minimap still runs
  without a background (log `Map: loader stopped (...)`).
- The whole map is shown by default; `FullMap = false` follows the map pieces the player has found.
- The maximum range is 200 m: the window is only 768 m wide, so room is left to move the window before the
  player reaches its edge.

## Graces

`map/graces.rs`: the grace table is `BonfireWarpParam` (only rows with an event flag); each grace converts to
map coordinates with `world::locate` (which needs the dungeon conversion table loaded, so it is retried every
frame until it is), and counts as found when its event flag is on (`CSEventFlagMan.virtual_memory_flag`). Every
frame on the game task only graces in the same space and within 1.2 times the minimap range are kept, as a list
of distances to the player. Drawn as a yellow dot with a small glow (`radar/ui.rs`), key `Minimap.Layers.Graces`.

## The game's own icons

The marks use the world map's own icons, read at run time as the tiles are. `map/sprites.rs`:

- A sprite sheet is a texture of `/menu/hi/01_common.tpf.dcx` (TPF, UTF-16 texture names, about 200 MB
  decompressed) and each sprite's place is in the layout `/menu/hi/01_common.sblytbnd.dcx`, XML
  `<TextureAtlas imagePath="SB_MapCursor_02.png"><SubTexture name=... x y width height/>`. It is read once on a
  background thread (about 1.3 s, a few hundred MB temporarily), each sprite is cropped, and `MapLayer` uploads
  them as textures during normal play.
- Used: `SB_MapCursor_02/MENU_MAP_01_Bonfire` (grace, 156x156, a yellow disc with a glow),
  `SB_MapCursor/MENU_MAP_Player_01` (the yellow arrow, only a hollow circle) plus `MENU_MAP_Player_02` (the small
  figure at the circle's centre), `MENU_MAP_Coop_01` + `_02` (the pink arrow of other players, also two parts),
  `MENU_MAP_Friend_00` (spirit, blue crossed swords), `MENU_MAP_Enemy_00` (locked enemy, red crossed swords).
  Also available: `MENU_MAP_Raid_01/02`, `Host`, `Guests`, `Marker` (beacon), `memo_*` and the point icons
  `MENU_MAP_NN`.
- The game's cursor is two sprites stacked: the arrow with its hollow circle and the figure inside it; both
  rotate around the circle's centre (about 54% of the arrow's height), not the image centre.
- If they cannot be read the log says `Map: icons not read (...)` and the hand-drawn shapes (yellow dot, arrow,
  red dot) are used.
- A player-arrow replacement from a mod loader is picked up: see [player_marker.md](player_marker.md).

## Points of interest (ruins, churches, caves...)

`map/points.rs`: the points are in `WorldMapPointParam` (each row has `iconId`, a block position, an open flag,
text rows and their flags). The visibility rule follows the game (through the MIT minimap mod, itself from a
community overlay's notes): the open flag must be on and, except rows allowed without text, at least one text
must be active; text of kind 1 switches to the alternative icon (`altIconId`); a completion flag dims it (a boss
that was beaten). Skipped: area label rows, icon 0, graces (1 and 2, drawn from the grace table) and the guiding
light rays (83 to 85).

The icon of a point is the sprite `MENU_MAP_<iconId>` (icon 3, the church, is `MENU_MAP_Church`) in three
sheets, `SB_MapCursor`, `SB_MapCursor_02` and `SB_MapCursor_03_dlc` (Shadow of the Erdtree).
`sprites::load_map_icons` reads all 98 once on a background thread (about 1.4 s); `MapLayer` shrinks them (at
most 96 px) and packs them into **one 1024x1024 atlas** uploaded once (hudhook leaks a buffer on every upload).
Log lines `Map: N point(s)` and `Map: N point icon(s) uploaded`; key `Minimap.Layers.Points`.

### Points that did not show

A diagnostic key that logged the map rows near the player found, at Elleh church (a 472-row table), that the
church is a row with icon 3 whose flag and text were fine, so only the sprite `MENU_MAP_03` was missing (fixed
by the name `MENU_MAP_Church`). The coordinate conversion was right (the player in `m60_42_36` at local (-63,
90), the church 25 m away, the grace 20 m). The NPC icon with a hooded figure has no row near: 71 rows with icon
80 but the nearest 208 m away, icon 87 738 m, icon 86 none. Elsewhere NPCs show normally, so Elleh may be a
special case (an NPC that does not go through `WorldMapPointParam` or is a player mark). **Open**.

## Beacons

`map/beacons.rs`. Beacons the player places have no type in fromsoftware-rs. The game keeps them in
`CSMenuMarkersSaveData`:

```text
CSMenuMan -> +0x80 -> +0x250 (world map view) -> +0x338 (marker list at view +0x300, data pointer at its +0x38)
  -> +0x08 slot array (16 bytes each), +0x10 slot count
slot: id i32 (negative = empty), x f32, y f32 (the game's map screen coordinates, Y down, 41 cells of 256 px),
      map u8 (0 overworld, 1 underground, 10 Land of Shadow), icon u8
```

**Checked in IDA**: the add-marker function (string `CSMenuMarkersSaveData.cpp`) writes exactly these fields
and its caller keeps the list at view +0x300 on **all four builds** (2.6.2.0 `0x818ED0`, 2.7.0.0 and 2.7.1.0
`0x819D50`, 2.7.2.0 `0x81A200`): slot array at +0x08, slot count at +0x10, 16-byte elements, id counter at
+0x48, data pointer at +0x38 of the list (`a1[7]`), list at `a1 + 768`. The start of the chain (`CSMenuMan +0x80
+0x250`) was not checked separately: it comes from the MIT minimap mod, whose author tried it on 2.7.2.0.

**Safety**: every pointer is read with `ReadProcessMemory` (a failure returns `None`), and the whole result is
dropped if any slot does not look like a beacon (slot count 0 or above 10, an id too large, a map other than 0,
1 and 10, a position outside the map or not finite). On a build where the chain is wrong there are simply no
beacons.

`from_game_map` converts the map screen (x, y) to metres on the overworld grid (cell (i, j) is
`m60_{i+28}_{j+24}`). The slot number (1 to 5...) is the number the game shows beside the beacon; it is not
drawn. The icon is `MENU_MAP_Marker` (64x324, a pale blue ray, the arrow head at the bottom is the position),
drawn 36 px high with its tip on the spot; a beacon out of range is pinned to the rim like other players. Key
`Minimap.Layers.Beacons`. Confirmed in game (overworld and DLC).

## Layer order and icon size

As in the game, from bottom to top: **points < NPCs < graces < beacons < spirits, partners, locked enemy <
player**. NPCs are icons 80, 86, 87, 88 (a person, a summon sign, an invader); the rest are points. One common
factor, `ICON_SCALE` (0.20), scales points, NPCs and graces (a 156 px icon becomes about 31 px); the size was
tuned in game from a first version twice as large. An earlier version moved a grace to the lower right when it
covered a point; dropped because the game does not do that and the layer order solves it. The rim of the disc
darkens in 4 faint rings so it blends with the map, and each N/E/S/W letter has a dark dot behind it to read on a
bright map.

## When reading starts

The archive-reading thread (map, icons, points) starts only **once the player is in the world** (the first frame
with `in_control`), not when the game launches: it needs a few hundred MB and a few seconds of disk and CPU and
should not compete with a game that is starting. A game crash was seen about 3 seconds after the DX12 hooks, before
the overlay was initialised (an access violation reading `0x30` in a getter, `sub_141E937F0`, called by
`sub_141E8B8B0`, related to `CGShaderResourceView`, on a thread with no frame of the mod; the same family as a
crash seen before JourneyHUD existed). A link to the mod was not shown; reading late removes one source of noise.
See also [heap_corruption.md](heap_corruption.md).

Technical notes from the original minimap mod: hudhook leaks an intermediate buffer at every texture upload, so a
new window is requested only when the player nears the edge of the old one (`margin = range + 20 m`); uploading a
texture on a loading screen can hang (the GPU queue waits for the game), so uploads happen only while
`hud_visible` and not fading (`Radar::in_control`).

## Map for Goblins

Layer `Minimap.Layers.MapForGoblins`, code in `src/map/mfg.rs`. The Map for Goblins DLL (from 2.1.6) exports
`MfgGetMapAPI` (its header `sdk/mfg_map_api.h`, ABI 1) so another mod can read the markers it draws; the reading
and the icon decoding follow the MIT minimap mod.

- **Finding the API**: `GetModuleHandleW("MapForGoblins.dll")` then `MfgGetMapAPI(1)`, checking the struct sizes.
  If not found it asks again every 2 seconds (DLL load order is not fixed); without the DLL the layer does and
  costs nothing.
- **Markers**: `view_points(layer)` with layer 0 = overworld, 1 = underground (the DLC layer is skipped, no map
  for it). Each point has `raw_map_id` (area, grid X, grid Z) and `x, z` inside the cell, converted through
  `world::locate` like a `WorldMapPointParam` row. The list is rebuilt at most 3 times a second; the highlight
  ring (bit 62 of the id) is dropped and the completion badge (bit 63) is drawn as a normal icon. The list
  already honours the player's choices in that mod's menu and items already picked up, so no flag is checked
  here.
- **Icons**: `get_icon` returns the body of an SWF DefineBitsLossless2 tag (format 3 palette or 5 ARGB,
  premultiplied alpha); zlib-decompressed and un-premultiplied. Decoded on its own thread when the API first
  appears and packed into a grid of 96 px cells (16 columns) in one texture; icons above 96 px are dropped. No
  image of that mod is shipped.
- **Drawing**: a 96 px icon is drawn at about 29 px at 1080p (`GOBLIN_SCALE` 0.30; the first 1/6 looked smaller
  than the game's icons), under graces and above the game's points; out of range it is not drawn (not pinned).
- Not tried in game with the DLL loaded.

## Land of Shadow (DLC)

The DLC map is in the same archive `/menu/71_maptile.tpfbhd`: `M10` (Land of Shadow, a 41x41 grid of 256 px
cells, the same tile format as `M00`) and `M11` (a small 16x10 area). The tiles decode with the existing reader
(all 41x41 cells of `M10` were stitched into one picture: a large island in the lower half, the Scadutree to the
north, a dark border).

- **Wiring**: `MapId::Shadow` (`M10`); `world::locate` returns `Some(Shadow)` for block area 61 (and area 61
  targets of the dungeon conversion table; the overworld grid step takes the units digit
  `(block.index() % 10).min(2)` so the tens digit of an event variant is ignored); points by `disp_mask02` of
  `WorldMapPointParam`, beacons by map 10, Map for Goblins markers by layer 2.
- **`M11`**: the DLC has no underground map. The mask name table in the executable
  (`menu:/71_MapTile.mtmskbnd`, 8 entries) lists only `M00`, `M01`, `M10`, `M03` to `M07`; `M11` is in the
  archive (1521 cells, every mask 0, not a map by pieces) but the game does not load it. Not used.
- **DLC map pieces**: the `M10` mask uses 5 bits (0 to 4; `M00` 21 bits, `M01` 5 bits), i.e. 5 pieces. A log
  line `Map: piece rows 0-20, 70-70, 100-105, 1000-1004, 1070-1070` (2.7.2.0) shows the DLC rows are **1000 to
  1004** (5 rows, matching 5 bits), not 200 to 204 as first guessed (with that guess `FullMap = false` made the
  whole region with found pieces dark). Rows 70 and 1070 are unexplained (not among the 5 bits). In the DLC
  `FullMap = false` uses base 1000.
- **Origin**: cell (0, 0) of `M10` is tile m61 (28, 24), the same frame as the base map (m60). A first guess that
  matched in game (the island spans cells x 12..31, z 10..33, i.e. m61 tiles about x 40..59, z 34..57).

## Legacy dungeons and caves

- `world::in_cave`: `Hide.InCaves` applies only to caves, catacombs and tunnels (areas 30 to 32 and 40 to 43),
  not to Divine Towers (area 34), not to legacy dungeons (areas 10, 11, 13 to 16, 18 to 22, 25, 28, 35;
  `world::is_legacy_dungeon`, the game has no flag for it) and not to the underground (area 12).
- The map background inside dungeons uses `WorldMapLegacyConvParam`: in most dungeons all reference points
  collapse to one target or shift evenly, so the nearest point to the player is right. In three the points
  contradict one another (vanilla table, the distance on the map against the one inside): Farum Azula 2683 m
  against 3673 m, the Haligtree 661 m against 0, the Finger Birthing Grounds 918 m against 10.
- **Farum Azula** (compared with the in-game map): the world map draws it as a whirlpool island in the eastern
  sea around tile `m60_61_46`, with its grace on that island. Three rows (IDs 10, 11, 132) give three places:
  row 11 (`isBasePoint` = 1, source (-2509.6, -668.0), target `m60_51_43`) puts the Crumbling Beast Grave grace in
  the right tile (61.1, 46.2), row 10 (not a base point) lands in tile (48.7, 53.4), row 132 (source (0, 0),
  target `m60_54_53` (0, 0), the point nearest to the player) lands in Flame Peak, so a first version showed a
  snow mountain. Rule: when rows contradict, use the first base-point row in param order for the whole dungeon
  (`legacy_conversion`, function `agree`). The Haligtree (row 18 before 136) and the Finger Birthing Grounds (row
  1000 before 1113 to 1116) go through the same rule and were **not compared with the in-game map**.
- An intermediate version dropped the background in every legacy dungeon; that lost the minimap in Stormveil,
  Leyndell and the Academy (which have a good conversion) and in Farum Azula (whose in-game map has a
  background). Dropped.

## Overworld blocks with an unusual index

The last digit of the block index `mAA_XX_ZZ_II` is the tile zoom (0 is 256 m, 1 is 512 m, 2 is 1024 m) and the
tens digit is an event variant. After Radahn is defeated the Mistwood region is `m60_44_37_16`: the last digit 6
is not a zoom, and an older version clamped it with `.min(2)` to a 1024 m tile, so the position fell into cell
(150, 126) of the 41x41 map, which has no tile and the minimap was blank (log `Map: no tile for Overworld cell
[149, 125]`). A last digit above 2 now counts as the small 256 m tile (`world::locate`); confirmed in game at the
Mistwood. The first hypothesis (map pieces, the Starfall Crater, `FullMap` off) was wrong; the player had
`FullMap` on. Diagnostic lines, **written only with `[Logging] MapProbe = true`** (off by default): `Map: block
... map ...` when the area changes, `Map: no tile for ...` for a missing tile, `Map: piece rows ...` when the
set of pieces changes (only with `FullMap` off).

## Open

- Check in game that the map matches the player's position (against the in-game world map), turns the right way
  with the camera, and that the window changes without a hitch.
- The underground (M01) and dungeons through the conversion table in more places; the Map for Goblins markers
  with the DLL loaded; the points and the graces in a long session.
