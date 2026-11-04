//! Every frame, writes `CSChrBehaviorModule.animation_speed` of the player (by the group of the
//! animation playing), Torrent, the victims of a critical hit, enemies and spirits. `All` is the
//! master key: any value other than 1 overrides the keys of its group. Groups and ranges:
//! `docs/action_groups.md`.

use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, CSThrowNode, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use shared::anim::{Group, current_anim_id, group_of};
use shared::logger;

use crate::config::{self, Player, Torrent};

/// Torrent's NpcParam ids: the base one and the skins after it, in steps of 10000.
const TORRENT_NPC_PARAM_IDS: std::ops::Range<i32> = 80_020_000..80_060_000;

fn group_speed(group: Group, player: &Player) -> f32 {
    match group {
        Group::Walk => player.walk,
        Group::Run => player.run,
        Group::Sneak => player.sneak,
        Group::Jump => player.jump,
        Group::Roll => player.roll,
        Group::Ladder => player.ladder,
        Group::Attack => player.attack,
        Group::Critical => player.critical,
        Group::Skill => player.skill,
        Group::Cast => player.cast,
        Group::Item => player.item,
        Group::Other => player.other,
    }
}

/// Torrent's own animations, by id: walk `0021xx`, run `0022xx` (dash included), jump take-off
/// `0061xx` and landing `0074xx`. Standing and turning are `Other`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TorrentGroup {
    Walk,
    Run,
    Jump,
    Other,
}

impl TorrentGroup {
    fn speed(self, torrent: &Torrent) -> f32 {
        match self {
            TorrentGroup::Walk => torrent.walk,
            TorrentGroup::Run => torrent.run,
            TorrentGroup::Jump => torrent.jump,
            TorrentGroup::Other => torrent.other,
        }
    }
}

pub fn torrent_group_of(anim_id: i32) -> TorrentGroup {
    match anim_id {
        2_100..=2_199 => TorrentGroup::Walk,
        2_200..=2_299 => TorrentGroup::Run,
        6_100..=6_199 | 7_400..=7_499 => TorrentGroup::Jump,
        _ => TorrentGroup::Other,
    }
}

/// Torrent's `ChrIns`: the active `summon_buddy_chr_set` entry with one of [TORRENT_NPC_PARAM_IDS].
pub fn torrent(world_chr_man: &mut WorldChrMan) -> Option<&mut ChrIns> {
    let mut found: Option<*mut ChrIns> = None;
    shared::player::for_each_active_chr(&world_chr_man.summon_buddy_chr_set, |chr| {
        if found.is_none() && TORRENT_NPC_PARAM_IDS.contains(&chr.npc_param_id) {
            found = Some(chr as *mut ChrIns);
        }
    });
    // SAFETY: the pointer came from a live entry of `world_chr_man`, which stays borrowed.
    found.map(|chr| unsafe { &mut *chr })
}

/// The raw `throw_state` of a character, `None` without a throw node. Read as a `u32` so a value
/// outside the known enum is not undefined behavior.
pub fn throw_state(chr: &ChrIns) -> Option<u32> {
    let module = &*chr.modules.throw;
    // `throw_node` is the third pointer-sized field (vftable, owner, node).
    // SAFETY: the module is a live `CSChrThrowModule`, whose third field is read as a pointer.
    let node = unsafe { *(module as *const _ as *const usize).add(2) };
    if node == 0 {
        return None;
    }
    let node = node as *const CSThrowNode;
    // SAFETY: a non-null `throw_node` points at a live node.
    Some(unsafe { std::ptr::addr_of!((*node).throw_state).cast::<u32>().read_volatile() })
}

/// `throw_state` of a backstab or riposte: the attacker goes 1, 3, 5 (the last one when the victim
/// dies), the victim 2, 4, 6, and back to 0 a second or two after the attacker.
fn is_throw_attacker(state: u32) -> bool {
    matches!(state, 1 | 3 | 5)
}

fn is_throw_victim(state: u32) -> bool {
    matches!(state, 2 | 4 | 6)
}

pub fn is_throw_victim_now(chr: &ChrIns) -> bool {
    throw_state(chr).is_some_and(is_throw_victim)
}

