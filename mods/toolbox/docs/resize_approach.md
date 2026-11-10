# Resizing the game window: `WS_THICKFRAME` and a window-procedure subclass

Status: **Confirmed** in game: dragging the edges resizes as intended. Code: `src/resize/window.rs`. No reverse
engineering was needed: the mod only uses Win32, no AOB or RVA and no game struct, so a game update cannot break
it. Conventions: [research notes](../../../docs/research/README.md).

## Question

The game's resolution menu has a smallest entry of 800x450 (earlier 800x420). Does lifting that limit need
reverse engineering in IDA? **No.** It was tried from outside with PowerShell before opening IDA:

1. `SetWindowPos` forcing the window to 480x300: the game **keeps** that size, still renders the whole scene
   (DXGI shrinks the swap chain into the client area) and does not pull it back. So nothing in the game needs a
   patch.
   - The first try looked like a failure: PowerShell is not DPI aware and the screen was at 150%, so "480x300"
     became 720x450 real pixels. Call `SetProcessDPIAware` before measuring or setting sizes.
2. The original window style is `0x14CA0000`, **without `WS_THICKFRAME`**, so it cannot be resized by dragging;
   800x450 is just the smallest entry of the resolution list.
3. Adding `WS_THICKFRAME` from outside: `WM_NCHITTEST` at the edge returns HTLEFT / HTRIGHT / HTBOTTOM /
   HTBOTTOMRIGHT correctly, but **no resize arrow appears**, because the game handles `WM_SETCURSOR` itself and
   forces its own cursor.

Conclusion: (a) add `WS_THICKFRAME`, (b) subclass the window procedure to hand `WM_SETCURSOR` over the border to
`DefWindowProcW`.

## Design

- **Finding the window**: `EnumWindows` takes the visible window of the current process whose class starts with
  `ELDEN RING` (the process also has hidden windows `IME` and `DIEmWin`). The overlay's `game_window()` is not
  used because it only returns when the game is the foreground window.
- **Subclassing** with `SetWindowLongPtrW(GWLP_WNDPROC)`, not `SetWindowSubclass`, which refuses a call from a
  thread other than the window's creator (the mod runs on a worker thread).
- **Own window procedure**, acting only while the border the mod added is on:
  - `WM_SETCURSOR` with hit-test 10..=17: handed to `DefWindowProcW` (shows the arrow).
  - `WM_GETMINMAXINFO`: `ptMinTrackSize` = `MinWidth` / `MinHeight` plus the frame.
  - `WM_SIZING`: enforces the minimum and the aspect `AspectWidth:AspectHeight` on the **client area**, moving
    only the dragged edge. Dragging the top or bottom edge lets the height decide the width; every other edge or
    corner lets the width decide the height.
  - The frame size comes from `AdjustWindowRectExForDpi`, not the real rect (the rect is 0 while minimised).
- **A 1 s watchdog**: the game overwrites the style on every screen mode or resolution change, so the border is
  added again periodically; the window is hooked again if it is recreated. Fullscreen and Borderless (no
  `WS_CAPTION`) are skipped. `Resizable = false` removes the border, only if the mod added it.

## Start-up size (`Width` / `Height`)

Both keys are the size of the **client area** in real pixels. When the game window is found the mod calls
`SetWindowPos` to that size (keeping the position). Both empty: untouched. One set: the other follows
`AspectWidth:AspectHeight` (with the ratio off, that axis keeps its current size). Still clamped by `MinWidth` /
`MinHeight`.

- **Applied every second for the first 20 seconds** (`START_SIZE_WINDOW`) after the hook, because the mod hooks
  the window as soon as it is created, before the game applies the resolution of its settings to it; a single
  apply can be overwritten right after (10 seconds at first, raised to 20 after testing).
- **Stops as soon as the player starts dragging** (`WM_ENTERSIZEMOVE` sets `USER_RESIZED`), so it does not
  take back the size the player just chose.
- **DPI**: the mod's thread calls `SetThreadDpiAwarenessContext(PER_MONITOR_AWARE_V2)`, so `Width` / `Height` are
  always real pixels, not multiplied by Windows scaling as in the PowerShell test. The game is already DPI aware
  (`GetDpiForWindow` = 144 at 150%; an unaware window always returns 96), so this is only a precaution. The
  window frame, when setting the start-up size, is measured from the real rects (`GetWindowRect -
  GetClientRect`) so it is in the same coordinate system as `SetWindowPos`. The hook logs the window's DPI.

## Dropped: hot reload for this part

An earlier version had no reload key here: hot reload was the only part registering code in the game's task
system and calling game functions (the "Config reloaded" banner), while the resize only uses Win32. Without it
the mod touches the game as little as possible, so a config change needed a restart. The reload key now exists
for the whole mod (hotkeys, the buildup amount and the window settings; the patches and the input bridge need a
restart). `window::run` polls for the window every second, so it does not wait for the game to finish starting.

## Open

- While the game has focus it calls `ClipCursor` to trap the cursor in the client area, so it may be impossible
  to reach the border with the mouse while playing: click outside the window first, then drag. If that is
  annoying, hook `ClipCursor` to widen the rect by the border thickness.
- The game still renders at the resolution of its settings and shrinks it, so FPS does not rise with a smaller
  window.

## Resolution list (`src/resize/resolution_list.rs`)

Question: why does a window set to 2560x1080 still render at 1920x1080, stretched?

- **Confirmed** (static analysis, builds 2.6.2.0 to 2.7.2.0): the windowed resolution list is not read from the
  monitor. `sub_140E9A9D0` (RVA `0xE9A9D0` on 2.7.2.0) fills it from the minimum entry (800x450) plus a fixed table of
  11 sizes, all 16:9, 1024x576 up to 3840x2160 (`.data`, RVA `0x3B50148`), and stops at the first size larger than the
  desktop. The size from `GraphicsConfig.xml` is then rounded down to the biggest entry that is not larger in either
  axis (`sub_140E9B190`), so 2560x1080 became 1920x1080. `SetWindowPos` does not change this: the game keeps rendering
  at its own size and DXGI stretches it into the client area.
- **Confirmed** (code): `with_size` replaces the entry with the closest area by `Width` x `Height` and sorts the table
  by width then height, which the descending scan needs. The table is found from the `lea rdi, [rip+table]` of the
  function above (AOB, unique in all four builds) and checked to be 11 sizes of 16:9 before anything is written.
- **Confirmed** (in game, 2.7.2.0, 2560x1600 screen): with `Width = 2560`, `Height = 1080` the game renders at that
  size, the picture is not stretched and the minimap is round. The mod that moves the HUD to the edges for 21:9
  depends on the game's own resolution, so this is what makes it usable on a screen that has no such mode.
- Limit: the table is patched once at start-up, before the game builds its list; a restart is needed after changing
  `Width` / `Height`.
