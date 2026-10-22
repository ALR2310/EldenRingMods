//! RiseArcher's weapon buffs on the live `EquipParamWeapon` rows (bows,
//! crossbows, ballistas, arrows, bolts), ported from `csv/RiseArcher.MASSEDIT`
//! with every multiplier coming from the ini. Row filters, fields and the
//! baseline snapshot: `docs/param_patching.md`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{EquipParamWeapon, SoloParamRepository};
use eldenring::param::EQUIP_PARAM_WEAPON_ST;

use common::config;

/// FromSoftware's marker for an NPC-only duplicate row (hidden from the
/// player's menu): always excluded, like the `!prop sortId 9999999` filter on
/// every line of the original Mass Edit.
const NPC_ONLY_SORT_ID: i32 = 9999999;
const WEAPON_CATEGORY_BOW: u8 = 10;
/// Crossbow AND Ballista share this category, split by `wepType`.
const WEAPON_CATEGORY_CROSSBOW_FAMILY: u8 = 11;
const WEP_TYPE_CROSSBOW: u16 = 55;
const WEP_TYPE_BALLISTA: u16 = 56;
const WEAPON_CATEGORY_ARROW: u8 = 13;
const WEAPON_CATEGORY_BOLT: u8 = 14;

/// Which of the 5 buckets a row falls into: the one place that decides both
/// what is relevant and which multipliers apply.
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
        WEAPON_CATEGORY_CROSSBOW_FAMILY if wep_type == WEP_TYPE_CROSSBOW => Some(WeaponKind::Crossbow),
        WEAPON_CATEGORY_CROSSBOW_FAMILY if wep_type == WEP_TYPE_BALLISTA => Some(WeaponKind::Ballista),
        WEAPON_CATEGORY_ARROW => Some(WeaponKind::Arrow),
        WEAPON_CATEGORY_BOLT => Some(WeaponKind::Bolt),
        _ => None,
    }
}

/// The multipliers shared by every launcher (bow, crossbow, ballista).
#[derive(Clone, Copy)]
struct Launcher {
    damage: f64,
    weight: f64,
    sell_value: f64,
}

impl Launcher {
    fn load(prefix: &str, default_damage: f64) -> Self {
        Self {
            damage: config::get_double(&format!("{prefix}.DamageMultiplier"), default_damage),
            weight: config::get_double(&format!("{prefix}.WeightMultiplier"), 0.5),
            sell_value: config::get_double(&format!("{prefix}.SellValueMultiplier"), 2.0),
        }
    }
}

struct Config {
    bow: Launcher,
    bow_scaling: f64,
    bow_unlock_aow: bool,
    crossbow: Launcher,
    ballista: Launcher,
    arrow_max_quantity: u8,
    bolt_max_quantity: u8,
}

impl Config {
    fn load() -> Self {
        Self {
            bow: Launcher::load("Bow", 1.5),
            bow_scaling: config::get_double("Bow.ScalingMultiplier", 2.0),
            bow_unlock_aow: config::get_bool("Bow.UnlockAOW", true),
            crossbow: Launcher::load("Crossbow", 3.0),
            ballista: Launcher::load("Ballista", 4.0),
            arrow_max_quantity: config::get_int("Bullet.Arrow.MaxQuantity", 255).clamp(0, 255) as u8,
            bolt_max_quantity: config::get_int("Bullet.Bolt.MaxQuantity", 255).clamp(0, 255) as u8,
        }
    }
}

/// The game's own original values that are scaled, by row index, captured once
/// before any edit so every reload rescales from the true baseline instead of
/// compounding.
#[derive(Clone, Copy)]
struct Baseline {
    attack: [u16; 5],
    correct: [f32; 5],
    weight: f32,
    sell_value: i32,
}

/// Recovered from poisoning at every lock: `apply` is retried across a
/// `catch_unwind` (see `lib.rs`), and one panic while the lock was held must
/// not turn every retry into a second, unrelated panic.
static ORIGINALS: Mutex<Option<HashMap<usize, Baseline>>> = Mutex::new(None);

