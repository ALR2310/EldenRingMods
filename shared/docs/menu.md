# In-game menu through ERNativeUI

Status: **Confirmed** in game on 2.7.1.0 (host 1.1) and 2.7.2.0 (host 1.3). Conventions:
[research notes](../../docs/research/README.md). Code: `shared/src/menu/`.

## Question

How can a mod offer settings inside the game without owning a native UI, and without breaking
for players who do not install the host?

## Decision

The mods use [ERNativeUI](https://github.com/Flammrock/ERNativeUI) (MIT) as an **optional**
dependency. Its `ERNativeUI.dll` hooks the game's options page builder and exposes a C ABI
(`erui.h`); each mod registers rows at start-up. Without the host a mod logs one line and runs
from its config file as before.

**Rejected:** an ImGui overlay through `hudhook`. It conflicts with other mods that hook the
same graphics calls, and it draws over the game instead of inside its menu. Building and
hooking the native pages ourselves duplicated what the host already does.

## Host facts

- Supported executables: 2.7.0.0, 2.7.1.0 and, from host 1.3.0, 2.7.2.0, matched by PE
  timestamp and image size (fail closed). 2.6.2.0 is not supported. Offline play only.
- **Confirmed:** from host 1.3.0 the menu works without loose GFX files. Earlier hosts needed
  patched `02_040_optionsetting.gfx` / `02_042_pc_graphicsetting.gfx`; without them submenu
  rows lost their left label and Game Options had only two free rows.
- A slider's value is **one byte** (0 to 255) kept in game memory. It cannot show decimals or
  negatives. Use a numeric slider (API 1.2) or a typed text row for those.
- Each row holds exactly one widget.
- **Registration is only accepted in the host's first seconds** (quiet for 750 ms, at most
  5 s). Connect and register at the very start of the mod's thread, or the host answers
  `ERUI_REGISTRATION_CLOSED`.
- Callbacks run on the host's worker thread (buttons: the UI thread): keep them short, never
  panic across the ABI, never block. Setting a row value from the mod does not call the
  callback again.
- `provider_id` is unique in the process (`[A-Za-z0-9_.-]`, 1 to 255 characters);
  `owner_module` is the mod's `HMODULE`.
- The host does not store values. The mod writes its own config.
- Paging is a page switch, not scrolling: the page capacity is a fixed number of row slots in
  the game's GFX. A mod keeps to one submenu row with all its settings inside.

## Client in `shared`

Low level (`host.rs`, `abi.rs`): `connect` (asks for API 1.2, falls back to 1.1),
`register_provider`, `add_submenu`, `add_toggle`, `add_text_input`, `add_number`, `commit`,
`set_row_value`, `set_text`. A failed `commit` drops the draft (`Drop` aborts it), because the
host would otherwise keep it.

Mod level (`builder.rs`, `settings.rs`):

- `static MENU: OnceLock<Menu>` and a `build(&mut MenuBuilder)` function that adds rows with
  `submenu`, `toggle` and `number`. Each row comes with a `read` function that reads its value
  from the mod's config, so `Menu::sync()` can show a reloaded config.
- `spawn_menu(&MENU, hmodule, id, name, build)` registers on its own thread.
- Saving: a `Setting::flag(key)` is the row's `user_data` together with `setting_changed`.
  `start_saver(write, on_saved)` starts one thread that sleeps until something changes,
  merges changes by key, waits for a second of quiet, calls `write` with the latest
  `(key, text)` pairs and then `on_saved`. With `TomlConfig` pass `save_values` and a
  `"Section.Key"` key.
- Numbers: `NumberSetting` with `MenuBuilder::number`. The player types `1.5` (a comma also
  works); the value is clamped to `[min, max]` and text that is not a number puts the previous
  value back. The value is written with a decimal point (`2.0`) so a TOML float stays a float,
  and shown rounded to six decimals so a value that went through `f32` reads `1.2`.
  `stepped(step)` makes the row a native numeric slider when the host has API 1.2 and the range
  has 2 to 256 positions that the step reaches exactly; `slider(from, to, step)` does the
  same for a range narrower than the accepted one. Otherwise the row stays a text row.
- Menu text comes from `shared::i18n` and one JSON file per language in the mod's `locales/`.

## Open risks

- API 1.3 of the host adds a dedicated tab; the client still asks for 1.2.
- Not re-measured with host 1.3: free rows on Game Options and the left labels without GFX.
