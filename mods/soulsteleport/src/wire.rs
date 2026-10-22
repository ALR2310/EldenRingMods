//! The two packets of the position exchange, as bytes: `WHERE` (asks a
//! partner for its position) and `HERE` (the answer). No game or Steam types,
//! so the format is unit tested. Why the exchange is on demand and how it is
//! hardened: `docs/position_exchange.md`.

use crate::warp::Spot;

pub const MAGIC_WHERE: [u8; 4] = *b"STW1";
pub const MAGIC_HERE: [u8; 4] = *b"STH1";
/// Same size as `PlayerGameData.character_name`.
pub const NAME_UNITS: usize = 17;
/// magic + sender SteamID + request id.
pub const WHERE_LEN: usize = 4 + 8 + 4;
/// magic + sender SteamID + request id + block id + 4 floats + name.
pub const HERE_LEN: usize = 4 + 8 + 4 + 4 + 4 * 4 + NAME_UNITS * 2;

/// Block-local coordinates beyond this are garbage, not a real position
/// (blocks are a few km across at most, even the overworld tiles).
const MAX_ABS_COORD: f32 = 100_000.0;

/// Whether a position can be warped to: a real block and finite, plausible
/// coordinates.
pub fn spot_is_sane(spot: &Spot) -> bool {
    spot.block_id != -1
        && [spot.x, spot.y, spot.z].iter().all(|v| v.is_finite() && v.abs() < MAX_ABS_COORD)
        && spot.yaw.is_finite()
}

pub struct Where {
    pub sender: u64,
    pub request_id: u32,
}

pub struct Here {
    pub sender: u64,
    pub request_id: u32,
    pub spot: Spot,
    pub name: [u16; NAME_UNITS],
}

pub enum Packet {
    Where(Where),
    Here(Here),
}

fn u32_at(p: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(p[i..i + 4].try_into().unwrap())
}

fn u64_at(p: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(p[i..i + 8].try_into().unwrap())
}

fn f32_at(p: &[u8], i: usize) -> f32 {
    f32::from_le_bytes(p[i..i + 4].try_into().unwrap())
}

impl Where {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = MAGIC_WHERE.to_vec();
        out.extend_from_slice(&self.sender.to_le_bytes());
        out.extend_from_slice(&self.request_id.to_le_bytes());
        out
    }
}

impl Here {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = MAGIC_HERE.to_vec();
        out.extend_from_slice(&self.sender.to_le_bytes());
        out.extend_from_slice(&self.request_id.to_le_bytes());
        out.extend_from_slice(&self.spot.block_id.to_le_bytes());
        for v in [self.spot.x, self.spot.y, self.spot.z, self.spot.yaw] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for unit in self.name {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out
    }
}

impl Packet {
    /// The packet in `p`, or `None` if it isn't exactly a `WHERE` or a `HERE`
    /// (wrong magic or length).
    pub fn decode(p: &[u8]) -> Option<Packet> {
        if p.len() == WHERE_LEN && p[..4] == MAGIC_WHERE {
            return Some(Packet::Where(Where { sender: u64_at(p, 4), request_id: u32_at(p, 12) }));
        }
        if p.len() == HERE_LEN && p[..4] == MAGIC_HERE {
            let spot = Spot {
                block_id: u32_at(p, 16) as i32,
                x: f32_at(p, 20),
                y: f32_at(p, 24),
                z: f32_at(p, 28),
                yaw: f32_at(p, 32),
            };
            let name = std::array::from_fn(|k| u16::from_le_bytes([p[36 + k * 2], p[37 + k * 2]]));
            return Some(Packet::Here(Here { sender: u64_at(p, 4), request_id: u32_at(p, 12), spot, name }));
        }
        None
    }

    /// The SteamID the payload claims to come from (same offset in both kinds).
    pub fn sender(&self) -> u64 {
        match self {
            Packet::Where(w) => w.sender,
            Packet::Here(h) => h.sender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot() -> Spot {
        Spot { block_id: 0x3C2A2500, x: 15.0, y: 110.0, z: -16.5, yaw: 1.25 }
    }

    #[test]
    fn where_round_trips() {
        let bytes = Where { sender: 7656119800000042, request_id: 99 }.encode();
        assert_eq!(bytes.len(), WHERE_LEN);
        let Some(Packet::Where(w)) = Packet::decode(&bytes) else { panic!("not a WHERE") };
        assert_eq!((w.sender, w.request_id), (7656119800000042, 99));
    }

    #[test]
    fn here_round_trips_with_the_name() {
        let mut name = [0u16; NAME_UNITS];
        for (unit, c) in name.iter_mut().zip("Tarnished".encode_utf16()) {
            *unit = c;
        }
        let bytes = Here { sender: 5, request_id: 6, spot: spot(), name }.encode();
        assert_eq!(bytes.len(), HERE_LEN);
        let Some(Packet::Here(h)) = Packet::decode(&bytes) else { panic!("not a HERE") };
        assert_eq!((h.sender, h.request_id), (5, 6));
        assert_eq!(h.spot.block_id, 0x3C2A2500);
        assert_eq!((h.spot.x, h.spot.y, h.spot.z, h.spot.yaw), (15.0, 110.0, -16.5, 1.25));
        assert_eq!(h.name, name);
    }

    #[test]
    fn a_negative_block_id_survives_the_round_trip() {
        let bytes = Here { sender: 1, request_id: 1, spot: Spot { block_id: -1, ..spot() }, name: [0; NAME_UNITS] }.encode();
        let Some(Packet::Here(h)) = Packet::decode(&bytes) else { panic!() };
        assert_eq!(h.spot.block_id, -1);
    }

    #[test]
    fn wrong_magic_or_length_is_not_a_packet() {
        let good = Where { sender: 1, request_id: 2 }.encode();
        assert!(Packet::decode(&good).is_some());
        assert!(Packet::decode(&good[..good.len() - 1]).is_none()); // truncated
        let mut extra = good.clone();
        extra.push(0);
        assert!(Packet::decode(&extra).is_none()); // padded
        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        assert!(Packet::decode(&bad_magic).is_none());
        // A WHERE with a HERE's length (and the reverse) is not accepted.
        let mut mixed = vec![0u8; HERE_LEN];
        mixed[..4].copy_from_slice(&MAGIC_WHERE);
        assert!(Packet::decode(&mixed).is_none());
        assert!(Packet::decode(&[]).is_none());
    }

    #[test]
    fn the_claimed_sender_is_the_same_for_both_kinds() {
        assert_eq!(Packet::decode(&Where { sender: 42, request_id: 1 }.encode()).unwrap().sender(), 42);
        let here = Here { sender: 43, request_id: 1, spot: spot(), name: [0; NAME_UNITS] };
        assert_eq!(Packet::decode(&here.encode()).unwrap().sender(), 43);
    }

    #[test]
    fn rejects_unusable_positions() {
        assert!(spot_is_sane(&spot()));
        assert!(!spot_is_sane(&Spot { block_id: -1, ..spot() }));
        assert!(!spot_is_sane(&Spot { x: f32::NAN, ..spot() }));
        assert!(!spot_is_sane(&Spot { y: f32::INFINITY, ..spot() }));
        assert!(!spot_is_sane(&Spot { z: 1.0e9, ..spot() }));
        assert!(!spot_is_sane(&Spot { yaw: f32::NAN, ..spot() }));
    }
}
