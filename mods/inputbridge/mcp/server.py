"""MCP server for the InputBridge mod: lets an agent play Elden Ring while the
game window sits in the background and the user keeps working.

Input goes through the mod's command file (`InputBridge.cmd` in the game's
`mod` folder, consumed by the mod within ~10 ms). Screenshots capture the game window
without focusing it (`PrintWindow`). Needs the `mcp` and `pillow` packages.

    python mods/inputbridge/mcp/server.py

Configuration: the game's `mod` folder (where InputBridge.dll lives) comes from the
`ER_MOD_DIR` environment variable, or from an `ER_MOD_DIR=` line in the repo's `.env`
(git-ignored, see `.env.example`). There is no default: it is specific to each machine.
"""

from __future__ import annotations

import base64
import ctypes
import functools
import io
import os
import sys
import time
from ctypes import wintypes
from pathlib import Path
from typing import Any

from mcp.server.mcpserver import MCPServer
from mcp.types import ImageContent, TextContent


def _mod_dir() -> Path | None:
    value = os.environ.get("ER_MOD_DIR", "")
    if not value:
        env_file = Path(__file__).resolve().parents[3] / ".env"
        if env_file.is_file():
            for line in env_file.read_text(encoding="utf-8").splitlines():
                key, _, raw = line.partition("=")
                if key.strip() == "ER_MOD_DIR":
                    value = raw.strip().strip('"').strip("'")
    return Path(value) if value else None


MOD_DIR = _mod_dir()
WINDOW_TITLE = "ELDEN RING"

# Key names the mod understands (mods/inputbridge/src/keys.rs).
KEYS = set("abcdefghijklmnopqrstuvwxyz0123456789") | {
    "esc", "escape", "tab", "enter", "return", "ctrl", "lctrl", "shift", "lshift",
    "alt", "lalt", "space", "up", "down", "left", "right",
    *(f"f{n}" for n in range(1, 13)),
}
BUTTONS = {"left", "right", "middle"}

server = MCPServer(
    name="eldenring-input",
    instructions="""\
Call notes() first: it holds what earlier sessions learned (key bindings, how to get from the
title screen into the world, timings). When the user corrects you or a test proves something
new, record it with add_note(); never leave a wrong note, fix it in the file instead.

Play Elden Ring in a background window. The user keeps using their computer; their keyboard
and mouse never reach the game, only the commands you send here do.

- Look first (screenshot), act, then look again. Frames are stale by the time you read them.
- Menus and the map fade in over 3-5 s: wait before the next screenshot.
- Loading a character takes 5-10 s after choosing Continue.
- Keys use the user's bindings: W/A/S/D move, Space jumps, M opens the map, E interacts and
  confirms. Camera and attacks use the mouse: camera(dx, dy, seconds), click("left").
- camera(6, 0, 1) turns about 25-30 degrees to the right. Positive dy looks down.
- release_all() if you lose track of what is held.
- Offline only: never use this with Easy Anti-Cheat running.
""",
)


def _send(*lines: str) -> str:
    """Hands command lines to the mod. Waits for the previous file to be consumed
    first, otherwise the rename would overwrite a command the mod has not read."""
    if MOD_DIR is None or not MOD_DIR.is_dir():
        return f"error: mod folder not found ({MOD_DIR}); set ER_MOD_DIR to the game's mod folder"
    cmd = MOD_DIR / "InputBridge.cmd"
    deadline = time.monotonic() + 2.0
    while cmd.exists() and time.monotonic() < deadline:
        time.sleep(0.01)
    tmp = MOD_DIR / "InputBridge.tmp"
    tmp.write_text("\n".join(lines) + "\n", encoding="utf-8")
    os.replace(tmp, cmd)
    return "ok: " + "; ".join(lines)


def _key(name: str) -> str:
    key = name.strip().lower()
    if key not in KEYS:
        raise ValueError(f"unknown key '{name}'")
    return key


def _button(name: str) -> str:
    button = name.strip().lower()
    if button not in BUTTONS:
        raise ValueError(f"unknown button '{name}' (left, right, middle)")
    return button


def _guard(fn):
    @functools.wraps(fn)
    def run(*args, **kwargs) -> str:
        try:
            return fn(*args, **kwargs)
        except ValueError as exc:
            return f"error: {exc}"
    return run


@server.tool(description="Tap a key for a while (seconds, default 0.1). Waits for it to be released.")
@_guard
def press(key: str, seconds: float = 0.1) -> str:
    seconds = max(0.02, min(seconds, 30.0))
    result = _send(f"press {_key(key)} {seconds:g}")
    time.sleep(seconds)
    return result


@server.tool(description="Hold a key down until release(key) or release_all(). Use for walking or blocking.")
@_guard
def hold(key: str) -> str:
    return _send(f"down {_key(key)}")


