# Animation ids and action groups

Status: **Confirmed** in game through the mods that classify animations (SpeedMultiplier, AnimCancel,
AutoRegen, FasterRevival); grouping rules are covered by unit tests. Conventions:
[research notes](../../docs/research/README.md). Code: `shared/src/anim.rs`.

## Question

An animation id looks like `aXXX_YYYYYY` (a TAE number and a suffix). How does a mod tell walking from a roll,
an attack from a spell, without a table per weapon?

## Reading the current animation

`current_anim(chr)` returns the `CSChrTimeActModuleAnim` at `time_act.read_idx` of the character's time-act
module (`anim_queue[read_idx % len]`); `current_anim_id(chr)` is its `anim_id`. `format_anim_id(id)` prints
`a020_032000`; a negative id prints as is.

## Grouping (`group_of(anim_id)`)

By **TAE prefix** (`id / 1_000_000`) for spells and Sword Arts, by **suffix** (`id % 1_000_000`) for the rest.

| Rule | Group |
|---|---|
| prefix 400 to 599 | `Cast` |
| prefix 600 to 999, except the special-weapon movesets 831, 832, 839, 852, 935, 953 | `Skill` |
| suffix 20100 to 20199, 22100 to 22199 | `Walk` (the third suffix digit is the gait) |
| suffix 20200 to 20299, 22200 to 22299 | `Run` |
| suffix 300000 to 399999 | `Sneak` |
| suffix 202000 to 202199 | `Jump` (take-off 2020xx, landing 2021xx; only the range seen) |
| suffix 27000 to 27999 | `Roll` |
| suffix 28000 to 28999 | `Ladder` (grabbing, climbing, sliding and attacking share one range) |
| suffix 31700 to 31799 | `Critical` (paired with the victim's animation, so its own group, checked before `Attack`) |
| suffix 30000 to 39999 | `Attack` |
| suffix 40000 to 49999 | `Skill` |
| suffix 50190 | `Other` (Torrent's whistle is the first part of mounting, not an item) |
| suffix 50000 to 59999 | `Item` |
| anything else, and negative ids | `Other` |

Two exceptions: the weapon movesets whose TAE number falls inside the Sword Arts range are not `Skill`; and the
beam phase of a boss spell (prefix 451, suffix 45110 to 45119) is `Other`, because it runs on the game's own
clock and must not be sped up (its wind-up 045100 stays `Cast`).

## Open risks

The suffix ranges are the ones seen in play; a range that was never observed (a rare jump or ladder variant)
falls into `Other` and is left alone, which is the safe side for the mods that change speed or cancel an
animation.
