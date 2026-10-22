//! On-demand position exchange over Steam P2P (`steam`): the requester sends
//! `WHERE` to one partner and waits up to [REPLY_TIMEOUT]; a session member
//! that gets a `WHERE` answers `HERE` with its main player's block and
//! block-local position (right away, or once settled back in the world).
//! Someone outside the session gets no answer. The packet bytes are in
//! `wire.rs`; the design and the hardening: `docs/position_exchange.md`.

use std::time::{Duration, Instant};

use eldenring::cs::{CSSessionManager, ProtocolState, WorldChrMan};
use fromsoftware_shared::FromStatic;

use crate::party::{self, Member};
use crate::steam::{Received, SteamMessages};
use crate::warp::Spot;
use crate::wire::{Here, Packet, Where, NAME_UNITS, spot_is_sane};
use common::logger;

/// Our own P2P channel - SoulsChat uses 42; nothing else on the same Steam
/// session sees these messages, and we never see theirs.
const CHANNEL: i32 = 0x5354; // "ST"

// Generous on purpose: the partner may be mid-load (fast travel, respawn) and
// only answers once back in the world. Was 2 s in the first draft - too short.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// A peer's messages are dropped by Steam until we accept its session (or
/// send to it first) - re-accepted for every partner this often so a
/// partner's first `WHERE` isn't lost.
const ACCEPT_EVERY: Duration = Duration::from_secs(2);

/// How long we must have been continuously in the world before answering a
/// `WHERE`. Co-op test 5 (2026-09-25): warping to a player who is still on a
/// loading screen (fast travel to a grace) kicked the warping player back to
/// the title screen, both ways round - while loading, `WorldChrMan` can
/// still hold the old main player, so the old code answered with a position
/// in a map the answerer was leaving. Warping to someone done loading works.
const SETTLE_TIME: Duration = Duration::from_secs(2);

pub enum Event {
    Arrived { character_name: String, spot: Spot },
    TimedOut { target_name: String },
}

struct Pending {
    target: u64,
    target_name: String,
    request_id: u32,
    sent: Instant,
}

/// A partner's `WHERE` that arrived while we weren't in the world (loading,
/// respawning) - answered as soon as we are, until it would be too late for
/// the asker anyway.
struct Deferred {
    asker: u64,
    asker_name: String,
    request_id: u32,
    received: Instant,
}

pub struct Net {
    steam: SteamMessages,
    /// Since when we've been continuously "in the world" (see
    /// [in_world_now]); `None` while loading / in menus.
    in_world_since: Option<Instant>,
    pending: Option<Pending>,
    deferred: Vec<Deferred>,
    last_accept: Option<Instant>,
}

/// Unpredictable, non-zero request id: `RandomState` is seeded from the OS
/// per call; the time is mixed in as well.
fn random_request_id() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    (h.finish() as u32).max(1)
}

/// The sender Steam reports must be the one the payload claims.
fn sender_matches(packet: &Received, claimed: u64) -> bool {
    packet.sender == Some(claimed)
}

/// Loaded into the world right now: a main player exists, no move-map is
/// pending (`GameMan.warp_requested`), and the session isn't in one of its
/// (re)load phases. Any one alone isn't enough - see [SETTLE_TIME].
fn in_world_now() -> bool {
    if common::player::main_player_chr_ins_ptr().is_none() || crate::warp::warp_pending() {
        return false;
    }
    match unsafe { CSSessionManager::instance() } {
        Ok(session) => !matches!(
            session.protocol_state,
            ProtocolState::WaitInitData
                | ProtocolState::WaitReloadWait
                | ProtocolState::WaitReload
                | ProtocolState::WaitReload2
                | ProtocolState::WaitReentryToMap
        ),
        Err(_) => true,
    }
}

/// Our main player's position + character name as a `HERE`, `None` while
/// loading / in the menus. Only called once [Net::settled].
fn own_here(own_steam_id: u64, request_id: u32) -> Option<Vec<u8>> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let player = world_chr_man.main_player.as_ref()?;
    let pos = &player.block_position;
    let spot = Spot { block_id: player.current_block_id.0, x: pos.x, y: pos.y, z: pos.z, yaw: pos.yaw };
    let name: [u16; NAME_UNITS] = unsafe { player.player_game_data.as_ref() }.character_name;
    Some(Here { sender: own_steam_id, request_id, spot, name }.encode())
}

impl Net {
    pub fn new(steam: SteamMessages) -> Self {
        Self { steam, in_world_since: None, pending: None, deferred: Vec::new(), last_accept: None }
    }

    /// Asks `target` for its position. Replaces any request still waiting.
    pub fn request(&mut self, own_steam_id: u64, target: &Member) {
        let request_id = random_request_id();
        let result = self.steam.send_to(target.steam_id, &Where { sender: own_steam_id, request_id }.encode(), CHANNEL);
        logger::log(&format!(
            "WHERE -> '{}' ({}) id={request_id} send={result}",
            target.display_name(),
            target.steam_id
        ));
        self.pending = Some(Pending {
            target: target.steam_id,
            target_name: target.display_name().to_string(),
            request_id,
            sent: Instant::now(),
        });
    }