@server.tool(description="Release a key held with hold().")
@_guard
def release(key: str) -> str:
    return _send(f"up {_key(key)}")


@server.tool(description=(
    "Turn the camera by moving the mouse every frame for a while: dx, dy are mouse counts per "
    "frame (about 60 frames per second). camera(6, 0, 1) turns about 25-30 degrees right."))
@_guard
def camera(dx: int, dy: int = 0, seconds: float = 0.5) -> str:
    seconds = max(0.05, min(seconds, 10.0))
    result = _send(f"look {int(dx)} {int(dy)} {seconds:g}")
    time.sleep(seconds)
    return result


@server.tool(description="Move the mouse once by dx, dy counts (menus, fine camera adjustments).")
@_guard
def mouse_move(dx: int, dy: int) -> str:
    return _send(f"mouse {int(dx)} {int(dy)}")


@server.tool(description="Press a mouse button for a while (seconds, default 0.1): left attacks, right guards.")
@_guard
def click(button: str = "left", seconds: float = 0.1) -> str:
    seconds = max(0.02, min(seconds, 10.0))
    result = _send(f"mclick {_button(button)} {seconds:g}")
    time.sleep(seconds)
    return result


@server.tool(description="Hold a mouse button down until mouse_release() or release_all().")
@_guard
def mouse_hold(button: str = "left") -> str:
    return _send(f"mdown {_button(button)}")


@server.tool(description="Release a mouse button held with mouse_hold().")
@_guard
def mouse_release(button: str = "left") -> str:
    return _send(f"mup {_button(button)}")


@server.tool(description="Scroll the mouse wheel by n notches (positive up).")
@_guard
def wheel(notches: int) -> str:
    return _send(f"wheel {int(notches)}")


@server.tool(description="Release every key and mouse button and stop camera movement.")
@_guard
def release_all() -> str:
    return _send("release_all")


NOTES_FILE = Path(__file__).resolve().parents[1] / "docs" / "game_notes.md"
NOTES_TITLE = "# Ghi chú cho agent khi điều khiển Elden Ring"


@server.tool(description=(
    "Read the shared notes about controlling Elden Ring (key bindings, how to reach the world, "
    "timings). Call this first in every session."))
def notes() -> str:
    if not NOTES_FILE.is_file():
        return f"no notes yet (they will be created at {NOTES_FILE})"
    return f"[{NOTES_FILE}]\n\n" + NOTES_FILE.read_text(encoding="utf-8")


@server.tool(description=(
    "Record a fact in the shared notes under a topic heading (Phím, Quy trình, Thời gian, ...). "
    "source is 'user' when the user told you, 'observed' when you saw it yourself. Use this when "
    "the user corrects you or a test proves something new. A wrong note must be fixed by editing "
    "the file at the path notes() shows, not by adding a contradicting line."))
def add_note(topic: str, note: str, source: str = "observed") -> str:
    source = source.strip().lower()
    topic, note = topic.strip(), " ".join(note.split())
    if source not in ("user", "observed"):
        return "error: source must be 'user' or 'observed'"
    if not topic or not note:
        return "error: topic and note must not be empty"
    NOTES_FILE.parent.mkdir(parents=True, exist_ok=True)
    text = NOTES_FILE.read_text(encoding="utf-8") if NOTES_FILE.is_file() else NOTES_TITLE
    line = f"- {time.strftime('%Y-%m-%d')} [{source}] {note}"
    lines = text.splitlines()
    if line in lines:
        return "already noted"
    heading = f"## {topic}"
    if heading in lines:
        at = lines.index(heading) + 1
        while at < len(lines) and not lines[at].startswith("## "):
            at += 1
        while at > 0 and lines[at - 1].strip() == "":
            at -= 1
        lines.insert(at, line)
    else:
        lines += ["", heading, "", line]
    NOTES_FILE.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return f"noted under '{topic}': {line}"


@server.tool(description="Wait for the game to react (menus and the map fade in over 3-5 s).")
def wait(seconds: float = 3.0) -> str:
    seconds = max(0.0, min(seconds, 60.0))
    time.sleep(seconds)
    return f"waited {seconds:g} s"


