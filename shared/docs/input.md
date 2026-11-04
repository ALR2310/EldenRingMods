# Hotkeys

Status: **Confirmed** in released mods of the old repository; the key parsing is covered by unit tests. Not yet
re-tested in game from this one. Conventions: [research notes](../../docs/research/README.md). Code:
`shared/src/input.rs`.

## Question

How does a mod turn a key name from its config into a virtual-key code, and poll a key without stealing
presses from other windows or other game instances?

## Key names (`parse_virtual_key(name, fallback)`)

Accepted, case-insensitively: a hex code (`"0x2D"`), a decimal code (`"112"`), `"F1"` to `"F24"` (`0x70` to
`0x87`), or a single letter or digit (their VK code equals the ASCII value, so `"F"` is the letter, not a
function key). A code must be in `1..=0xFF`. Anything else returns `fallback`, so a typo in the config never
disables a hotkey silently with a wrong key. TOML fields that hold a key use
`toml_config::key_name`, which also accepts a number.

## Polling (`is_key_pressed(vk)`)

`GetAsyncKeyState` keeps a "pressed since the last call" bit that is **one flag for the whole system**: with two
game instances the other window lost its presses. So the state is kept per key in this process:

- It never polls while another window is in the foreground (the foreground window's process id must be this
  process), so it cannot consume that window's press. Losing focus resets the key to "unknown".
- On regaining focus the first poll only records the key state, so a press made elsewhere does not fire.
- It fires on the "pressed since last call" bit (catches short taps) or an up-to-down transition, and when the
  key was released and pressed again between two polls.
- Only one caller should poll a given key, because the state is per key. `reload::run` is the only poller of
  the reload key.

Poll it periodically (the mods do it on `FrameBegin`).
