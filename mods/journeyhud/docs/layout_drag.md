# Dragging the minimap and the target bar with the mouse

Status: **Confirmed** in game: the minimap and the target bar drag, the camera stays still and the file is
written when the button is released. Not tried together with ERSS-FG. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/drag.rs`, `src/ui.rs`,
`shared/src/overlay/` ([overlay](../../../shared/docs/overlay.md)).

## Question

How can the player move the minimap and the target bar with the mouse, when the game hides the cursor, keeps
it pinned to the window centre and reads relative mouse input for the camera?

## Layout before the drag mode

- **One column, one anchor.** The minimap and the target bar share one column anchored at the top right of
  the 16:9 frame (`Placement.area`): `Minimap.OffsetX` (from the right edge; negative moves it outside, which
  suits ultrawide) and `Minimap.OffsetY` (from the top), in 1080p units times `res = area.h / 1080`. Minimap:
  `radar/ui.rs` (`center = [area.x + area.w - offset_x * res - radius, area.y + offset_y * res + radius]`).
  Target bar: `ui.rs::draw_target`, just under the minimap at
  `y = top + offset_y * res + radar_ui::column_height(res)`.
- **A way to change and save the offsets already existed**: the `Minimap.OffsetX/OffsetY` sliders of the
  ERNativeUI menu call `settings::set_offset_x/y` and `config::save`, which keeps every comment of the
  template. Dragging is another way to enter the same two numbers.

## Why the mouse is the hard part

In normal play the game hides the cursor, holds it at the window centre (`SetCursorPos`, `ClipCursor`) and
reads relative mouse input (`WM_INPUT`, DirectInput, `DLUID::MouseDevice::Poll`). Dragging needs: (1) the
cursor released, (2) absolute coordinates, (3) the mouse kept from the game so the camera does not turn and a
click is not an attack. `shared::overlay` does all three (input hooks and `feed_mouse`).

## Options considered

| Option | How | For | Against |
|---|---|---|---|
| A. Arrow keys | A key enters an edit mode; Tab picks a frame, arrows move 1 px, Shift 10 px | No extra hook, no clash with other DLLs | Not a drag |
| **B. Mouse drag with hooks (chosen)** | The edit mode captures the mouse: cursor released, ImGui cursor drawn, absolute position from `GetCursorPos` + `ScreenToClient`, mouse hidden from the game | What was wanted | Needs the overlay's input hooks; other DLLs hook the same functions |
| C. Drag while a game menu is open | Frames are drawn while the game's own cursor is visible | No extra hook | The player must open a menu first; the click still reaches the menu below; the minimap is hidden in menus |

A third-party HUD mod that offers a drag option works as C: its config key (`false` by default, no hotkey)
removes the `NoInputs` flags of its ImGui window, the game still receives every click, and it saves the
position at every frame of the drag. Lessons: save only when the button is released, and beware of adding and
subtracting the offset every frame when switching between window coordinates and the config offset (that mod
once drifted left).

## Mechanism (option B)

- `General.DragKey` (default F7) switches the mode. The cursor and mouse-block hooks
  (`ClipCursor`, `SetCursorPos`, the game's mouse poll, DirectInput reads) are installed **lazily** the first
  time the mode starts, so a player who never uses it has no extra hook in the game.
- While the mode is on `overlay::MENU_OPEN` is set so the game sees no mouse; a frame is drawn around the
  minimap and the target bar. Dragging changes the offsets at once (`settings::set_offset_*`) and
  `config::save` writes them when the button is released.
- Position model: dragging the column adds the mouse delta (px) divided by `res` to `Minimap.OffsetX/OffsetY`
  (the sign of X is reversed because it is measured from the right edge), clamped to the frame; a negative X
  reaches the side areas of an ultrawide screen. The target bar can also be dragged on its own through
  `Target.OffsetX/OffsetY` (default 0, added to its default place under the minimap) so old
  configs do not change.
- **The target bar slides up when the minimap is hidden** (in caves, in boss fights) only while both
  `Target.OffsetX/Y` are 0; a bar that was dragged stays put, and in the drag mode it always sits where it
  would with the minimap (`radar_ui::column_height(res, slides)`).

## Risks

1. **Two DLLs hooking one function.** Each DLL has its own copy of MinHook and a function can be hooked once
   per DLL; two `hudhook` DLLs hooking DX12 at the same moment have overwritten each other before. `ClipCursor`,
   `SetCursorPos` and DirectInput are hooked by several mods. In a test with another mod that had hooked the
   mouse poll first, the poll hook of this mod reported `false` but the DirectInput hooks still blocked the
   mouse. Not tried together with ERSS-FG, which logs "Installing overlay input hooks".
2. The `DLUID::MouseDevice::Poll` pattern depends on the executable build; it is unique on 2.6.2 to 2.7.2
   (`scripts/ida/run.ps1 aob -Version all`).
3. `LoadDelay` does not matter: the hooks are installed lazily at the key press.

## Reset, reach and the rune counter

- **Reset**: in the drag mode a right click on the minimap, the target bar or the rune counter sets its offsets back to the
  config template's (`drag::reset`) and saves them at once. A part dragged back by hand is not affected.

- The offsets used to be clamped to -400..400 (the top to 0..400), so the minimap could not be dragged from the right
  side to the left. They now reach -400..1700 across and 0..900 down (the target bar's own offsets -1700..1700 and
  -900..900), which covers the whole picture.
- The rune counter (`HUD.RuneCounter.Enabled`) is a third part with its own offsets `HUD.RuneCounter.OffsetX/Y`, saved with
  the others (`docs/rune_counter.md`).
- The player's buildup gauge is a fourth part: `HUD.Buildup.OffsetX/Y` move it from where it sits under the status icons
  (right and down, -2000..4000 across and -1000..1000 down), so it follows the icons and the edge shift
  (`ui::gauge_origin`).

The `[Layout]` section is gone: the minimap's offsets are `Minimap.OffsetX/OffsetY`, the panel's `Target.OffsetX/OffsetY`
(still added to the minimap's, so the panel follows the minimap until it is moved apart on purpose). Config format 2
migrates a version 1 file (`config::v1_to_v2`).

## Wide screens with the HUD moved to the edges

A third-party mod that fixes the game's HUD on 21:9 and wider screens leaves the stage of the HUD movies at
1920x1080 and moves each element out of it, to the screen's edges. The overlay still places everything in the
16:9 picture centered in the screen, which is right for the stage but not for where that mod put the game's own HUD:
the minimap, the target bar and the player's buildup gauge sat about 320 px inside at 2560x1080.

- **Confirmed** (in game, 2.7.2.0, 2560x1080): the rune counter's own clip is at x 1762.0 on the game's layout, at any
  resolution, and at 2081.8 with that mod: 319.8 px more, which is (2560 - 1920) / 2.
- `rune_counter::edge_shift` reads that difference (the clip's place is read before the mod moves it) and clamps it
  to what the screen leaves beyond the 16:9 picture on each side, so a 16:9 screen always gets 0. The right-hand group
  (the minimap and the target bar, the drag frames) moves right by it; the left-hand group (the times under the
  status icons and the player's buildup gauge) moves left by the same.
- **Inferred**: both sides move out by the same amount (the left side was only measured on one screenshot, about 308
  against 320) and the shift is the same for the other layouts of that mod; the boss bars stay centered.