/// The victim of a critical hit is paired with the player's animation, so it runs at the player's
/// speed while the player is in the throw (`docs/critical_victim.md`). `sped` holds the victims
/// written to, to put them back to 1.
fn sync_victims(world_chr_man: &WorldChrMan, sped: &mut Vec<usize>) {
    let speed = world_chr_man.main_player.as_ref().and_then(|player| {
        throw_state(&player.chr_ins)
            .filter(|&state| is_throw_attacker(state))
            .map(|_| clamped(player.chr_ins.modules.behavior.animation_speed))
    });
    let mut seen = Vec::new();
    crate::enemy::for_each_active(world_chr_man, |chr| {
        let key = chr as *const ChrIns as usize;
        let victim = is_throw_victim_now(chr);
        match speed {
            Some(speed) if victim => {
                set_animation_speed(chr, speed);
                seen.push(key);
            }
            _ if sped.contains(&key) => set_animation_speed(chr, 1.0),
            _ => {}
        }
    });
    *sped = seen;
}

/// Every live spirit: the `summon_buddy_chr_set` entries that are not Torrent.
fn for_each_spirit(world_chr_man: &WorldChrMan, mut f: impl FnMut(&mut ChrIns)) {
    shared::player::for_each_active_chr(&world_chr_man.summon_buddy_chr_set, |chr| {
        if !TORRENT_NPC_PARAM_IDS.contains(&chr.npc_param_id) {
            f(chr);
        }
    });
}

/// 0 or less would freeze or reverse the animation.
fn clamped(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.1, 10.0) } else { 1.0 }
}

pub fn set_animation_speed(chr: &mut ChrIns, value: f32) {
    let behavior = &mut chr.modules.behavior;
    if (behavior.animation_speed - value).abs() > 0.0001 {
        behavior.animation_speed = value;
    }
}

/// The master `all` key when it is not 1, else the group's own.
fn pick_speed(all: f32, group: f32) -> f32 {
    let master = clamped(all);
    if (master - 1.0).abs() > 0.0001 { master } else { clamped(group) }
}

struct State {
    /// The active `[[Override]]`s of the last frame, so a change is logged once.
    last_active: Vec<usize>,
    getter_fix: crate::seamless::GetterFix,
    /// Characters written to, to put them back to 1 when they stop counting.
    sped_victims: Vec<usize>,
    sped_enemies: Vec<usize>,
}

fn apply(state: &mut State) {
    if shared::player::main_player_chr_ins_ptr().is_none() {
        return;
    }
    // SAFETY: called from a game task; `instance_mut` returns `Err` while the singleton does not exist.
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let config = config::get();

    // The player's SpEffects decide which `[[Override]]`s are on, Torrent's too (a buff on the rider).
    let sp_effects: Vec<i32> = match world_chr_man.main_player.as_ref() {
        Some(player) if !config.overrides.is_empty() => {
            player.chr_ins.special_effect.entries().map(|entry| entry.param_id).collect()
        }
        _ => Vec::new(),
    };
    let load = world_chr_man.main_player.as_ref().and_then(|player| {
        config::LoadClass::from_weight_type(player.chr_ins.chr_ctrl.weight_type)
    });
    let (speeds, active) = config.effective_speed(|id| sp_effects.contains(&id), load);
    if active != state.last_active {
        let list = active.iter().map(|i| format!("#{}", i + 1)).collect::<Vec<_>>().join(", ");
        logger::log(&format!("Speed: overrides {}", if list.is_empty() { "none" } else { &list }));
        state.last_active = active;
    }

    // Seamless Co-op hooks the game's read of the speed (`seamless.rs`): tell the stub which
    // characters are ours, then check the hook.
    let player_ptr =
        world_chr_man.main_player.as_ref().map_or(0, |p| &p.chr_ins as *const ChrIns as usize);
    let torrent_ptr = torrent(world_chr_man).map_or(0, |t| t as *const ChrIns as usize);
    crate::seamless::set_own(player_ptr, torrent_ptr);
    state.getter_fix.check();

    if let Some(player) = world_chr_man.main_player.as_mut() {
        let chr = &mut player.chr_ins;
        let group = group_of(current_anim_id(chr));
        set_animation_speed(chr, pick_speed(speeds.player.all, group_speed(group, &speeds.player)));
    }
    // With a hooked getter everyone else reads 1: the stub passes only the player's and Torrent's speed.
    sync_victims(world_chr_man, &mut state.sped_victims);
    crate::enemy::apply(world_chr_man, clamped(config.enemy.all), &mut state.sped_enemies);
    let spirit_speed = clamped(config.spirit.all);
    for_each_spirit(world_chr_man, |chr| set_animation_speed(chr, spirit_speed));
    if let Some(torrent) = torrent(world_chr_man) {
        let group = torrent_group_of(current_anim_id(torrent)).speed(&speeds.torrent);
        set_animation_speed(torrent, pick_speed(speeds.torrent.all, group));
    }
}