    /// Per-frame step, from the game's own task thread: answers partners'
    /// `WHERE`s, and returns the outcome of our own pending request, if any.
    pub fn tick(&mut self) -> Option<Event> {
        if in_world_now() {
            self.in_world_since.get_or_insert_with(Instant::now);
        } else {
            self.in_world_since = None;
        }
        let packets = self.steam.receive_all(CHANNEL);
        let accept_due = self.last_accept.is_none_or(|t| t.elapsed() >= ACCEPT_EVERY);
        self.deferred.retain(|d| d.received.elapsed() < REPLY_TIMEOUT);
        // Only walk the session/character lists when something needs them -
        // this runs every frame.
        let needs_members = accept_due || !packets.is_empty() || !self.deferred.is_empty();
        let members = if needs_members { party::members() } else { Vec::new() };

        if !self.deferred.is_empty() {
            self.flush_deferred(&members);
        }

        if accept_due {
            self.last_accept = Some(Instant::now());
            for m in members.iter().filter(|m| m.is_teleport_target()) {
                self.steam.accept(m.steam_id);
            }
        }

        let mut event = None;
        for packet in packets {
            let Some(decoded) = Packet::decode(&packet.payload) else {
                continue;
            };
            if !sender_matches(&packet, decoded.sender()) {
                logger::log(&format!(
                    "Net: dropped packet, claimed sender {} != Steam {:?}",
                    decoded.sender(),
                    packet.sender
                ));
                continue;
            }
            match decoded {
                Packet::Where(w) => self.answer_where(&members, w.sender, w.request_id),
                Packet::Here(h) => {
                    if let Some(e) = self.take_here(h) {
                        event = Some(e);
                    }
                }
            }
        }

        if event.is_none()
            && let Some(p) = self.pending.as_ref().filter(|p| p.sent.elapsed() >= REPLY_TIMEOUT)
        {
            logger::log(&format!("WHERE: no HERE from '{}' within {:?}", p.target_name, REPLY_TIMEOUT));
            event = Some(Event::TimedOut { target_name: p.target_name.clone() });
            self.pending = None;
        }
        event
    }

    fn answer_where(&mut self, members: &[Member], from: u64, request_id: u32) {
        let Some(asker) = members.iter().find(|m| m.steam_id == from) else {
            logger::log(&format!("WHERE <- {from}: ignored (not in session)"));
            return;
        };
        if asker.is_hostile() || party::own_is_hostile() {
            logger::log(&format!(
                "WHERE <- '{}': ignored (hostile)",
                asker.display_name()
            ));
            return;
        }
        let Some(own) = party::own_steam_id(members) else {
            return;
        };
        let reply = if self.settled() { own_here(own, request_id) } else { None };
        let Some(reply) = reply else {
            logger::log(&format!(
                "WHERE <- '{}': deferred (not in world)",
                asker.display_name()
            ));
            // One deferred request per asker: a newer WHERE replaces theirs.
            self.deferred.retain(|d| d.asker != from);
            self.deferred.push(Deferred {
                asker: from,
                asker_name: asker.display_name().to_string(),
                request_id,
                received: Instant::now(),
            });
            return;
        };
        let result = self.steam.send_to(from, &reply, CHANNEL);
        logger::log(&format!(
            "HERE -> '{}' id={request_id} send={result}",
            asker.display_name()
        ));
    }

    /// Answers every deferred `WHERE` once we're back in the world.
    fn flush_deferred(&mut self, members: &[Member]) {
        let Some(own) = party::own_steam_id(members) else {
            return;
        };
        if !self.settled() {
            return;
        }
        // Re-checked: a deferred asker (or we) may have turned hostile since.
        let own_hostile = party::own_is_hostile();
        self.deferred.retain(|d| {
            let allowed = !own_hostile && members.iter().any(|m| m.steam_id == d.asker && !m.is_hostile());
            if !allowed {
                logger::log(&format!("WHERE <- '{}': deferred dropped (hostile)", d.asker_name));
            }
            allowed
        });
        let mut still_waiting = Vec::new();
        for d in std::mem::take(&mut self.deferred) {
            match own_here(own, d.request_id) {
                Some(reply) => {
                    let result = self.steam.send_to(d.asker, &reply, CHANNEL);
                    logger::log(&format!(
                        "HERE -> '{}' id={} deferred={:.1}s send={result}",
                        d.asker_name,
                        d.request_id,
                        d.received.elapsed().as_secs_f32()
                    ));
                }
                None => still_waiting.push(d),
            }
        }
        self.deferred = still_waiting;
    }

    fn settled(&self) -> bool {
        self.in_world_since.is_some_and(|t| t.elapsed() >= SETTLE_TIME)
    }

    fn take_here(&mut self, here: Here) -> Option<Event> {
        let pending = self.pending.as_ref()?;
        if pending.target != here.sender || pending.request_id != here.request_id {
            return None; // late answer to an older / replaced request
        }
        let spot = here.spot;
        let character_name = party::character_name(&here.name);
        if !spot_is_sane(&spot) {
            logger::log(&format!(
                "HERE <- '{character_name}': dropped (unusable position, block {:#010X}, ({}, {}, {}))",
                spot.block_id, spot.x, spot.y, spot.z
            ));
            return None; // keep waiting: times out normally if nothing valid comes
        }
        logger::log(&format!(
            "HERE <- '{character_name}' id={} {:.0}ms block {:#010X} local ({:.2}, {:.2}, {:.2})",
            here.request_id,
            pending.sent.elapsed().as_secs_f32() * 1000.0,
            spot.block_id,
            spot.x,
            spot.y,
            spot.z
        ));
        self.pending = None;
        Some(Event::Arrived { character_name, spot })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(sender: Option<u64>) -> Received {
        Received { payload: Where { sender: 42, request_id: 7 }.encode(), sender }
    }

    #[test]
    fn sender_must_match_steam_identity() {
        assert!(sender_matches(&packet(Some(42)), 42));
        assert!(!sender_matches(&packet(Some(43)), 42));
        assert!(!sender_matches(&packet(None), 42));
    }

    #[test]
    fn request_ids_are_nonzero_and_vary() {
        let ids: std::collections::HashSet<u32> = (0..32).map(|_| random_request_id()).collect();
        assert!(!ids.contains(&0));
        assert!(ids.len() > 1);
    }
}
