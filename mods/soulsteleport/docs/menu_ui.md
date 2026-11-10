# The menu: Dear ImGui over DX12, input and display

Status: **Confirmed** in game: all three window modes, a long list with a scrollbar, together with another
overlay mod. The menu is not controllable with a gamepad. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/ui.rs`, `shared::overlay`
([overlay](../../../shared/docs/overlay.md)).

## Question

How does a mod show a clickable menu over the game, with a free mouse cursor and without the mouse reaching
the game?

## From an add-on to a standalone mod

The first version was an add-on of a third-party chat mod with a `/teleport <name>` command. Its static analysis
(IDA, the unpacked build): the mod has no plugin API (only `DllMain` and two hot-patch functions); its command
handler compares command names with a helper function. A hook worked: hook the shared `trim` function (22
callers), act only when the return address is the call made on Enter, record the name of a `/teleport ...` line
and return an empty string so the chat clears its input box (a 12-byte `mov rax, imm64; jmp rax` patch over the
first 8 `push` instructions of `trim`, two AOB patterns that matched exactly once in the unpacked memory).

That direction was **dropped**: command and name suggestions would need deep changes to another mod's data and
code. It became a standalone mod with its own GUI. A name of the form "Other mod: Feature" was rejected because it
reads like an official module of the other mod.

## GUI technology

Two other overlay mods were studied: one with Dear ImGui and MinHook hooking the DXGI `Present` and
`ExecuteCommandLists` of D3D12 (with a transparent-window fallback), one with no DirectX hook and its own Win32
window drawn with GDI. The choice is **ImGui through hudhook** (the `dx12` feature only), which works in every
window mode. The menu is drawn on the game's render thread (inside the hooked `Present`); everything that touches
the session, Steam or the warp stays on the `FrameBegin` task. They share only a snapshot of the partner list
(refreshed every 500 ms while the menu is open), a status line and the SteamID the player clicked.

## Cursor and keeping the mouse from the game

The game hides and locks the cursor and reads the mouse through raw input, so hudhook's `WM_MOUSEMOVE` is not
usable. What is done now (`shared::overlay`):

- **Feeding the mouse to ImGui** (`feed_mouse`, in `before_render` while the menu is open and the game window has
  focus): `GetCursorPos` and `ScreenToClient`, converted from client pixels to the swap chain size
  (`display_size`, which also handles Windows scaling of 150% when the game is not DPI aware); the buttons come
  from `GetAsyncKeyState` and are sent only when they change and released when the menu closes. `before_wnd_proc`
  returns `Break` for `WM_INPUT` and every `WM_MOUSE*` while the menu is open, so hudhook's relative events
  (added on top of ImGui's "no position" value, which is never valid) do not overwrite the absolute position.
- **Hooks of `SetCursorPos` and `ClipCursor`** (MinHook inside hudhook): the game keeps re-locking the cursor
  and pulling it to the centre for the camera. With the menu open `SetCursorPos` does not move (it pretends to
  succeed) and `ClipCursor` always passes `NULL`.
- **Keeping the mouse from the game** (`input_block.rs`): `message_filter` has no effect because the game does
  not read input through window messages: the imports of `eldenring.exe` are only
  `DINPUT8!DirectInput8Create` and `XINPUT1_4` by ordinal. The mod creates a temporary `IDirectInput8` and a
  temporary keyboard device to read the vtable and hooks `GetDeviceState` (slot 9) and `GetDeviceData` (slot
  10); dinput8 shares the device implementation, so the game's own device is hit too. While the menu is open the
  original is still called (to drain the buffer so closing the menu does not replay keys), then the mouse state
  (`DIMOUSESTATE`/`2`) is cleared and the events of a device recognised as a mouse are dropped. **Only the mouse
  is blocked**; keyboard and gamepad movement still works. The joystick format is not cleared because an
  all-zero `DIJOYSTATE` is a stick pushed to its limit. An early version also hooked XInput and blocked the
  keyboard; dropped.

### The cursor vanished in small windows: `ScaleAllSizes` rounds the cursor size

At 1680x945 and below no cursor was visible although the log showed the position was exactly right (clicking
blind worked), so the fault was in **drawing**. `ImGuiStyle::ScaleAllSizes` does
`MouseCursorScale = ImFloor(MouseCursorScale * scale_factor)`. With the resolution scale, 1080 px gives 1.0 (so
1), 1440 px gives 1.48 (so 1), 945 px gives 0.875 and **0**: the cursor is drawn at size 0. Fix: after
`scale_all_sizes`, set `mouse_cursor_scale = original * scale` without rounding. (A first guess, that 1680x945
is the largest window fitting a 2560x1440 screen at 150% scale, was not the cause.)

## Display

- **Automatic scaling**: the reference is a 1080p game window; everything is multiplied by the window height over
  1080 (clamped to 0.6 to 3.0), read from `io.display_size` every frame so a resolution change while playing is
  followed. The font is loaded once at 40 px and shown at `22 px * scale` through `font_global_scale`; the style
  uses `scale_all_sizes(scale)` from a base style saved at start-up (never stacked on an already scaled one).
  When the scale changes, the next open resets the window size and position once.
- **`Menu.Scale`**: a user factor on top of the automatic one, read once at start-up (restart after changing it).
  Text, style and the **size** of the menu follow the total scale; the **position** follows only the resolution
  (`pos_scale`), so changing `Scale` does not move the menu from where it was put. The font raster is
  `40 * Scale` clamped to 40 to 64 px (no higher because the atlas may also hold CJK).
- **Own theme** (`apply_theme`, applied once **before** the base style is saved so scaling stays right): colours in
  the tone of the game's menus (warm translucent black, cream text, bronze-gold accents), rounded corners, wide
  padding, centred title. ImGui's default theme (black with blue borders) was judged too crude.
- **Layout**: no default title bar; the header is the mod name centred in gold, an `X` close button at the right
  and a small **Reset** button at the left; one line per player with the role; the status line turns light red on
  an error.
- **The status line clears itself** after 15 seconds (`STATUS_TTL`) unless a request is pending. Two earlier
  faults: "Teleporting to X..." was never cleared after a successful warp, and closing and reopening the menu
  cleared the status while the button stayed locked.
- **Saved position and size**: four keys in `[Menu]` (`X`, `Y`, `Width`, `Height`), -1 = default, in 1080p units
  (divided by the UI scale). Every frame the real position and size are read; when they differ from the saved
  values by 1 unit or more **and** the left button was released, the file is written once (not every frame while
  dragging). Opening the menu the first time writes nothing. The Reset button writes -1 for the four keys and
  resets the layout on the next frame, clearing the comparison so that the default layout just applied is not
  written back as numbers.

## Embedded font

The first font list hard-coded `C:\Windows\Fonts\segoeui.ttf`, `arial` and `tahoma`: it fails if Windows is on
another drive, the font is missing or on **Proton / Steam Deck** (Windows fonts are usually absent in Wine). No
crash (ImGui's default font is used) but that font is ASCII only, so accented names became `?`. Now:

- **The main font is embedded in the DLL**: `shared/assets/NotoSans-Subset.ttf` (Noto Sans Regular cut with
  `pyftsubset` to U+0020-024F, U+0300-036F, U+1E00-1EFF, U+2000-206F, U+20AC: 1007 glyphs, 91 KB), SIL OFL 1.1
  (`shared/assets/NotoSans-OFL.txt`, no Reserved Font Name). When publishing, credit the font and ship the
  licence file: `scripts/build-mod.ps1` with `-Zip` packs every `*.txt` of the crate folder.
- **Optional CJK**: the real Windows folder is found with `GetWindowsDirectoryW`, the first of
  `msyh`/`msjh`/`meiryo`/`msgothic`/`simsun` is merged with `chinese_simplified_common`; none means none.
  **Korean is not supported**: ImGui's Hangul range is all 11k syllables, too large for the atlas at a 40 px
  raster.
- No more `Box::leak`: `FontAtlas::add_font` copies the font data into the atlas.

## The menu key

An early version had several keys (save or warp to a saved spot, log the player list, a reload key...); they are
gone. The config has the menu key and a few menu and log settings. The default changed from `F10` to **Tab**;
`parse_virtual_key` does not understand the name "TAB", so the file holds the hex code and a link to the key
table. A third-party chat mod also binds Tab by default, so using both with defaults clashes.