def _find_window() -> tuple[int, str] | None:
    """The biggest visible top-level window whose title contains WINDOW_TITLE."""
    user32 = ctypes.windll.user32
    found: list[tuple[int, int, str]] = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def visit(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        buf = ctypes.create_unicode_buffer(256)
        user32.GetWindowTextW(hwnd, buf, 256)
        if WINDOW_TITLE.lower() in buf.value.lower():
            rect = wintypes.RECT()
            user32.GetClientRect(hwnd, ctypes.byref(rect))
            found.append((rect.right * rect.bottom, hwnd, buf.value))
        return True

    user32.EnumWindows(visit, 0)
    if not found:
        return None
    _, hwnd, title = max(found)
    return hwnd, title


def _capture_client(hwnd: int):
    """Client area of a window as a Pillow image, without focusing it. None on failure."""
    from PIL import Image

    user32, gdi32 = ctypes.windll.user32, ctypes.windll.gdi32
    rect = wintypes.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(rect))
    width, height = rect.right, rect.bottom
    if width <= 0 or height <= 0:
        return None

    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [("biSize", wintypes.DWORD), ("biWidth", wintypes.LONG),
                    ("biHeight", wintypes.LONG), ("biPlanes", wintypes.WORD),
                    ("biBitCount", wintypes.WORD), ("biCompression", wintypes.DWORD),
                    ("biSizeImage", wintypes.DWORD), ("biXPelsPerMeter", wintypes.LONG),
                    ("biYPelsPerMeter", wintypes.LONG), ("biClrUsed", wintypes.DWORD),
                    ("biClrImportant", wintypes.DWORD)]

    for fn in (gdi32.CreateCompatibleDC, gdi32.CreateCompatibleBitmap, gdi32.SelectObject):
        fn.restype = ctypes.c_void_p
    gdi32.CreateCompatibleDC.argtypes = [ctypes.c_void_p]
    gdi32.CreateCompatibleBitmap.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
    gdi32.SelectObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    gdi32.DeleteObject.argtypes = [ctypes.c_void_p]
    gdi32.DeleteDC.argtypes = [ctypes.c_void_p]
    gdi32.GetDIBits.argtypes = [ctypes.c_void_p, ctypes.c_void_p, wintypes.UINT, wintypes.UINT,
                                ctypes.c_void_p, ctypes.c_void_p, wintypes.UINT]
    user32.GetDC.restype = ctypes.c_void_p
    user32.GetDC.argtypes = [wintypes.HWND]
    user32.ReleaseDC.argtypes = [wintypes.HWND, ctypes.c_void_p]
    user32.PrintWindow.argtypes = [wintypes.HWND, ctypes.c_void_p, wintypes.UINT]

    screen = user32.GetDC(None)
    mem = gdi32.CreateCompatibleDC(screen)
    bitmap = gdi32.CreateCompatibleBitmap(screen, width, height)
    old = gdi32.SelectObject(mem, bitmap)
    try:
        # PW_CLIENTONLY | PW_RENDERFULLCONTENT: client area, including DirectX content.
        if not user32.PrintWindow(hwnd, mem, 3):
            return None
        header = BITMAPINFOHEADER(ctypes.sizeof(BITMAPINFOHEADER), width, -height, 1, 32, 0)
        pixels = ctypes.create_string_buffer(width * height * 4)
        if not gdi32.GetDIBits(mem, bitmap, 0, height, pixels, ctypes.byref(header), 0):
            return None
        return Image.frombuffer("RGBA", (width, height), pixels, "raw", "BGRA", 0, 1).convert("RGB")
    finally:
        gdi32.SelectObject(mem, old)
        gdi32.DeleteObject(bitmap)
        gdi32.DeleteDC(mem)
        user32.ReleaseDC(None, screen)


@server.tool(description=(
    "Capture the Elden Ring window without focusing it. Returns the frame as a JPEG. "
    "The game window may be small; max_width limits the returned size."))
def screenshot(max_width: int = 640) -> list[Any]:
    if sys.platform != "win32":
        return [TextContent(type="text", text="error: Windows only")]
    ctypes.windll.user32.SetProcessDPIAware()  # real pixels, not scaled ones
    window = _find_window()
    if window is None:
        return [TextContent(type="text", text="error: no Elden Ring window")]
    hwnd, title = window
    if ctypes.windll.user32.IsIconic(hwnd):
        return [TextContent(type="text", text="error: the game window is minimised")]
    image = _capture_client(hwnd)
    if image is None:
        return [TextContent(type="text", text="error: capture failed")]
    width, height = image.size
    if image.getextrema() == ((0, 0), (0, 0), (0, 0)):
        return [TextContent(type="text", text=f"error: the frame is black ({width}x{height})")]
    if width > max_width > 0:
        image = image.resize((max_width, max(1, height * max_width // width)))
    out = io.BytesIO()
    image.save(out, "JPEG", quality=70)
    info = (f"Window: {title} | client {width}x{height} | frame {image.size[0]}x{image.size[1]} | "
            f"{time.strftime('%H:%M:%S')}")
    return [TextContent(type="text", text=info),
            ImageContent(type="image", data=base64.b64encode(out.getvalue()).decode(),
                         mime_type="image/jpeg")]


def main() -> None:
    server.run(transport="stdio")


if __name__ == "__main__":
    main()
