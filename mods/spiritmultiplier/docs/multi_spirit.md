# MultiSpirit and CloneSpirit: summoning several Ashes at once

Status: **Confirmed** in game on vanilla, Reforged and Seamless Co-op (2.7.1.0 and later). Conventions:
[research notes](../../../docs/research/README.md). Code: `src/multi_spirit.rs`. Reforged with Spirit Fury
is in [open_issues.md](open_issues.md).

## Question

Vanilla allows one spirit group at a time: using another Ash sends the old spirits back. How can several
different Ashes stay out, and how can one Ash be summoned more than once?

## Vanilla mechanism (2.7.1.0, offsets after `SummonBuddyManager` of fromsoftware-rs)

- `GetBuddyState(mgr, goodsId)` = `sub_1404B72F0`: 2 = a spirit is out and `goodsId` equals the Ash
  out (`+0xE4`) or `goodsId = -1`; 1 = moved to another stone area; -1 = nothing.
- `DoSummon(mgr, speffect)` = `sub_1404B85B0`: asks `GetBuddyState(mgr, -1)`, and when any spirit is out
  raises `disappear_requested` (`+0x28`) and returns (the "one group" rule); otherwise it spawns with
  `+0x20 = speffect`. The SpEffect is rounded `100 * (a2 / 100)` before the lookup.
- `Update` = `sub_1404B86D0`: on a request it finds the group in `groups` (`+0x70`); a group with spirits
  is recalled whole by `sub_1404BB9A0` before the new group spawns.
- `CanUseItem` = `sub_14068EE60`: for an Ash (type 7 or 8) `usable = GetBuddyState(mgr, goodsId) != -1`
  (`0x140690383`). This is what greys out the other Ashes.
- `sub_1403C0930` builds the use cost: when `GetBuddyState(mgr, -1) == 2` FP (`+0xB8`) and HP (`+0xB4`)
  are 0 (vanilla treats it as a send-back, so it is free).
- Goods to SpEffect: every Ash has `refCategory = 2` and `refId_default` = the trigger SpEffect (the key of
  `trigger_speffect_to_buddy_map`); each upgrade level is its own goods (+0 = `207000`, +4 = `207004`),
  rounded `100 * (id / 100)` to 86 keys, and no two Ashes share a key.

## Patches

Every patch only **redirects an existing `call` or `jmp`** (`codepatch::redirect_rel32`: the four
displacement bytes change and nothing else). Seamless Co-op looks up its own patch sites by byte
signature with the displacement wildcarded, for example `E8 ? ? ? ? 83 F8 02 41 0F 44 FC` (the HP call of
patch 7), and aborted the game with "No such pattern" when an earlier version rewrote the instructions
around those calls. A redirected call still matches. See [seamless_conflict.md](seamless_conflict.md).

1. **DoSummon** (AOB `E8 ?? ?? ?? ?? 83 F8 01 76 1B 83 F8 02 75 04 C6 43 28 01`, `0x1404B85C9`): the
   `call GetBuddyState` goes through a stub. On state 2, `decide_state` checks whether the requested Ash
   (SpEffect rounded) still has live entries in `groups` (`!disappear_requested`, `!is_remote`, with a
   `buddy_param_id` in the chain of that SpEffect). Yes: remember `DISMISS_TARGET`, return 2 (send back).
   No: return 0 (summon it too). The first version compared the SpEffect with the active or pending one
   (`+0x24`/`+0x20`), which only knows the **last** Ash summoned; `Update` sets `+0x20 = -1` after the
   spawn, so re-using the same Ash looked like another one.
2. **Update** (AOB `8B 10 49 8B CF E8 ?? ?? ?? ?? 41 83 BF 88 00 00 00 00`, `0x1404B89AB`): the
   `call sub_1404BB9A0` (recall the old group) is skipped while the feature is on.
3. **UI x3** (`0x1407C3C9E`, `0x140847A1B`, `0x140847CDF`): the real `GetBuddyState`, then `eax = 0` if on
   and the result is >= 1. This removes the "send the spirit back?" dialog (20000600) and the greyed
   labels. The cost and label helpers `sub_140847830` / `sub_140847B20` only display the cost; no function
   both reads `EquipParamGoods+0x79` (`useLimitSummonBuddy`) and calls the summon API.
