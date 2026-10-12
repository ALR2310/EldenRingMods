//! Classifies the anim id a character is playing into a coarse action group
//! (walk, attack, skill...), from `CSChrTimeActModule.anim_queue`. Shared by
//! `speedmultiplier` (a speed per group) and `animcancel` (which animations
//! may be cancelled). Anim ids are `aXXX_YYYYYY`: `anim_id / 1_000_000` is the
//! `XXX` prefix (which TAE), `anim_id % 1_000_000` the `YYYYYY` action
//! suffix; see `mods/speedmultiplier/docs/action_groups.md`.

use eldenring::cs::ChrIns;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Walk,
    Run,
    Sneak,
    Jump,
    Roll,
    /// Everything on a ladder (`PlayerLadder`), see [group_of].
    Ladder,
    Attack,
    /// Backstabs and ripostes (`PlayerCritical`), see [group_of].
    Critical,
    Skill,
    Cast,
    Item,
    Other,
}

/// Special-weapon movesets whose TAE number falls inside the Sword Arts
/// range (Axe of Godfrey, Starscourge Greatsword, Ghiza's Wheel, Ornamental
/// Straight Sword, Repeating Crossbow, Rakshasa's Great Katana).
const WEAPON_TAES_IN_SKILL_RANGE: [i32; 6] = [831, 832, 839, 852, 935, 953];

/// Classifies a player anim id: by TAE prefix for spells/Sword Arts, by
/// action suffix for everything else.
pub fn group_of(anim_id: i32) -> Group {
    if anim_id < 0 {
        return Group::Other;
    }
    let prefix = anim_id / 1_000_000;
    // Placidusax's Ruin (a451, only that spell - TAE list): the beam phase
    // 045110 spawns a laser on the game's own clock, so a sped-up anim
    // desyncs from it (Nexus, bloodaxis, 2026-10-04; probe 2026-10-06:
    // 045100 -> 045110, ~6 s). Out of Cast, into Other (1 by default); the
    // wind-up 045100 stays Cast. Tested in game 2026-10-06: no more desync.
    if prefix == 451 && (45_110..=45_119).contains(&(anim_id % 1_000_000)) {
        return Group::Other;
    }
    if (400..600).contains(&prefix) {
        return Group::Cast;
    }
    if (600..1000).contains(&prefix) && !WEAPON_TAES_IN_SKILL_RANGE.contains(&prefix) {
        return Group::Skill;
    }
    match anim_id % 1_000_000 {
        // Walk / run / sneak (probe, 2026-10-02 - the 3rd suffix digit is
        // the gait, the same standing or sneaking): walk 0201xx (stop
        // 0221xx), run - holding the dash key - 0202xx (stop 0222xx),
        // sneaking 3xxxxx (idle 300000, sneak walk 3201xx, sneak run
        // 3202xx, going into the sneak stance 390000). The rest of the old Movement range
        // (standing 000000, 0200xx = the legs while using an item on the
        // move, jumps...) is Other since `PlayerMovement` was split.
        20_100..=20_199 | 22_100..=22_199 => Group::Walk,
        20_200..=20_299 | 22_200..=22_299 => Group::Run,
        300_000..=399_999 => Group::Sneak,
        // Jumps (probe, 2026-10-02): taking off 2020xx (202000 standing,
        // 202020 walking, 202030/202040 running), landing 2021xx (202100,
        // 202115, 202126). Only the range seen - other 20xxxx anims were
        // noted around Torrent before.
        202_000..=202_199 => Group::Jump,
        27_000..=27_999 => Group::Roll,
        // Ladders (probe, 2026-10-06), one range for all of it: grabbing
        // 028999, idle on the ladder 028030, climbing up 0280 1x (+ 028100
        // from the bottom), down 0280 2x, sliding down 028000-028002, attack
        // / kick 028040, 028045, 028036 (transition).
        28_000..=28_999 => Group::Ladder,
        // Critical hits (riposte 031700, backstab 031719 -> 031710 - probe,
        // 2026-10-02) are paired with the victim's anim, which this mod
        // doesn't speed up: a faster player pulled the blade out while the
        // enemy was still falling (user test). Own key `PlayerCritical`,
        // default 1 (vanilla) - first fixed at 1, made a key the same day
        // at the user's request.
        31_700..=31_799 => Group::Critical,
        30_000..=39_999 => Group::Attack,
        40_000..=49_999 => Group::Skill,
        // Torrent's whistle is an item, but in game it's the first part of
        // mounting (050190 -> 101004 mount -> 100000 riding), so it stays
        // with the other riding anims.
        50_190 => Group::Other,
        50_000..=59_999 => Group::Item,
        _ => Group::Other,
    }
}

/// The anim id currently playing, off `CSChrTimeActModule.anim_queue`.
pub fn current_anim_id(chr: &ChrIns) -> i32 {
    let time_act = &chr.modules.time_act;
    let idx = (time_act.read_idx as usize) % time_act.anim_queue.len();
    time_act.anim_queue[idx].anim_id
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn placidusax_ruin_beam_is_not_cast() {
        assert_eq!(group_of(451_045_100), Group::Cast); // wind-up
        assert_eq!(group_of(451_045_110), Group::Other); // beam
        assert_eq!(group_of(435_045_110), Group::Cast); // other incantations
    }
}
