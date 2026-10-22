//! Per-slot accessors of an `ITEMLOT_PARAM_ST` row. A row has 8 slots and
//! `fromsoftware-rs` exposes one method per slot, so each field is read or
//! written as an array here.

use eldenring::param::ITEMLOT_PARAM_ST;

pub type Points = [u16; 8];
pub type ItemIds = [i32; 8];
pub type Nums = [u8; 8];

/// `lot_item_base_point0N`: the slot's weight in the row's lottery.
pub fn base_points(row: &ITEMLOT_PARAM_ST) -> Points {
    [
        row.lot_item_base_point01(),
        row.lot_item_base_point02(),
        row.lot_item_base_point03(),
        row.lot_item_base_point04(),
        row.lot_item_base_point05(),
        row.lot_item_base_point06(),
        row.lot_item_base_point07(),
        row.lot_item_base_point08(),
    ]
}

pub fn set_base_points(row: &mut ITEMLOT_PARAM_ST, points: Points) {
    row.set_lot_item_base_point01(points[0]);
    row.set_lot_item_base_point02(points[1]);
    row.set_lot_item_base_point03(points[2]);
    row.set_lot_item_base_point04(points[3]);
    row.set_lot_item_base_point05(points[4]);
    row.set_lot_item_base_point06(points[5]);
    row.set_lot_item_base_point07(points[6]);
    row.set_lot_item_base_point08(points[7]);
}

/// `lot_item_id0N`: 0 is the "nothing drops" slot.
pub fn item_ids(row: &ITEMLOT_PARAM_ST) -> ItemIds {
    [
        row.lot_item_id01(),
        row.lot_item_id02(),
        row.lot_item_id03(),
        row.lot_item_id04(),
        row.lot_item_id05(),
        row.lot_item_id06(),
        row.lot_item_id07(),
        row.lot_item_id08(),
    ]
}

/// `lot_item_num0N`: how many of the item the slot hands out.
pub fn nums(row: &ITEMLOT_PARAM_ST) -> Nums {
    [
        row.lot_item_num01(),
        row.lot_item_num02(),
        row.lot_item_num03(),
        row.lot_item_num04(),
        row.lot_item_num05(),
        row.lot_item_num06(),
        row.lot_item_num07(),
        row.lot_item_num08(),
    ]
}

pub fn set_nums(row: &mut ITEMLOT_PARAM_ST, nums: Nums) {
    row.set_lot_item_num01(nums[0]);
    row.set_lot_item_num02(nums[1]);
    row.set_lot_item_num03(nums[2]);
    row.set_lot_item_num04(nums[3]);
    row.set_lot_item_num05(nums[4]);
    row.set_lot_item_num06(nums[5]);
    row.set_lot_item_num07(nums[6]);
    row.set_lot_item_num08(nums[7]);
}

/// `lot_item_category0N` (1 = goods).
pub fn categories(row: &ITEMLOT_PARAM_ST) -> [i32; 8] {
    [
        row.lot_item_category01(),
        row.lot_item_category02(),
        row.lot_item_category03(),
        row.lot_item_category04(),
        row.lot_item_category05(),
        row.lot_item_category06(),
        row.lot_item_category07(),
        row.lot_item_category08(),
    ]
}

/// `getItemFlagId0N`: the slot's pickup event flag (non-zero = obtainable once).
pub fn slot_flags(row: &ITEMLOT_PARAM_ST) -> [u32; 8] {
    [
        row.get_item_flag_id01(),
        row.get_item_flag_id02(),
        row.get_item_flag_id03(),
        row.get_item_flag_id04(),
        row.get_item_flag_id05(),
        row.get_item_flag_id06(),
        row.get_item_flag_id07(),
        row.get_item_flag_id08(),
    ]
}
