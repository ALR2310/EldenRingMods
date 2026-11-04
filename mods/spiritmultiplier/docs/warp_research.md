# Teleporting spirits back to the player (`Warp*`)

Status: **Confirmed** in game: lowering the thresholds and `WarpWhenFar` (Convergence), and the Seamless
cases below (host plus a second instance). Defaults: `WarpDistance = 25`, `WarpBlockedTime = 1`,
`WarpStuckTime = 2`, `WarpStuckRange = 3`; `WarpWhenFar` is off by default in the code and on in the shipped
config. Conventions: [research notes](../../../docs/research/README.md). Code: `src/warp.rs`.

## Question

When does the game teleport a spirit to the player, and why do spirits sometimes stay behind, especially
under Seamless Co-op?

## Game mechanism (2.7.1.0, from IDA)

`SummonBuddyManager.warp_manager` holds a `std::map<FieldInsHandle, entry>` (sentinel at `+8`), one entry per
spirit. Node: left +0, parent +8, right +16, isnil +25, key +32, stage byte +48, flags dword +96, blocked-ray
counter +100, counter +104, stuck time +128/+132.

- `sub_1404C28D0` rebuilds the flags every frame: bit0 a flag of the chr; bit1 the chr has no live group;
  bit2 `chr->vtable[464]()->+192->+55752->+40 == 82` (often off); bit3 path-stuck time > `+32`; bit4 a ray
  to the player is blocked (time accumulates at +100); bit5 blocked time > `+24`; bit6 squared distance >=
  `+28` squared; bit7 a common state (`sub_14037C970(.., 31) == 6`); bit8 farther than the threshold for 1.5 s.
- `sub_1404C2FD0` sets stage 0 to 1 (RequestWarp) when `(flags & 7) == 7` and one of: (bit3 and (bit4 or
  bit6)), bit5, or (bit7 and ...). **Bit 8 alone is read by nobody**: a spirit that is only far away but
  still sees the player runs on foot. Bit 2 is often 0, so spirits blocked for 5 s stayed put.
- `sub_1404C3180` moves stage 1 to 2 (pick a point near the player with `sub_1404C0EC0`, move the chr),
  3 (fade in) and 0.
- Four runtime thresholds sit in `warp_manager` (`+0x18` blocked time, `+0x1C` distance, `+0x20` stuck time,
  `+0x24` stuck distance), copied from `GameSystemCommonParam` `buddyWarp_*` (defaults 5 s, 33 m, 3 s, 1 m) and
  reset on every area load.
- The whole update is `sub_1404C2890` (called from `SummonBuddyManager::Update` at `0x1404B8B79`).

## What the mod does

