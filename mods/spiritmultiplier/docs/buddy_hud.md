# Spirit HP bars on the HUD (`HubHpBars`)

Status: **Confirmed** in game on 2.7.2.0: 10 and 30 bars with many spirits, no crash, the raised panel sits
between the status frame and the skill frame (2560x1600). The panel places itself at run time for the
number of bars in use: 5 spirits stay where vanilla puts them, more grow upward. Older executables were
only checked on code (see below). Conventions: [research notes](../../../docs/research/README.md).
Code: `src/buddy/hud.rs`; Scaleform helpers: [gfx](../../../shared/docs/gfx.md).

## Question

The game shows 5 spirit HP bars at most. How can it show up to 30?

## Why the game has 5

Three places hard-code 5, so widening one is not enough:

- **Data.** The per-frame routine `sub_140771ED0` walks the summoned spirits, runs 3 checks on each (not
  dead, `sub_1404B7CE0`, not a Blob Mimic `sub_1404B7A10`) and writes them into **5 slots** at
  `a1+19936`, 24 bytes each: chr handle (8), HP (4), padding (4), lost max HP (4), max HP (4). An empty
  slot is `-1`. The check `if (v3 < 5)` only guards the array write; the counter keeps counting.
- **Display loop.** `sub_1408C2600` (updates `CS::BuddyViewParts`) gets the block `a3 = a1+19800` (136
  byte header, then the slots) and loops over **exactly 5 slots**. Each is bound to an HP bar component
  (key `+200` = handle, empty = `-1`).
- **Row list of the view.** The view constructor `sub_1408C1D20` collects `Anim/GaugeList/Item_%d` with a
  lambda that appends rows to a `DLFixedVector` of **fixed capacity 5** inside the view object
  (`+896`, count at `+944`) and skips the sixth row on. It cannot grow in place (the next member is at
  `+952`).

A trap: when the vector is full the lambda calls `sub_140734640(row, 0)`, which sets **`visible = false`
on the row's clip**. Vanilla needs that to hide spare rows; the mod's hook must not call it for rows
another view owns.

## Movie structure (`menu/01_000_fe.gfx`)

```text
Buddy (root, depth 447, 18 frames)
`- Anim
   |- Base, ItemIcon, StandbyIcon, ItemName
   `- GaugeList -> Item_4 (y 0), Item_3 (-300 twips), ... Item_0 (-1200); each is Timeline_178 with an HP clip
```

Only `Item_0..Item_4`, 300 twips (15 px) apart, `Item_4` lowest. The movie has timelines only; all logic
is in the executable.

## Mechanism

Each view keeps 5 rows, so extra views are built on the **same `Buddy` clip** (no second panel):

1. **Movie** (`gfx::hook`, `gfx`): adds `Item_5..Item_{N-1}` to `GaugeList` with the same character and
   scale, spaced like the last two rows.
2. **`hud_blob`**: redirects the `call sub_1404B7A10` inside `sub_140771ED0` (also where `InstantMimic`
   drops the Blob). That function is called only for spirits that passed every other check, so it is the
   exact list of spirits that get a slot; they are recorded in order.
3. **`hud_build`**: redirects the `call sub_140771ED0` in `sub_140772F00`; runs the original (first 5
   slots), then builds the slots after the fifth from the list of step 2 (handle `chr+8`, `data.hp`,
   `max_hp`, `max_uncapped_hp`).
4. **`hud_ctor`**: redirects the `call sub_1408C1D20` in the HUD constructor `sub_1408C6C80`; builds the
   game's view, then `ceil(N/5) - 1` extra views with the same function, each allocated separately
   (0x1000 bytes, never freed).
5. **`hud_item`**: redirects the `jmp` of the lambda wrapper (`add rcx,8; jmp sub_1408C24C0`, found with
   `memscan::find_jumps_to`); view k accepts only rows `5k..5k+4`, others are skipped **without** the
   hide-clip call.
6. **`hud_view`**: redirects the `call sub_1408C2600` in `sub_1408CA1C0`; after the game's view, updates
   each extra view with the game's header and its own 5 slots.
7. **Panel position** (in `hud_view`): `Anim`, the clip with background, bars, icon and name, sits at
   (0, 0) inside `Buddy`. Each frame the mod computes the rows in use (`min(spirits with a slot, N)`,
   at least 5) and, when it changes, moves `Anim` up by `(rows - 5) * 15 px`: 5 bars or fewer stay in the
   vanilla place, more grow upward into the free space. This copies the game's own menu code: the clip
   proxy of the view's `Anim` component (view+776, proxy at +8) has a vtable slot 1 returning a `Value`,
   then `sub_140D872C0(value, x, y)` sets `_x` and `_y`. When the HUD is rebuilt the applied position is
   reset to 0.

