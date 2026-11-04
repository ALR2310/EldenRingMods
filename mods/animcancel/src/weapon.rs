//! What the player holds, for the guard rule: guard (L1) only cancels an animation while L1 really
//! guards. With a twin blade, or one weapon in each hand, L1 is an attack and letting it cut the
//! animation would shorten the combo. The shield test is `EquipParamWeapon.wepmotionCategory`
//! (47 greatshield, 48 small shield, 49 medium shield, 57 shield-like).

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{
    ChrAsm, ChrAsmArmStyle, ChrAsmSlot, EquipParamWeapon, SoloParamRepository, WorldChrMan,
};
use fromsoftware_shared::FromStatic;

const SHIELD_MOTIONS: [u8; 4] = [47, 48, 49, 57];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weapon {
    pub dual_blade: bool,
    pub motion: u8,
}

impl Weapon {
    pub fn is_shield(&self) -> bool {
        SHIELD_MOTIONS.contains(&self.motion)
    }
}

static ROWS: Mutex<Option<HashMap<u32, usize>>> = Mutex::new(None);

/// Rows exist per affinity (hundreds); the upgrade level is the last two digits of the equipped ID
/// (9080009 = row 9080000 +9).
fn weapon(id: i32) -> Option<Weapon> {
    if id <= 0 {
        return None;
    }
    // SAFETY: called from game tasks; `instance_mut` is `Err` until the singleton exists.
    let repo = unsafe { SoloParamRepository::instance_mut() }.ok()?;
    let mut rows = ROWS.lock().unwrap_or_else(|e| e.into_inner());
    if rows.is_none() {
        let ids = shared::params::row_ids::<EquipParamWeapon>(repo)?;
        *rows = Some(ids.into_iter().enumerate().map(|(index, id)| (id, index)).collect());
    }
    let map = rows.as_ref()?;
    let index = *map.get(&(id as u32)).or_else(|| map.get(&((id - id % 100) as u32)))?;
    let row = repo.get_row_by_index::<EquipParamWeapon>(index)?;
    Some(Weapon { dual_blade: row.is_dual_blade(), motion: row.wepmotion_category() })
}

fn chr_asm<'a>() -> Option<&'a ChrAsm> {
    // SAFETY: called from game tasks; `instance_mut` is `Err` until the singleton exists.
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let player = world_chr_man.main_player.as_mut()?;
    // SAFETY: the player's game data pointer is non-null while the player exists.
    let data = unsafe { player.player_game_data.as_ref() };
    Some(&data.equipment.chr_asm)
}

fn active_weapon_ids(asm: &ChrAsm) -> (i32, i32) {
    let id = |slot: ChrAsmSlot| asm.equipment_param_ids[slot as usize];
    (id(asm.equipment.active_left_weapon_slot()), id(asm.equipment.active_right_weapon_slot()))
}

fn held() -> Option<(ChrAsmArmStyle, i32, i32)> {
    let asm = chr_asm()?;
    let (left, right) = active_weapon_ids(asm);
    Some((asm.equipment.arm_style, left, right))
}

/// What the player holds as one line, for the probe's log.
pub fn describe_held() -> Option<String> {
    let asm = chr_asm()?;
    let equipment = &asm.equipment;
    let (left, right) = active_weapon_ids(asm);
    let info = |id: i32| match weapon(id) {
        Some(w) => format!(
            "dual={} motion={} shield={}",
            w.dual_blade as u8,
            w.motion,
            w.is_shield() as u8
        ),
        None => "no row".to_string(),
    };
    Some(format!(
        "arm={:?} slot=L{}/R{} weapon=L{}/R{} left[{}] right[{}]",
        equipment.arm_style,
        equipment.selected_slots.left_weapon_slot,
        equipment.selected_slots.right_weapon_slot,
        left,
        right,
        info(left),
        info(right)
    ))
}

/// Whether the guard button guards (not attacks): with a weapon in both hands, except a twin
/// blade; one-handed, only with a shield in the left hand (two weapons is dual wielding).
pub fn guard_possible(arm: ChrAsmArmStyle, left: Option<&Weapon>, right: Option<&Weapon>) -> bool {
    let two_handed = |held: Option<&Weapon>| held.is_some_and(|w| !w.dual_blade);
    match arm {
        ChrAsmArmStyle::OneHanded => left.is_some_and(Weapon::is_shield),
        ChrAsmArmStyle::RightBothHands => two_handed(right),
        ChrAsmArmStyle::LeftBothHands => two_handed(left),
        ChrAsmArmStyle::EmptyHanded => false,
    }
}

type HeldKey = (u32, i32, i32);

// The last check, so the params are read again only when the weapons or the grip change.
static LAST: Mutex<Option<(HeldKey, bool)>> = Mutex::new(None);

/// False when it can't be told (data not loaded): then guard cancels nothing.
pub fn guard_button_guards() -> bool {
    let Some((arm, left_id, right_id)) = held() else {
        return false;
    };
    let key = (arm as u32, left_id, right_id);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((cached, answer)) = *last
        && cached == key
    {
        return answer;
    }
    let answer = guard_possible(arm, weapon(left_id).as_ref(), weapon(right_id).as_ref());
    *last = Some((key, answer));
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(motion: u8) -> Weapon {
        Weapon { dual_blade: false, motion }
    }

    fn twin_blades() -> Weapon {
        Weapon { dual_blade: true, ..weapon(60) }
    }

    #[test]
    fn guard_rules_by_grip() {
        use ChrAsmArmStyle::*;
        let (shield, katana, greatsword) = (weapon(49), weapon(29), weapon(21));
        assert!(guard_possible(OneHanded, Some(&shield), Some(&katana)));
        assert!(!guard_possible(OneHanded, Some(&katana), Some(&katana)));
        assert!(!guard_possible(OneHanded, None, Some(&katana)));
        assert!(guard_possible(RightBothHands, None, Some(&katana)));
        assert!(guard_possible(RightBothHands, Some(&katana), Some(&katana)));
        assert!(guard_possible(RightBothHands, Some(&shield), Some(&greatsword)));
        assert!(guard_possible(LeftBothHands, Some(&katana), Some(&katana)));
        assert!(!guard_possible(RightBothHands, Some(&shield), Some(&twin_blades())));
        assert!(!guard_possible(RightBothHands, Some(&katana), None));
        assert!(!guard_possible(EmptyHanded, None, None));
    }

    #[test]
    fn only_shield_animation_sets_count_as_shields() {
        for motion in [47, 48, 49, 57] {
            assert!(weapon(motion).is_shield(), "{motion}");
        }
        // Katana (29), Rellana's Twin Blades (60), unarmed.
        for motion in [29, 60, 0] {
            assert!(!weapon(motion).is_shield(), "{motion}");
        }
    }
}
