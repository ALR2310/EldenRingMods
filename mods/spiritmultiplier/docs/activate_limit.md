# The 60 active character limit and vanishing enemies

Status: **Confirmed** on 2.7.1.0 at Stormgate with 60 spirits. Conventions:
[research notes](../../../docs/research/README.md). Code: `src/activate_limit.rs`.

## Question

Why do enemies vanish when about 60 spirits are summoned, and how can that be prevented?

## Mechanism

Spirits take slots of the engine's **60 active characters** limit, so nearby enemies get cut.

- Rejected: omission (`ChrIns::omission_mode`, budget at `WorldChrMan+0x1F220/+0x1F224`). It only
  lowers the update rate, so it stutters but never removes a character.
- Rejected: the distance threshold (`chr_activate_threshold` at `+0x40C`, flag `+0x1CA` bit 2). It does
  not count characters.
- Cause: a routine collects activation candidates into the vector at `WorldChrMan+0x1F1D0` and bumps the
  counter at `WorldChrMan+0x1E618` for every already-active character. The activating routine then
  activates only `limit - counter` candidates and cuts the rest.
- The `limit` lives in a singleton set by its constructor: `+0xE8 = 1` (limit on), `+0xEC = 60` and
  `+0xF0 = 60` (the two limits read), `+0xF4` = alternate-mode flag, `+0xF8 = 40` and `+0xFC = 40`
  (the limits of that mode). It applies only when `+0xE8` or `+0xF4` is non-zero.

## Fix

A unique AOB (`48 8B 05 ?? ?? ?? ?? 80 B8 E8 00 00 00 00 75 0D 80 B8 F4 00 00 00 00 0F 84`) yields
the singleton address. Every second the mod counts the occupied slots of `summon_buddy_chr_set` from the
band start (20) up, skipping Torrent and counting visible co-op spirits, and sets
`+EC/+F0/+F8/+FC = original + count`. It reads only entry pointers, never a `ChrIns`. With no spirit out
the game runs at its vanilla limit. The originals `[60, 60, 40, 40]` are captured the first time the
singleton is seen. The fields are written on a tick because the singleton may not exist at start-up,
which also makes the setting hot-reloadable.

A debug value `ActiveCharacterLimit > 0` overrode it with a fixed number during research: 120 made the
soldiers stay, going back to 60 made them vanish again, 0 restored the originals.

## Lessons

- A debug probe that read the `ChrIns` of `Unloaded` entries (stale freed pointers) and later walked
  `chr_sets` while the map streamed **crashed twice**. Do not walk the sets that way; use the game's own
  distance-sorted accessor if a probe is needed again.
- `WorldChrMan+0x1E618` counts characters activated in the frame; bit 9 of `load_state` does **not**
  mean "cut" and is not evidence.
- The real cost of the raised limit is the game processing more characters, so CPU load grows with many
  spirits and many enemies at once.
- Vanishing enemies were a defect of the mod, so there is no on/off setting for this.

## Open risks

- Only tested on 2.7.1.0 with the AOB unique there; other builds were not checked.
