# SpiritMultiplier: open issues (Reforged, Seamless, Convergence)

Status: **Open**. Spirit Fury and Fortune of the Spiritcaller of Reforged were fixed earlier; the items
below are not. Conventions: [research notes](../../../docs/research/README.md).

## Reforged

In Reforged, pressing an Ash again while its spirit is out **raises Spirit Fury** instead of sending it
back; the Spirit-Severing Blade sends spirits back. "Aggro" is Spirit Fury, "un-aggro'ed" is the default
calm state.

- **Spirits summoned red (Fury) instead of white** (a user report). Not established whether the mod causes
  it (`GhostColor = false` removes Reforged's vfx 70000, 70010 and 70020 so only the Fury colour is left)
  or whether it is Reforged's design. **Hypothesis**: compare `GhostColor = true` against `false`.
- **MultiSpirit with Fury**: while one group is in Fury every Ash is greyed out. Needs how Reforged marks
  "in Fury" and what greys the Ash (a SpEffect, or its DLL); mind the balance.
- **`NoRestResummon` has no effect after the Spirit-Severing Blade**: this is Reforged's own rule (the Ash
  slot stays grey until resting at a grace). `NoRestResummon` only lifts the vanilla lock
  (`summonedEventFlagId` of the stone). The standard summon SpEffects `9540`, `9541`, `9545`, `9547` and
  `9560` (`GameSystemCommonParam` `onBuddySummon_*`) are almost empty in Reforged, so **inferred**: an
  event flag set by Reforged's scripts (EMEVD or ESD) or its DLL.
- **Long term**: detect Reforged (its module in the loaded list) and leave the "press the Ash again"
  action to Spirit Fury. MultiSpirit takes exactly that action (patches of `DoSummon`, `DisappearAll`,
  `GetBuddyState`, `CanUseItem` and the UI) and can override a hook of the Reforged DLL. It needs a
  reproduction first (Fortune of the Spiritcaller, dropped by a boss of Road's End Catacombs).

## Seamless Co-op: spirits lose the ghost colour

- Ruled out (with a temporary probe): PhantomParam 200 and 201 are identical to offline; the spirit still
  has SpEffect 295000; the vfx module (`ChrIns.modules +0xB8`, field +0x44) still selects 200; the team
  (47 offline, 1 under Seamless) is not the cause.
- Cause: Seamless hooks the start of `sub_1403F0C20` (2.7.1.0), the "PhantomParam of a ChrIns" function
  (override `+0x540`, then vfx `+0x44`, then the default by chr type). Found by diffing game code in RAM
  against `eldenring.exe` (363 places exist only under Seamless). It also hooks `sub_1403F1C90` (get team).
- **Rejected**: setting `phantom_param_override = 200` on spirits: no effect, since the hook ignores the
  override as well.
- **Remaining idea**: change the rel32 of the `E9` Seamless writes at the start of `sub_1403F0C20` to
  point at a stub of ours: for a spirit (vfx `+0x44 >= 0`, in `summon_buddy_chr_set`) return that id,
  otherwise continue into Seamless's hook. Install only after Seamless has hooked (first byte `E9`).

## Convergence: Cemetery Shade makes every spirit invisible

Reproduced from a user report. Cemetery Shade Ashes (SpEffect `400000`, npc `136640000`): summon it first
or in the middle, then summon one more Ash, and **all** spirits out turn invisible (their health bars stay,
they are alive and still in `groups`). Other Ashes (for example `202000`, `239000`) can be used many times
normally; summoning Cemetery Shade **last** is also fine.

Evidence (slot diagnostics): after Cemetery Shade appears the player holds SpEffect `295921`; on the next
summon `ChrIns.camouflage_transparency` of every spirit falls `1.00 -> 0.00` within about 1 s and stays at
0 (`opacity`, `tint`, `base` and the `render` flag stay normal), and `295921` is removed about 2 s after
that summon. The summon effect (white particles) also spreads to spirit 1 when Cemetery Shade is the
second. The mod touches neither `camouflage_transparency` nor `295921`: **inferred** to be Convergence's own
mechanism for Cemetery Shade (normally the only Ash out), applied to every spirit when several are out. It
does not depend on how the Ash is used (hotkey or inventory); the order of summons only looked related.

Not done: read `SpEffectParam` `295921` and `400000` with Convergence's regulation to see whether the
camouflage is intentional or a defect with several Ashes. Possible patch: an option, off by default,
forcing `camouflage_transparency = 1` for live spirits (it would break intended stealth if any). For now:
summon Cemetery Shade last.
