# Overlay: a shared Dear ImGui window over DX12, and the mouse hooks

Status: **Confirmed** in game: the window host by SoulsTeleport and Toolbox, the input hooks on their own by
JourneyHUD (its drag mode). Conventions: [research notes](../../docs/research/README.md). Code:
`shared/src/overlay/` (Cargo feature `overlay`, which pulls in `hudhook` from `vendor/hudhook`).

## Question

A mod that draws its own Dear ImGui window over the game's DX12 (through `hudhook`) needs the same
infrastructure every time: fonts, scaling, a mouse cursor the game does not fight, and a saved window layout.
What does `shared::overlay` provide, and why is each piece there?

## Pieces

| Item | Use |
|---|---|
| `install(WindowConfig, draw_body)` | Hooks DX12 and shows the window while `MENU_OPEN` is set; `draw_body(ui)` draws its contents. Call once from the mod's worker thread. |
| `set_store(Store { get, save })` | Connects the overlay to the mod's config: it reads and saves the keys `MenuScale`, `MenuX`, `MenuY`, `MenuWidth`, `MenuHeight` (a mod maps them to its `[Menu]` table, see `mods/soulsteleport/src/config.rs`). Call before `install`. |
| `save_values(pairs)` | Saves `(key, text)` pairs through the same `Store`. |
| `MENU_OPEN`, `is_open()`, `toggle()` | Whether the game must ignore the mouse because the window is open. |
| `install_input_hooks()` | Installs the cursor and mouse-block hooks once (`install` does it after the DX12 hooks); a mod with its own `ImguiRenderLoop` (JourneyHUD's drag mode) calls it lazily. |
| `feed_mouse(ctx, buttons_down)` | Feeds the cursor position and the left and right buttons to ImGui and frees the cursor from the game's clip rectangle. |
| `blocks_window_message(umsg)` | Whether the game must not see a window message, for a mod's own `before_wnd_proc`. |
| `set_input_extension(InputExtension)` | Lets a mod that also works on the game's input (Toolbox's input bridge) join the existing DirectInput and cursor hooks. |
| `GOLD`, `GOLD_DIM`, `TEXT`, `TEXT_MUTED`, `ERROR_TEXT`, `rgba`, `centered_text` | The theme's colours and a helper. |
| `overlay::mh` | MinHook (`MhHook`...), for a mod that hooks more than the overlay does. |

## What the window host does

1. **Fonts**: `shared/assets/NotoSans-Subset.ttf` (about 90 KB, Latin and Vietnamese; SIL OFL 1.1, licence in
   `shared/assets/NotoSans-OFL.txt`) is embedded, and the first CJK font found in the real Windows fonts folder
   (`GetWindowsDirectoryW`) is merged in.
2. **Scaling**: reference 1080p (`display_size[1] / 1080`, clamped to 0.6..=3.0) times `MenuScale`. After
   `scale_all_sizes` the cursor scale is set again to `base * scale` without rounding, otherwise the cursor
   disappears in windows below 1080p ([menu_ui](../../mods/soulsteleport/docs/menu_ui.md)).
3. **Cursor and mouse**: while `MENU_OPEN` is set the cursor position is read each frame (`GetCursorPos` and
   `ScreenToClient`, converted to `display_size`, also with Windows scaling) and the left and right buttons come
   from `GetAsyncKeyState`; `WM_INPUT` and `WM_MOUSE*` are blocked in `before_wnd_proc` so hudhook's relative
   events do not overwrite the absolute position.
4. **Theme and chrome**: a warm translucent black window, cream text, bronze-gold accents; a header with a
   `Reset` button on the left, the centred title and an `X` button on the right.
5. **Saved layout** (`MenuX`, `MenuY`, `MenuWidth`, `MenuHeight`, in 1080p units): written only when the left
   button is released after a drag or resize. Reset writes -1 (the default).
6. **No clash with other hudhook DLLs**: two DLLs calling `ImguiDx12Hooks::new()` (`MH_CreateHook`) at the same
   time before one has applied its queue each build a trampoline from the unhooked prologue, and the later
   `MH_ApplyQueued` overwrites the earlier jump. `install` waits for `CSTaskImp` plus 800 ms (after a standalone
   HUD such as JourneyHUD has hooked at the `CSTaskImp` mark, and before the game creates its DX12 window about 3
   s in) and holds a process-wide named mutex `Local\EldenRingMod_OverlayInit_<pid>`.

## The mouse hooks (`input_block.rs`)

In normal play the game hides the cursor, holds it at the window centre (`SetCursorPos`, `ClipCursor`) and
reads relative mouse input for the camera (`WM_INPUT`, DirectInput, `DLUID::MouseDevice::Poll`). Window messages
are not enough to keep the mouse from the game, so three things are hooked:

1. **`SetCursorPos` and `ClipCursor`**: neutralised while `MENU_OPEN` is set, so the cursor is free.
2. **The game's mouse poll** `DLUID::MouseDevice::Poll` (pattern `40 53 48 83 ec 20 48 8b d9 48 8b 89 e0 07 00
   00 48 85 c9 75 08 32 c0 48 83 c4 20 5b c3 80 bb fc 07 00 00 00`, unique on 2.6.2.0 to 2.7.2.0): right after
   the game polls DirectInput it zeroes `DIMOUSESTATE2` (`+0x7E8`, 20 bytes) and the float axes (`+0x800`, 12
   bytes). This works even when a `dinput8.dll` proxy in the game folder redirects the game's DirectInput vtable
   to `System32\dinput8.dll`.
3. **DirectInput device reads** (`GetDeviceState`, `GetDeviceData`) on both the system `dinput8.dll` and any
   local proxy, probing `GUID_SysMouse` and `GUID_SysKeyboard`: up to 4 vtable targets; a mouse state read is
   zeroed and a buffered mouse read returns no events while `MENU_OPEN` is set.

Keyboard and gamepad still reach the game, so the player can keep moving. A function can be hooked once per
DLL, so a mod that works on the same functions (Toolbox's input bridge) hands its logic to
`set_input_extension` instead of installing a second hook: its `device_state`, `device_data` and `unpin_cursor`
functions are called from the hooks here.

## Risks

- Other DLLs hooking `ClipCursor`, `SetCursorPos` or DirectInput are chained by MinHook, but two `hudhook` DLLs
  hooking DX12 at the same moment have overwritten each other; the delay and the mutex above are what prevent it.
  The poll hook reports `false` when another DLL hooked the same function first; the DirectInput hooks still
  block the mouse then.
- Korean is not covered by the embedded font and the CJK fallback (the Hangul range is too large for the atlas
  at the raster size used).
