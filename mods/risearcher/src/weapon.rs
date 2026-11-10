//! RiseArcher's weapon buffs on the live `EquipParamWeapon` rows (bows, crossbows, ballistas,
//! arrows, bolts), ported from `RiseArcher.MASSEDIT` with every multiplier coming from the config.
//! Row filters, fields and the baseline snapshot: `docs/param_patching.md`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{EquipParamWeapon, SoloParamRepository};
use eldenring::param::EQUIP_PARAM_WEAPON_ST;

use crate::config::{self, Config};

/// FromSoftware's marker for an NPC-only duplicate row, hidden from the player's menu. It is always
/// excluded, like the `!prop sortId 9999999` filter on every line of the original Mass Edit.
const NPC_ONLY_SORT_ID: i32 = 9999999;
const WEAPON_CATEGORY_BOW: u8 = 10;
/// Crossbow AND Ballista share this category, split by `wepType`.
const WEAPON_CATEGORY_CROSSBOW_FAMILY: u8 = 11;
const WEP_TYPE_CROSSBOW: u16 = 55;
const WEP_TYPE_BALLISTA: u16 = 56;
const WEAPON_CATEGORY_ARROW: u8 = 13;
const WEAPON_CATEGORY_BOLT: u8 = 14;

/// Which of the 5 buckets a row falls into: the one place that decides both what is relevant and
/// which multipliers apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeaponKind {
    Bow,
    Crossbow,
    Ballista,
    Arrow,
    Bolt,
}

fn weapon_kind(category: u8, wep_type: u16) -> Option<WeaponKind> {
    match category {
        WEAPON_CATEGORY_BOW => Some(WeaponKind::Bow),
        WEAPON_CATEGORY_CROSSBOW_FAMILY if wep_type == WEP_TYPE_CROSSBOW => {
            Some(WeaponKind::Crossbow)
        }
        WEAPON_CATEGORY_CROSSBOW_FAMILY if wep_type == WEP_TYPE_BALLISTA => {
            Some(WeaponKind::Ballista)
        }
        WEAPON_CATEGORY_ARROW => Some(WeaponKind::Arrow),
        WEAPON_CATEGORY_BOLT => Some(WeaponKind::Bolt),
        _ => None,
    }
}

/// The game's own values that are scaled, by row index, captured once before any edit so every
/// reload rescales from the true baseline instead of compounding.
#[derive(Clone, Copy)]
struct Baseline {
    attack: [u16; 5],
    correct: [f32; 5],
    weight: f32,
    sell_value: i32,
}

/// A poisoned lock is still used: `apply` is retried across a `catch_unwind` (see `lib.rs`), and
/// one panic while the lock was held must not turn every retry into a second panic.
static ORIGINALS: Mutex<Option<HashMap<usize, Baseline>>> = Mutex::new(None);

/// Whether `row` is one RiseArcher touches: not an NPC-only duplicate, and a bow, crossbow, ballista,
/// arrow or bolt.
fn is_relevant(row: &EQUIP_PARAM_WEAPON_ST) -> bool {
    row.sort_id() != NPC_ONLY_SORT_ID
        && weapon_kind(row.weapon_category(), row.wep_type()).is_some()
}

fn snapshot(row: &EQUIP_PARAM_WEAPON_ST) -> Baseline {
    Baseline {
        attack: [
            row.attack_base_physics(),
            row.attack_base_magic(),
            row.attack_base_fire(),
            row.attack_base_thunder(),
            row.attack_base_dark(),
        ],
        correct: [
            row.correct_strength(),
            row.correct_agility(),
            row.correct_magic(),
            row.correct_faith(),
            row.correct_luck(),
        ],
        weight: row.weight(),
        sell_value: row.sell_value(),
    }
}

