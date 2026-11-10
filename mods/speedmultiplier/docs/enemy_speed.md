# Enemy speed

Status: **Confirmed** in game on 2.7.1.0 (common enemies, two bosses, the Tree Sentinel). Not checked
with Seamless Co-op. Conventions:
[research notes](../../../docs/research/README.md).

## Question

How to give hostile enemies and bosses one speed (`[Enemy] All`) without touching allies, animals or
NPCs?

## Mechanism

`animation_speed` is written like the player's, one key for all: every enemy has its own animation
ids, so `group_of` does not apply.

`WorldChrMan.chr_sets` is walked (only `Active` and `ReadyForActivation` entries). A character counts
when all of these hold:

1. **`chr_type` is `Npc` or `Unk7`.** Bosses are `Unk7`; the Tree Sentinel is an `Npc`. The first
   version took only `Npc` and bosses did not move faster. There is no matching param column
   (`NpcParam.npcType` is close but not equal).
2. **`team_type` is not one of the non-enemy teams**: none, live, ghosts, ally, decoy (Torrent, 10),
   battle ally, invaders, neutral, charmed, host, co-op, friendly NPC, object, the mad phantoms and
   spirit summon (47). The others (6 enemy, 7 boss, 9 hostile ally, 27 hostile NPC and the unknown ones)
   are enemies. The list is `NOT_ENEMY_TEAMS`.
3. **`NpcParam.threatLv > 0`.** It is 0 for scarabs, deer, goats, sheep, boars, birds, turtles, the
   wandering noble, ballistae, dummies, bonfires and Torrent; bosses are 1 or more. `ChrIns` does not
   carry it, so a table of id to `threatLv` is built once from `shared::params::row_ids` and
   `get_row_by_index`.

`team` alone is not enough: goats share team 6 with common enemies.

Victims of a backstab or riposte are driven by `sync_victims` and skipped here ([critical_victim.md](critical_victim.md)).
When the key goes back to 1, the enemies written to are put back to 1.0 on the next walk.

## Open risks

- Under Seamless Co-op ([animation_speed.md](animation_speed.md)) the stub passes only the player's and Torrent's
  speed through, so enemies read 1.0.
- The Nomadic Merchant (team 27) can count as an enemy (`threatLv > 0`).
- Bosses that change phase, cutscenes and effects on their own clock may drift; none seen on the bosses
  tested.

## Tooling

`EnemyProbe = true` logs `Probe: lock-on ...` when a target is locked: set, `npc_param`, `chr_type`,
team, HP, animation. The target is matched by position nearest to `lock_on_target_position` (error of
8 to 25 m), so it is a guide; check it against `npc_param`.
