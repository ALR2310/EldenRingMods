# Input for a game window in the background

Status: **Confirmed** on 2.7.2.0: keyboard and mouse can be injected into a game that is not in the foreground,
including for mods that read keys with `GetAsyncKeyState` and a menu cursor placed by coordinates (from the title
screen into the world, walking with W, opening and closing the map with M, turning the camera, the left button
swinging the sword); the real keyboard and mouse are filtered out while the game is not in front. Not done: the
gamepad (only filtered), a pipe channel, reading state directly. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/input/`.

## Question

How can an agent play the game while its window sits in the background and the user keeps working elsewhere?

## Conclusion

`PostMessage` never reaches the game: it does not read input through window messages. `eldenring.exe` only
imports `DINPUT8!DirectInput8Create` and `XINPUT1_4` (`XInputGetState` / `XInputSetState`). Sending keys while the
game is in the background means removing **three layers** of blocking, none enough on its own:

| Layer | Symptom | Fix (`src/`) |
|---|---|---|
| 1. DirectInput loses the device when focus is lost | the keyboard `GetDeviceState` returns `0x8007001E` (`DIERR_INPUTLOST`) | `dinput.rs`: hook `GetDeviceState` / `GetDeviceData`; on an error return `DI_OK` with an empty state, then OR in the injected keys |
| 2. The game's "foreground only" flags | reading the keys succeeds but the game ignores them | `gate.rs`: clear the 3 bytes `+0x88E` (pad), `+0x88F` (keyboard), `+0x890` (mouse) of the input singleton |
| 3. A `GetForegroundWindow` compare with the game window | still ignored once the first two are removed | `focus.rs`: hook `GetForegroundWindow` to return the game window, with `ClipCursor` / `SetCursorPos` so the game does not trap the user's mouse |

Evidence: each layer was isolated. Layers 1 and 2 only: the keyboard reads with no error in the background but E
does nothing; when the game really has focus the same E works at once (so the injection is right); adding layer 3
makes E work in the background.

## The gate of layer 2 (IDA, 2.7.2.0)

The singleton is the global `qword_144864D48` (RVA `0x4864D48`; found by AOB, not hard-coded). Byte `+0x88D` =
"window active", `+0x88E/0x88F/0x890` = "SetForeground" for pad, keyboard and mouse, loaded at initialisation
from the config flags `Ext.UserInput.CooperativeLevel.SetForeGround.{Pad,Keyboard,Mouse}` (function
`0x1F2C7B0`). The pad read function `0x1F6F030` skips when `[+0x88E] && ![+0x88D]`; `0x1F70C20` builds the
keyboard device (reads `+0x88F`). The gate pattern is unique on 2.6.2.0, 2.7.0.0, 2.7.1.0 and 2.7.2.0
(`scripts/ida/run.ps1 aob -Version all`).

`GetForegroundWindow` is called from two places in the executable: `0x22AC530` (audio configuration) and
`0x2671EB0` (compare with the game window, called by 6 functions: `0x9BEBB0`, `0x9BF350`, `0xE21B80`,
`0xE3785F`, `0xE85F20`, `0xE93D80`).

## A fault: do not use `GetWindowTextW` on the game window

Finding the game window with `EnumWindows` and `GetWindowTextW` crashed the game at start-up
(`eldenring.exe+0x1E937F0`, `a1=0x20`): for a window of the same process `GetWindowTextW` sends `WM_GETTEXT`
straight to the window procedure, which the game had not finished initialising. Use `InternalGetWindowText`
(reads directly, sends no message).

## Test protocol

Write `Toolbox.cmd` next to the DLL (write a temporary name, then rename); the mod reads and deletes it: `down
<key>`, `up <key>`, `press <key> [seconds]`, `release_all`; mouse: `mouse <dx> <dy>`, `look <dx> <dy> <seconds>`
(shifts per frame; `look 6 0 1` turns the camera about 25 to 30 degrees), `mdown` / `mup` / `mclick <left|right|
middle> [seconds]`, `wheel <n>`. Key codes are DIK (`src/input/keys.rs`). The mouse is injected into
`DIMOUSESTATE` / `DIMOUSESTATE2` (lX, lY, lZ, then each button byte) in the same `GetDeviceState` hook. Some
actions take a few seconds to show (the map), and `look` after 3 to 5 seconds. `Toolbox.log` has a line every 5
seconds: `fg` (the real foreground), `gate=active/pad/kb/mouse`, the reads and errors of each device. The MCP
server of this folder (`mcp/server.py`) drives the same file.

## The Win32 path (`src/input/winput.rs`)

Another mod in the process reading keys with `GetAsyncKeyState` (`shared::input::is_key_pressed`: a reload key, a
zoom key) does not see the fake DirectInput keys. `winput.rs` hooks `GetAsyncKeyState`: it returns the "down" bit
(and the "pressed since the last call" bit, once per press) for a fake key or mouse button, converting the
virtual key to a scan code with `MapVirtualKeyW(vk, MAPVK_VK_TO_VSC_EX)` (`0xE0xx` becomes `0x80|xx`, as DIK).
While the game is not in front and `IsolateInput` is on, the real keys are hidden from this hook as well (the
same idea as the DirectInput filter). Tried: `press minus` changed the range of another mod's minimap.

`eldenring.exe` imports `GetCursorPos`, `ScreenToClient`, `SetCursorPos`, `GetKeyState` and `GetKeyboardState`.
`GetCursorPos` is hooked: while `moveto x y` is on and the game is not in front it returns the screen position of
the point (x, y) of the game window's client area (converted with `ClientToScreen` on every call, so a dragged
window stays right). On the map, `moveto` onto a grace icon showed the grace label in the right place. Not tried:
`mclick` there, and whether other menus (lists, dialogs) follow this cursor; `GetKeyState` and
`GetKeyboardState` are not hooked.

## Open

- The gamepad (`XInputGetState`) is only filtered (buttons and sticks cleared while the game is in the
  background), not injected.
- Real combat was not tried (lock-on, guard, roll).
- The buffered path (`GetDeviceData`) is not used by the game on the title screen (0 calls).
- The mod carries its own MinHook; do not load it together with another DLL hooking the same DirectInput
  functions without the shared extension point (`shared::overlay::InputExtension`).
