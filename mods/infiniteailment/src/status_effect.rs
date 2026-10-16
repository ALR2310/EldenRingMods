//! Makes Scarlet Rot's and Poison's damage-over-time ticks last for a
//! configurable duration (seconds) instead of their vanilla 90s, by
//! editing every `SpEffectParam` row that matches a specific field
//! signature - confirmed live, in-game, against the running
//! `SoloParamRepository` (not just a static CSV export).
//!
//! ## How the row set was found (Scarlet Rot)
//!
//! Earlier attempts assumed a single shared row (`505`, "Poison/Scarlet
//! Rot (Cycled)") controlled the DoT's duration - dropped once static CSV
//! analysis couldn't be trusted (see git history of this file). Instead,
//! the actual running game's `SpEffectParam` table was queried directly
//! and progressively narrowed down by adding one field condition at a
//! time, logging the matching row count and a wide field snapshot after
//! each addition, until the exact intended row set was isolated:
//!
//! - `SpCategory == 10005` and `StateInfo == 5`: Scarlet Rot's own
//!   category.
//! - `ChangeHpRate != 0`: only rows that actually drain HP over time
//!   (excludes pure buildup-boost/talisman rows).
//! - `EffectEndurance == 90`: the vanilla duration itself, which is also
//!   what separates the rows the *player inflicts on enemies* (this
//!   module's actual target) from unrelated rows that happened to share
//!   the first 3 conditions - confirmed by cross-referencing the matched
//!   IDs against SmithBox by hand:
//!   - `EffectEndurance=180`: environmental "Scarlet Rot Swamp" hazards
//!     (`TargetEnemy=false`, unlike every other match) and "NPC: Scarlet
//!     Rot" rows (an NPC-only AI-driven variant, unrelated to what the
//!     player's own weapons/spells inflict).
//!   - `EffectEndurance=300`: Malenia's own unique rot effect - a boss
//!     special case, not something to touch here.
//!   - `EffectEndurance=30`: 2 unnamed rows with no `CycleOccurrenceSpEffectId`
//!     (i.e. they don't feed back into the DoT cycle at all - structurally
//!     unrelated one-shots).
//!   - Excluding all of the above by requiring `EffectEndurance == 90`
//!     leaves exactly the "Type 1/Special N", "Low/Medium/High -
//!     Addition/Innate +0..+25" families, plus a handful of item/talisman/
//!     incantation rows that share the same signature - all genuinely
//!     player-inflicted Scarlet Rot.
//!
//! Poison (`SpCategory == 10004`) mirrors Scarlet Rot's own family names
//! row for row ("Type 1/Special N", "Low/Medium/High -
//! Addition/Innate/Special +N") and the same `EffectEndurance == 90`
//! outliers to exclude ("Type 2 - Special N" at `30`, "Poison Swamp"/"NPC:
//! Poison" at `180`, "NPC: Fast Poison" at `30`) - but **not** the same
//! `StateInfo`: assuming `StateInfo == 5` (Scarlet Rot's own value) for
//! Poison too matched 0 rows live in-game, since Poison actually uses
//! `StateInfo == 2` (confirmed by the user reading it directly in
//! SmithBox) - each ailment's own `state_info` is a separate field on
//! [Ailment] rather than a shared constant, precisely because it isn't
//! shared.
//!
//! Each ailment's `PercentDamage`/`FixedDamage` ini defaults below
//! (`ChangeHpRate`/`ChangeHpPoint`) are read from its own vanilla rows
//! directly in this same exported data, not guessed - a previous default
//! guess for Scarlet Rot's `FixedDamage` (assumed `0`) turned out wrong
//! (real vanilla is `15`, confirmed by the user reading it themselves in
//! SmithBox), so trust what's actually in the CSV/live data over
//! assumption.
//!
//! Supports `General.ReloadKey` hot reload via [`common::reload`], so
//! settings can be tweaked and re-applied without restarting the game -
//! each ailment's row IDs are snapshotted once on its first successful
//! match (while `EffectEndurance` still holds its vanilla value) and
//! reused on every later reload, rather than re-running the same
//! field-matching query against the live table. That query includes
//! `EffectEndurance == 90` itself, so once a row's been edited to
//! something other than `90` it would silently stop matching its own
//! query on the very next reload - looking like the edit "didn't take"
//! even though it did, just not reapplied. Re-scanning is only needed the
//! one time per ailment.

use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration as StdDuration;

use eldenring::cs::{CSTaskGroupIndex, SoloParamRepository, SpEffectParam};
use eldenring::param::SP_EFFECT_PARAM_ST;
use fromsoftware_shared::FromStatic;

use common::config;
use common::logger;