/// Whether `row` is one RiseArcher touches: not an NPC-only duplicate, and in
/// the bow / crossbow / ballista / arrow / bolt family.
fn is_relevant(row: &EQUIP_PARAM_WEAPON_ST) -> bool {
    row.sort_id() != NPC_ONLY_SORT_ID && weapon_kind(row.weapon_category(), row.wep_type()).is_some()
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

/// Damage, weight and sell value: what every launcher gets.
fn apply_launcher(row: &mut EQUIP_PARAM_WEAPON_ST, baseline: &Baseline, launcher: &Launcher) {
    set_damage(row, baseline.attack, launcher.damage);
    row.set_weight(scale_f32(baseline.weight, launcher.weight));
    row.set_sell_value(scale_i32(baseline.sell_value, launcher.sell_value));
}

fn apply_bow(row: &mut EQUIP_PARAM_WEAPON_ST, baseline: &Baseline, cfg: &Config) {
    apply_launcher(row, baseline, &cfg.bow);
    row.set_correct_strength(scale_f32(baseline.correct[0], cfg.bow_scaling));
    row.set_correct_agility(scale_f32(baseline.correct[1], cfg.bow_scaling));
    row.set_correct_magic(scale_f32(baseline.correct[2], cfg.bow_scaling));
    row.set_correct_faith(scale_f32(baseline.correct[3], cfg.bow_scaling));
    row.set_correct_luck(scale_f32(baseline.correct[4], cfg.bow_scaling));
    if cfg.bow_unlock_aow {
        row.set_gem_mount_type(2); // every bow accepts an Ash of War; some vanilla bows ship with 0
    }
}

/// Applies every weapon buff to the live rows, always from each row's cached
/// original values, so it is safe to call repeatedly (reload) without
/// compounding. Returns how many rows were touched.
pub fn apply(repo: &mut SoloParamRepository) -> usize {
    let cfg = Config::load();

    let mut snapshot_guard = ORIGINALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let originals = snapshot_guard.get_or_insert_with(|| {
        let mut map = HashMap::new();
        common::params::for_each_row_mut::<EquipParamWeapon>(repo, |index, row| {
            if is_relevant(row) {
                map.insert(index, snapshot(row));
            }
        });
        map
    });

    let mut changed = 0;
    common::params::for_each_row_mut::<EquipParamWeapon>(repo, |index, row| {
        let Some(baseline) = originals.get(&index) else {
            return; // not one of RiseArcher's rows
        };
        let Some(kind) = weapon_kind(row.weapon_category(), row.wep_type()) else {
            return;
        };
        match kind {
            WeaponKind::Bow => apply_bow(row, baseline, &cfg),
            WeaponKind::Crossbow => apply_launcher(row, baseline, &cfg.crossbow),
            WeaponKind::Ballista => apply_launcher(row, baseline, &cfg.ballista),
            WeaponKind::Arrow => row.set_max_arrow_quantity(cfg.arrow_max_quantity),
            WeaponKind::Bolt => row.set_max_arrow_quantity(cfg.bolt_max_quantity),
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

    /// The defaults in code and in the ini template are written twice; they
    /// must agree, or a missing key and a fresh file would behave differently.
    #[test]
    fn code_defaults_match_the_ini_template() {
        let ini = include_str!("../RiseArcher.ini");
        let value = |key: &str| -> f64 {
            let line = ini.lines().find(|l| l.trim_start().starts_with(&format!("{key}="))).unwrap();
            let text = line.split_once('=').unwrap().1.trim();
            match text {
                "true" => 1.0,
                "false" => 0.0,
                other => other.parse().unwrap(),
            }
        };
        let cfg = Config::load();
        for (prefix, launcher) in [("Bow", cfg.bow), ("Crossbow", cfg.crossbow), ("Ballista", cfg.ballista)] {
            assert_eq!(value(&format!("{prefix}.DamageMultiplier")), launcher.damage, "{prefix} damage");
            assert_eq!(value(&format!("{prefix}.WeightMultiplier")), launcher.weight, "{prefix} weight");
            assert_eq!(value(&format!("{prefix}.SellValueMultiplier")), launcher.sell_value, "{prefix} sell");
        }
        assert_eq!(value("Bow.ScalingMultiplier"), cfg.bow_scaling);
        assert_eq!(value("Bow.UnlockAOW"), cfg.bow_unlock_aow as u8 as f64);
        assert_eq!(value("Bullet.Arrow.MaxQuantity"), cfg.arrow_max_quantity as f64);
        assert_eq!(value("Bullet.Bolt.MaxQuantity"), cfg.bolt_max_quantity as f64);
    }
}
