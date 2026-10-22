//! RiseArcher's projectile tuning on the live `Bullet` rows (Arrow, Great
//! Arrow, Radahn's Spear, Bolt), ported from the ~1000 per-ID lines of
//! `csv/RiseArcher.MASSEDIT`. `Bullet` has no field to filter by, so the rows
//! are data: each arrow / bolt family has a base ID plus fixed offsets for its
//! Ash of War variants (`FAMILIES` below encodes that once). Layout and
//! baseline: `docs/param_patching.md`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{Bullet, SoloParamRepository};
use eldenring::param::BULLET_PARAM_ST;

use common::{config, logger};

/// What an offset within a family means, i.e. which field(s) it tunes. Only
/// Rain of Arrows splits into separate rows (range / count / spread); every
/// other bullet is plain [Speed].
#[derive(Clone, Copy)]
enum Role {
    /// `initVellocity`/`maxVellocity` - plain projectile speed.
    Speed,
    /// `dist` (Attenuation Range - where damage falloff *starts*, not travel
    /// distance) - Rain of Arrows' spread-out sub-bullet for range.
    RainOfArrowsRange,
    /// `numShoot` - Rain of Arrows' sub-bullet controlling arrow count. Set
    /// to an absolute count rather than multiplied - the vanilla value is
    /// always 1, so "count" is a more honest ini key than a multiplier of an
    /// always-1 base.
    RainOfArrowsCount,
    /// `shootAngleYMaxRandom`/`shootAngleXMaxRandom` - Rain of Arrows'
    /// sub-bullet controlling the spread cone's angle.
    RainOfArrowsSpread,
}

/// The offsets that exist for a full-featured arrow (every AOW it can carry).
/// `+0` = the arrow itself, `+11/+20/+30` = Mighty Shot/Barrage/Enchanted
/// Shot (all just faster, like the base arrow), `+50..+54` = Rain of Arrows
/// split across 3 sub-bullets, `+70` = Through and Through.
const ARROW_OFFSETS: &[(u32, Role)] = &[
    (0, Role::Speed),
    (11, Role::Speed),
    (20, Role::Speed),
    (30, Role::Speed),
    (50, Role::Speed),
    (51, Role::RainOfArrowsRange),
    (52, Role::RainOfArrowsCount),
    (54, Role::RainOfArrowsSpread),
    (70, Role::Speed),
];

/// Great Arrows can't be fired with Mighty Shot/Barrage/Enchanted Shot, so
/// they skip straight from the base arrow to Rain of Arrows.
const GREAT_ARROW_OFFSETS: &[(u32, Role)] = &[
    (0, Role::Speed),
    (50, Role::Speed),
    (51, Role::RainOfArrowsRange),
    (52, Role::RainOfArrowsCount),
    (54, Role::RainOfArrowsSpread),
    (70, Role::Speed),
];

/// Radahn's Spear (Lion Greatbow's exclusive ammo) is the one irregular
/// case: no `+0`/`+70` rows exist for it at all - only Rain of Arrows.
const RADAHNS_SPEAR_OFFSETS: &[(u32, Role)] = &[
    (50, Role::Speed),
    (51, Role::RainOfArrowsRange),
    (52, Role::RainOfArrowsCount),
    (54, Role::RainOfArrowsSpread),
];

/// Bolts (crossbow/ballista ammo) never carry an Ash of War - just the one
/// plain bullet.
const BOLT_OFFSETS: &[(u32, Role)] = &[(0, Role::Speed)];

/// Base IDs for every regular Arrow type (Arrow, Fire Arrow, Serpent Arrow,
/// the various bone arrows, ...). Each one combines with [ARROW_OFFSETS].
const ARROW_BASES: &[u32] = &[
    20000000, 20000100, 20000200, 20000300, 20000400, 20000500, 20000600, 20000700, 20000800, 20000900,
    20001000, 20001100, 20001200, 20001300, 20001400, 20001500, 20001600, 20001700, 20001800, 20001900,
    20002000, 20002100, 20002300, 20002400, 20002500, 20002600, 20002700, 20002800, 20002900, 20003000,
    20003100, 20003200,
];

