# Player roles and blocking teleport between hostile players

Status: **Confirmed** in game from the invader side (the host side was not tested separately). Conventions:
[research notes](../../../docs/research/README.md). Code: `src/party.rs`, `src/net.rs`, `src/ui.rs`. How the two
machines exchange the position: [position_exchange.md](position_exchange.md).

## Roles in the menu

Every row shows the role on the right (`Member::role`): `Host` (from `SessionManagerPlayerEntry.is_host`), then by
`chr_type`: `Co-op` (Seamless Co-op reports everyone as `Local`), `Cooperator`, `Invader`, `Hunter`,
`Bloody Finger`, `Recusant`, `Arena`, `Other`; `Unknown` when the character was never seen (that player has not
reached the world, no `PlayerIns` yet; not an error). Colours: Host gold, hostile light red, Unknown and Other
grey. The information joins `CSSessionManager.players` (SteamID, Steam name) with `player_chr_set` (character
name, `chr_type`) through `PlayerIns.session_manager_player_entry.steam_id`. `KNOWN_ROLES` remembers the last
role seen, to show it while the `PlayerIns` is briefly gone.

## Stages of the rule

1. **Partners only**: only players not hostile and with a `PlayerIns` count.
2. **No filter, show everyone**: every player in the session is a teleport target and gets an answer to `WHERE`,
   with the role shown so the player knows.
3. **Block between hostile players**: below; the current rule.

## Current rule

A report on the Nexus page: "make sure this doesn't work for invaders, otherwise Reds are gonna tele to hosts".
True: from stage 2 on, an invader in the same Seamless Co-op session could teleport straight to the host,
ruining PvP. The rule: a pair can teleport only when **neither side is hostile**.

- **Hostile** (`party::is_hostile_type`): `Duelist` (Invader), `BloodyFinger` / `FesteringBloodyFinger` /
  `BloodyFingerNpc`, `Recusant` / `RecusantNpc`. **A Hunter (`BluePhantom`) is still allowed** because it comes to
  help the host.
- **The asker**: `party::own_is_hostile()` (reads `chr_type` of `WorldChrMan.main_player`): while invading, the
  menu shows only "Teleport is disabled while invading." and `request_partner_position` refuses. Other hostile
  players are not in the list (`Member::is_teleport_target` excludes `Member::is_hostile()`).
- **The one asked (more important)**: `Net::answer_where` ignores a `WHERE` if the asker is hostile or oneself is
  hostile, so the host stays protected even if an invader uses a modified build of the mod. `flush_deferred`
  checks again because the asker's role can change while waiting.
- An unknown role (`Unknown`, never seen) counts as not hostile. There is no config key to switch the rule off.

## Verification

Extra log lines (only on change): `Own role: <ChrType>.` for oneself and `Role of '<name>': <ChrType>.` for each
player. In game: invading another world gave `Own role: Duelist.` and the menu said "Teleport is disabled while
invading."; going back gave `Own role: Local.`. The invader's machine also sees itself as `Duelist` through
`player_chr_set` (the same path the host's machine uses to recognise an invader). The host (invaded) side was not
tested separately.

## Limits

Sessions of three or more players were not tried. Teleport is not blocked when the other player is riding Torrent
in the air, in a boss fight or dying.