Install order is view, row, list, constructor, so a failure midway leaves 5 bars. The layout (header 136,
5 slots of 24) is checked with `LAYOUT_AOB` before installing; on a mismatch the feature stays off. Every
redirect is a `call` or `jmp rel32`, so another mod hooking the same place can chain; no game bytes are
overwritten.

## Patterns (unique on 2.6.2 to 2.7.2)

| Site | Pattern (shortened) | Position |
|---|---|---|
| Blob check | `84 C0 74 ?? 48 8B D6 49 8B CE E8 ... 83 FF 05` | `call` at +10 |
| List builder call | `E8 .. 0F 28 CE 48 8B CB E8 .. 0F 28 CE 48 8B CB E8 .. 4C 8B C7` | `call` at +11 |
| View update call | `48 81 C2 .. 00 00 48 8D 8E .. E8 .. 4C 8B 46 60 49 81 C0 .. 00 00 48 8D 8E .. 0F 28 CE E8` | `call` at +40 |
| View constructor call | `4C 8D 05 .. 48 8D 54 24 .. 49 8B CE E8 .. 90 49 8D 8E .. 48 8B D0 E8` | `call` at +31 |
| Row-append function | `40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF ... 48 83 B8 B0 03 00 00 05 0F 83` | function start; the wrapper is a `jmp` into it, preceded by `48 83 C1 08` |
| Slot layout | `49 8D BF 88 00 00 00 41 BC FF FF FF FF 48 8D 77 78 48 3B FE` | check only |
| Clip `_x/_y` setter | `48 8B C4 48 89 58 10 55 48 8D 68 C8 48 81 EC 30 01 00 00 ...` | function start (`sub_140D872C0`) |

The view-update pattern must keep the preceding context (`mov r8,[rsi+60h]`): the short form matches 7
places.

## Check on older executables

`scripts/ida/run.ps1 buddy_hud_check -Version all` (re-run for this port) gave 35 `RESULT OK` and no `FAIL`
on each of **2.6.2.0, 2.7.0.0, 2.7.1.0 and 2.7.2.0**: all 8 patterns are unique (the 7 above plus the clip
`_x/_y` setter) and the function relations hold. The Blob call lies inside the function the list-builder call
targets; the view-update function contains the slot window (136 + 5 x 24 bytes); the view constructor
references `Anim/GaugeList`, has a vtable whose slot 2 is the `add rcx,8; jmp` wrapper of the row-append
function (exactly one) and places the `Anim` component at `+0x308`; the clip setter is unique with 19 callers
on 2.7.2.0; the file-opener function starts exactly 16 bytes before its pattern and has 3 pointer cells in
data. 2.7.0.0 and 2.7.1.0 share addresses and differ only in the file opener. Examples on 2.7.2.0 (RVA): Blob
call site `7722BE`, view-update call `8CACC6`, row-append function `8C24C0` with its wrapper `jmp` at `8C29D7`,
clip setter `D872C0`, file opener `D6E970`. This is a check on code, **not a run** on those builds.

## Limits and risks

- At most 30 bars (6 views); raising `MAX_BARS` is enough to add views. The practical limit is screen
  height: with N bars the panel rises `(N - 5) * 15 px` (375 px for 30), still between the status and
  skill frames; beyond that it would touch the status frame.
- Extra views are never freed (a few KB leak if the game rebuilds the HUD) and hold the movie reference
  until exit.
- The header (name, icon, background) is written by every view with the same values, so it looks the
  same as with one view.
- Offsets (`136`, `24`) and the view size (0x430) were measured on 2.7.2.0; `LAYOUT_AOB` rejects a
  different layout, the view size only has the 0x1000 safety margin.

## Failures worth remembering

- The `gfx::hook` AOB starting at the prologue did not match once another mod had hooked the start of
  the file-opener (log `file opener pattern not found`): see [gfx](../../../shared/docs/gfx.md).
- Scanning `find_data_pointers` by section size crashed on a guard page: see
  [memscan](../../../shared/docs/memscan.md).
- Calling the "vector full" exit for a row owned by another view hid the first 5 bars (clip hidden): see
  above.
