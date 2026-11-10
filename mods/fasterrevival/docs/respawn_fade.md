# The pause after "YOU DIED"

Status: **Confirmed** in game on 2.7.1.0. Conventions: [research notes](../../../docs/research/README.md).
Code: `src/fade.rs`. Moving the "dead" point earlier is in [death_flow.md](death_flow.md); together the two
take a normal respawn from 12.2 to 14.4 s down to about 4.2 to 4.7 s.

## Question

What decides how long the game waits on the "YOU DIED" screen, and can it be shortened without a code patch?

## Mechanism

A third-party mod patches the jump `74 06` (`jz`) found by the pattern `74 ?? F3 0F 10 19` into `EB 06`
(`jmp`). It does not say what the jump controls; in IDA (2.7.1.0, the only match at `0x1405A7F23`; 2.6.2.0
at `0x1405A70D3`):

- It is inside `sub_1405A7E00`, a step of `CSDeathRestartEvent::Start` (`sub_14059F670`, which carries that
  function's name as a string). The step schedules the callback `sub_1405A7FA0` through
  `sub_1405942D0(a2, 4006, cb, delay, 0, 4, 1)`.
- `delay` (xmm3) is `*sub_140D2FE90()`, a pointer into param slot 143 (`MenuCommonParam`), row 0, first
  field **`soloPlayDeath_ToFadeOutTime`** (shown as "[YOU DIED] Fade Out Duration", vanilla 3.8;
  fromsoftware-rs accessor `solo_play_death_to_fade_out_time`). The patch forces the branch `xorps xmm3,
  xmm3`, so the delay becomes 0.

The delay is only a param value, so no code is patched: `fade.rs` writes `MenuCommonParam[0]
.soloPlayDeath_ToFadeOutTime = 0` **once**, logging the old value, when the main player exists and
`shared::params::check` confirms the slot holds the right param. The mod is always on, so the original
value is never restored. `partyGhostDeath_ToFadeOutTime` (co-op, vanilla 3.3) is left alone, as in the
mod it follows.

## Open risks

The next step (stage 5, in `sub_1405A7FA0`) has one more delay, read from `GameSystemCommonParam[0] + 0x37C`.
The mod it follows does not touch it and which field it is has not been identified (it measured 0).

## A slow respawn some of the time is not a fault

After this change a normal death respawned in about 4.4 to 4.7 s (before, about 8.4 s, matching the 3.8 s
vanilla `soloPlayDeath_ToFadeOutTime`), but some deaths took 7.8, 14 or 20 s. From the logs, the still
animation (the fade) at about 1.7 s and the loading gap (no frames) at 2.3 to 2.5 s were **the same** in
every run; the difference was time the game kept running frames after the fade.

Rejected: that stage 5 of `CSDeathRestartEvent` waits when `is_death_penalty_skip`, with a delay from
`GameSystemCommonParam[0] + 0x37C`. A temporary log showed `penalty_skip = false` and `+0x37C = 0`.

The real cause, noticed by the player: dying near a **Stake of Marika** makes the game show a menu to
respawn at the Stake or at the last grace, and the time spent in that menu is counted. It has nothing to do
with the mod.
