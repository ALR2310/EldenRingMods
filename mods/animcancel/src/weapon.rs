//! What the player holds, for the guard rule: guard (L1) only cancels an
//! animation while L1 really guards. With a twin blade, or one weapon in each
//! hand (dual wielding), L1 is an attack, and letting it cut the animation
//! shortens the combo instead (2026-10-17 reports and logs,
//! `docs/animation_cancel.md`).
//!
//! The shield test is the weapon's `wepmotionCategory` (the animation set:
//! 47 greatshield, 48 small shield, 49 medium shield, 57 shield-like), read
//! from `EquipParamWeapon`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{ChrAsmArmStyle, ChrAsmSlot, EquipParamWeapon, SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;

/// `wepmotionCategory` values of shields.
const SHIELD_MOTIONS: [u8; 4] = [47, 48, 49, 57];

/// The `EquipParamWeapon` fields that decide what L1 does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weapon {
    pub wep_type: u16,
    pub enable_guard: bool,
    pub dual_blade: bool,
    pub motion: u8,
    pub guard_motion: u8,
}

/// `EquipParamWeapon` ID -> row index, built on first use.
static ROWS: Mutex<Option<HashMap<u32, usize>>> = Mutex::new(None);

/// The equipped ID's row: rows exist per affinity (hundreds); the upgrade
/// level is the last two digits of the equipped ID (9080009 = row 9080000 +9).
pub fn weapon(id: i32) -> Option<Weapon> {
    if id <= 0 {
        return None;
    }
    let repo = unsafe { SoloParamRepository::instance_mut() }.ok()?;
    let mut rows = ROWS.lock().unwrap_or_else(|e| e.into_inner());
    if rows.is_none() {
        let ids = common::params::row_ids::<EquipParamWeapon>(repo)?;
        *rows = Some(ids.into_iter().enumerate().map(|(index, id)| (id, index)).collect());
    }
    let map = rows.as_ref()?;
    let index = *map.get(&(id as u32)).or_else(|| map.get(&((id - id % 100) as u32)))?;
    let row = repo.get_row_by_index::<EquipParamWeapon>(index)?;
    Some(Weapon {
        wep_type: row.wep_type(),
        enable_guard: row.enable_guard(),
        dual_blade: row.is_dual_blade(),
        motion: row.wepmotion_category(),
        guard_motion: row.guardmotion_category(),
    })
}

/// How the weapons are held and which ones (param IDs): arm style, the
/// active left and right weapon slots' IDs.
pub fn held() -> Option<(ChrAsmArmStyle, i32, i32)> {
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let player = world_chr_man.main_player.as_mut()?;
    let data = unsafe { player.player_game_data.as_ref() };
    let asm = &data.equipment.chr_asm;
    let equipment = &asm.equipment;
    let id = |slot: ChrAsmSlot| asm.equipment_param_ids[slot as usize];
    Some((equipment.arm_style, id(equipment.active_left_weapon_slot()), id(equipment.active_right_weapon_slot())))
}

pub fn is_shield(weapon: &Weapon) -> bool {
    SHIELD_MOTIONS.contains(&weapon.motion)
}

/// Whether the guard button guards (not attacks) for these weapons:
/// - a weapon held in both hands: yes (the other slot's weapon is put away,
///   whatever it is), except a twin blade, which attacks with it;
/// - one-handed: only with a shield in the left hand; two weapons, one in
///   each hand, is dual wielding and the button attacks.
pub fn guard_possible(arm: ChrAsmArmStyle, left: Option<&Weapon>, right: Option<&Weapon>) -> bool {
    let two_handed = |held: Option<&Weapon>| held.is_some_and(|w| !w.dual_blade);
    match arm {
        ChrAsmArmStyle::OneHanded => left.is_some_and(is_shield),
        ChrAsmArmStyle::RightBothHands => two_handed(right),
        ChrAsmArmStyle::LeftBothHands => two_handed(left),
        ChrAsmArmStyle::EmptyHanded => false,
    }
}

/// (what is held, answer) of the last check, so the params are read again
/// only when the weapons or the grip change.
static LAST: Mutex<Option<((u32, i32, i32), bool)>> = Mutex::new(None);

/// Whether the guard button guards right now (see [guard_possible]). False
/// when it can't be told (data not loaded): then guard cancels nothing.
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
        Weapon { wep_type: 0, enable_guard: false, dual_blade: false, motion, guard_motion: 0 }
    }

    fn twin_blades() -> Weapon {
        Weapon { dual_blade: true, ..weapon(60) }
    }

    #[test]
    fn guard_rules_by_grip() {
        use ChrAsmArmStyle::*;
        let (shield, katana, greatsword) = (weapon(49), weapon(29), weapon(21));
        // One hand: only a shield in the left hand guards.
        assert!(guard_possible(OneHanded, Some(&shield), Some(&katana)));
        assert!(!guard_possible(OneHanded, Some(&katana), Some(&katana)));
        assert!(!guard_possible(OneHanded, None, Some(&katana)));
        // One weapon in both hands: guards, whatever the other slot holds.
        assert!(guard_possible(RightBothHands, None, Some(&katana)));
        assert!(guard_possible(RightBothHands, Some(&katana), Some(&katana)));
        assert!(guard_possible(RightBothHands, Some(&shield), Some(&greatsword)));
        assert!(guard_possible(LeftBothHands, Some(&katana), Some(&katana)));
        // Twin blades attack with the guard button.
        assert!(!guard_possible(RightBothHands, Some(&shield), Some(&twin_blades())));
        // Unknown weapon or no weapon: nothing.
        assert!(!guard_possible(RightBothHands, Some(&katana), None));
        assert!(!guard_possible(EmptyHanded, None, None));
    }

    #[test]
    fn only_shield_animation_sets_count_as_shields() {
        for motion in [47, 48, 49, 57] {
            assert!(is_shield(&weapon(motion)), "{motion}");
        }
        // Katana (29), Rellana's Twin Blades (60), unarmed.
        for motion in [29, 60, 0] {
            assert!(!is_shield(&weapon(motion)), "{motion}");
        }
    }
}
