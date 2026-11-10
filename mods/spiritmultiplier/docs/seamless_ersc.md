# Seamless Co-op: what `ersc.dll` does to the ghost colour of spirits

Status: **Confirmed** for the structure of `ersc.dll` and the hook chain, **Hypothesis** for the cause of the
lost ghost colour. Seen with Seamless Co-op 2.0.1 (SHA-256 `fcd11a18...`) and a build of the 27th of
October 2026 (`0a093d84...`), on the game build [open_issues.md](open_issues.md) uses (2.7.1.0).
Conventions: [research notes](../../../docs/research/README.md).

## Question

Why do spirits lose their ghost tint when Seamless Co-op is loaded, and how much of `ersc.dll` can be read
to find out?

## How readable `ersc.dll` is

- The protector is Themida 3.x: a section `ERSC` of about 11 MB (RWX) holds the entry point.
  `unlicense` cannot dump 2.0.1: it detects the wrong version, and forcing `--target_version=3` does not
  reach the original entry point within 60 or 240 seconds.
- A dump of the loaded module once in the world shows `.text` identical to the file on disk and `ERSC`
  99.94% identical: nothing is decrypted at run time. The real code is already in the file; the dump only
  adds the real base (`0x180000000`) and the initialised `.data`.
- On the 27 October build (IDA 9.3, auto analysis): 10112 functions, 4868 in `.text` and 5244 in `ERSC`;
  the first 300 `.text` functions all decompile. Only 21 `.text` functions call or jump into `ERSC` (counted
  by IDA's cross references; scanning for raw `E8`/`E9` gives 119, mostly false positives). Those are the
  mutated or virtualised functions, the only part that cannot be read directly.
- Seamless does not keep its hook signatures as plain bytes: the first 24 bytes of `sub_1403F0C20`, and
  every 9 to 16 byte piece of them, appear in no file.

## The hook chain on the game side

Read from the running game (the game's memory dumped with `--peek 3F0C20`):

1. `E9` at the start of `sub_1403F0C20` (the PhantomParam of a ChrIns) and `sub_1403F1C90` (get team).
2. A stub allocated just below the image of `eldenring.exe` (`jmp [rip+0]`).
3. A thunk `mov rax,imm64; movq xmm5,rax; mov r11,<target>; jmp r11`.
4. A target in `ersc.dll` (`0x180095B20`, `0x180095E60`), both in `.text`.

The targets only call a `std::function`-like callback: the context travels in `xmm5`, the callback is an
object with a vtable in `.rdata`, and its slot 2 is a thunk `fn(*(obj+8), args)` whose `obj+16` is the real
function. The real functions are known only at run time (heap): `ersc+0x83260` replaces `sub_1403F0C20`
(ghost colour) and `ersc+0x8E600` replaces `sub_1403F1C90` (get team). Both sit in `.text` and decompile.

## What the replacement of the ghost colour does

`sub_180083260`:

- With its flag on, it picks a **PhantomParam row `8380001..8380005`** from the player's slot (table
  `0x1801DD880`), by default `8380000` (`8380006` in one other case), and looks the row up with
  `sub_1800A95C0`.
- For a spirit (group at `WorldChrMan+69528`) it first finds the owner with `sub_180028700`, then uses the
  owner's colour. The colour applies only when the owner's Seamless player entry has the flag `0x200`.
- It returns `*(chr+1344)` straight away when that value is `>= 0`; the game's own function does the same.
  Otherwise Seamless returns the owner's row and the game returns `*(*(*(chr+400)+184)+68)`.
- `sub_180028700` (owner of a spirit) walks the player array (`WorldChrMan+69360`) and matches when
  `*(player+1408)+2384` is the spirit's handle, or the spirit is in the main player's buddy list at
  `WorldChrMan+124216`, or the id at `+1456` equals `*(spirit+536)`. No match: no owner, no tint.

## Reading the running game

`WorldChrMan` is found with the AOB `48 8B 05 ?? ?? ?? ?? 48 85 C0 74 0F 48 39 88` (global at
`exe+0x3D6D018` on that build).

- With no spirit out: the player array holds one entry, the main player (`+0x1E508`).
- After two summons (`CloneSpirit = true`, `Multiplier = 3`, six spirits, handles `0xFFFFFFFF17100014..19`):
  `*(player+1408)+2384` stays `0xFFFFFFFF17100000`, an object of the same kind as the spirits (vtable
  `exe+0x2A49090`, stride `0x5F0`), most likely Torrent. So that cell is the Torrent branch and **does not hold a
  spirit handle**.
- All six spirits have `+536 = 0` and valid `+1408`/`+1456`; original and clone spirits do not differ in
  these fields.
- A spirit shows `+1344 = -1` (no override), `+104 = 5`, `+524 = -1` and `200` at
  `*(*(*(+400)+184)+68)`; Torrent shows `-2` in the last field.

## Reproduction

Solo is enough: `GhostColor = true`, Seamless loaded, summon Skeletal Bandit Ashes. The spirit looks solid,
without the blue ghost layer, although the log still says `ghost tint restored on 2 vfx row(s)` (so the tint
lives somewhere other than those two rows). The flag `*(*(v3+152)+641) & 0x40` is already `0xD3` (on) in
solo play, so Seamless's hook replaces the game's function there. With `GhostColor = false` there is no
tint to lose.

## Rejected paths

- **The get-team hook**: the game's `sub_1403F1C90` only writes `team = *(chr+108)` (and `15` with SpEffect
  132). The other hooks around `0x3EE7B0..0x3FC4A0` are draw-order setters (`sub_1403F0700` builds the
  15 bit draw key), unrelated to the tint.
- **Writing `*(spirit+1344) = 200`** on three spirits that were already out, then back to `-1`: two screenshots
  of the same frame were identical and the tint did not return. Either the ChrIns PhantomParam is computed
  once at spawn, or the cause is not this value.
- **"`+2384` holds the spirit's handle"**: wrong, see above.

## Open risks

- **Hypothesis** (unconfirmed): under Seamless a spirit takes the colour row of its owner's slot (solid) instead of
  PhantomParam row `200` (the ghost tint of vanilla). That is Seamless's "spirits wear the summoner's colour"
  behaviour, not a bug of this mod. A fix would write `*(spirit+0x540) = 200` at spawn, before the spirit is
  shown, or redirect the `E9` of the hook as described in [open_issues.md](open_issues.md).
- Not done: the same setup **without** Seamless, to read the same fields (`+1344`, `+104`, chain `+68`) on a
  vanilla spirit; the buddy list at `*(WorldChrMan+124216)` and the table of Seamless's function pointers
  (`a1[32]`, `a1[33]`), to see whether a clone is in it; the 21 `.text` functions that call into `ERSC`;
  a diff of 2.0.1 against the 27 October build (addresses moved, match by function); the enemy scaling
  mechanism (factor in `ersc_settings.ini`, SpEffect applied to enemies by player count).
- What Seamless patches in `eldenring.exe` is in `dumps/seamless-diff/` (git-ignored), and the animation speed
  side of the same problem is in [seamless_conflict.md](seamless_conflict.md) and the speed mod's
  [animation_speed.md](../../speedmultiplier/docs/animation_speed.md).