4. **CanUseItem** (AOB `48 8B C8 41 8B D5 E8 ?? ?? ?? ?? 83 F8 FF 41 0F 95 C5`, `0x14069037D`): on -1 it asks
   again with goodsId -1; 2 returns 0 (usable). Away from a stone area it stays greyed as in vanilla.
5. **Stone kept**: `+0x38` becomes 0 after the first summon, so a second summon looked like "left the
   area" and both groups vanished after about 10 s. `decide_state` sets `+0x38 = +0x3C` first when it is
   0 (see [summon_anywhere.md](summon_anywhere.md)).
6. **Per-Ash send-back**: vanilla only has `DisappearAll`, so pressing one Ash again sent back every group.
   The `call DisappearAll` of the `+0x28` branch in `sub_1404B92B0` (`0x1404B9587`, AOB
   `41 80 7F 28 00 74 08 49 8B CF E8 ...`) goes to `disappear_all_hook`. With the feature on, `+0x28` set
   and a `DISMISS_TARGET`, it temporarily marks every live entry of the other Ashes `disappear_requested`,
   calls the original, removes the marks, and restores the `+0x3C` the original just cleared if other
   groups remain. If the target Ash has no live spirit the original is not called. Every other reason
   (leaving the area, death, SpEffect 202) keeps vanilla (send back all). Log:
   `sent back Ash (SpEffect N): X spirit(s), Y other(s) kept`.
7. **FP/HP cost of an extra Ash** (`0x1403C0B79` FP, `0x1403C0C2F` HP): `cost_state` returns 2 (free) only
   when the pressed Ash itself still has live spirits (a send-back); otherwise the normal price.
8. **Can I afford it** in `CanUseItem` (`0x14069018A` HP, `0x14069022D` FP): two more
   `GetBuddyState(mgr, goodsId) == 2` checks zero the cost the check requires when the pressed Ash is
   the one out. With `CloneSpirit` that press summons, so they ask `cost_state` as well: free only when the
   press sends the Ash back. The goods row is `[rsp+0x50]` of the caller (and `[rbp-0x38]` in
   `sub_1403C0930`). A clone could otherwise be summoned with too little FP or HP.

**Not a Spirit Ash**: Reforged's Spirit-Severing Blade goes through the same `DoSummon` with a SpEffect
that belongs to no Ash. `ash_alive` returns `None` when the SpEffect has no chain, so `decide_state` and
`cost_state` keep vanilla behaviour (send back all, free).

All stubs read one byte (`ENABLED`) that a tick keeps in sync with the config, so F5 switches the behaviour
live without re-patching.

## CloneSpirit: summoning the same Ash again

Status: **Confirmed** in game (several copies, FP/HP cost and the affordability check). Needs
`MultiSpirit`; off by default.

`dismisses_on_reuse` replaces the check "Ash already out, so send it back". An Ash with no live spirit
summons as usual. When it has some, and the player's live spirits plus one cast (the chain length) stay
within `MaxSpirits`, `decide_state` returns 0 (summon more, still paying FP/HP because `cost_state`
returns 0); above that it sends that Ash back as before. `Update` no longer recalls the old group, so the
new group joins the same `groups` list (keyed by owner, not by Ash: `sub_1404B86D0`).

Checked in game: the second spawn of the same Ash goes through. **Not established**: whether the game
refuses a second spawn of the same BuddyParam or slot, and whether an identical
`active_summon_speffect_id` makes `Update` treat it as a duplicate cast. A Mimic Tear copies the player's
equipment on appearance, so each copy may differ. `MaxSpirits` (the band) is the ceiling for the decision;
the active character limit and the 80 slot set stay as in [slots_and_band.md](slots_and_band.md).

## Lessons and failures

- An upgraded Ash (+1 to +10) summoned again instead of being sent back: the lookup used `207004` directly
  and found nothing. The rounding `100 * (speffect / 100)` is now done in one place and shared by
  `decide_dismiss`, `disappear_all_hook` and `cost_state`.
- Mimic Tear: three `133201000` disappearing after about 8 s are its appearance effect, not a send-back.
- Test isolation: one run was contaminated by a third-party DLL left in the mod folder; keep the folder
  clean when testing.

## Conflicts

A third-party mod that patches the same `SummonBuddyManager` functions (a "multi spirit" or "max spirits"
mod) cannot be used together with this feature; the Nexus page says so.