/// One status ailment's own `SpCategory`/`StateInfo` values, ini key
/// prefix, and vanilla `ChangeHpRate`/`ChangeHpPoint` defaults - see the
/// module doc comment for where each of these came from. `StateInfo`
/// differs per ailment (`5` for Scarlet Rot, `2` for Poison) - confirmed
/// live by the user reading it in SmithBox after the shared hardcoded `5`
/// matched 0 Poison rows.
struct Ailment {
    name: &'static str,
    /// The vanilla `EffectEndurance` of the targeted rows - also part of the
    /// filter signature itself (see the module doc comment for why it is
    /// required, not just `!= 0`).
    vanilla_duration: f32,
    /// `None` = any category (Black Flame rows come in categories 10 and 20).
    sp_category: Option<u16>,
    state_info: u16,
    /// `Some` = only these row IDs, and only if they still have the signature
    /// (Black Flame has no field signature of its own: its DoT rows look like
    /// Destined Death's and the NPC burns, see `docs/black_flame.md`).
    only_ids: Option<&'static [u32]>,
    /// `false` = a Max HP debuff row: only `EffectEndurance` is written (these
    /// rows have no `ChangeHpRate`), and the vanilla duration varies per row.
    damage: bool,
    duration_key: &'static str,
    default_duration: f64,
    ini_prefix: &'static str,
    default_percent_damage: f64,
    default_fixed_damage: i32,
    matched_row_ids: Mutex<Option<Vec<(u32, usize)>>>,
}

static SCARLET_ROT: Ailment = Ailment {
    name: "Scarlet Rot",
    vanilla_duration: 90.0,
    sp_category: Some(10005),
    state_info: 5,
    only_ids: None,
    damage: true,
    duration_key: "ScarletRot.Duration",
    default_duration: -1.0,
    ini_prefix: "ScarletRot",
    default_percent_damage: 0.18,
    default_fixed_damage: 15,
    matched_row_ids: Mutex::new(None),
};

static POISON: Ailment = Ailment {
    name: "Poison",
    vanilla_duration: 90.0,
    sp_category: Some(10004),
    state_info: 2,
    only_ids: None,
    damage: true,
    duration_key: "Poison.Duration",
    default_duration: -1.0,
    ini_prefix: "Poison",
    default_percent_damage: 0.07,
    default_fixed_damage: 7,
    matched_row_ids: Mutex::new(None),
};

/// Black Flame's burn rows the player inflicts (names from the SmithBox export):
/// Black Flame Tornado (835), Godslayer's Greatsword (855), Gargoyle's Blackblade
/// (860), and the incantations Black Flame (1621000), Scouring Black Flame
/// (1623000), Noble Presence (1624010), Black Flame Ritual (1625000) and Black
/// Flame Blade (1626001). NPC burns (29000...) and Destined Death are left out.
const BLACK_FLAME_ROWS: [u32; 8] = [835, 855, 860, 1621000, 1623000, 1624010, 1625000, 1626001];

static BLACK_FLAME: Ailment = Ailment {
    name: "Black Flame",
    vanilla_duration: 2.0,
    sp_category: None,
    state_info: 0,
    only_ids: Some(&BLACK_FLAME_ROWS),
    damage: true,
    duration_key: "BlackFlame.Duration",
    default_duration: 10.0,
    ini_prefix: "BlackFlame",
    default_percent_damage: 0.1,
    default_fixed_damage: 1,
    matched_row_ids: Mutex::new(None),
};

/// Destined Death's burn rows: "DoT Main" (6850) and its variants (6840, 6841,
/// 6851), same numbers as Black Flame's (see `docs/black_flame.md`).
const DESTINED_DEATH_ROWS: [u32; 4] = [6840, 6841, 6850, 6851];

static DESTINED_DEATH: Ailment = Ailment {
    name: "Destined Death",
    vanilla_duration: 2.0,
    sp_category: None,
    state_info: 0,
    only_ids: Some(&DESTINED_DEATH_ROWS),
    damage: true,
    duration_key: "DestinedDeath.Duration",
    default_duration: 10.0,
    ini_prefix: "DestinedDeath",
    default_percent_damage: 0.1,
    default_fixed_damage: 1,
    matched_row_ids: Mutex::new(None),
};

/// Destined Death's "Max HP Debuff" rows: 6860 (15 s) and 6865 (45 s), which
/// lower max HP to 90% instead of ticking damage.
const DESTINED_DEATH_DEBUFF_ROWS: [u32; 2] = [6860, 6865];

static DESTINED_DEATH_MAX_HP: Ailment = Ailment {
    name: "Destined Death max HP loss",
    vanilla_duration: 0.0,
    sp_category: None,
    state_info: 0,
    only_ids: Some(&DESTINED_DEATH_DEBUFF_ROWS),
    damage: false,
    duration_key: "DestinedDeath.MaxHpDuration",
    default_duration: -1.0,
    ini_prefix: "DestinedDeath",
    default_percent_damage: 0.0,
    default_fixed_damage: 0,
    matched_row_ids: Mutex::new(None),
};

/// The 3 user-configurable knobs applied to every row targeted by one
/// [Ailment], read fresh on every [apply] call (startup and every reload).
struct Settings {
    duration: f32,
    percent_damage: f32,
    fixed_damage: i32,
}

fn build_settings(ailment: &Ailment) -> Settings {
    Settings {
        duration: config::get_double(ailment.duration_key, ailment.default_duration) as f32,
        percent_damage: config::get_double(
            &format!("{}.PercentDamage", ailment.ini_prefix),
            ailment.default_percent_damage,
        ) as f32,
        fixed_damage: config::get_int(
            &format!("{}.FixedDamage", ailment.ini_prefix),
            ailment.default_fixed_damage,
        ),
    }
}