pub fn run() {
    let cs_task = shared::task::wait_for_cs_task();
    let mut state = State {
        last_active: Vec::new(),
        getter_fix: crate::seamless::GetterFix::new(),
        sped_victims: Vec::new(),
        sped_enemies: Vec::new(),
    };
    shared::task::run_recurring_safe(
        cs_task,
        "Speed",
        CSTaskGroupIndex::ChrIns_PreBehavior,
        move |_data: &eldenring::fd4::FD4TaskData| apply(&mut state),
    );
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_master_key_other_than_one_overrides_the_group() {
        assert_eq!(pick_speed(2.0, 5.0), 2.0);
        assert_eq!(pick_speed(0.5, 1.0), 0.5);
        assert_eq!(pick_speed(1.0, 1.5), 1.5); // master at 1: the group decides
    }

    #[test]
    fn speeds_stay_within_a_sane_range() {
        assert_eq!(clamped(0.0), 0.1); // 0 would freeze the animation
        assert_eq!(clamped(-3.0), 0.1);
        assert_eq!(clamped(50.0), 10.0);
        assert_eq!(clamped(f32::NAN), 1.0);
        assert_eq!(clamped(f32::INFINITY), 1.0);
        assert_eq!(pick_speed(1.0, 0.0), 0.1);
    }

    #[test]
    fn throw_states_split_attacker_and_victim() {
        for state in [1, 3, 5] {
            assert!(is_throw_attacker(state) && !is_throw_victim(state), "{state}");
        }
        for state in [2, 4, 6] {
            assert!(is_throw_victim(state) && !is_throw_attacker(state), "{state}");
        }
        assert!(!is_throw_attacker(0) && !is_throw_victim(0));
    }

    #[test]
    fn torrent_gaits_are_split() {
        assert_eq!(torrent_group_of(2_100), TorrentGroup::Walk);
        assert_eq!(torrent_group_of(2_110), TorrentGroup::Walk);
        assert_eq!(torrent_group_of(2_220), TorrentGroup::Run);
        assert_eq!(torrent_group_of(2_210), TorrentGroup::Run); // slowing down
        assert_eq!(torrent_group_of(0), TorrentGroup::Other); // standing
        assert_eq!(torrent_group_of(5_102), TorrentGroup::Other); // turning
        assert_eq!(torrent_group_of(6_110), TorrentGroup::Jump); // standing jump
        assert_eq!(torrent_group_of(6_130), TorrentGroup::Jump); // moving jump
        assert_eq!(torrent_group_of(7_400), TorrentGroup::Jump); // landing
        assert_eq!(torrent_group_of(7_451), TorrentGroup::Jump); // moving landing
    }

    #[test]
    fn ladder_anims_are_their_own_group() {
        for tail in [
            28_999, 28_100, 28_030, 28_011, 28_012, 28_013, 28_020, 28_023, 28_000, 28_002, 28_036,
            28_040, 28_045,
        ] {
            assert_eq!(group_of(tail), Group::Ladder, "{tail}");
        }
        assert_eq!(group_of(27_999), Group::Roll);
        assert_eq!(group_of(29_000), Group::Other);
        let player = Player { ladder: 1.5, ..crate::config::template().player };
        assert_eq!(group_speed(Group::Ladder, &player), 1.5);
    }

    #[test]
    fn critical_hits_are_their_own_group() {
        assert_eq!(group_of(23_031_700), Group::Critical); // riposte
        assert_eq!(group_of(23_031_719), Group::Critical); // backstab start
        assert_eq!(group_of(23_031_710), Group::Critical); // backstab
        assert_eq!(group_of(23_030_000), Group::Attack);
        assert_eq!(group_of(23_031_699), Group::Attack);
        assert_eq!(group_of(23_031_800), Group::Attack);
    }

    #[test]
    fn critical_hits_use_their_own_key() {
        let defaults = crate::config::template().player;
        let player = Player { attack: 3.0, ..defaults.clone() };
        assert_eq!(group_speed(Group::Critical, &player), defaults.critical);
        let player = Player { critical: 1.5, ..crate::config::template().player };
        assert_eq!(group_speed(Group::Critical, &player), 1.5);
        assert_eq!(group_speed(Group::Attack, &player), defaults.attack);
    }
}
