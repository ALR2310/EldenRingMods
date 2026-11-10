# Status buildup bars of the locked enemy

Status: **Inferred**. The data location and the module layout were checked statically in IDA on 2.6.2.0,
2.7.0.0, 2.7.1.0 and 2.7.2.0. The direction of the values (counting down from the maximum) and the way
immune enemies are recognised come from a third-party HUD mod that does the same, and were not
re-measured in game. Conventions: [research notes](../../../docs/research/README.md). Code:
`Target.Buildup`, `target.rs::read_buildup`, `ui.rs::draw_buildup`.

## Question

Under the health and posture bars of the locked enemy, show its seven buildup bars (poison, scarlet
rot, bleed, death blight, frost, sleep, madness) with the game's own icons and frames. Where does the
game keep the values?

## Mechanism

- The data is in the enemy's own resist module: `ChrIns.modules` (+0x190), fifth pointer (+0x20), a
  `CSChrResistModule`. Seven current values at +0x10 and seven maximums at +0x2C, in the order poison,
  scarlet rot, bleed, death blight, frost, sleep, madness.
- The values **count down**: a bar shows `maximum - value`, the part built up. A bar is drawn only when
  the maximum is above 0 and the built-up part is above 0, so an enemy with nothing built up has no bar.
- An enemy immune to a kind is read from `NpcParam` (`resist_poison`, `resist_desease` = scarlet rot,
  `resist_blood`, `resist_curse` = death blight, `resist_freeze`, `resist_sleep`, `resist_madness`); a
  value of 999 or more is immune and is not drawn.
- The crate `eldenring` has no type for this module, so the offsets are read raw, like the poise in
  `target.rs`, with a bounds check and `const` asserts on the layout.

## Verification (static, 4 builds)

- The initialiser of `CSChrResistModule` is identical on all four builds (RVA 43D790, 43DCF0, 43DCF0,
  43DD80): vtable assigned, then a loop of 7 that clears seven arrays of seven elements at `+0x10`,
  `+0x2C`, `+0x48`, `+0x64`, `+0x80` (integers) and `+0x9C` (floats, initialised to 1.0), then `+0xB8`
  (dword) and `+0xBC` (byte). This matches the layout above.
- The module pointer at +0x20 of `ChrInsModuleContainer` (`resist: usize`, fifth after `data`,
  `action_flag`, `behavior_script`, `time_act`) matches fromsoftware-rs.
- **Not checked**: what the arrays at +0x48, +0x64 and +0x80 are (thresholds or factors), and the count-down
  direction of the first two. The module's update function is not reached through its vtable (which only
  has the destructor), so the writes to `+0x10` would have to be found from the damage and status system.

## Icons and frames

The game's menu sprite sheets already hold them, read as the other sprites are in `map/sprites.rs`; no art
is shipped:

- `SB_Condition`: `MENU_RE_Poison`, `MENU_RE_Plague` (scarlet rot), `MENU_RE_bleed`, `MENU_RE_Death`
  (death blight), `MENU_RE_Cold` (frost), `MENU_RE_Sleep`, `MENU_RE_Mad`; about 126 x 108 px each.
- `SB_FE`: `MENU_FL_ConditionWaku` (frame 840 x 48) and `MENU_FL_ConditionWaku_L` (left cap 12 x 48).
- `SB_FE_01`: `MENU_FL_Condition_Bar` and `MENU_FL_Condition_BarBlack` (bar and background, 820 x 26), the
  same family as the `Bar.png` used for the health bar.

## Layout

One row of 25 px (at 1080p) per kind under the beige line: the `SB_Condition` icon 24 px high with the
transparent margin of the sprite cropped away (at 16 px it looked too small), a bar 12 px thick (like the
health bar) tinted by kind and joined to the icon, rows 1 px apart. With `Target.Numbers` on each bar has
`built up / maximum` in the middle at the same 10 px size as the health numbers. The posture bar is 12 px
thick too, without numbers. A line `Buildup: <kind> <built up>/<maximum>` is logged whenever the set of kinds
building up changes.

## Open risks

- The values really count down from the maximum: hit an enemy with a poison or bleed weapon and watch.
- The immunity threshold: hit an immune boss and see what maximum the module holds (999 or more, or 0).
- Multi-phase bosses, enemies killed or swapped while locked on.
- **Co-op**: the third-party HUD mod's buildup was reported not to sync between two players. **Hypothesis**:
  each player reads the resist module of the enemy copy on their own machine and sees only what that
  machine knows; the real state is decided on the host. Test: two machines hit one enemy and the
  `Buildup:` log lines are compared.