/// Whether `row` is one of the player-inflicted rows `ailment` targets -
/// see the module doc comment for how this exact signature was confirmed
/// against the live game. Only ever evaluated against a row still holding
/// its vanilla `EffectEndurance` (each ailment's first scan) - see
/// [Ailment::matched_row_ids].
fn matches(ailment: &Ailment, row: &SP_EFFECT_PARAM_ST) -> bool {
    if !ailment.damage {
        return row.effect_endurance() > 0.0;
    }
    row.effect_endurance() == ailment.vanilla_duration
        && ailment.sp_category.is_none_or(|category| row.sp_category() == category)
        && row.state_info() == ailment.state_info
        && row.change_hp_rate() != 0.0
}

/// Sets `EffectEndurance`, `ChangeHpRate`, and `ChangeHpPoint` from
/// `settings` on every row `ailment` targets - matched by field signature
/// on the first call for this ailment (caching the IDs found), and by the
/// cached IDs directly on every later call. Returns the IDs touched, for
/// logging.
fn apply(ailment: &Ailment, repo: &mut SoloParamRepository, settings: &Settings) -> Vec<u32> {
    let mut cache = ailment.matched_row_ids.lock().unwrap();
    if cache.is_none() {
        // Matched by row index (not `rows()`, which panics on some modded
        // regulations - see `common::params`); the ID is kept for logging.
        // A failed scan is not cached, so the next reload tries again.
        let row_ids = match common::params::row_ids_or_reason::<SpEffectParam>(repo) {
            Ok(ids) => ids,
            Err(reason) => {
                logger::warn(&format!("{}: couldn't read the SpEffectParam row ids: {reason}", ailment.name));
                return Vec::new();
            }
        };
        let mut found = Vec::new();
        common::params::for_each_row_mut::<SpEffectParam>(repo, |index, row| {
            let Some(&id) = row_ids.get(index) else {
                return;
            };
            let listed = ailment.only_ids.is_none_or(|ids| ids.contains(&id));
            if listed && matches(ailment, row) {
                found.push((id, index));
            }
        });
        *cache = Some(found);
    }
    let ids = cache.as_ref().expect("filled above");

    let mut touched = Vec::new();
    for &(id, index) in ids.iter() {
        if let Some(row) = repo.get_row_by_index_mut::<SpEffectParam>(index) {
            row.set_effect_endurance(settings.duration);
            if ailment.damage {
                row.set_change_hp_rate(settings.percent_damage);
                row.set_change_hp_point(settings.fixed_damage);
            }
            touched.push(id);
        }
    }
    touched
}

fn log_result(ailment: &Ailment, settings: &Settings, touched: &[u32], suffix: &str) {
    if !ailment.damage {
        logger::log(&format!(
            "SpEffectParam: EffectEndurance={} set on {} {} row(s){suffix}",
            settings.duration,
            touched.len(),
            ailment.name,
        ));
        return;
    }
    logger::log(&format!(
        "SpEffectParam: EffectEndurance={}, ChangeHpRate={}, ChangeHpPoint={} set on {} {} \
         row(s){suffix}",
        settings.duration,
        settings.percent_damage,
        settings.fixed_damage,
        touched.len(),
        ailment.name,
    ));
}

fn apply_and_log(ailment: &'static Ailment, repo: &mut SoloParamRepository, suffix: &str) {
    let settings = build_settings(ailment);
    let touched = apply(ailment, repo, &settings);
    log_result(ailment, &settings, &touched, suffix);
}

/// Applies both ailments' settings once, waiting (forever) for
/// `SoloParamRepository` to actually be populated, then watches
/// `General.ReloadKey` (via [`common::reload`]) on the game's own
/// `FrameBegin` task group for the rest of the DLL's lifetime, re-reading
/// and re-applying both on every press. Meant to run on its own worker
/// thread spawned from `DllMain`; never returns.
pub fn run() {
    let mut last_seen_generation = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);

    let repo = common::player::wait_for_solo_param_repository();
    apply_and_log(&SCARLET_ROT, repo, "");
    apply_and_log(&POISON, repo, "");
    apply_and_log(&BLACK_FLAME, repo, "");
    apply_and_log(&DESTINED_DEATH, repo, "");
    apply_and_log(&DESTINED_DEATH_MAX_HP, repo, "");

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
            // Not in the world (e.g. a change made from the title screen's
            // menu): keep the request and apply it once the world is loaded.
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
            apply_and_log(&SCARLET_ROT, repo, " (hotkey pressed)");
            apply_and_log(&POISON, repo, " (hotkey pressed)");
            apply_and_log(&BLACK_FLAME, repo, " (hotkey pressed)");
            apply_and_log(&DESTINED_DEATH, repo, " (hotkey pressed)");
            apply_and_log(&DESTINED_DEATH_MAX_HP, repo, " (hotkey pressed)");
        },
    );

    loop {
        std::thread::sleep(StdDuration::from_secs(60));
    }
}
