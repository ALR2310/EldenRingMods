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
  the 16:9 frame (`Placement.area`): `Layout.OffsetX` (from the right edge; negative moves it outside, which
  suits ultrawide) and `Layout.OffsetY` (from the top), in 1080p units times `res = area.h / 1080`. Minimap:
  `radar/ui.rs` (`center = [area.x + area.w - offset_x * res - radius, area.y + offset_y * res + radius]`).
  Target bar: `ui.rs::draw_target`, just under the minimap at
  `y = top + offset_y * res + radar_ui::column_height(res)`.
- **A way to change and save the offsets already existed**: the `Layout.OffsetX/OffsetY` sliders of the
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
- Position model: dragging the column adds the mouse delta (px) divided by `res` to `Layout.OffsetX/OffsetY`
  (the sign of X is reversed because it is measured from the right edge), clamped to the frame; a negative X
  reaches the side areas of an ultrawide screen. The target bar can also be dragged on its own through
  `Layout.TargetOffsetX/TargetOffsetY` (default 0, added to its default place under the minimap) so old
  configs do not change.
- **The target bar slides up when the minimap is hidden** (in caves, in boss fights) only while both
  `TargetOffsetX/Y` are 0; a bar that was dragged stays put, and in the drag mode it always sits where it
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