- `WarpDistance`, `WarpBlockedTime`, `WarpStuckTime`, `WarpStuckRange` (0 = the game's own): written every
  second into the four thresholds; the game's value is remembered and restored.
- `WarpWhenFar`: ten times a second, a spirit of this player at stage 0 with bit8, bit5, or (bit3 and (bit4 or
  bit6)) set gets stage = 1 and the engine does the rest. Entries of other players are left alone (the key is
  compared with the non-remote `groups`).
- Safety: `layout_known` looks (AOB) for two pieces of engine code that read those offsets; if not found
  (a game update) the feature stays off. Every node is checked readable before it is touched.
- Results (Convergence): lower thresholds make spirits teleport sooner when out of sight (flags with bit5,
  `0x177`). Running fast and far without being blocked: vanilla runs on foot; with `WarpWhenFar` the spirit
  teleports after about 1.5 s (flags `0x147`, stage 1 to 2 to 3 to 0). Blocked for 4.8 s with flags `0x033`
  (bit2 missing): the engine does not teleport; `WarpWhenFar` handles it.
- Not done: a whole group usually teleports at once (they are all far); per-spirit handling is missing. An
  earlier try that only lowered the thresholds, before a state viewer existed, wrongly concluded it had no
  effect.

## Seamless Co-op: spirits are never registered

The same config and DLL in the same area: vanilla has 3 entries in the warp list right after a summon;
Seamless showed `entries: none` all session. Cause (diff of the game in RAM plus IDA):

- The game registers a spirit with the warp manager at the end of `sub_1404BAEA0` (creates the spirit chr and
  calls `sub_1404C0DD0(warp_manager, handle)`; it is removed in `sub_1404BBAA0` through `sub_1404C26F0`).
  Seamless redirects the call to `sub_1404BAEA0` (`0x1404B7E0E`) and hooks the start of `DoSummon` and
  `sub_1404B92B0`, so its spirits are never registered.
- Registering is not enough: the flags stayed 0 and the requested stage got stuck at 1, so the warp update
  (`sub_1404C2890`) does not run for those spirits either.

Handling (`warp.rs`): every 0.1 s register the live spirits of this player that have no entry (and remove
entries of spirits that left, only those the mod registered); when the mod has entries, call
`sub_1404C2890` itself every frame with a fake `FrameTime` (only the seconds at +8 are read); measure the
distance to the player and set stage = 1 when it stays above the threshold for 1.5 s. After that the engine
runs the flags and stages 1, 2, 3, 0 (the spirit appears next to the player). Vanilla does not change: the
game registers by itself, so the mod holds no entries of its own. Solo with `ersc.dll` loaded (checked after
`CSTaskImp` exists, when every DLL has loaded): vanilla behaviour, since the game registers the spirits (and
deliberately does not in arenas).

### Crash with Mimic Tear under Seamless

Reproduced: Seamless plus a Mimic Tear crashed the game right after the mod registered the spirit with the
`warp_manager` (vanilla did not). `sub_1404C28D0` fetches the chr with `sub_140508A50(handle)` and reads
`chr+400` without a null check. Fix: `resolvable()` before registering and before every update call; an
entry of the mod whose chr cannot be resolved is removed at once. Confirmed in game. There is no dump, so
the exact cause (the Mimic not resolvable, or vanishing between two ticks) is **inferred**.

### Guest's spirits warped to the host

Both players summon; when the host runs far away the partner's spirits teleport to the host, and when the
partner comes close they are pulled back to the host. Reproduced with host plus second instance; logs of
both machines.

- The first hypothesis (the mod registers another player's spirit because `is_remote` is wrong) was
  **rejected**: on both machines `is_remote` is right (the other player's spirits `true`), the `groups` key
  is right (owner id: host 1, guest 2), and the mod only registers local spirits.
- Guest log: its own spirit at 58 m from its owner, a warp requested every ~2 s, stage 2 to 3 completing each
  time and the distance still 58 m: the destination is not the local player.
- Cause (IDA, 2.7.1.0): `sub_1404C36B0(manager)`, the function returning "the player to warp to" for
  `sub_1404C0EC0` (find a point) and `sub_1404C3180` (rotate), returns `WorldChrMan.main_player` only when
  **not** in a multiplayer session. The condition (`sub_1409FA320`): the session singleton `qword_143D7E540`
  has a non-empty list at `+120..+128` (256-byte elements), or `*(qword_143D6D988+3472)+20 >= 2`; then the
  target is the player in `player_chr_set` whose handle is stored at `session+144` (the host). The distance
  and ray flags in `sub_1404C28D0` also use a position from `sub_1404C37D0` with the same branch.
- **First fix** (redirect the two `sub_1404C36B0` calls to `main_player`): the guest's spirits warped to
  their owner but were **requested to warp continuously (every ~3 s) and blinked** while the host stood far.
  Log: flags `0x033` / `0x073` (bit4 blocked ray, bit5 blocked long) after every warp, because the flags
  measure the ray and the distance to a position from `sub_1404C37D0` / `sub_1404C39D0`, which also branch on
  the session.
- **Final fix** (`force_local_warp_target`, only when `ersc.dll` is loaded): all four functions
  (`sub_1404C36B0`, `37D0`, `39D0`, `28D0`) ask `sub_1409FA320` and then `test al,al; jz`; those four calls are
  redirected to `xor eax,eax; ret` (unique AOB on 2.6.2.0 to 2.7.2.0, `codepatch::redirect_rel32`), so the
  target, the position and the flags all follow the local player as in solo. Confirmed with host plus second
  instance: both players' spirits stay in the right place, no blinking, no repeated warp requests.
- Not checked: whether vanilla co-op (without Seamless) shows the same fault.

## Diagnostics

`Logging.WarpProbe` (off by default) logs the warp manager (thresholds, every entry with stage, flags and
timers) and the spirit groups with their owner key, `is_remote` and handle, whenever either changes.

## Independence from the game version

The warp code finds the game's functions by AOB and checks the node layout (`layout_known`), so it is not tied
to one build. The one version-bound spot was `WorldChrMan::chr_ins_by_handle` of fromsoftware-rs: it casts the
chr through `rva::get()` (a table for 2.7.1.0 only), so on 2.7.2.0 the warp tick panicked every frame as soon
as a spirit existed. `resolvable` now calls the ChrSet's `get_chr_ins_by_handle` directly (vtable slot 3, no
cast). SpiritMultiplier and `shared` no longer call any `eldenring` API that goes through `rva::get()`
(`GameMan`, `GameDataMan`, `ChrInsExt`, `as_subclass`). Not re-tested on 2.7.2.0 after that change.
