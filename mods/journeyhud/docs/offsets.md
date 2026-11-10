# Offsets and checks per executable build

Status: **Confirmed** statically in IDA on 2.6.2.0, 2.7.0.0, 2.7.1.0 and 2.7.2.0; the poise maximum (+0x14) could
not be confirmed in IDA. Conventions: [research notes](../../../docs/research/README.md). Code: `src/target.rs`,
`src/radar/mod.rs`.

The panel reads through the structs of fromsoftware-rs and never through `rva::get()`. The offsets below are
enforced by `const` asserts in `src/target.rs`: if a fromsoftware-rs update moves a field the build stops.

| Data | Offset | Checked in IDA |
|---|---|---|
| Locked target handle (`PlayerIns::locked_on_enemy`) | +0x6B0 | Yes, all 4 builds |
| Module list (`ChrIns::modules`) | +0x190 | Already used by released mods |
| HP / max HP (`CSChrDataModule`) | +0x138 / +0x13C | Already used by released mods |
| Container: `data` / `super_armor` | +0x00 / +0x40 | From fromsoftware-rs, not checked separately |
| Current poise (`CSChrSuperArmorModule`) | +0x10 | Yes (the update function uses `a1+16` as the current value) |
| Maximum poise | +0x14 | **No**, see below |
| Character lookup by handle | slot 3 of the ChrSet vtable | Yes, all 4 builds |
| Boss health bars (`CSFeManImp::boss_health_displays`) | +0x5BF0, 3 entries x 0x20, handle at +8 (+0x5BF8), empty = -1 | Yes, all 4 builds |
| Death count (`GameDataMan::death_count`) | +0x94 of `GameDataMan` | Pattern unique on all 4 builds; the value matched 341 in game, not compared with the statistics screen |
| In-game time (`WorldAreaTime::clock`) | +0x00 (`time64`), hours at bits 34 to 38 of the packed number at +0x08 | Yes, all 4 builds |

## Boss health bars (`Minimap.Hide.InBossFights`)

A small function looks for a handle in 3 entries (RVA 76DA70 / 76E8F0 / 76E8F0 / 76EDA0 on 2.6.2.0 / 2.7.0.0 /
2.7.1.0 / 2.7.2.0, the same shape on all): `for (i = a1 + 23544; ...; i += 8 dwords) { ... if (++n >= 3) return -1 }`.
`23544` is 0x5BF8, the handle of the first entry (entries start at 0x5BF0, `fmg_id` at +0, handle at +8), stride
32 bytes, exactly 3 entries; for the searched handle `0xFFFFFFFF` it compares the low dword with `0xFFFFFFFF`,
so **an empty entry has handle -1**, matching `FieldInsHandle::is_empty`. **Not tried**: whether the entry is
cleared at once when a boss dies.

## In-game time (`Minimap.Clock`)

The `WorldAreaTime` singleton is registered by name at `sub_14009CF30` (same RVA on all 4 builds), so
`WorldAreaTime::instance()` finds it by name and no RVA table is involved. `CS::WorldAreaTimeImp::Update`
(643BC0 / 644A10 / 644A10 / 644EC0) copies `*a1` and `*(a1+8)` to `+16` and `+24` (current clock to the previous
tick's clock), checks the target hour, minute and second at `+40/+44/+48` (<= 23, 59, 59) and takes the hour as
`(*(a1+8) >> 34) & 0x1F`, matching `PackedDate::hours` (bits 34 to 38) of fromsoftware-rs. Minutes follow (bits
39 to 44).

**The in-game day count is not usable**: `PackedDate` has a `day` field (bits 29 to 33), but it carries the
machine's **real date** (a log showed the day of the test); the game only changes hours, minutes and seconds
with game time. Whether crossing midnight in game raises `day` was not checked; if it does, it counts days
since entering the game, and every entry restarts from the real date.

## Locked target handle (+0x6B0)

On all 4 builds `fieldaccess 6b0` gives the same accessors: one write of `-1` (qword) and one write of the
handle (`mov [r13+6B0h], rcx`) in the same function, plus a reader at `0x344DA0 / 0x344DB0`.

| Build | Writing function (RVA) |
|---|---|
| 2.6.2.0 | 716260 |
| 2.7.0.0, 2.7.1.0 | 7170B0 |
| 2.7.2.0 | 717560 |

Decompiled on 2.7.2.0 it is the lock-on camera update: for the player character it writes `-1` to `+1712`
(= 0x6B0) when the lock is dropped and `*(target chr + 8)`, the target's `field_ins_handle` (ChrIns +0x8), while
locked. So `locked_on_enemy` is exactly the handle needed, no position match required.

## Character lookup by handle (slot 3)

- `CS::ChrSet` slot 3 looks up by handle: it turns the handle into an index and calls slot 1 (`vtable+8`). Same
  shape on 2.6.2.0 (4937A0), 2.7.1.0 (493D00) and 2.7.2.0 (493DE0).
- `CS::OpenFieldChrSet` overrides slot 3 with a binary search in the handle table (2.7.1.0 `4EC850`, 2.7.2.0
  `4EC930`, 2.6.2.0 `4EBA80`), the same signature.
- Calling through the object's own vtable is right for both classes.

## Poise (not fully checked)

The game's function uses `+0x10` as the current value (float), `+0x18` as an addition to the maximum and `+0x1C`
as the recovery counter. The effective maximum is `vtable[2]() + [+0x18]` (in the base class `vtable[2]` returns
0.0). No write to `+0x14` was found in IDA; a third-party HUD mod and fromsoftware-rs both put "max" there, so a
subclass may set it. The panel shows the posture row only when the maximum is above 0 and clamps the ratio to
0..1; a `Target: ... poise (cur, max)` log line shows the real values in game. **Open**: compare the log with the
real stagger bar (hit, wait for recovery); if `+0x14` is wrong find the real subclass of
`CSChrSuperArmorModule`.

## Death count (`Minimap.Deaths`)

`GameDataMan::instance()` goes through the RVA table and panics on 2.7.2.0, so `radar/mod.rs::death_count` finds the
global by the pattern `48 8B 05 ?? ?? ?? ?? 48 85 C0 74 05 48 8B 40 08 C3` (a small function that reads
`GameDataMan` and returns `[+8]`, i.e. `main_player_game_data`). Unique on all builds (2.6.2.0 8AE3A0, 2.7.0.0 and
2.7.1.0 8AF520, 2.7.2.0 8AF9D0). The variable address is `rip + rel32` of the `mov rax, [rip+..]` at the start of
the function; `death_count` at +0x94 (per fromsoftware-rs, enforced by a `const` assert).
