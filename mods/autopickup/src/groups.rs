//! The `ActionButtonParam` rows of the interactions that get picked up
//! automatically, by group. The IDs start from Elden_Ring_game_tools'
//! `AutoPickup.cpp` (its `k*ActionButtonIds` arrays), checked against the
//! vanilla and Convergence `ActionButtonParam` tables and corrected (the DLC
//! materials, a few plain pickups, chests apart, "Pillage" rows as corpse loot). Levers, doors, graces and talking are not
//! in any of them. See `docs/exec_action_button.md`.

/// One group of interactions, switched on and scaled together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Materials,
    Items,
    CorpseLoot,
    LostRunes,
    Chests,
}

impl Group {
    pub const ALL: [Group; 5] =
        [Group::Materials, Group::Items, Group::CorpseLoot, Group::LostRunes, Group::Chests];

    /// This group's bit in the enabled mask the hook reads.
    pub const fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The group's row IDs, ascending.
    pub const fn ids(self) -> &'static [u32] {
        match self {
            Group::Materials => &MATERIALS,
            Group::Items => &ITEMS,
            Group::CorpseLoot => &CORPSE_LOOT,
            Group::LostRunes => &LOST_RUNES,
            Group::Chests => &CHESTS,
        }
    }
}

/// Plants, mushrooms and other crafting materials.
const MATERIALS: [u32; 112] = [
    7800, 7810, 7811, 7812, 7813, 7814, 7815, 7816, 7817, 7818, 7819, 7820, 7821, 7822, 7823,
    7824, 7825, 7826, 7827, 7828, 7850, 7860, 7861, 7862, 7863, 7864, 7865, 7866, 7867, 7868,
    7869, 7870, 7871, 7872, 7873, 7874, 7875, 7876, 7877, 7878, 207800, 207810, 207811, 207812,
    207813, 207814, 207815, 207816, 207817, 207818, 207819, 207820, 207821, 207822, 207823,
    207824, 207825, 207826, 207827, 207828, 207829, 207830, 207831, 207832, 207833, 207834,
    207835, 207836, 207837, 207838, 207839, 207840, 207841, 207842, 207843, 207844, 207850,
    207860, 207861, 207862, 207863, 207864, 207865, 207866, 207867, 207868, 207869, 207870,
    207871, 207872, 207873, 207874, 207875, 207876, 207877, 207878, 207879, 207880, 207881,
    207882, 207883, 207884, 207885, 207886, 207887, 207888, 207889, 207890, 207891, 207892,
    207893, 207894,
];

/// Items lying on the ground ("Pick up item").
const ITEMS: [u32; 14] = [
    4000, 4110, 4250, 4251, 4252, 4253, 4260, 4261, 4270, 4280, 4350, 6361, 209210, 209524,
];

/// Loot on corpses and remains ("Pillage corpse", "Pillage remains").
const CORPSE_LOOT: [u32; 5] = [
    4100, 4200, 4201, 4202, 9532,
];

/// Treasure chests ("Search treasure chest"). Own group, off by default: a mimic
/// opens the same way.
const CHESTS: [u32; 1] = [
    4300,
];

/// Your dropped runes after dying.
const LOST_RUNES: [u32; 1] = [
    1000,
];

/// The group `id` belongs to. Plain slice searches: the hook calls this, so no
/// allocation.
pub fn classify(id: u32) -> Option<Group> {
    Group::ALL.into_iter().find(|group| group.ids().binary_search(&id).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_is_ascending_without_repeats() {
        for group in Group::ALL {
            assert!(group.ids().windows(2).all(|pair| pair[0] < pair[1]), "{group:?}");
        }
    }

    #[test]
    fn groups_do_not_overlap() {
        for (i, a) in Group::ALL.into_iter().enumerate() {
            for b in Group::ALL.into_iter().skip(i + 1) {
                assert!(a.ids().iter().all(|id| !b.ids().contains(id)), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn bits_are_distinct() {
        let all = Group::ALL.into_iter().fold(0u8, |mask, group| mask | group.bit());
        assert_eq!(all.count_ones() as usize, Group::ALL.len());
    }

    #[test]
    fn classify_finds_each_group() {
        assert_eq!(classify(7800), Some(Group::Materials));
        assert_eq!(classify(207844), Some(Group::Materials));
        assert_eq!(classify(4000), Some(Group::Items));
        assert_eq!(classify(4100), Some(Group::CorpseLoot));
        assert_eq!(classify(4200), Some(Group::CorpseLoot));
        assert_eq!(classify(4300), Some(Group::Chests));
        assert_eq!(classify(207894), Some(Group::Materials));
        assert_eq!(classify(209210), Some(Group::Items));
        assert_eq!(classify(1000), Some(Group::LostRunes));
        assert_eq!(classify(0), None);
        assert_eq!(classify(5000), None);
    }
}
