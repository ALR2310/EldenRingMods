//! Classifies the anim id a character is playing (`aXXX_YYYYYY`) into a
//! coarse action group, shared by `speedmultiplier` and `animcancel`. Why each
//! range is what it is: `mods/speedmultiplier/docs/action_groups.md`.

use eldenring::cs::{CSChrTimeActModuleAnim, ChrIns};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Walk,
    Run,
    Sneak,
    Jump,
    Roll,
    Ladder,
    Attack,
    Critical,
    Skill,
    Cast,
    Item,
    Other,
}

/// Special-weapon movesets whose TAE number falls inside the Sword Arts range.
const WEAPON_TAES_IN_SKILL_RANGE: [i32; 6] = [831, 832, 839, 852, 935, 953];

/// Whether a TAE prefix (`aXXX`) is an Ash of War / Sword Art moveset.
pub fn is_skill_tae(prefix: i32) -> bool {
    (600..1000).contains(&prefix) && !WEAPON_TAES_IN_SKILL_RANGE.contains(&prefix)
}

/// Classifies a player anim id: by TAE prefix for spells/Sword Arts, by
/// action suffix for everything else.
pub fn group_of(anim_id: i32) -> Group {
    if anim_id < 0 {
        return Group::Other;
    }
    let (prefix, suffix) = (anim_id / 1_000_000, anim_id % 1_000_000);
    // Placidusax's Ruin beam phase runs on the game's own clock, so it must
    // not be sped up. The wind-up 045100 stays Cast.
    if prefix == 451 && (45_110..=45_119).contains(&suffix) {
        return Group::Other;
    }
    if (400..600).contains(&prefix) {
        return Group::Cast;
    }
    if is_skill_tae(prefix) {
        return Group::Skill;
    }
    match suffix {
        // Walk / run (the 3rd suffix digit is the gait) and sneak.
        20_100..=20_199 | 22_100..=22_199 => Group::Walk,
        20_200..=20_299 | 22_200..=22_299 => Group::Run,
        300_000..=399_999 => Group::Sneak,
        // Jumps: take-off 2020xx, landing 2021xx (only the range seen).
        202_000..=202_199 => Group::Jump,
        27_000..=27_999 => Group::Roll,
        // Ladders: one range for grabbing, climbing, sliding and attacking.
        28_000..=28_999 => Group::Ladder,
        // Critical hits are paired with the victim's anim, so they get their
        // own group (before Attack).
        31_700..=31_799 => Group::Critical,
        30_000..=39_999 => Group::Attack,
        40_000..=49_999 => Group::Skill,
        // Torrent's whistle is the first part of mounting, not an item.
        50_190 => Group::Other,
        50_000..=59_999 => Group::Item,
        _ => Group::Other,
    }
}

/// An animation id as `aXXX_YYYYYY` for logs (`-1` and other negatives as is).
pub fn format_anim_id(id: i32) -> String {
    if id < 0 {
        return id.to_string();
    }
    format!("a{:03}_{:06}", id / 1_000_000, id % 1_000_000)
}

/// The animation currently playing, off `CSChrTimeActModule.anim_queue`.
pub fn current_anim(chr: &ChrIns) -> &CSChrTimeActModuleAnim {
    let time_act = &chr.modules.time_act;
    &time_act.anim_queue[(time_act.read_idx as usize) % time_act.anim_queue.len()]
}

/// The anim id currently playing.
pub fn current_anim_id(chr: &ChrIns) -> i32 {
    current_anim(chr).anim_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_ids_are_formatted_as_a_tae_and_a_suffix() {
        assert_eq!(format_anim_id(20_032_000), "a020_032000");
        assert_eq!(format_anim_id(2_100), "a000_002100");
        assert_eq!(format_anim_id(-1), "-1");
    }

    #[test]
    fn jumps_are_their_own_group() {
        assert_eq!(group_of(202_000), Group::Jump); // standing jump
        assert_eq!(group_of(202_020), Group::Jump); // walking jump
        assert_eq!(group_of(202_040), Group::Jump); // running jump
        assert_eq!(group_of(202_100), Group::Jump); // landing
        assert_eq!(group_of(202_126), Group::Jump); // running landing
        assert_eq!(group_of(202_200), Group::Other);
    }

    #[test]
    fn gaits_are_split() {
        assert_eq!(group_of(20_110), Group::Walk);
        assert_eq!(group_of(22_100), Group::Walk); // walk stop
        assert_eq!(group_of(20_210), Group::Run);
        assert_eq!(group_of(22_200), Group::Run); // run stop
        assert_eq!(group_of(300_000), Group::Sneak); // sneak idle
        assert_eq!(group_of(320_110), Group::Sneak); // sneak walk
        assert_eq!(group_of(320_210), Group::Sneak); // sneak run
        assert_eq!(group_of(390_000), Group::Sneak); // going into the sneak stance
        assert_eq!(group_of(0), Group::Other); // standing
        assert_eq!(group_of(20_010), Group::Other); // legs while using an item
    }

    #[test]
    fn suffix_ranges() {
        assert_eq!(group_of(27_010), Group::Roll);
        assert_eq!(group_of(28_030), Group::Ladder);
        assert_eq!(group_of(31_700), Group::Critical); // riposte
        assert_eq!(group_of(31_719), Group::Critical); // backstab
        assert_eq!(group_of(30_100), Group::Attack);
        assert_eq!(group_of(42_000), Group::Skill);
        assert_eq!(group_of(51_000), Group::Item);
        assert_eq!(group_of(50_190), Group::Other); // Torrent's whistle
        assert_eq!(group_of(-1), Group::Other);
    }

    #[test]
    fn tae_prefix_ranges() {
        assert_eq!(group_of(885_040_000), Group::Skill); // Ash of War moveset
        assert_eq!(group_of(831_030_000), Group::Attack); // weapon moveset inside the Skill range
        assert_eq!(group_of(831_000_000), Group::Other);
    }

    #[test]
    fn placidusax_ruin_beam_is_not_cast() {
        assert_eq!(group_of(451_045_100), Group::Cast); // wind-up
        assert_eq!(group_of(451_045_110), Group::Other); // beam
        assert_eq!(group_of(435_045_110), Group::Cast); // other incantations
    }
}
