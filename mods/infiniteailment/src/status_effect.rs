//! Rewrites the duration and damage of the damage-over-time effects the player inflicts (Scarlet
//! Rot, Poison, Black Flame, Destined Death) by editing the matching `SpEffectParam` rows of the
//! running game. Which rows, how the signature was found and why each `StateInfo` differs:
//! `docs/row_discovery.md` and `docs/black_flame.md`.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository, SpEffectParam};
use eldenring::param::SP_EFFECT_PARAM_ST;
use fromsoftware_shared::FromStatic;

use shared::logger;

use crate::config::{self, Config};

/// How an ailment's rows are recognised.
enum Target {
    /// By field signature: this `SpCategory` and `StateInfo`.
    Signature { category: u16, state_info: u16 },
    /// Only these row ids, and only if they still look like a damage row. Black Flame has no
    /// signature of its own: its rows look like Destined Death's and the NPC burns.
    Rows(&'static [u32]),
}

/// The three knobs applied to every row of one ailment.
struct Settings {
    duration: f32,
    percent_damage: f32,
    fixed_damage: i32,
}

struct Ailment {
    name: &'static str,
    target: Target,
    /// The vanilla `EffectEndurance` of the targeted rows. It is part of the signature, because it
    /// separates the player's rows from the NPC, swamp and boss ones.
    vanilla_duration: f32,
    /// `false` for a max HP debuff row: only `EffectEndurance` is written (no `ChangeHpRate`), and
    /// the vanilla duration differs per row.
    damage: bool,
    settings: fn(&Config) -> Settings,
    /// `(row id, row index)` of the targeted rows, found on the first scan.
    matched_rows: Mutex<Option<Vec<(u32, usize)>>>,
}

/// Black Flame's burn rows the player inflicts: Black Flame Tornado (835), Godslayer's Greatsword
/// (855), Gargoyle's Blackblade (860) and the incantations Black Flame (1621000), Scouring Black
/// Flame (1623000), Noble Presence (1624010), Black Flame Ritual (1625000) and Black Flame Blade
/// (1626001). The NPC burns (29000...) and Destined Death are left out.
const BLACK_FLAME_ROWS: [u32; 8] = [835, 855, 860, 1621000, 1623000, 1624010, 1625000, 1626001];
/// Destined Death's burn rows: "DoT Main" (6850) and its variants.
const DESTINED_DEATH_ROWS: [u32; 4] = [6840, 6841, 6850, 6851];
/// Destined Death's "Max HP Debuff" rows (15 s and 45 s): they lower max HP to 90% instead of
/// ticking damage.
const DESTINED_DEATH_DEBUFF_ROWS: [u32; 2] = [6860, 6865];

fn settings_of(duration: f64, percent_damage: f64, fixed_damage: i32) -> Settings {
    Settings { duration: duration as f32, percent_damage: percent_damage as f32, fixed_damage }
}

static SCARLET_ROT: Ailment = Ailment {
    name: "Scarlet Rot",
    target: Target::Signature { category: 10005, state_info: 5 },
    vanilla_duration: 90.0,
    damage: true,
    settings: |c| {
        settings_of(
            c.scarlet_rot.duration,
            c.scarlet_rot.percent_damage,
            c.scarlet_rot.fixed_damage,
        )
    },
    matched_rows: Mutex::new(None),
};

static POISON: Ailment = Ailment {
    name: "Poison",
    target: Target::Signature { category: 10004, state_info: 2 },
    vanilla_duration: 90.0,
    damage: true,
    settings: |c| settings_of(c.poison.duration, c.poison.percent_damage, c.poison.fixed_damage),
    matched_rows: Mutex::new(None),
};

static BLACK_FLAME: Ailment = Ailment {
    name: "Black Flame",
    target: Target::Rows(&BLACK_FLAME_ROWS),
    vanilla_duration: 2.0,
    damage: true,
    settings: |c| {
        settings_of(
            c.black_flame.duration,
            c.black_flame.percent_damage,
            c.black_flame.fixed_damage,
        )
    },
    matched_rows: Mutex::new(None),
};

static DESTINED_DEATH: Ailment = Ailment {
    name: "Destined Death",
    target: Target::Rows(&DESTINED_DEATH_ROWS),
    vanilla_duration: 2.0,
    damage: true,
    settings: |c| {
        settings_of(
            c.destined_death.duration,
            c.destined_death.percent_damage,
            c.destined_death.fixed_damage,
        )
    },
    matched_rows: Mutex::new(None),
};

static DESTINED_DEATH_MAX_HP: Ailment = Ailment {
    name: "Destined Death max HP loss",
    target: Target::Rows(&DESTINED_DEATH_DEBUFF_ROWS),
    vanilla_duration: 0.0,
    damage: false,
    settings: |c| settings_of(c.destined_death.max_hp_duration, 0.0, 0),
    matched_rows: Mutex::new(None),
};

const AILMENTS: [&Ailment; 5] =
    [&SCARLET_ROT, &POISON, &BLACK_FLAME, &DESTINED_DEATH, &DESTINED_DEATH_MAX_HP];

/// Whether `row` is one the player inflicts for `ailment`. Only evaluated against a row that still
/// holds its vanilla `EffectEndurance` (the first scan), since the signature includes it.
fn matches(ailment: &Ailment, row: &SP_EFFECT_PARAM_ST) -> bool {
    if !ailment.damage {
        return row.effect_endurance() > 0.0;
    }
    let (category_ok, state_info) = match ailment.target {
        Target::Signature { category, state_info } => (row.sp_category() == category, state_info),
        Target::Rows(_) => (true, 0),
    };
    row.effect_endurance() == ailment.vanilla_duration
        && category_ok
        && row.state_info() == state_info
        && row.change_hp_rate() != 0.0
}

/// Finds `ailment`'s rows by index, not with `rows()`, which panics on some modded regulations
/// (`shared/docs/params.md`).
fn scan(ailment: &Ailment, repo: &mut SoloParamRepository) -> Result<Vec<(u32, usize)>, String> {
    let row_ids = shared::params::row_ids_or_reason::<SpEffectParam>(repo)?;
    let mut found = Vec::new();
    shared::params::for_each_row_mut::<SpEffectParam>(repo, |index, row| {
        let Some(&id) = row_ids.get(index) else {
            return;
        };
        let listed = match ailment.target {
            Target::Rows(ids) => ids.contains(&id),
            Target::Signature { .. } => true,
        };
        if listed && matches(ailment, row) {
            found.push((id, index));
        }
    });
    Ok(found)
}

/// Sets `EffectEndurance` and, for damage rows, `ChangeHpRate` and `ChangeHpPoint` on every row of
/// `ailment`: found by signature on the first call, then by the cached indexes, since an edited row
/// no longer matches the signature. Returns the rows written.
fn apply(ailment: &Ailment, repo: &mut SoloParamRepository, settings: &Settings) -> usize {
    let mut cache = ailment.matched_rows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.is_none() {
        // A failed scan is not cached, so the next apply tries again.
        match scan(ailment, repo) {
            Ok(found) => *cache = Some(found),
            Err(reason) => {
                logger::warn(&format!(
                    "{}: SpEffectParam row ids unreadable ({reason})",
                    ailment.name
                ));
                return 0;
            }
        }
    }
    let Some(rows) = cache.as_ref() else {
        return 0;
    };

    let mut written = 0;
    for &(_, index) in rows {
        if let Some(row) = repo.get_row_by_index_mut::<SpEffectParam>(index) {
            row.set_effect_endurance(settings.duration);
            if ailment.damage {
                row.set_change_hp_rate(settings.percent_damage);
                row.set_change_hp_point(settings.fixed_damage);
            }
            written += 1;
        }
    }
    written
}

fn apply_all(repo: &mut SoloParamRepository, suffix: &str) {
    let config = config::get();
    for ailment in AILMENTS {
        let settings = (ailment.settings)(&config);
        let written = apply(ailment, repo, &settings);
        let values = if ailment.damage {
            format!(
                "duration {} percent {} fixed {}",
                settings.duration, settings.percent_damage, settings.fixed_damage
            )
        } else {
            format!("duration {}", settings.duration)
        };
        logger::log(&format!("{}: {values}, {written} row(s){suffix}", ailment.name));
    }
}

/// Set by a reload or a menu save; the tick applies it once the player is in the world.
static APPLY_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_apply() {
    APPLY_REQUESTED.store(true, Ordering::Relaxed);
}

/// Applies every ailment once the world is loaded, then again on every [request_apply]. Run it on
/// its own worker thread; never returns.
pub fn run() {
    apply_all(shared::player::wait_for_solo_param_repository(), "");

    let mut waiting_logged = false;
    let cs_task = shared::task::wait_for_cs_task();
    shared::task::run_recurring_safe(
        cs_task,
        "InfiniteAilment",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            if !APPLY_REQUESTED.load(Ordering::Relaxed) {
                return;
            }
            // A change made from the title screen's menu waits for the world.
            if shared::player::main_player_chr_ins_ptr().is_none() {
                if !waiting_logged {
                    logger::warn("Ailment: change deferred, not in world");
                    waiting_logged = true;
                }
                return;
            }
            waiting_logged = false;
            APPLY_REQUESTED.store(false, Ordering::Relaxed);
            // SAFETY: called from a game task; `instance_mut` returns `Err` while the singleton does not exist.
            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                logger::warn("Ailment: SoloParamRepository unavailable, change skipped");
                return;
            };
            apply_all(repo, " (changed)");
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_lists_do_not_overlap() {
        let lists: [&[u32]; 3] =
            [&BLACK_FLAME_ROWS, &DESTINED_DEATH_ROWS, &DESTINED_DEATH_DEBUFF_ROWS];
        for (i, a) in lists.iter().enumerate() {
            for b in &lists[i + 1..] {
                assert!(
                    a.iter().all(|id| !b.contains(id)),
                    "a row would be written by two ailments"
                );
            }
        }
    }

    #[test]
    fn settings_come_from_the_template() {
        let config = config::get();
        let rot = (SCARLET_ROT.settings)(&config);
        assert_eq!((rot.duration, rot.fixed_damage), (-1.0, 15));
        let max_hp = (DESTINED_DEATH_MAX_HP.settings)(&config);
        assert_eq!(max_hp.duration, -1.0);
    }
}
