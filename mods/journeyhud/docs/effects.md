# Active effects (SpEffect) with their time left

Status: **Confirmed** in game on 2.7.2.0 and with the Convergence pack: the enemy effect row under the buildup
bars, the time texts under the game's own state icons, and the moved and scaled player buildup gauge. Not
checked: the effect counter on 2.6.2.0 to 2.7.1.0, and the vertical order of the 7 gauge clips. Conventions:
[research notes](../../../docs/research/README.md). Code: `target.rs`, `state_icons.rs`, `ui.rs::draw_effects`,
`ui.rs::draw_icon_times`, `map/sprites.rs`.

## Question

Show what effects an enemy or the player has and how long they last, without drawing a second copy of the
game's own state icons.

## Where the data is

fromsoftware-rs types the list (`cs/sp_effect.rs`), so no raw offsets are needed except one:

| Field | Offset | Meaning |
|---|---|---|
| `ChrIns.special_effect` | +0x178 | pointer to `SpecialEffect` |
| `SpecialEffect.head` | +0x08 | head of the linked list (`entries()` walks it) |
| `SpecialEffectEntry.param_data` / `param_id` | +0x00 / +0x08 | the `SpEffectParam` row and the effect id |
| next entry (`next`) | +0x30 | list link |
| previous tick's counter (`removal_timer` in the crate) | +0x40 | value of the last tick |
| **real time left** (`unk_removal_timer` in the crate) | **+0x44** | seconds left, decreased every frame |
| `duration` | +0x48 | total time |
| `interval_timer` | +0x4C | cycle counter (poison, bleed pulses) |
| entry size | 0x78 | |

`SpEffectParam` columns used to filter: `iconId` (state icon; 0 = none), `effectEndurance` (duration, -1 =
permanent), `enableLifeTime`, `stateInfo`, `dispIconNonactive`.

**Checked in IDA (2.7.2.0 for the counters)**:

- The destructor of `CS::SpecialEffect` is the same on 2.6.2.0 (4F327E), 2.7.0.0 and 2.7.1.0 (4F404E), 2.7.2.0
  (4F4110): list head at `a1[1]` (+0x08), each entry links to the next at `*(entry + 48)` (+0x30).
- The entry initialiser `sub_1404FFEC0` sets +0x40, +0x44 and +0x48 to the row's `effectEndurance` times a
  factor; the per-frame update `sub_140501ED0` **decreases +0x44** (clamped around 0) and copies the previous
  tick's value to +0x40. So the real time left is **+0x44** (read raw, with a `const` assert). The decrement
  factor can change with the flags `0x20` at +356 and +364 of the param row, so the seconds are the game's
  counter, not wall-clock time.
- `SpEffectParam.iconId` maps to the sprite `MENU_StatusIcon_<iconId>` (40x40) in `SB_Status_00` of
  `menu/hi/01_common`, read through `map/sprites.rs`. In the vanilla table: 11354 rows, 695 with `iconId`, 150
  different icons, 332 with `effectEndurance > 0`. `stateInfo` is nearly useless for filtering icons (499 of the
  695 rows have 0), so filter by `iconId > 0` and a time left above 0.

## What a log showed (player, Golden Vow and a pickled liver)

- +0x44 counts down for real: `20501140 icon 20408 120s` then `67s` after 53 seconds; `1660000 icon 20406 80s`
  then `13s` after 68 seconds; the effect disappears when it runs out.
- Side rows `1660001` and `1660002` (Golden Vow "Cycled", `effectEndurance` 0.1) have icons but always 0 s:
  dropped by the filter `duration > 1` (field +0x48).
- Two different effects can share an icon (`20501140` and `1660002` both 20408, the generic buff icon); the game
  merges them by icon.
- `505` (Poison/Scarlet Rot (Cycled), 900 s, `stateInfo 275`) appears with a poison or rot effect: an internal
  marker, ignored.

## Enemies

- Enemies rarely carry an effect with an icon (from `NpcParam.spEffectID0..31` with `iconId > 0`, vanilla): only
  8 effects. Six permanent: 17354, 17131, 18917 (icon 20286), 13865 (icon 20261), 11720, 10695 (icon 20285); two
  60-second ones in the DLC (20502025, 20502015, icon 20406).
- The effects of a **buildup that has popped** have a time but no icon (`iconId = -1`): `6511 30/30s` (Poison -
  Type 2 - Special 2), `6600 90/90s` (Scarlet Rot - Type 1 - Special 1), `6702 30/30s` (Frostbite - Type 1 -
  Special 3), counting down (`6600 85/90s`, `68/90s`). They are drawn with the `MENU_RE_*` icon of the buildup bar.
