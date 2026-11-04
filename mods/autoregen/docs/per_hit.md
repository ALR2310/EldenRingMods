# Restoring on hit

Status: **Confirmed** in game on 2.7.1.0, including stance weapon arts. Conventions:
[research notes](../../../docs/research/README.md).

## Question

How to know when the player lands a hit, how much damage it did, whether it was melee or ranged, and
whether it came from a weapon art, without breaking the game's own hit handling?

## Mechanism

`src/hit_hook.rs` hooks the entry of the game's hit-resolution function `(ctx, attacker, hit_info, _,
flag)`. The hook runs before the real function and only observes. The function is found by a pattern of
its whole prologue (15 relocatable bytes, then `SUB RSP,0x148`); it was checked on 2.7.1.0 only. The
first 15 bytes are moved to a buffer from `shared::codepatch::alloc_executable` that ends with a jump
back, and the entry is overwritten with `mov rax, trampoline; jmp rax`. The trampoline saves the four
argument registers, calls the Rust callback, restores them and jumps to the relocated prologue.

Data read:

| What | Where |
|---|---|
| Damage | `hit_info + 0x228` |
| Source object | `hit_info + 0x1D8`; its vtable slot `+0x10` returns `sourceType`: 1 direct (melee), 3 projectile |
| Attacker | the second argument, compared with the main player |
| Target | `[ctx + 8]`, compared with the main player |

The hook marks combat activity for the player's hits and for hits taken. For the player's own hits it
restores HP, FP and Stamina per `[PerHit]`, and adds the amounts to counters that the tick logs: file I/O
inside hit resolution can disturb tightly timed animations. Nothing in the hook allocates or locks; its
settings are atomics.

The function was found by scanning the game for writes to the HP field (over a thousand sites) and
walking up three levels from the hit resolution: `SetHp`, `ApplyHpDelta(module, delta)`, then this
function.

## Modes

- `Mode`: 0 fixed points, 1 percent of the maximum, 2 percent of the damage dealt. An early version read
  the old `HpOnDamage` as "heal a fixed amount when hit"; the intent was a percent of the damage dealt.
- `DamageType`: 0 melee only (`sourceType == 1`), 1 ranged and spells only, 2 both; one setting for the
  three stats. A spell with a melee motion (the Carian Greatsword, for one) reads as `sourceType == 1`.

## Excluding weapon arts

No `atkId` threshold and no `AtkParam` field tells a weapon art from a normal attack: about 200 fields
were compared between the `[AOW]` group and the default one in `AtkParam_Pc`, and the best field still
overlapped by 44.6%. A heavy attack of some weapons shares an id range with many arts.

What works is the pad input in `CSChrActionRequestModule`: a weapon art is always `L2`. Reading `L2` at
the moment of the hit misses it (the button was released long ago), so the **latest press** is latched:
`R1`, `R2` or `L1` means a normal attack, `L2` a weapon art, kept until the next press. `L1` counts as
normal because it never starts a weapon art.

Stance arts enter the stance with `L2` and strike with `R1` or `R2`, so the latch reads "normal attack"
and the hit still restored FP. The second layer is the **TAE prefix** of the animation playing: 600 to
999 is a weapon art, except six weapons with their own move set (`a831`, `a832`, `a839`, `a852`,
`a935`, `a953`) whose normal attacks sit in that range. It is the same rule and the same list as
`shared::anim::group_of`. A hit is excluded when the latch says weapon art **or** the animation does.

Probe evidence: Unsheathe is `a614_040060`, Square-Off `a615_040060` and `040070`, all with the latch at
"normal" and all excluded; normal attacks (`a029_0320x0`, `atkId` 900200 to 900240) still restore. A
single-press art (Spinning Slash) stays correct through the latch. Not tested: Stomp. The arts of the six
weapons above still depend on the latch alone.

## Rejected paths and failures

- **Classifying weapons and spells by `atkParamId` above 100000**: wrong for the Glintstone Nail, dash,
  jump and backstab.
- **Reading `atkPhys` and `atkMag` of `AtkParam`**: many weapons get their damage from a SpEffect and
  enchanted weapons share an `AtkParam` row.
- **"Stamina just dropped" as the sign of melee**: wrong for the Bestial Incantations.
- **Lost backstab and critical animations (2.3.0).** The function takes five arguments; the fifth (one
  byte, from `hitInfo + 0xD9 == 2`) is passed on the stack at `[rsp + 0x20]` and decides whether the
  critical-hit block runs. A trampoline that forwards four and creates its own shadow space leaves garbage
  there. Switching the feature off did not help because the patch stayed. The fix reads that byte first
  and writes it back after the shadow space. The current trampoline hooks the entry and leaves the stack
  arguments where the caller put them.
- **Crash entering Volcano Manor (2.4.0).** A real fault of the mod: a hand-written trampoline that
  hijacked a call site in the middle of the function, probably without unwind data for the hijacked
  range. A build without the fifth-argument forwarding did not crash. Hooking the entry of the function
  fixed it.
- **Conflict with Seamless Co-op (the C++ version).** Seamless scans for the same pattern as the old hook
  and aborts the game when it does not match. The hook is now installed once in game, after the other
  mods have scanned, and it uses a different site.
- A trampoline that aligned `rsp` and then pushed left it 8 bytes off and `movaps` faulted; the current
  one saves the original `rsp` in `r12`.
- A relocation buffer too short for its jump back let the CPU run on into the next data and crash with
  an access violation on the first hit. `RELOCATED_BUF_LEN` covers the jump.
- Logging every hit, even with `PerHit` off, flooded the log. The tick now logs one line per frame that
  restored something.

## Open risks

The pattern is verified on one game build. When it is not found the install is retried every ten seconds
and the mod logs one error after 15 seconds; the per-tick part keeps working.
