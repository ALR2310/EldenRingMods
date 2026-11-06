# Animation speed

Status: **Confirmed** on 2.6.2.0, 2.7.0.0, 2.7.1.0 and 2.7.2.0. Conventions:
[research notes](../../../docs/research/README.md).

## Question

Which field makes a character's animations run faster or slower, who else reads or writes it, and what
happens when another mod hooks the read?

## Mechanism

`CSChrBehaviorModule.animation_speed` (`+0x17C8` from the behavior module, reached through
`ChrIns +0x190`, `+0x28`) scales the time of every animation of one character: walking, attacks,
casting, rolling, drinking. The game does not reset it, so it is written only when the value changes.
The mod writes it every frame in `ChrIns_PreBehavior`, per group ([action_groups.md](action_groups.md)).

The game reads the field in **one place**: the behavior update calls a thunk (`jmp <getter>`, the
getter returns `[rcx+0x18]`, `rcx = behavior + 0x17B0`) and multiplies `dt` by the result and by
`behavior + 0x15C0` before handing it to the animation system. A hardware breakpoint on the field showed
no other reader. Consequence: the speed is one number per character, so a layer such as the upper body
cannot be sped up alone.

## Other fields tried

| Field | Effect | Result |
|---|---|---|
| HKS `act(SetMovementScaleMult, x)` | scales the distance moved, not the animation | not what is wanted |
| `CSChrPhysicsModule.motion_multiplier` | reset to 1.0 by the engine between PreBehavior and PrePhysics on the player | useless on the player |
| `CSChrBehaviorDataModule.hks_root_motion_mult` | the target of `SetMovementScaleMult` (0.98 seen while running) | not the speed |
| `CSChrBehaviorDataModule.hks_animation_speed_multiplier` | set to 0.60 by the script in two weapon-art animations, else 1.0 | not used |
| `SpEffectParam`, `RideParam` | no speed field | **Rejected** |
| TAE `AnimSpeedGradient` | the way other mods change attack and cast speed | too rigid for a DLL with a config file |

Forcing `animation_speed` in PreBehavior holds through PrePhysics and PostPhysics on the player and on
Torrent. A weapon art that "stops working" at high speed was out of FP (`040005`/`040006` is the
out-of-FP animation); with FP it plays `040000`, `040001`, `040020` at double speed.

## Torrent

Torrent is the `summon_buddy_chr_set` entry with an NpcParam id in `80020000..80060000` (the base id and
the Spectral Steed Attire skins after it, in steps of 10000; the second id was seen, the rest assumed).
Only `Active` and `ReadyForActivation` entries are dereferenced.

## Seamless Co-op hooks the getter

Seamless Co-op replaces the target of the thunk's `jmp`, so the value written here is never read. Seen
as: the log says `speed=3.00` and nothing is faster. A comparison of the code in memory against the
executable showed the rel32 of that `jmp` changed. `src/seamless.rs` handles it:

- It finds the call to the thunk with an AOB (`48 8D 8F B0 17 00 00 E8 ?? ?? ?? ?? 0F 28 F0 F3 0F 59
  B7 C0 15 00 00`, the call at +7). The pattern is unique on 2.6.2.0 (`0x41D7C0`), 2.7.0.0 and 2.7.1.0
  (`0x41DD00`).
- Every tick, if the thunk's `jmp` no longer lands inside `eldenring.exe`, only its rel32 is changed to
  a stub in executable memory near the thunk. The stub returns `animation_speed` when the behavior's
  owner is the player or Torrent (`OWN_CHRS`, refreshed every frame) and jumps on to Seamless's code
  for everyone else.
- Changing only the rel32 and doing it after Seamless has hooked keeps its own signature intact. The target is looked at only when it changes.

Verified in game on 2.7.0.0 with two game instances connected through Seamless: both bypass the hook. The
pictures are not synchronised between the two machines: the player with a high speed attacks fast on
their own screen and slower on the other one. Each machine only speeds up its own character.

## Probes

`SpeedProbe`, `EffectProbe` and `EnemyProbe` in `[Logging]` write to the log only
(`src/probe.rs`). The probe task runs in `ChrIns_PostPhysics`, after the speed is written; a line for the
first frame of a new animation can still show the previous group's value because the order of tasks
inside one group is not fixed.
