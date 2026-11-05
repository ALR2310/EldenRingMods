# Action groups

Status: **Confirmed** in game on 2.7.1.0, every group tested. Conventions:
[research notes](../../../docs/research/README.md).

## Question

Which animations make up each group of player actions (`Walk`, `Run`, `Roll`, ...), so one speed can
be set per group?

## Mechanism

The animation playing is read from `CSChrTimeActModule.anim_queue` as `aXXX_YYYYYY` (prefix table: [tae_ids.md](tae_ids.md)). `group_of` in
`shared/src/anim.rs` classifies it, with a test per range.

1. **By the `XXX` prefix** for spells and weapon arts, which share the tail `04xxxx` and cannot be told
   apart by it:
   - `400..600` is **Cast**.
   - `600..1000` is **Skill**, except the prefixes of weapons with their own move set that fall inside
     that range (`WEAPON_TAES_IN_SKILL_RANGE`).
2. **By the tail** (`anim_id % 1_000_000`) for everything else:

| Group | Tail | Notes |
|---|---|---|
| Walk | `0201xx`, `0221xx` | walking and stopping |
| Run | `0202xx`, `0222xx` | running with the dash key held |
| Sneak | `300000`-`399999` | crouch idle `300000`, entering `390000` |
| Jump | `2020xx`, `2021xx` | take-off (`202000` standing, `202020` walking, `202030/40` running), landing (`202100`, `202115`, `202126`) |
| Roll | `027xxx` | rolling, backstepping |
| Ladder | `028000`-`028999` | entering `028999`, idle `028030`, climbing `0280 1x` and `028100`, descending `0280 2x`, sliding `028000`-`028002`, attack and kick `028040`/`028045`, transition `028036` |
| Critical | `031700`-`031799` | riposte `031700`, backstab `031719` then `031710`; checked before Attack |
| Attack | `030000`-`039999` | normal attacks |
| Skill | `040000`-`049999` | weapon arts that do not go through the prefix |
| Item | `050000`-`059999` | using items |
| Other | the rest | standing `000000`, `0200xx`, resting at a grace `068xxx`, loading `063000` |

3. Anything that matches nothing is **Other**.

Torrent has its own small table (`torrent_group_of` in `src/speed.rs`): idle `000000`, walk `0021xx`,
run `0022xx` (dash and slowing down included), turning `0051xx`, jump `0061xx` and landing `0074xx`,
giving **Walk / Run / Jump / Other**. Torrent follows the SpEffects of the player when `[[Override]]`
is used.

## Decisions and rejected paths

- **`All` overrides the group keys** when it is not 1; at 1 the group keys apply. Torrent is not
  affected by the player's `All`.
- **A single `Movement` group was dropped** and split into Walk, Run and Sneak after a request: the
  keyboard has only two gaits, and the third digit of the tail is the gait. `0200xx` (`020010`) only
  appears when using an item while moving, not as "normal running".
- **Item grew over time.** It started as `050xxx`; throwing daggers and crystal darts use `055000` and
  fell into Other, so the range is `05xxxx`. No other animation of the logs has a tail in `05xxxx`.
- **The Spectral Steed whistle (`050190`) is Other**, not Item: it is the first step of mounting
  (`050190`, then `101004` jumping on, then `100000` riding) and goes with the riding animations.
- **Critical** has its own key because the victim's animation was not sped up with the player's, so the
  player finished while the enemy was still falling. The first version pinned it to 1.0; it is now a key
  like the others, vanilla by default. The victim now follows the player's speed
  ([critical_victim.md](critical_victim.md)).
- **One spell's beam is Other.** The beam of Placidusax's Ruin (`a451_045110`-`045119`) is spawned by
  the game and runs on its own clock, so it drifts when the cast is sped up. It is checked before Cast
  and takes the Other speed; the first part of the cast (`045100`) is still Cast. With several Cast
  values the beam always lasted about 8 to 9 seconds. When reading the log, the line of `045110` shows
  the Cast speed of the animation before it, because the line is written before the speed is applied.
- **Ladder** is its own group because the climbing and sliding distance may follow root motion or a
  timer of its own, which could drift; the player decides. The default for a new file is slightly faster.

## Open risks

- **Drinking a flask while running belongs to Walk or Run**, not Item. The upper-body animation never
  enters `anim_queue`, so the running animation decides. The game has one animation speed for the whole
  character ([animation_speed.md](animation_speed.md)), so the upper body cannot be sped up on its own.
  An upper-body value at `CSChrTimeActModule +0xD0` exists but is not cleared when the animation ends, and
  `+0xC8`, `+0xCC` and `+0xD4` do not say whether one is still playing; the attempt and its probe code
  were removed.
- The walking animation when overweight is `020020`, outside the Walk range, so it is Other.
- Animations of dying and being grabbed have no "always 1.0" group.
