# Overlay input hooks

Status: **Confirmed** in game by `mods/journeyhud` (its drag mode); not tried together with other overlay DLLs
beyond the case in `layout_drag.md`. Conventions: [research notes](../../docs/research/README.md). Code:
`shared/src/overlay/` (Cargo feature `overlay`, which pulls in `hudhook` from `vendor/hudhook`).

## Question

A mod that draws its own Dear ImGui overlay and wants the mouse (for a drag, a menu) needs the cursor released,
absolute coordinates and the mouse kept from the game. How?

## Pieces

| Item | Use |
|---|---|
| `overlay::MENU_OPEN` / `is_open()` | While set, the game gets no mouse and its cursor is not pinned. |
| `install_input_hooks()` | Installs the cursor and mouse-block hooks once; later calls do nothing. Install lazily (at the first use) so a player who never uses the feature has no extra hook. |
| `feed_mouse(ctx, buttons_down)` | Feeds the cursor position (in `display_size` units) and the left and right buttons to ImGui and frees the cursor from the game's clip rectangle. |
| `blocks_window_message(umsg)` | Whether the game must not see a window message (a mouse message or raw input while `MENU_OPEN` is set), for the mod's own `before_wnd_proc`. |
| `overlay::mh` | MinHook (`MhHook`...), for a mod that hooks more than the overlay does. |

## Why the mouse is the hard part

In normal play the game hides the cursor, holds it at the window centre (`SetCursorPos`, `ClipCursor`) and reads
relative mouse input for the camera (`WM_INPUT`, DirectInput, `DLUID::MouseDevice::Poll`). Window messages are
not enough to keep the mouse from the game, so three things are hooked (`input_block.rs`):

1. **`SetCursorPos` and `ClipCursor`**: neutralised while `MENU_OPEN` is set, so the cursor is free.
2. **The game's mouse poll** `DLUID::MouseDevice::Poll` (pattern `40 53 48 83 ec 20 48 8b d9 48 8b 89 e0 07 00
   00 48 85 c9 75 08 32 c0 48 83 c4 20 5b c3 80 bb fc 07 00 00 00`, unique on 2.6.2.0 to 2.7.2.0): right after
   the game polls DirectInput it zeroes `DIMOUSESTATE2` (`+0x7E8`, 20 bytes) and the float axes (`+0x800`, 12
   bytes). This works even when a `dinput8.dll` proxy in the game folder redirects the game's DirectInput
   vtable to `System32\dinput8.dll`.
3. **DirectInput device reads** (`GetDeviceState`, `GetDeviceData`) on both the system `dinput8.dll` and any
   local `dinput8.dll` proxy, probing `GUID_SysMouse` and `GUID_SysKeyboard`: up to 4 vtable targets; a mouse
   state read is zeroed and a buffered mouse read returns no events while `MENU_OPEN` is set.

Keyboard and gamepad still reach the game, so the player can keep moving.

## Risks

- A function can be hooked once per DLL and every DLL has its own MinHook; other DLLs hooking `ClipCursor`,
  `SetCursorPos` or DirectInput are chained by MinHook, but two `hudhook` DLLs hooking DX12 at the same moment
  have overwritten each other. The poll hook reports `false` when another DLL hooked the same function first;
  the DirectInput hooks still block the mouse then.
- An Overlay menu host (the cursor, fonts and the window chrome) used by other mods is not part of this module
  yet.