- **Recognising them by `stateInfo`, not by id range.** The id ranges 6500-6705 are only weapon rows and a few
  sources; the rot the player gets in the swamps is `4051`, `4053`, `4055` (180 s) and enemy hits use
  `21000-21102` (180 s) and `107000-107275` (90 s). The `stateInfo` column gives the kind whatever the row
  (counts in vanilla): **2 poison** (309 rows), **5 scarlet rot** (227), **6 blood loss** (284), **116 death
  blight** (210), **260 frostbite** (276), **436 sleep** (224), **437 madness** (65). Rows that cure a status have
  other values (10, 11, 12, 13, 118, 276...) and 275 is the game's internal marker (`505`).
  `target::proc_kind` maps `stateInfo` to a `BUILDUP` index only for effects without an icon and with a time
  (`duration > 1`; permanent ones are dropped). Blood loss, madness and death blight are instant (1 s or 0) and
  are not shown.
- A Black Blade Kindred has only "NPC: Disable ..." immunity effects (`iconId = -1`) and AI/scale effects, so no
  cell: as designed. An "immune" row could be added later.

## Layout

- Each effect is a horizontal cell: icon, the time right under it (whole seconds only, so the longest text is 4
  characters: `24s`, `9s`, `1:30` from 60 s; a permanent effect has the icon only). A row of cells instead of a
  vertical list: eight effects take one row instead of 240 px.
- **Locked enemy**: under the buildup rows in the right column, **right-aligned**. Icon 27 px, cell 30 px, text
  10 px (the enemy's icons are 30% smaller than the game's); a row fits `floor(width / cell)` cells, at most 6,
  at most 2 rows (so at most 12). When there are more, those with the most time left go (permanent first), in the
  game's order. Key `Target.Effects`. A log line `Effects: enemy|player ...` is written when the set changes.
- **Player**: no second row of icons. The game's row `PlayerHUD/Gauge/Equip/Fade/Item_0_0..Item_0_41` is
  **42 cells in rows of 14** (cell i at x = 10 + 42 * (i mod 14), y = 15 + 41 * (i div 14) in 1080p units), each
  only an `Icon` clip, no text field. `ui::draw_icon_times` draws the time text just under each icon of the game
  (slot k centred at x = 155.5 + 42 * k, y 142, measured on a screenshot): for the icon of a slot it takes the
  effect with that icon that has the **most time left**; a permanent one has no text. Key `Player.EffectTimer`. Limit:
  the position follows the default HUD layout (a different HUD size in the game options shifts the text); not
  tried.
- **Fading with the game's row**: the game hides the row (and the HP/FP bars) outside fights; the update function
  `sub_1408D58F0` passes the flag at `model + 2730` (the byte after the 42 flicker flags) to
  `sub_14076B850(view + 136, flag, ...)`, which plays `FadeIn` for non-zero and `FadeOut` for 0.
  `state_icons::shown()` reads it; `ui.rs` keeps `icons_alpha` (the panel's `FADE_RATE`) and multiplies the text.
  Confirmed in game.
- Sizes measured on a 2560x1600 screenshot (the game's frame is 16:9, 2560x1440, so 1 px at 1080p = 1.333 px):
  the game's state icons are about 38 px wide, 42 px apart, from x 137, y 105 to 138.

## The game's state icon row (decoded)

- Class `CS::PlayerStateIconViewParts` (vtable RVA 0x2AEFD18 on 2.7.2.0). The constructor `sub_1408D5470`
  gathers the cells `Fade/Item_0_%d` into a vector (begin `+112`, end `+120`, cells of 304 bytes, the `Icon` clip
  at `+120` of a cell). The per-frame update **`sub_1408D58F0(view, a2, model)`** is called from `sub_1408CA1C0`
  with `view = hud + 126608` and `model = *(hud + 96) + 14344`.
- `model` (2736 bytes): 42 slots of 64 bytes at `model + 64 * i`, a flicker flag per slot at
  `model + 2688 + i`, a common flag at `model + 2730`. The icon id is the **last dword (byte +60)** of a slot
  (`4FB6` = 20406, `4FB7` = 20407, `4FB8` = 20408); an empty slot has `FFFFFFFF` there. Slots are filled
  contiguously from 0 in the order of the SpEffect list; when an effect expires the later slots move up.
  Effects with the same icon take one slot (the generic buff icon of two effects shows once), and the "Cycled"
  side rows still get a slot.
- `state_icons.rs` redirects the call to `sub_1408D58F0` (a pattern unique on 2.6.2.0 to 2.7.2.0) to keep the
  `model` pointer.

## The player's buildup gauge

The gauge with an icon at the bottom centre is `ConditionGaugeList/Item_*` of the HUD movie, one clip per
status (same order as `BUILDUP`); it shows both the built-up part and the remaining time of a popped status.

- **Update**: `sub_1408D2AE0(view, index, model)` is called per status in `sub_1408CA1C0` (view
  `hud + 80848 + 1144 * i`, `model = *(hud + 96) + 9888 + 104 * i`). It plays `FadeIn`/`FadeOut` from the byte
  `model[65]` and uses the rest for the fill. The call pattern is unique on 2.6.2.0 to 2.7.2.0.
- **Moved, not hidden.** Hiding it (and drawing a cell for a popped status in the icon row) left the cell
  floating when the game hid the row, so the original gauge is **moved to the right column, under the enemy's
  effect row** (up to just under the clock when there is no target). `gauge_update` calls the original, then
  sets `_x/_y` of the gauge's clip (`sub_140D872C0`, the same pattern as `HubHpBars`; the clip comes from slot 1
  of the vtable of the component at `view + 1032`); position of clip k = reported origin + 48.85 * k (stacked
  top-down). It is set only when the position changes or the game rebuilds the view. `ui.rs` reports every frame
  (`anchor_gauges`) the column's right edge and the top of the first gauge, in 1080p points.
- **Scaled** with `sub_140D87530(value, [x, y])` (sets `_xscale`/`_yscale` to x, y percent; unique pattern on
  2.6.2.0 to 2.7.2.0): scale = column width / 334, the same width as the mod's health bar (default 220 px: 0.66,
  icon about 28 px, bar about 192 x 9 px), following `Target.Width` and `Target.Scale`. The original size
  (icon 42 px, bar 292 x 14 px, about 334 px in all) looked out of place next to the mod's HUD.
