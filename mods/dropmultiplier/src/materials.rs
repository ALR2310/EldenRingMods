//! Multiplies how many crafting / upgrade materials an item lot hands out
//! (`lot_item_num0N`), in `ItemLotParam_map` (gathering spots) and
//! `ItemLotParam_enemy` (drops): the `[Materials]` section, independent of
//! the drop chance. Each slot is grouped as Crafting, Upgrade or Unique (a
//! pickup flag means obtainable once) from the live `EquipParamGoods`. The
//! classification rules and the data behind them: `docs/materials.md`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{EquipParamGoods, ItemLotParam_enemy, ItemLotParam_map, SoloParam, SoloParamRepository};
use eldenring::param::ITEMLOT_PARAM_ST;

use common::{config, logger};

use crate::lot;

const CATEGORY_GOODS: i32 = 1;
const GOODS_TYPE_CRAFTING: u8 = 2;
const GOODS_TYPE_UPGRADE: u8 = 14;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    None,
    Crafting,
    Upgrade,
    Unique,
}

struct RowSnapshot {
    nums: [u8; 8],
    groups: [Group; 8],
}

struct Snapshot {
    enemy: Vec<RowSnapshot>,
    map: Vec<RowSnapshot>,
}

static SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);

struct Factors {
    crafting: f64,
    upgrade: f64,
    unique: f64,
}

impl Factors {
    fn from_config() -> Self {
        Self {
            crafting: config::get_double("Crafting", 1.0).max(0.0),
            upgrade: config::get_double("Upgrade", 1.0).max(0.0),
            unique: config::get_double("Unique", 1.0).max(0.0),
        }
    }

    fn get(&self, group: Group) -> Option<f64> {
        match group {
            Group::None => None,
            Group::Crafting => Some(self.crafting),
            Group::Upgrade => Some(self.upgrade),
            Group::Unique => Some(self.unique),
        }
    }
}

/// `EquipParamGoods` row ID -> `goodsType`, walked by index through
/// [`common::params::row_ids`] rather than `repo.get()` (see that module).
fn goods_types(repo: &SoloParamRepository) -> Option<HashMap<u32, u8>> {
    let ids = common::params::row_ids::<EquipParamGoods>(repo)?;
    let mut types = HashMap::with_capacity(ids.len());
    for (index, id) in ids.into_iter().enumerate() {
        types.insert(id, repo.get_row_by_index::<EquipParamGoods>(index)?.goods_type());
    }
    Some(types)
}

fn classify(row: &ITEMLOT_PARAM_ST, goods_types: &HashMap<u32, u8>) -> RowSnapshot {
    let item_ids = lot::item_ids(row);
    let categories = lot::categories(row);
    let slot_flags = lot::slot_flags(row);
    let row_flag = row.get_item_flag_id();
    let mut groups = [Group::None; 8];
    for i in 0..8 {
        if item_ids[i] <= 0 || categories[i] != CATEGORY_GOODS {
            continue;
        }
        let farmable_group = match goods_types.get(&(item_ids[i] as u32)) {
            Some(&GOODS_TYPE_CRAFTING) => Group::Crafting,
            Some(&GOODS_TYPE_UPGRADE) => Group::Upgrade,
            _ => continue,
        };
        groups[i] = if row_flag != 0 || slot_flags[i] != 0 { Group::Unique } else { farmable_group };
    }
    RowSnapshot { nums: lot::nums(row), groups }
}

fn snapshot_table<P: SoloParam<StructType = ITEMLOT_PARAM_ST>>(
    repo: &mut SoloParamRepository,
    goods_types: &HashMap<u32, u8>,
) -> Vec<RowSnapshot> {
    let mut rows = Vec::new();
    common::params::for_each_row_mut::<P>(repo, |_, row| rows.push(classify(row, goods_types)));
    rows
}

/// `count` scaled by `factor`: at least 1 (a factor below 1 must not turn a
/// pickup into nothing), at most 255 (`lot_item_num` is a u8).
fn scale_count(count: u8, factor: f64) -> u8 {
    (count as f64 * factor).round().clamp(1.0, u8::MAX as f64) as u8
}

/// Rescales one table from its snapshot. Returns how many slots it touched.
fn apply_table<P: SoloParam<StructType = ITEMLOT_PARAM_ST>>(
    repo: &mut SoloParamRepository,
    snapshot: &[RowSnapshot],
    factors: &Factors,
) -> usize {
    let mut changed = 0;
    common::params::for_each_row_mut::<P>(repo, |index, row| {
        let Some(original) = snapshot.get(index) else {
            return;
        };
        let mut nums = original.nums;
        for ((&count, &group), out) in original.nums.iter().zip(&original.groups).zip(nums.iter_mut()) {
            let Some(factor) = factors.get(group) else {
                continue;
            };
            if count == 0 {
                continue; // no count to scale
            }
            *out = scale_count(count, factor);
            changed += 1;
        }
        lot::set_nums(row, nums);
    });
    changed
}

/// Applies `[Materials]` to both item lot tables. Logs its own result; does
/// nothing (logged as an error) if a param slot isn't what we expect or
/// `EquipParamGoods`' IDs can't be read.
pub fn apply(repo: &mut SoloParamRepository, suffix: &str) {
    for check in [
        common::params::check::<ItemLotParam_enemy>(repo),
        common::params::check::<ItemLotParam_map>(repo),
        common::params::check::<EquipParamGoods>(repo),
    ] {
        if let Err(reason) = check {
            logger::error(&format!("{reason} - material amounts left unchanged"));
            return;
        }
    }

    let mut snapshot_guard = SNAPSHOT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if snapshot_guard.is_none() {
        let Some(goods_types) = goods_types(repo) else {
            logger::error(&format!(
                "could not read item IDs of {} - material amounts left unchanged",
                common::params::describe::<EquipParamGoods>(repo)
            ));
            return;
        };
        let snapshot = Snapshot {
            enemy: snapshot_table::<ItemLotParam_enemy>(repo, &goods_types),
            map: snapshot_table::<ItemLotParam_map>(repo, &goods_types),
        };
        let count = |group: Group| {
            snapshot.enemy.iter().chain(&snapshot.map).flat_map(|r| r.groups).filter(|&g| g == group).count()
        };
        logger::log(&format!(
            "Materials: {} goods, {} crafting, {} upgrade, {} unique slot(s)",
            goods_types.len(),
            count(Group::Crafting),
            count(Group::Upgrade),
            count(Group::Unique)
        ));
        *snapshot_guard = Some(snapshot);
    }
    let Some(snapshot) = snapshot_guard.as_ref() else {
        return;
    };

    let factors = Factors::from_config();
    let changed = apply_table::<ItemLotParam_enemy>(repo, &snapshot.enemy, &factors)
        + apply_table::<ItemLotParam_map>(repo, &snapshot.map, &factors);
    logger::log(&format!(
        "Materials: Crafting={:.3} Upgrade={:.3} Unique={:.3} -> {changed} slot(s){suffix}",
        factors.crafting, factors.upgrade, factors.unique
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_scale_with_a_floor_of_one_and_a_ceiling_of_255() {
        assert_eq!(scale_count(3, 2.0), 6);
        assert_eq!(scale_count(3, 0.1), 1); // never turns a pickup into nothing
        assert_eq!(scale_count(200, 10.0), 255);
        assert_eq!(scale_count(1, 1.5), 2); // rounds
        assert_eq!(scale_count(5, 0.0), 1);
    }
}