/// A carry limit as the game's `u8`.
fn max_quantity(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn scale_u16(value: u16, factor: f64) -> u16 {
    (value as f64 * factor).round().clamp(0.0, u16::MAX as f64) as u16
}

fn scale_f32(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

fn scale_i32(value: i32, factor: f64) -> i32 {
    (value as f64 * factor).round() as i32
}

fn set_damage(row: &mut EQUIP_PARAM_WEAPON_ST, attack: [u16; 5], factor: f64) {
    row.set_attack_base_physics(scale_u16(attack[0], factor));
    row.set_attack_base_magic(scale_u16(attack[1], factor));
    row.set_attack_base_fire(scale_u16(attack[2], factor));
    row.set_attack_base_thunder(scale_u16(attack[3], factor));
    row.set_attack_base_dark(scale_u16(attack[4], factor));
}

/// Damage, weight and sell value: what every launcher gets, as `(damage, weight, sell value)`
/// multipliers.
fn apply_launcher(
    row: &mut EQUIP_PARAM_WEAPON_ST,
    baseline: &Baseline,
    (damage, weight, sell_value): (f64, f64, f64),
) {
    set_damage(row, baseline.attack, damage);
    row.set_weight(scale_f32(baseline.weight, weight));
    row.set_sell_value(scale_i32(baseline.sell_value, sell_value));
}

fn apply_bow(row: &mut EQUIP_PARAM_WEAPON_ST, baseline: &Baseline, cfg: &Config) {
    let bow = &cfg.bow;
    apply_launcher(
        row,
        baseline,
        (bow.damage_multiplier, bow.weight_multiplier, bow.sell_value_multiplier),
    );
    let scaling = bow.scaling_multiplier;
    row.set_correct_strength(scale_f32(baseline.correct[0], scaling));
    row.set_correct_agility(scale_f32(baseline.correct[1], scaling));
    row.set_correct_magic(scale_f32(baseline.correct[2], scaling));
    row.set_correct_faith(scale_f32(baseline.correct[3], scaling));
    row.set_correct_luck(scale_f32(baseline.correct[4], scaling));
    if bow.unlock_aow {
        row.set_gem_mount_type(2); // every bow accepts an Ash of War; some vanilla bows ship with 0
    }
}

/// Applies every weapon buff to the live rows, always from each row's cached original values, so it
/// is safe to call again on a reload without compounding. Returns the rows touched.
pub fn apply(repo: &mut SoloParamRepository) -> usize {
    let cfg = config::get();

    let mut snapshot_guard = ORIGINALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let originals = snapshot_guard.get_or_insert_with(|| {
        let mut map = HashMap::new();
        shared::params::for_each_row_mut::<EquipParamWeapon>(repo, |index, row| {
            if is_relevant(row) {
                map.insert(index, snapshot(row));
            }
        });
        map
    });

    let mut changed = 0;
    shared::params::for_each_row_mut::<EquipParamWeapon>(repo, |index, row| {
        let Some(baseline) = originals.get(&index) else {
            return; // not one of RiseArcher's rows
        };
        let Some(kind) = weapon_kind(row.weapon_category(), row.wep_type()) else {
            return;
        };
        match kind {
            WeaponKind::Bow => apply_bow(row, baseline, &cfg),
            WeaponKind::Crossbow => {
                let c = &cfg.crossbow;
                apply_launcher(
                    row,
                    baseline,
                    (c.damage_multiplier, c.weight_multiplier, c.sell_value_multiplier),
                );
            }
            WeaponKind::Ballista => {
                let c = &cfg.ballista;
                apply_launcher(
                    row,
                    baseline,
                    (c.damage_multiplier, c.weight_multiplier, c.sell_value_multiplier),
                );
            }
            WeaponKind::Arrow => {
                row.set_max_arrow_quantity(max_quantity(cfg.bullet.arrow_max_quantity))
            }
            WeaponKind::Bolt => {
                row.set_max_arrow_quantity(max_quantity(cfg.bullet.bolt_max_quantity))
            }
        }
        changed += 1;
    });

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_sorted_into_the_five_buckets() {
        assert_eq!(weapon_kind(10, 0), Some(WeaponKind::Bow));
        assert_eq!(weapon_kind(11, 55), Some(WeaponKind::Crossbow));
        assert_eq!(weapon_kind(11, 56), Some(WeaponKind::Ballista));
        assert_eq!(weapon_kind(13, 0), Some(WeaponKind::Arrow));
        assert_eq!(weapon_kind(14, 0), Some(WeaponKind::Bolt));
        assert_eq!(weapon_kind(11, 1), None); // another category-11 type
        assert_eq!(weapon_kind(12, 55), None);
    }

    #[test]
    fn scaling_rounds_and_clamps() {
        assert_eq!(scale_u16(100, 1.5), 150);
        assert_eq!(scale_u16(100, -2.0), 0); // a negative factor gives 0
        assert_eq!(scale_u16(60_000, 10.0), u16::MAX);
        assert_eq!(scale_u16(3, 0.5), 2); // 1.5 rounds up
        assert_eq!(scale_i32(1000, 2.0), 2000);
        assert_eq!(scale_i32(7, 0.5), 4);
        assert_eq!(scale_f32(10.0, 0.5), 5.0);
    }

    #[test]
    fn carry_limits_fit_a_byte() {
        assert_eq!((max_quantity(-5), max_quantity(99), max_quantity(1000)), (0, 99, 255));
    }
}
