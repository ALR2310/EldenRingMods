# Design of the HUD: target panel, layout and game textures

Status: **Confirmed** in game for the target panel (health and posture); the minimap and its map layer are in
[radar.md](radar.md) and [map.md](map.md). Offsets per executable build: [offsets.md](offsets.md). Conventions:
[research notes](../../../docs/research/README.md).

## Goal

A DX12 overlay (Dear ImGui through hudhook) at the right of the screen:

1. **Target panel**: health and posture of the locked enemy. No stamina, no FP, no damage table.
2. **HUD frame**: a layout anchor so the panel sits right under the minimap, or in the corner without one.
3. **Minimap**: the player, partners, spirits and the locked target, turning with the camera.
4. **Map layer**: optional, drawn under the minimap dots.

## Sources of ideas

| Project | Licence | Used for |
|---|---|---|
| A third-party target-HUD mod (C++) | **GPL-3.0** | Where to find the target and where health and stagger are read. Nothing is copied or translated line by line. |
| An MIT-licensed minimap mod (Rust) | MIT | Block-to-map coordinates, reading the map tiles from the game install, the camera, the frame. Reused with attribution, see `THIRD-PARTY-NOTICES.txt`. |

Because the first is GPL-3.0 the target panel is written independently, from reverse-engineering results and
the Rust types of fromsoftware-rs.

## Layout and colour

- **Drawing area.** The game draws its HUD in a 16:9 frame that fits the window (black bands on the other
  axis; a 2560x1600 screen has 80 px bands top and bottom). The overlay anchors to that frame, not to the
  whole screen, and scales with its height (anchoring to the whole screen made the panel large and cover the
  bands). Ultrawide (21:9) counts as black side bands.
- **Size.** The player's health bar measured on a 2560x1440 screenshot: about 949 px long (about 712 at
  1080p, varying with the character's maximum HP), about 16 px thick (health 13 px, posture 12 px, 4 px gap),
  its top edge 63 px below the frame top (47 at 1080p). The target bar is as thick as the player's but shorter
  (`Width`, default 250): the full 700 was too large in a test.
- **Bar style** (measured on game screenshots): a flat dark-red core (about RGB 92,23,20) about 8 px high, a
  pinkish lower edge (110,67,55), a thin beige outline (135,128,95) with caps at both ends, a nearly black
  background. The boss bar: a flat red core (76,0,0), a yellow block (143,117,23) for recently lost health with
  a cream line (226,228,191) where it is split, one beige line (150,143,114) at the bottom and small caps, no
  outline. No background box and no numbers, as the game has none. A first version drew a box, bright red and
  text inside the bar, which looked out of place.

## Textures of the game

`assets/Bar.png`, `BarBG.png`, `BarEdge.png`, `BarEdge2.png` are the health-bar textures taken from the game's
own HUD, embedded with `include_bytes!` and loaded once through `RenderContext::load_texture`
(`src/textures.rs`).

- `Bar.png` (820x26, grey, 24 visible rows): tinted by the vertex colour and cut by UV to the health ratio; also
  used for the block of recently lost health (yellow) and posture (yellow-brown). The colour is divided by the
  texture's mean brightness 0.86 to get the measured colour.
- `BarBG.png`: a translucent inner background, tinted dark, on a black base.
- `BarEdge.png` (6x36): the dividing line at the head of the health when there is a yellow block.
- `BarEdge2.png` (98x40): the glow at the head of the health, fading while the yellow block is held.
- **Frame**: `BarWaku.png` (840x48, `MENU_FL_ConditionWaku` of the `SB_FE` sheet) and `BarWakuL.png` (12x48,
  `MENU_FL_ConditionWaku_L`) are the box the game draws around the player's buildup bar: thin lines at rows 6 to
  12 (centre 9.2) and 35 to 41 (centre 37.7), a right cap at columns 820 to 840, the left cap its own image.
  `ui::textured_frame` draws one frame per bar: two thick lines of `0.4 * s` per texel row (2.4 px, as the
  game's) centred on the top and bottom edge of the bar, two caps stretched vertically to join them, texels
  `s / 2` wide like the bar. The health and posture frames are drawn after both bars. The gap between the bars
  is 4 px because each line takes 1.2 px of it. `BarDeco` (`MENU_FL_BarDeco`) is the beige line under the
  player's health bar; it was tried and dropped in favour of the buildup frame.
- A failed texture load logs `Overlay: textures failed (...)` and draws flat bars.

## Implementation notes

- **The locked target is a handle on the player**, not inferred from a position. The handle is
  `PlayerIns::locked_on_enemy: FieldInsHandle` in fromsoftware-rs (+0x6B0). A match by
  `lock_on_target_position` was off by 8 to 25 m and was dropped.
- **Handle to ChrIns**: slot 3 of the ChrSet vtable (`get_chr_ins_by_handle`), as in
  `mods/spiritmultiplier/src/warp.rs::resolvable`. `WorldChrMan::chr_ins_by_handle` goes through `rva::get()`,
  which panics on 2.7.2.0.
- **Health**: `chr.modules.data.hp` and `max_hp`.
- **Posture**: `chr.modules.super_armor.sa_durability` and `sa_durability_max`; some bosses have none, so the
  posture row is hidden then.
- **Snapshot model**: the task on `CSTaskGroupIndex::FrameBegin` captures a snapshot on the game thread and the
  render thread only reads it; the render thread never touches game objects. The mod gates reading on "in game"
  from `shared::player`, and finds game data by pattern and checks it per build instead of locking the
  executable version.
- **Pitfalls of similar overlays**: a crash at start-up when a menu pointer is still null, clashes with other
  DXGI hooks (a monitoring overlay), and the need to wait before initialising the overlay.
- **Coordinates** (for the minimap): an overworld tile `mAA_XX_ZZ_LL` is `256 << LL` metres wide with the
  position relative to the tile centre; old dungeons convert to world coordinates through
  `WorldMapLegacyConvParam` (pick the nearest reference point); underground areas use overworld coordinates
  with their own map. The map background comes from the player's own install at run time (archive decoding and
  texture decompression); no game image is shipped.
- **Co-op partners**: the block position of other players is not set, so it is derived from the physical
  position difference to the main player (the same Havok space).
- **Hidden when**: a menu, the world map, a loading screen, a screen fade, optionally in a boss fight.