/// Base IDs for every Great Arrow type. Combines with [GREAT_ARROW_OFFSETS].
/// Note the gap at 20005400 - that base belongs to Radahn's Spear instead
/// (see [RADAHNS_SPEAR_BASE]), which doesn't have a `+0` row of its own.
const GREAT_ARROW_BASES: &[u32] = &[20005000, 20005100, 20005200, 20005300, 20005500, 20005600];

const RADAHNS_SPEAR_BASE: u32 = 20005400;

/// Base IDs for every Bolt type (crossbow bolts and ballista bolts/greatbolts
/// alike). Combines with [BOLT_OFFSETS].
const BOLT_BASES: &[u32] = &[
    20007000, 20007100, 20007200, 20007300, 20007400, 20007500, 20007600, 20007700, 20007800, 20007900,
    20008000, 20008100, 20008200, 20008300, 20008400, 20008500, 20008600, 20008700, 20008800, 20008900,
    20009000, 20009100, 20009200, 20009300,
];

struct Config {
    speed_multiplier: f64,
    range_multiplier: f64,
    spread_multiplier: f64,
    rain_of_arrows_count: u16,
}

impl Config {
    fn load() -> Self {
        Self {
            speed_multiplier: config::get_double("Bullet.SpeedMultiplier", 3.5),
            range_multiplier: config::get_double("Bullet.RangeMultiplier", 2.0),
            spread_multiplier: config::get_double("Bullet.SpreadMultiplier", 2.0),
            rain_of_arrows_count: config::get_int("Bullet.RainOfArrowsCount", 6).clamp(0, u16::MAX as i32) as u16,
        }
    }
}