- **Where it ends up (final)**: top left, its left edge on the left edge of the game's icon row
  (`GAME_ICONS_CENTER_X - GAME_ICONS_PITCH / 2`) and its top under the time texts
  (`GAME_ICONS_BOTTOM + text size * 1.5 + 6`); separate from the target panel because the player's gauge next to
  an enemy's buildup bars was easy to confuse. Size `Player.Scale` (1 = the game's size, 334 px wide at 1080p).
  Key `Player.Buildup`.
- **Layouts**: the game's own: `ConditionGaugeList` at (960, 520) of the 1080p frame, item k at (-150, 197.25 -
  48.85 * k) relative to it (stacked bottom-up); icon 42 px wide, 26 px left of the origin, bar ending 308 px
  right of it. The **Convergence** pack replaces `01_000_fe.gfx`: `ConditionGaugeList` at (557.5, 222.45), items
  at x = 1260.15, y = -96.05 + 34.5 * k with scale (-1, -1) (really a 180 degree rotation: the game reports
  rotation 180 and scale +100), mirroring the gauges to the top right; the `Item` clip is the same, only its
  `Icon` child differs (position (-14.65, -0.2), scale -0.4375 instead of (-20, 0) and 0.625, so the icon is 0.7
  times smaller). The mod does not read the root: the first time it sees a view, `place_gauge` reads the clip's
  place with `GetDisplayInfo` (vtable +216 of the interface object; struct x, y, rotation, xscale, yscale at +0,
  +8, +0x10, +0x18, +0x20 as `f64`, flags at +0xD4) and compares x with `LAYOUTS`: -150 is the game's (root (960,
  520)), 1260.15 is Convergence (root (557.5, 222.45)). Target = the 1080p target minus the root; then it sets
  `_x/_y`, the scale and rotation 0 (rotation through slot +224 with flag 0x04, reading the struct with
  `GetDisplayInfo` first). An unknown layout logs `Gauge: unknown layout` and is left alone.
- **With `Player.Buildup` off** the mod writes nothing, and puts the clip back to the saved position, rotation
  and scale (`CLIP_BEFORE`) if it had moved it; it never touches the game's or another pack's position.
- Measured: the bar is 291 px long in the game's layout and 296 px in Convergence, 16 px high in both; icons 42
  px and 29 px (the length differs by the interface file, the icon by the `Icon` clip).
- **Not done**: setting the `Icon` child on its own (needs `GetMember` or the icon component at `view + 0`) so
  the Convergence icon is 42 px.
- Assumed, not checked: the order of the 7 clips equals `BUILDUP`. The game plays the clips for the active
  statuses in turn, like the icon row, so a clip index does not identify a status: the first clip's frame
  (`model[0]` = the `Icon` clip's frame, 1 = poison, 2 = rot...) does.

## Open risks

- The counter at +0x44 on 2.6.2.0, 2.7.0.0 and 2.7.1.0 (only 2.7.2.0 was checked).
- Permanent effects (`effectEndurance` = -1): currently filtered out for enemies; they are shown without a time
  for the player.
- Under Seamless Co-op the time left of an effect on an enemy may be right only for what the local machine knows,
  as with buildup ([buildup.md](buildup.md)).
