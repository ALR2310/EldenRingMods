# Getting a partner's position over Steam P2P (WHERE / HERE)

Status: **Confirmed** in game with Seamless Co-op (host and joiner, both ways). Conventions:
[research notes](../../../docs/research/README.md). Code: `src/net.rs`, `src/steam.rs`, `src/party.rs`. The warp
itself is in [warp_mechanism.md](warp_mechanism.md); roles and the invader block in
[roles_and_invaders.md](roles_and_invaders.md).

## Question

Why must the mod send positions over the network itself, and how does it do that safely?

## Why the position has to be sent

A survey of Seamless Co-op (two instances on one machine, the second Steam account in a sandbox):

- A partner **is** in `WorldChrMan.player_chr_set` and Seamless reports both as `chr_type Local`.
- `PlayerIns.current_block_id` and `block_position` are updated **only for oneself**; for a partner they are `-1`
  and `0`.
- The Havok position (`CSChrPhysicsModule.position`) of a partner is **live and exact when near** (about 0.9 m
  apart), but when that player goes far the entry stays and Havok drops to `(0, 0, 0)`: the machine no longer
  knows the position. `ChrIns.block_id` is always the old block, not reliable.

So each instance reads the block and position of itself (`PlayerIns.current_block_id` / `block_position`, always
right for the main player) and sends them. The Havok formula is only a fallback when near.

## Transport (`src/steam.rs`)

A minimal binding to `ISteamNetworkingMessages`, taken with `GetProcAddress` from `steam_api64.dll`, which the
game has loaded (flat C API, no Steamworks SDK or crate). Only the first fields of `SteamNetworkingMessage_t` are
read and the message is released through the export `SteamAPI_SteamNetworkingMessage_t_Release`, so no deeper
struct layout is assumed. Its own channel `0x5354` ("ST"); a chat mod uses 42, so they do not clash. Messages are
sent **reliably** (one `WHERE` and one `HERE` per teleport; a lost packet means waiting for the timeout). The
player list comes from `CSSessionManager.players` (the singleton is found by name, no version lock; each entry
has `steam_id`, `steam_name`, `is_local_player`); confirmed usable under Seamless Co-op.

## From broadcast to asking on demand

The first version **broadcast** the position every 500 ms to everyone (a 66-byte packet, magic `STP1`): simple
and always has data to study, but only a stopgap. The position received was only 0.0 to 0.4 s old, a warp across
the loading screen worked and the co-op session stayed up. The real version **asks when needed**: no continuous
broadcast when nobody teleports, the position is always the latest, and it is not shown to everyone who has the
mod. (`CSSessionManager` also lists strangers when Seamless's `cooppassword` was not changed; the broadcast
version sent the position to them too, although the packet was ignored as they lack the mod.)

- **The asker** sends `WHERE` (16 bytes: magic `STW1`, the asker's SteamID, a request id) to exactly one player
  and waits at most **10 seconds** (2 seconds was too short: the partner may be loading or respawning and
  answers only when back in the world).
- **The one asked** answers `HERE` (70 bytes: magic `STH1`, SteamID, request id, block, x/y/z/yaw in the block,
  character name). If `WHERE` arrives while loading or respawning, it is held and answered as soon as the world
  is back (within 10 seconds); past that it is dropped and the asker says "did not respond". One asker may have
  at most **one** deferred `WHERE` (a new one replaces the old).
- A late `HERE` or one of an old request is ignored (SteamID and request id compared).
- `AcceptSessionWithUser` is called for everyone in the session every 2 seconds; without it the first `WHERE`
  from someone never contacted before is dropped by Steam. The session list is read only when needed.

## Answer only when really "in the world"

A fault seen: warping to someone on a loading screen **threw the warper to the title screen** (both ways). Cause:
while loading, `WorldChrMan` still has the old main player, so the machine asked answered `HERE` at once with the
position of the map being left. Fix: answer only after being "in the world" continuously for 2 seconds
(`SETTLE_TIME`), where "in the world" means a main player **and** `GameMan.warp_requested` false **and**
`CSSessionManager.protocol_state` not in a reload phase (`WaitInitData`, `WaitReloadWait`, `WaitReload`,
`WaitReload2`, `WaitReentryToMap`). Until then the request is held as above. **Not verified**: whether Seamless
updates `protocol_state`; if not, `warp_requested` and the 2 seconds are still the main guard. The warper also
does not warp while it has a move-map pending itself.

## Remembering a partner's role while `PlayerIns` is gone

The joiner's first click on the host was wrongly refused: the host was on a loading screen, so the host's
`player_chr_set` briefly lacked the joiner's `PlayerIns`, the role became `None`, and the rule "unknown role is
not trusted" rejected it at the check made **before** "not yet in the world: hold and answer later" (so the
request was dropped instead of deferred). The log first looked as if the host were far away; it was loading. Fix
(`party.rs`): `KNOWN_ROLES` remembers the last `chr_type` seen for each SteamID, updated each time the session
list is read (at least every 2 seconds), and a player without a `PlayerIns` uses the remembered role; only
someone **never seen** counts as unknown.

## One key press, two instances teleport

Seen with two instances on one desktop: pressing the key in one window made **both** games warp. The first
reading (a fromsoftware-rs function only takes keys of the process's main window) was **wrong**:
`eldenring::util::input::is_key_pressed` uses `GetKeyState`, which the mod calls from the game's `FrameBegin`
task; that thread has no input queue of its own, so it sees the shared key state of the whole desktop. Fix: use
`shared::input::is_key_pressed` ([input](../../../shared/docs/input.md)), which checks that the foreground window
belongs to this process before reading the key (an unfocused instance reads nothing) and can be called from any
thread (`GetAsyncKeyState`).

## Forged packets

Earlier the sender came only from the SteamID inside the payload and request ids were 1, 2, 3...: a player in the
session with a modified mod could send a fake `HERE` at the right moment and force a warp to a garbage block or
position (crash, falling out of the map, stuck). Now:

- `steam.rs` also reads `m_identityPeer` of `SteamNetworkingMessage_t` (offset 16, after `m_pData`, `m_cbSize`
  and `m_conn`): the sender is authenticated by **Steam**. `net.rs` drops every packet whose payload SteamID
  differs from the real sender (`sender_matches`).
- The request id is random (`RandomState` and the time, never 0).
- A `HERE` with block `-1`, NaN or infinite coordinates, or above 100,000 is dropped (`spot_is_sane`); the asker
  keeps waiting and reports as usual on timeout.
- Retested: `WHERE` with a random id, `HERE` after 28 ms, a successful warp, no `Dropped a packet claiming to be
  from ...` line in either log, so reading `m_identityPeer` at offset 16 is the right layout. A second test both
  ways: `WHERE` to `HERE` in **33 ms**.

## Limits

- Anyone in the session who has the mod (and is not hostile) can ask for the player's position: set a private
  `cooppassword` in Seamless.
- It depends on the memory layout per game build for the session list, the position and `protocol_state` (read
  through fromsoftware-rs).
- Sessions of three or more players were not tried.