fn scale_f32(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

/// The game's own original values that are scaled, by row ID, captured once
/// before any edit so every reload rescales from the true baseline.
/// `num_shoot` is set to an absolute count, so it needs none.
#[derive(Clone, Copy)]
struct Baseline {
    init_vellocity: f32,
    max_vellocity: f32,
    dist: f32,
    angle_y: f32,
    angle_x: f32,
}

/// Recovered from poisoning at the lock: `apply` is retried across a
/// `catch_unwind` (see `lib.rs`), and one panic must not poison it for good.
static ORIGINALS: Mutex<Option<HashMap<u32, Baseline>>> = Mutex::new(None);

fn snapshot(row: &BULLET_PARAM_ST) -> Baseline {
    Baseline {
        init_vellocity: row.init_vellocity(),
        max_vellocity: row.max_vellocity(),
        dist: row.dist(),
        angle_y: row.shoot_angle_y_max_random(),
        angle_x: row.shoot_angle_x_max_random(),
    }
}

fn apply_role(row: &mut BULLET_PARAM_ST, role: Role, baseline: &Baseline, cfg: &Config) {
    match role {
        Role::Speed => {
            row.set_init_vellocity(scale_f32(baseline.init_vellocity, cfg.speed_multiplier));
            row.set_max_vellocity(scale_f32(baseline.max_vellocity, cfg.speed_multiplier));
        }
        Role::RainOfArrowsRange => {
            row.set_dist(scale_f32(baseline.dist, cfg.range_multiplier));
        }
        Role::RainOfArrowsCount => {
            row.set_num_shoot(cfg.rain_of_arrows_count);
        }
        Role::RainOfArrowsSpread => {
            row.set_shoot_angle_y_max_random(scale_f32(baseline.angle_y, cfg.spread_multiplier));
            row.set_shoot_angle_x_max_random(scale_f32(baseline.angle_x, cfg.spread_multiplier));
        }
    }
}

/// Every Bullet ID RiseArcher tunes, paired with which fields to touch
/// there - the flattened combination of every family's bases and offsets.
fn all_ids() -> Vec<(u32, Role)> {
    let mut ids = Vec::new();
    for &base in ARROW_BASES {
        for &(offset, role) in ARROW_OFFSETS {
            ids.push((base + offset, role));
        }
    }
    for &base in GREAT_ARROW_BASES {
        for &(offset, role) in GREAT_ARROW_OFFSETS {
            ids.push((base + offset, role));
        }
    }
    for &(offset, role) in RADAHNS_SPEAR_OFFSETS {
        ids.push((RADAHNS_SPEAR_BASE + offset, role));
    }
    for &base in BOLT_BASES {
        for &(offset, role) in BOLT_OFFSETS {
            ids.push((base + offset, role));
        }
    }
    ids
}

/// Applies every bullet tuning to the live `Bullet` rows from each row's
/// cached original values (no compounding on reload). Returns how many rows
/// were found and touched; fewer than expected means the ID layout drifted
/// and the base lists need re-checking against a fresh export.
pub fn apply(repo: &mut SoloParamRepository) -> usize {
    let cfg = Config::load();
    let ids = all_ids();
    // ID -> row index, built from the param file itself: `get_mut` goes
    // through a lookup table that is shorter than the file on some modded
    // regulations (see `common::params`).
    let index_by_id = match common::params::row_ids_or_reason::<Bullet>(repo) {
        Ok(row_ids) => row_ids.into_iter().enumerate().map(|(index, id)| (id, index)).collect::<HashMap<u32, usize>>(),
        Err(reason) => {
            logger::warn(&format!("Bullet: couldn't read the row ids ({reason}), arrow tuning skipped"));
            return 0;
        }
    };

    let mut snapshot_guard = ORIGINALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let originals = snapshot_guard.get_or_insert_with(|| {
        let mut map = HashMap::new();
        for &(id, _role) in &ids {
            if let Some(row) = index_by_id.get(&id).and_then(|&index| repo.get_row_by_index_mut::<Bullet>(index)) {
                map.insert(id, snapshot(row));
            }
        }
        map
    });

    let mut changed = 0;
    let mut missing = 0;
    for &(id, role) in &ids {
        let row = index_by_id.get(&id).and_then(|&index| repo.get_row_by_index_mut::<Bullet>(index));
        match (row, originals.get(&id)) {
            (Some(row), Some(baseline)) => {
                apply_role(row, role, baseline, &cfg);
                changed += 1;
            }
            _ => missing += 1,
        }
    }

    if missing > 0 {
        logger::warn(&format!(
            "Bullet: {missing} expected ID(s) not found, the ID layout may have drifted (see bullet.rs)"
        ));
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_list_has_the_expected_size_and_no_duplicates() {
        let ids = all_ids();
        // 32 arrows x 9 + 6 great arrows x 6 + Radahn's Spear 4 + 24 bolts x 1.
        assert_eq!(ids.len(), 32 * 9 + 6 * 6 + 4 + 24);
        let unique: std::collections::HashSet<u32> = ids.iter().map(|(id, _)| *id).collect();
        assert_eq!(unique.len(), ids.len(), "two entries tune the same row");
    }

    /// The defaults in code and in the ini template are written twice; they
    /// must agree.
    #[test]
    fn code_defaults_match_the_ini_template() {
        let ini = include_str!("../RiseArcher.ini");
        let value = |key: &str| -> f64 {
            let line = ini.lines().find(|l| l.trim_start().starts_with(&format!("{key}="))).unwrap();
            line.split_once('=').unwrap().1.trim().parse().unwrap()
        };
        let cfg = Config::load();
        assert_eq!(value("Bullet.SpeedMultiplier"), cfg.speed_multiplier);
        assert_eq!(value("Bullet.RangeMultiplier"), cfg.range_multiplier);
        assert_eq!(value("Bullet.SpreadMultiplier"), cfg.spread_multiplier);
        assert_eq!(value("Bullet.RainOfArrowsCount"), cfg.rain_of_arrows_count as f64);
    }
}
