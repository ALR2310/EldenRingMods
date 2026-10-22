//! Rewrites the duration and damage of the damage-over-time effects the
//! player inflicts (Scarlet Rot, Poison, Black Flame, Destined Death) by
//! editing the matching `SpEffectParam` rows of the running game. Which rows
//! are targeted, how the signature was found, and why each ailment's
//! `StateInfo` differs: `docs/row_discovery.md` and `docs/black_flame.md`.

use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository, SpEffectParam};
use eldenring::param::SP_EFFECT_PARAM_ST;
use fromsoftware_shared::FromStatic;

use common::{config, logger};

/// How an ailment's rows are recognised.
enum Target {
    /// By field signature: this `SpCategory` and `StateInfo`.
    Signature { category: u16, state_info: u16 },
    /// Only these row IDs, and only if they still look like a damage row
    /// (Black Flame has no signature of its own, its rows look like Destined
    /// Death's and the NPC burns).
    Rows(&'static [u32]),
}

/// One status ailment: which rows it targets and its ini keys and defaults.
struct Ailment {
    name: &'static str,
    target: Target,
    /// The vanilla `EffectEndurance` of the targeted rows; part of the
    /// signature itself, because it is what separates the player's rows from
    /// the NPC, swamp and boss ones.
    vanilla_duration: f32,
    /// `false` = a max-HP debuff row: only `EffectEndurance` is written (no
    /// `ChangeHpRate`), and the vanilla duration varies per row.
    damage: bool,
    /// Ini key prefix (`ScarletRot` -> `ScarletRot.PercentDamage`...).
    prefix: &'static str,
    duration_key: &'static str,
    default_duration: f64,
    default_percent_damage: f64,
    default_fixed_damage: i32,
    /// `(row ID, row index)` of the targeted rows, found on the first scan.
    matched_rows: Mutex<Option<Vec<(u32, usize)>>>,
}

/// Black Flame's burn rows the player inflicts: Black Flame Tornado (835),
/// Godslayer's Greatsword (855), Gargoyle's Blackblade (860), and the
/// incantations Black Flame (1621000), Scouring Black Flame (1623000), Noble
/// Presence (1624010), Black Flame Ritual (1625000), Black Flame Blade
/// (1626001). NPC burns (29000...) and Destined Death are left out.
const BLACK_FLAME_ROWS: [u32; 8] = [835, 855, 860, 1621000, 1623000, 1624010, 1625000, 1626001];
/// Destined Death's burn rows: "DoT Main" (6850) and its variants.
const DESTINED_DEATH_ROWS: [u32; 4] = [6840, 6841, 6850, 6851];
/// Destined Death's "Max HP Debuff" rows (15 s and 45 s): they lower max HP
/// to 90% instead of ticking damage.
const DESTINED_DEATH_DEBUFF_ROWS: [u32; 2] = [6860, 6865];

static SCARLET_ROT: Ailment = Ailment {
    name: "Scarlet Rot",
    target: Target::Signature { category: 10005, state_info: 5 },
    vanilla_duration: 90.0,
    damage: true,
    prefix: "ScarletRot",
    duration_key: "ScarletRot.Duration",
    default_duration: -1.0,
    default_percent_damage: 0.18,
    default_fixed_damage: 15,
    matched_rows: Mutex::new(None),
};

static POISON: Ailment = Ailment {
    name: "Poison",
    target: Target::Signature { category: 10004, state_info: 2 },
    vanilla_duration: 90.0,
    damage: true,
    prefix: "Poison",
    duration_key: "Poison.Duration",
    default_duration: -1.0,
    default_percent_damage: 0.07,
    default_fixed_damage: 7,
    matched_rows: Mutex::new(None),
};

static BLACK_FLAME: Ailment = Ailment {
    name: "Black Flame",
    target: Target::Rows(&BLACK_FLAME_ROWS),
    vanilla_duration: 2.0,
    damage: true,
    prefix: "BlackFlame",
    duration_key: "BlackFlame.Duration",
    default_duration: 10.0,
    default_percent_damage: 0.1,
    default_fixed_damage: 1,
    matched_rows: Mutex::new(None),
};

static DESTINED_DEATH: Ailment = Ailment {
    name: "Destined Death",
    target: Target::Rows(&DESTINED_DEATH_ROWS),
    vanilla_duration: 2.0,
    damage: true,
    prefix: "DestinedDeath",
    duration_key: "DestinedDeath.Duration",
    default_duration: 10.0,
    default_percent_damage: 0.1,
    default_fixed_damage: 1,
    matched_rows: Mutex::new(None),
};

static DESTINED_DEATH_MAX_HP: Ailment = Ailment {
    name: "Destined Death max HP loss",
    target: Target::Rows(&DESTINED_DEATH_DEBUFF_ROWS),
    vanilla_duration: 0.0,
    damage: false,
    prefix: "DestinedDeath",
    duration_key: "DestinedDeath.MaxHpDuration",
    default_duration: -1.0,
    default_percent_damage: 0.0,
    default_fixed_damage: 0,
    matched_rows: Mutex::new(None),
};

const AILMENTS: [&Ailment; 5] = [&SCARLET_ROT, &POISON, &BLACK_FLAME, &DESTINED_DEATH, &DESTINED_DEATH_MAX_HP];

/// The three knobs applied to every row of one ailment, read fresh on every
/// apply (startup and each reload).
struct Settings {
    duration: f32,
    percent_damage: f32,
    fixed_damage: i32,
}

fn build_settings(ailment: &Ailment) -> Settings {
    let prefix = ailment.prefix;
    Settings {
        duration: config::get_double(ailment.duration_key, ailment.default_duration) as f32,
        percent_damage: config::get_double(&format!("{prefix}.PercentDamage"), ailment.default_percent_damage) as f32,
        fixed_damage: config::get_int(&format!("{prefix}.FixedDamage"), ailment.default_fixed_damage),
    }
}

/// Whether `row` is one the player inflicts for `ailment`. Only evaluated
/// against a row still holding its vanilla `EffectEndurance` (the first scan):
/// the signature includes it, so an edited row would stop matching.
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

/// Finds `ailment`'s rows by index (not `rows()`, which panics on some modded
/// regulations, see `common::params`); the ID is kept for the row lists.
fn scan(ailment: &Ailment, repo: &mut SoloParamRepository) -> Result<Vec<(u32, usize)>, String> {
    let row_ids = common::params::row_ids_or_reason::<SpEffectParam>(repo)?;
    let mut found = Vec::new();
    common::params::for_each_row_mut::<SpEffectParam>(repo, |index, row| {
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

/// Sets `EffectEndurance` (and, for damage rows, `ChangeHpRate` and
/// `ChangeHpPoint`) from `settings` on every row of `ailment`: found by
/// signature on the first call, then by the cached indexes. Returns how many
/// rows were written.
fn apply(ailment: &Ailment, repo: &mut SoloParamRepository, settings: &Settings) -> usize {
    let mut cache = ailment.matched_rows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.is_none() {
        // A failed scan is not cached, so the next reload tries again.
        match scan(ailment, repo) {
            Ok(found) => *cache = Some(found),
            Err(reason) => {
                logger::warn(&format!("{}: couldn't read the SpEffectParam row ids ({reason})", ailment.name));
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

fn apply_and_log(ailment: &Ailment, repo: &mut SoloParamRepository, suffix: &str) {
    let settings = build_settings(ailment);
    let written = apply(ailment, repo, &settings);
    let values = if ailment.damage {
        format!(
            "duration={} percent={} fixed={}",
            settings.duration, settings.percent_damage, settings.fixed_damage
        )
    } else {
        format!("duration={}", settings.duration)
    };
    logger::log(&format!("{}: {values} -> {written} row(s){suffix}", ailment.name));
}

fn apply_all(repo: &mut SoloParamRepository, suffix: &str) {
    for ailment in AILMENTS {
        apply_and_log(ailment, repo, suffix);
    }
}

/// Applies every ailment once the world is loaded, then follows
/// `RELOAD_GENERATION` for the rest of the DLL's life (`common::reload` owns
/// the reload key). Run it on its own worker thread; never returns.
pub fn run() {
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    apply_all(common::player::wait_for_solo_param_repository(), "");

    let mut waiting_logged = false;
    let cs_task = common::task::wait_for_cs_task();
    common::task::run_recurring_safe(
        cs_task,
        "InfiniteAilment",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if generation == last_seen_generation {
                return;
            }
            // Not in the world (a change made from the title screen's menu):
            // keep the request and apply it once the world is loaded.
            if common::player::main_player_chr_ins_ptr().is_none() {
                if !waiting_logged {
                    logger::warn("Reload: deferred, not in world");
                    waiting_logged = true;
                }
                return;
            }
            waiting_logged = false;
            last_seen_generation = generation;
            crate::menu::sync_from_config();

            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                logger::warn("Reload: SoloParamRepository unavailable, skipped");
                return;
            };
            apply_all(repo, " (reload)");
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INI: &str = include_str!("../InfiniteAilment.ini");

    /// The ini template's value for `key`.
    fn ini_value(key: &str) -> f64 {
        let line = INI
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{key}=")))
            .unwrap_or_else(|| panic!("the template has no {key}"));
        line.split_once('=').unwrap().1.trim().parse().unwrap()
    }

    /// The defaults in code and in the ini template are written twice; they
    /// must agree, or a missing key and a fresh file would behave differently.
    #[test]
    fn code_defaults_match_the_ini_template() {
        for ailment in AILMENTS {
            assert_eq!(ini_value(ailment.duration_key), ailment.default_duration, "{}", ailment.duration_key);
            if ailment.damage {
                let prefix = ailment.prefix;
                assert_eq!(ini_value(&format!("{prefix}.PercentDamage")), ailment.default_percent_damage, "{prefix}");
                assert_eq!(ini_value(&format!("{prefix}.FixedDamage")), ailment.default_fixed_damage as f64, "{prefix}");
            }
        }
    }

    #[test]
    fn row_lists_do_not_overlap() {
        let lists: [&[u32]; 3] = [&BLACK_FLAME_ROWS, &DESTINED_DEATH_ROWS, &DESTINED_DEATH_DEBUFF_ROWS];
        for (i, a) in lists.iter().enumerate() {
            for b in &lists[i + 1..] {
                assert!(a.iter().all(|id| !b.contains(id)), "a row would be written by two ailments");
            }
        }
    }
}
