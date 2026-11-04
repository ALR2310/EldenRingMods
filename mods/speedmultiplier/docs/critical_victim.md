# Victims of critical hits

Status: **Confirmed** in game on 2.7.1.0. Conventions:
[research notes](../../../docs/research/README.md).

## Question

The player's backstab or riposte runs at the `Critical` speed, but the grabbed enemy did not: the player
finished and the enemy was still bent over before being thrown. Can the victim follow the player?

## Mechanism

`ChrIns.modules.throw` (`CSChrThrowModule`) leads to `throw_node.throw_state`. The values, read with the
probe, are the same for backstab (`a0xx_031710`) and riposte (`a0xx_031700`):

| | Hit that does not kill | Hit that kills |
|---|---|---|
| Player | 0, 1, 3, 0 | 0, 1, 3, 5, 0 |
| Enemy | 0, 2, 4, 0 | 0, 2, 4, 6, 0 |

While the player is in 1, 3 or 5, an enemy in 2, 4 or 6 gets the player's current `animation_speed`
(`sync_victims` in `src/speed.rs`). When the player goes back to 0 the enemy goes back to 1.0 at once. The
enemy returns to state 0 one or two seconds later (getting up); that part keeps the original speed.

## Rejected

- **Keeping the speed until the enemy's state ends.** A surviving enemy was sped up while getting up too.
- **A key to stop at a given state.** The player only passes 1, 3, 5 and a whole hit is one state 3, so no
  number cuts it halfway.
- **Reading the victim's animation time.** The victim's time-act is a placeholder animation
  (`a004_043000`, 0.033 s); the paired animation plays elsewhere. The player's own is 5.37 s
  (`a042_031700`). If a cut in the middle is ever needed, use the player's `play_time / anim_length`.

## Open risks

Under Seamless Co-op ([animation_speed.md](animation_speed.md)) the victim reads 1.0, which is
harmless.
