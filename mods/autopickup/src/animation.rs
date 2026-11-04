//! `NoPickupAnimation`: replaces the pickup animation IDs (treasure shine, corpse, chest, dropped
//! item) with one the game doesn't have, and restores the game's own values when switched off.
//! See `docs/pickup_animation.md`.

use std::sync::Mutex;

use eldenring::cs::{GameSystemCommonParam, PlayerCommonParam, SoloParamRepository};

use shared::{logger, params};

const NO_ANIMATION: i32 = 999_999;

#[derive(Clone, Copy)]
struct Vanilla {
    drop_item_pick: i32,
    shine: i32,
    corpse: i32,
    chest: i32,
}

static VANILLA: Mutex<Option<Vanilla>> = Mutex::new(None);

fn target(vanilla: i32, disable: bool) -> i32 {
    if disable { NO_ANIMATION } else { vanilla }
}

pub fn apply(repo: &mut SoloParamRepository, disable: bool) -> Result<(), String> {
    params::check::<PlayerCommonParam>(repo)?;
    params::check::<GameSystemCommonParam>(repo)?;

    let mut stored = VANILLA.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let vanilla = match *stored {
        Some(vanilla) => vanilla,
        None => {
            let drop_item_pick = repo
                .get_row_by_index_mut::<PlayerCommonParam>(0)
                .ok_or("PlayerCommonParam row 0 not found")?
                .anime_id_drop_item_pick();
            let system = repo
                .get_row_by_index_mut::<GameSystemCommonParam>(0)
                .ok_or("GameSystemCommonParam row 0 not found")?;
            let vanilla = Vanilla {
                drop_item_pick,
                shine: system.msb_event_geom_treasure_info_item_get_anim_id_shine(),
                corpse: system.msb_event_geom_treasure_info_item_get_anim_id_corpse(),
                chest: system.msb_event_geom_treasure_info_item_get_anim_id_box(),
            };
            logger::log(&format!(
                "Animation: drop item pick {}, shine {}, corpse {}, chest {}",
                vanilla.drop_item_pick, vanilla.shine, vanilla.corpse, vanilla.chest
            ));
            *stored = Some(vanilla);
            vanilla
        }
    };

    repo.get_row_by_index_mut::<PlayerCommonParam>(0)
        .ok_or("PlayerCommonParam row 0 not found")?
        .set_anime_id_drop_item_pick(target(vanilla.drop_item_pick, disable));
    let system = repo
        .get_row_by_index_mut::<GameSystemCommonParam>(0)
        .ok_or("GameSystemCommonParam row 0 not found")?;
    system.set_msb_event_geom_treasure_info_item_get_anim_id_shine(target(vanilla.shine, disable));
    system
        .set_msb_event_geom_treasure_info_item_get_anim_id_corpse(target(vanilla.corpse, disable));
    system.set_msb_event_geom_treasure_info_item_get_anim_id_box(target(vanilla.chest, disable));
    Ok(())
}

pub fn current(repo: &mut SoloParamRepository) -> Option<String> {
    let drop_item_pick =
        repo.get_row_by_index_mut::<PlayerCommonParam>(0)?.anime_id_drop_item_pick();
    let system = repo.get_row_by_index_mut::<GameSystemCommonParam>(0)?;
    Some(format!(
        "drop item pick {drop_item_pick}, shine {}, corpse {}, chest {}",
        system.msb_event_geom_treasure_info_item_get_anim_id_shine(),
        system.msb_event_geom_treasure_info_item_get_anim_id_corpse(),
        system.msb_event_geom_treasure_info_item_get_anim_id_box()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_option_swaps_between_the_games_id_and_none() {
        assert_eq!(target(60070, true), NO_ANIMATION);
        assert_eq!(target(60070, false), 60070);
    }
}
