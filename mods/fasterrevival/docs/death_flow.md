# How the game kills the player, and moving that earlier

Status: **Confirmed** in game on 2.7.1.0: a normal death, a death in the middle of an attack, a grab, a
fall and a death near a Stake of Marika. Respawning after a normal death goes from 12.2 to 14.4 s in
vanilla to about 4.2 to 4.7 s together with [respawn_fade.md](respawn_fade.md). Conventions:
[research notes](../../../docs/research/README.md). Code: `src/death.rs`.

## Question

The game registers a death only through an event near the end of the death animation, so "YOU DIED"
appears late. Can the registration move to the start of the animation without editing animation files?

## Where the idea comes from

A third-party mod does this offline: a console tool reads `c0000.anibnd.dcx`, opens `a00.tae` and, for
every animation that has both TAE events of type 0 (`ChrActionFlag`) **action 12 (Kill Character)**, the
first one with `StartTime != 0`, and **action 20 (Send Ghost Info)**, moves both to the start of the
animation, keeping its length. The animation still plays in full; only the logic point of "dead" moves.
Its repository has no license, so only the idea was taken, no code.

Doing it as a DLL avoids replacing `c0000.anibnd.dcx`, a file that many animation mods replace as well:
those conflict, and players would have to run the tool again on an already modded file.

## The game's death flow (IDA, 2.7.1.0)

1. HP reaching 0 sets bit 7 of `ChrIns + 0x1C5` (`chr_flags1c5.death_flag`). Every frame,
   `sub_1403F84C0` either requests the death animation or, for a death without one, kills at once.
2. TAE events of type 0 are split by `CSChrTaeAnimEvent` (vtable `0x142A372B0`) between two handlers:
   `sub_140427B30` (a 143-case switch; action 20 sets bit 1 of `ChrCtrlModifier + 0x18`, which is
   `send_ghost_info`) and `sub_140428DE0`, **the handler of action 12**: if the byte at `event + 0x18` is
   set and `sub_14041BA80(behavior module, 0)` (a thunk into packed code, not understood) returns false,
   it calls the kill function **`sub_1403EDA70(ChrIns*)`**. Action 47 ("Kill Character (No Item Drop)")
   calls the near twin `sub_1403EDB60`.
3. The kill function is **idempotent**: it returns at once if bit 0 of `ChrCtrl (+0x58) -> modifier (+0xC8)
   + 0x24` is set, and it sets that bit first thing, so calling it early makes the later call from the
   TAE event a no-op. It skips `ChrType` 3 (Ghost) and 10 (BloodstainGhost).

So there is nothing to patch in memory and nothing to hook: the mod calls the kill function itself.

## What the mod does

- **Finding the function.** An AOB (`KILL_CHR_PATTERN`) unique on both 2.7.1.0 (`0x1403EDA70`) and 2.6.2.0
  (`0x1403ED840`). If it is not found the mod logs an error and the early kill is off.
- **Detecting death.** A task on `FrameBegin` tests `hp <= 0` (`CSChrDataModule.hp`), because `death_flag`
  cannot be read: the game sets it and clears it in the character update of the same frame, so a task at the
  start of the frame never sees it (the first test produced no log line).
- **Which deaths.** Only when the animation playing is one with the action-12 event (`has_kill_event`:
  `a000_017xxx`, `070xxx`, `075003`, `117xxx` and `000150`, taken from the tool's output on
  `c0000.anibnd.dcx`: 91 animations). Animation ids in `CSChrTimeActModule` are
  `category * 1_000_000 + id`.
- **Log.** Every death logs `Death: died`, `killed early`, `registered` and `respawned` with times; the
  `[Debug] DeathProbe` key adds the animation trace (id, play time, length) every 100 ms.

## Rejected paths

1. **Patching the TAE in memory** after the game loads `c0000` (a 1:1 port of the tool): the events may
   point into shared time tables, so editing a float in place could shift other events.
2. **Hooking the TAE event walk or the handlers of actions 12 and 20:** not needed, since the kill function
   is idempotent.
3. **Seeking `play_time` or speeding up the animation:** no reverse engineering, but it differs from the
   mod it follows (the animation would also look faster).
4. **The first trigger**, "the animation changed since HP hit 0, or after 300 ms": wrong, because after HP 0
   the animation can still change (`10020113` to `10020110`, an attack in progress) one frame **before**
   `17002`, so it killed before the death animation had started.

## Verification (2.7.1.0)

Time from HP 0, before the pause after "YOU DIED" was removed:

| Death | Animation | Registered by the game (on / off) | Respawn (on / off) |
|---|---|---|---|
| A fall | `4100`, `4101` | first frame / first frame | 8063 ms / 8128 ms (unchanged) |
| Hit by an enemy | `17002` (6.667 s) | +53 ms / +5997 ms | 8379 ms / 14392 ms |
| Hit in the middle of an attack | `17022` (4.0 s) | +49 to 50 ms / +3827 ms | 8411 to 8421 ms / 12227 ms |
| A grab | `70440`, `70441`, `70442` | the same frame | 8395 ms |

- The saving is close to the whole length of the death animation (about 6 s for `17002`, 3.8 s for
  `17022`). With the mod on, the respawn is always about 8.4 s whatever the animation; the rest is the
  "YOU DIED" screen, the fade and loading.
- A fall does not change: the game kills on the first frame, and `4100`/`4101` are not among the 91
  animations with the event, so the mod it follows has no effect there either.
- A grab still plays `70440`, `70441`, `70442` in full (the lying pose at the end is held until the
  respawn), nothing gets stuck. The tool has the kill event only at `70441`, so the range
  `70_000..=70_999` kills about 0.7 s earlier than it does here. The whole range is kept because it ran
  well and also catches grab animations that animation mods add.
- `17032` (4.8 s) and `17012` (4.667 s) kill at +33 ms as expected. After the death animation the game
  always moves to the matching `18xxx` (`17022` to `18022`).

## Open risks

- The mod does not test `sub_14041BA80` before killing (it is a thunk into packed code). If that condition
  ever blocks a kill in some case, the mod would kill at the wrong moment there.
- Action 20 "Send Ghost Info" is not moved: its handler has one more condition (`sub_1404C8730`) that is
  not understood, and with a DLL mod installed the player is offline, so no bloodstain or ghost would be
  sent anyway.
- The kill function is called from a `FrameBegin` task, not from inside the character update as the game
  does; it ran well in every test.
- Not tested while riding Torrent.
