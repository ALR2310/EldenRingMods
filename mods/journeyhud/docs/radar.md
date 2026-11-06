# The minimap dot layer (radar)

Status: **Confirmed** in game for the dots, the hostile-enemy dots (Mistwood, Limgrave, ModEngine 2) and the
trail was written to match; caves, legacy dungeons and the DLC were not tried for the enemy dots. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/radar/mod.rs` (reads the game), `src/radar/ui.rs`
(draws). The map background is [map.md](map.md).

The player sees a **Minimap**; "radar" in the code is the layer of dots (partners, spirits, enemies, facing).
The map layer is drawn under the dots, so the `radar*` file names stay right.

A round minimap at the top right, on top of the column with the target bar just under it (it moves with the
minimap size): the player's arrow in the middle, turning with the camera (what the camera faces is up), dots
for partners, the player's spirits and the locked enemy, and N/E/S/W letters on the rim. It is drawn with ImGui
shapes and uses no asset.

## Mechanism

Ideas taken from an MIT-licensed minimap mod (see `THIRD-PARTY-NOTICES.txt`):

- **Snapshot on the game task, the render thread only reads the copy.** Same model as the target panel.
- **Camera direction**: `CSCamera::instance().pers_cam_1.matrix` (view matrix): `camera_right = (m.0.0, m.0.2)`,
  `camera_forward = (m.2.0, m.2.2)` on the ground. To the screen: `[dx*rx + dz*rz, -(dx*fx + dz*fz)]` (forward
  is up).
- **Character direction**: `chr_ctrl.physics_model_matrix`; the character looks along the model's **-Z**:
  `facing = (-m.2.0, -m.2.2)`.
- **Relative position, no world coordinates**: the physics matrices of all characters share the Havok space, so
  the difference `m.3 - player.m.3` is enough. Only the map background needs the block to world conversion
  (`locate`, `WorldMapLegacyConvParam`).
- **Seamless partners**: their block position is not set but the physical position is live, so they are placed
  by the difference to the main player, through `world_chr_man.player_chr_set` without oneself.
- **Smooth arrow**: the player's facing changes instantly so it is smoothed by angle (`wrap_angle`); a partner
  out of range is pinned to the rim and faded.
- **Hidden when** `CSFeManImp.hud_state != Default` (a menu is changing the HUD) or `CSFade` covers the screen
  (loading, warp; the maximum of `fade_plates[i].current_color.a`): it fades, then hides, because menus and the
  world map are drawn where the radar sits.
- **Cutscenes** (`Snapshot::in_cutscene`): the minimap also fades while bytes `+0x18` and `+0x19` of
  `CSMenuManImp` (`unk18`) are both 0; they are 1 otherwise. Found on 2.7.2.0 by diffing the bytes of
  `CSFeManImp`, `CSMenuManImp`, `CSCamera` and the player between two marks set at the start and end of a
  boss intro; confirmed on two boss intros and the opening cutscene. What the bytes mean is unknown. Not
  usable: `hud_state` and `CSCamera::camera_mask` do not change; the flags of `CSFeManImp` (`+0x94`, `+0xa8`
  `enable_equip_hud`, ...) and `CSMenuManImp::ui_states[23]` follow the HUD's own auto-hide when idle.
- **hudhook**: loading a texture at a strange moment can hang the loading screen, and every reload leaks a
  buffer; JourneyHUD loads textures exactly once at start-up and the radar uses none.

Not taken: the whole map part (archive decoding, tile stitching, `archive.rs`, `formats.rs`), the Map for Goblins
icons (`mfg.rs`), the frame and arrows as images (`assets/`).

## Differences from the map layer

- The radar has no map background; the circle has a translucent dark fill, two guide rings and a beige rim (the
  colour of the line under the health bar).
- Spirits are JourneyHUD's own dots (from `summon_buddy_chr_set`, including other players' spirits if any).
- Singletons are read only through `instance()` by name (never `rva::get()`).

## Notes and open points

- The offsets of `CSCamera`, `CSFade`, `CSFeManImp` and `physics_model_matrix` come from fromsoftware-rs (the
  workspace revision). **Open**: does the arrow point the right way, does the radar turn the right way when the
  camera turns, are partners and spirits in the right place.
- "Same area" filter: only characters with the same `block_id.area()` and within 3 times the radar range are
  taken, because in another area the coordinates only look close.

## Hostile enemy dots

`Minimap.Layers.Enemies` (default `false`, a row in the menu). `src/radar/enemies.rs` walks `WorldChrMan::chr_sets`
(entries `Active` or `ReadyForActivation`, as `speedmultiplier` does) and keeps the live enemies
(`modules.data.hp > 0`) that satisfy all three: `chr_type` is `Npc` or `Unk7` (boss), `team_type` is not in the
friendly-team list (`NOT_ENEMY_TEAMS`, copied from `speedmultiplier/src/enemy.rs`) and `NpcParam.threatLv > 0`.
`radar/mod.rs` converts to offsets with the same same-space filter as partners; the locked enemy is dropped from the
list (it has its own sword icon). `radar/ui.rs` draws a small red dot (radius 3 px times the scale, alpha 0.8)
under every other mark, only inside the minimap (not pinned to the rim).

Only `chr_sets` is read, not `open_field_chr_set` (a type not verified in fromsoftware-rs). `Logging.EnemyProbe`
logs `Enemies: N hostile in S of C chr_set(s)` whenever the count changes. In game (Mistwood, Limgrave, ME2):
`29 hostile in 1 of 5 chr_set(s)`, red dots on the minimap. **Not tried**: caves, legacy dungeons and the DLC;
whether some open-field enemies sit outside `chr_sets`.

## Trail

`Radar::follow_trail` records the player's position each time it is at least 1.5 m from the last point and keeps
at most 800 points (about 1.2 km). It is cleared when the space changes (`Snapshot::space`: overworld tile,
underground, each dungeon) or on a jump of more than 150 m (teleport, fast travel). It is drawn as straight
segments joining the points to the centre, fading toward the old end; a segment with an end outside the minimap
is skipped. Not tried in game.
