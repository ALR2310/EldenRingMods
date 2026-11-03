//! The hostile enemies near the player, for the minimap's red dots (`Minimap.Layers.Enemies`).
//! The classification is the one `speedmultiplier` uses for `[Enemy] All` (type, team and
//! `NpcParam.threatLv > 0`, `mods/speedmultiplier/docs/enemy_speed.md`), copied here until it
//! moves to `common`. `docs/radar.md`.

use std::collections::HashMap;
use std::sync::Mutex;

use eldenring::cs::{ChrIns, ChrType, NpcParam, SoloParamRepository, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::logger;

use crate::settings;

/// `TEAM_TYPE` values (Smithbox enum) that are not enemies: None, Live, White / Black / Grey /
/// Wandering Ghost, Ally, Decoy, Battle Ally, Invader (13, 16-18), Neutral, Charmed, Host, Co-op,
/// Friendly NPC, Co-op NPC, Object, the Mad Phantoms, Spirit Summon. Torrent is team 10.
const NOT_ENEMY_TEAMS: [u8; 23] = [0, 1, 2, 3, 4, 5, 8, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 26, 28, 30, 31, 32, 47];

/// `NpcParam` id -> `threatLv`; None until built, empty if it could not be.
static THREAT: Mutex<Option<HashMap<i32, u32>>> = Mutex::new(None);

fn build_threat_table() -> HashMap<i32, u32> {
    let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return HashMap::new();
    };
    let Some(ids) = common::params::row_ids::<NpcParam>(repo) else {
        logger::warn("Enemies: could not read the NpcParam row ids, enemy dots are off");
        return HashMap::new();
    };
    let mut table = HashMap::with_capacity(ids.len());
    for (index, id) in ids.into_iter().enumerate() {
        if let Some(row) = repo.get_row_by_index::<NpcParam>(index) {
            table.insert(id as i32, row.threat_lv());
        }
    }
    logger::log(&format!("Enemies: threat level of {} NpcParam rows", table.len()));
    table
}

/// Bosses are `Unk7`, not `Npc`.
fn is_enemy_type(chr_type: ChrType) -> bool {
    matches!(chr_type, ChrType::Npc | ChrType::Unk7)
}

fn is_enemy(chr: &ChrIns, threat: &HashMap<i32, u32>) -> bool {
    is_enemy_type(chr.chr_type)
        && !NOT_ENEMY_TEAMS.contains(&chr.team_type)
        && threat.get(&chr.npc_param_id).is_some_and(|&level| level > 0)
}

/// `f(chr)` for every living hostile enemy among the active characters of `WorldChrMan::chr_sets`.
/// With `Logging.EnemyProbe` it logs how many there are and in how many sets, when that changes.
pub fn for_each(world_chr_man: &WorldChrMan, mut f: impl FnMut(&ChrIns)) {
    let mut guard = THREAT.lock().unwrap_or_else(|e| e.into_inner());
    let threat = guard.get_or_insert_with(build_threat_table);
    let (mut total, mut sets_with, mut sets) = (0usize, 0usize, 0usize);
    for set in world_chr_man.chr_sets.iter().flatten() {
        sets += 1;
        let before = total;
        common::player::for_each_active_chr(set, |chr| {
            if is_enemy(chr, threat) && chr.modules.data.hp > 0 {
                total += 1;
                f(chr);
            }
        });
        sets_with += usize::from(total > before);
    }
    if settings::enemy_probe() {
        probe(total, sets_with, sets);
    }
}

fn probe(total: usize, sets_with: usize, sets: usize) {
    static LAST: Mutex<(usize, usize, usize)> = Mutex::new((usize::MAX, 0, 0));
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if *last != (total, sets_with, sets) {
        *last = (total, sets_with, sets);
        logger::log(&format!("Enemies: {total} hostile in {sets_with} of {sets} chr_set(s)"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_npcs_and_bosses_can_be_enemies() {
        assert!(is_enemy_type(ChrType::Npc));
        assert!(is_enemy_type(ChrType::Unk7));
        assert!(!is_enemy_type(ChrType::Local));
    }

    #[test]
    fn friendly_and_summon_teams_are_not_enemies() {
        assert!(NOT_ENEMY_TEAMS.contains(&10)); // Torrent
        assert!(NOT_ENEMY_TEAMS.contains(&47)); // spirit summon
        assert!(!NOT_ENEMY_TEAMS.contains(&6)); // a hostile team
        assert!(NOT_ENEMY_TEAMS.windows(2).all(|w| w[0] < w[1]), "keep the list sorted and unique");
    }
}
