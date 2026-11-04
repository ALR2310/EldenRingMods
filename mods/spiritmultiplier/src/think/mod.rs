//! Spirit AI tweaks on the `NpcThinkParam` rows spirits use: the bundles
//! `ImproveSenses`, `ImproveAggression` and `ImproveFollow`. Rows are addressed
//! by index; each row's bytes are saved on the first run so switching a bundle
//! off puts the game's values back. Row selection and the bundles' origin:
//! `docs/think_override.md`.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use eldenring::cs::{BuddyParam, CSTaskGroupIndex, NpcThinkParam, SoloParamRepository};
use eldenring::param::NPC_THINK_PARAM_ST;
use fromsoftware_shared::FromStatic;

use shared::logger;

use crate::config::{self, Spirit};

mod fields;
use fields::{FIELDS, ROW_SIZE};

const TICK_INTERVAL_MS: f64 = 1000.0;

/// Byte offset of `isBuddyAI` in an `NpcThinkParam` row (checked against the
/// field table by a test).
const BUDDY_AI_OFFSET: usize = 0x99;

const _: () = assert!(size_of::<NPC_THINK_PARAM_ST>() == ROW_SIZE);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    U8,
    U16,
    I32,
    F32,
    /// One bit (0-based) of the byte at the offset; value 0 or 1.
    Bit(u8),
}

/// How a value combines with the row's own (original) value: a bundle must not
/// undo what another mod's regulation already did, so it only moves a value in
/// the improving direction.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    /// Always write the value.
    Set,
    /// Write the value if it is larger than the original.
    Max,
    /// Write the value if it is smaller than the original.
    Min,
}

/// One bundle entry resolved to the field's byte offset and type.
struct Override {
    offset: usize,
    kind: Kind,
    value: f64,
    mode: Mode,
}

use Mode::{Max, Min, Set};

/// `ImproveSenses`: spirits notice enemies from farther away and remember
/// them longer - sight, smell and hearing ranges / memory, and the wider
/// vision used while fighting. (Name, value, mode.)
const SENSES: &[(&str, f64, Mode)] = &[
    ("searchEye_dist", 15.0, Max),
    ("eye_dist", 15.0, Max),
    ("searchEye_angX", 180.0, Max),
    ("searchEye_angY", 180.0, Max),
    ("SightTargetForgetTime", 3.0, Max),
    ("nose_dist", 5.0, Max),
    ("searchTargetLv1ForgetTime", 5.0, Max),
    ("searchTargetLv2ForgetTime", 5.0, Max),
    ("MemoryTargetForgetTime", 5.0, Max),
    ("ear_dist", 5.0, Max),
    ("SoundTargetForgetTime", 5.0, Max),
    ("isUpdateBattleSight", 1.0, Max),
    ("battleEye_updateDist", 60.0, Max),
    ("battleEye_updateAngX", 180.0, Max),
    ("battleEye_updateAngY", 180.0, Max),
];

/// `ImproveAggression`: spirits start fighting from farther away, attack
/// together instead of circling, guard, and call for help.
const AGGRESSION: &[(&str, f64, Mode)] = &[
    ("TeamAttackEffectivity", 100.0, Max),
    ("BattleStartDist", 15.0, Max),
    ("callHelp_CallValidMinDistTarget", 5.0, Set),
    ("callHelp_CallValidRange", 15.0, Max),
    ("platoonReplyAddRandomTime", 2.0, Set),
    ("isGuard_Act", 1.0, Max),
    ("thinkAttr_doAdmirer", 1.0, Max),
    ("enableJumpMove", 2.0, Max),
    ("enableJumpMove_onBattle", 2.0, Max),
];

/// `ImproveFollow`: spirits return to the player beyond ~30 m instead of
/// chasing anywhere (vanilla 9999 m) and may take ladders, holes, navmesh
/// walls and ledges to keep up (lava is left out on purpose).
const FOLLOW: &[(&str, f64, Mode)] = &[
    ("maxBackhomeDist", 30.0, Min),
    ("backhomeDist", 30.0, Min),
    ("backhomeBattleDist", 60.0, Max),
    ("BackHome_LookTargetTime", 15.0, Max),
    ("BackHome_LookTargetDist", 15.0, Max),
    ("BackHomeLifeOnHitEneWal", 0.1, Min),
    ("backToHomeStuckAct", 1.0, Set),
    ("enableNaviFlg_Ladder", 1.0, Max),
    ("enableNaviFlg_Hole", 1.0, Max),
    ("enableNaviFlg_InSideWall", 1.0, Max),
    ("enableNaviFlg_Edge_Ordinary", 1.0, Max),
];

/// One bundle entry: field name, value, how it combines with the game's value.
type Entry = (&'static str, f64, Mode);

/// The bundles the config switches on, as `(name, switch, entries)`.
type Switch = fn(&Spirit) -> bool;
const BUNDLES: [(&str, Switch, &[Entry]); 3] = [
    ("ImproveSenses", |s| s.improve_senses, SENSES),
    ("ImproveAggression", |s| s.improve_aggression, AGGRESSION),
    ("ImproveFollow", |s| s.improve_follow, FOLLOW),
];

/// `NpcThinkParam` field table by normalized name.
static THINK_INDEX: LazyLock<HashMap<String, &'static (&'static str, usize, Kind)>> =
    LazyLock::new(|| FIELDS.iter().map(|f| (normalize(f.0), f)).collect());

/// The entries of the bundles that are on, as a bit set (for change
/// detection) and the list, in bundle order.
fn bundle_overrides() -> (u8, Vec<Override>) {
    let mut bits = 0u8;
    let mut out = Vec::new();
    let config = config::get();
    for (i, (_, switch, entries)) in BUNDLES.iter().enumerate() {
        if !switch(&config.spirit) {
            continue;
        }
        bits |= 1 << i;
        for &(name, value, mode) in entries.iter() {
            if let Some(&&(_, offset, kind)) = THINK_INDEX.get(&normalize(name)) {
                out.push(Override { offset, kind, value, mode });
            }
        }
    }
    (bits, out)
}

/// Lowercase without `_`, so `isGuard_Act` and `is_guard_act` match.
fn normalize(name: &str) -> String {
    name.chars().filter(|c| *c != '_').map(|c| c.to_ascii_lowercase()).collect()
}

/// Writes `o` into the row at `row`.
///
/// # Safety
/// `row` must point to a whole `NpcThinkParam` row.
unsafe fn write(row: *mut u8, o: &Override) {
    // SAFETY: `ptr` points at a whole row of the live param table.
    unsafe {
        let at = row.add(o.offset);
        if o.mode != Set {
            let current = read(at, o.kind);
            if (o.mode == Max && current >= o.value) || (o.mode == Min && current <= o.value) {
                return;
            }
        }
        match o.kind {
            Kind::U8 => at.write(o.value.round() as u8),
            Kind::U16 => (at as *mut u16).write_unaligned(o.value.round() as u16),
            Kind::I32 => (at as *mut i32).write_unaligned(o.value.round() as i32),
            Kind::F32 => (at as *mut f32).write_unaligned(o.value as f32),
            Kind::Bit(bit) => {
                let byte = at.read() & !(1 << bit);
                at.write(byte | ((o.value.round() as u8) << bit));
            }
        }
    }
}

/// The value of the field of type `kind` at `at`.
///
/// # Safety
/// `at` must point to a field of that type inside a row.
unsafe fn read(at: *const u8, kind: Kind) -> f64 {
    // SAFETY: `ptr` points at a whole row of the live param table.
    unsafe {
        match kind {
            Kind::U8 => at.read() as f64,
            Kind::U16 => (at as *const u16).read_unaligned() as f64,
            Kind::I32 => (at as *const i32).read_unaligned() as f64,
            Kind::F32 => (at as *const f32).read_unaligned() as f64,
            Kind::Bit(bit) => ((at.read() >> bit) & 1) as f64,
        }
    }
}

/// One spirit row: `(row index, original bytes)`.
type Snapshot = (usize, Vec<u8>);

/// Snapshots every `NpcThinkParam` row a `BuddyParam` row spawns a spirit
/// with, or `None` if the params couldn't be read safely. Second value: how
/// many IDs had no row.
fn capture(repo: &mut SoloParamRepository) -> Option<(Vec<Snapshot>, usize)> {
    for check in
        [shared::params::check::<BuddyParam>(repo), shared::params::check::<NpcThinkParam>(repo)]
    {
        if let Err(err) = check {
            logger::error(&format!("ThinkOverride: {err} - not touching params."));
            return None;
        }
    }
    let Some(ids) = shared::params::row_ids::<NpcThinkParam>(repo) else {
        logger::error("ThinkOverride: NpcThinkParam row IDs could not be read safely.");
        return None;
    };
    let index: HashMap<u32, usize> =
        ids.iter().copied().enumerate().map(|(i, id)| (id, i)).collect();

    let mut wanted: Vec<usize> = Vec::new();
    let mut missing = 0;
    shared::params::for_each_row_mut::<BuddyParam>(repo, |_, row| {
        for id in [row.npc_think_param_id(), row.npc_think_param_id_ridden()] {
            if id <= 0 {
                continue;
            }
            match index.get(&(id as u32)) {
                Some(&i) => wanted.push(i),
                None => missing += 1,
            }
        }
    });
    wanted.sort_unstable();
    wanted.dedup();

    let mut rows = Vec::with_capacity(wanted.len());
    for i in wanted {
        let row = repo.get_row_by_index::<NpcThinkParam>(i)?;
        // SAFETY: the pointer comes from the game's own live structure for this frame.
        let bytes = unsafe { std::slice::from_raw_parts(row as *const _ as *const u8, ROW_SIZE) };
        rows.push((i, bytes.to_vec()));
    }
    Some((rows, missing))
}

/// Whether the (original) row is dedicated to buddy AI. Spirits also borrow
/// enemies' rows (`isBuddyAI` 0: vanilla Demi-Humans, many rows in some
/// overhauls), and a bundle written there changes those enemies too.
///
/// # Safety
/// `row` must point to a whole `NpcThinkParam` row.
unsafe fn is_buddy_row(row: *const u8) -> bool {
    // SAFETY: the offset stays inside the structure it belongs to.
    unsafe { row.add(BUDDY_AI_OFFSET).read() == 1 }
}

/// Puts every row back from its snapshot, then writes the bundles onto the
/// buddy-AI rows. Returns how many rows were skipped as shared with enemies.
fn apply(repo: &mut SoloParamRepository, rows: &[Snapshot], bundles: &[Override]) -> usize {
    let mut skipped = 0;
    for (i, original) in rows {
        let Some(row) = repo.get_row_by_index_mut::<NpcThinkParam>(*i) else {
            continue;
        };
        let ptr = row as *mut _ as *mut u8;
        // SAFETY: `ptr` points at a whole row of the live param table.
        unsafe {
            std::ptr::copy_nonoverlapping(original.as_ptr(), ptr, ROW_SIZE);
            if is_buddy_row(ptr) {
                for o in bundles {
                    write(ptr, o);
                }
            } else {
                skipped += 1;
            }
        }
    }
    skipped
}

pub fn run() {
    let cs_task = shared::task::wait_for_cs_task();

    let mut elapsed_ms: f64 = 0.0;
    // None = not captured yet; Some(None) = capture failed, stay off.
    let mut rows: Option<Option<Vec<Snapshot>>> = None;
    // The set of bundles last applied.
    let mut applied: Option<u8> = None;

    let _handle = shared::task::run_recurring_safe(
        cs_task,
        "ThinkOverride",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < TICK_INTERVAL_MS {
                return;
            }
            elapsed_ms = 0.0;

            let (bits, bundles) = bundle_overrides();
            if applied == Some(bits) {
                return;
            }
            // Nothing wanted and nothing applied before: leave the params alone.
            if applied.is_none() && bits == 0 {
                applied = Some(bits);
                return;
            }
            if shared::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            // SAFETY: called from a game task; the accessor returns `Err` while the singleton does not exist.
            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                return;
            };
            let Some(rows) = rows.get_or_insert_with(|| {
                let (rows, missing) = capture(repo)?;
                logger::log(&format!(
                    "ThinkOverride: {} NpcThinkParam row(s) used by spirits ({missing} id(s) without a row).",
                    rows.len()
                ));
                Some(rows)
            }) else {
                return;
            };

            let skipped = apply(repo, rows, &bundles);
            let names: Vec<&str> = BUNDLES
                .iter()
                .enumerate()
                .filter(|(i, _)| bits & (1 << i) != 0)
                .map(|(_, b)| b.0)
                .collect();
            logger::log(&format!(
                "ThinkOverride: {names:?} written to {} row(s), skipped {skipped} shared with enemies",
                rows.len()
            ));
            applied = Some(bits);
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str) -> (usize, Kind) {
        let f = FIELDS.iter().find(|f| f.0 == name).unwrap();
        (f.1, f.2)
    }

    /// The generated offsets must match the real struct: set a field through
    /// the crate's own setter and read it back at the table's offset.
    #[test]
    fn offsets_match_struct() {
        // SAFETY: the row type is plain integers, so all zeros is a valid value.
        let mut row: NPC_THINK_PARAM_ST = unsafe { std::mem::zeroed() };
        row.set_team_attack_effectivity(77);
        row.set_battle_start_dist(1234);
        row.set_is_guard_act(9);
        row.set_memory_target_forget_time(2.5);
        row.set_logic_id(-5);
        row.set_surprise_anim_id(31337);
        let base = &row as *const _ as *const u8;
        // SAFETY: the offset stays inside the row.
        let at = |name: &str| unsafe { base.add(field(name).0) };
        // SAFETY: each offset is a field of `row`.
        unsafe {
            assert_eq!(at("team_attack_effectivity").read(), 77);
            assert_eq!((at("battle_start_dist") as *const u16).read_unaligned(), 1234);
            assert_eq!(at("is_guard_act").read(), 9);
            assert_eq!((at("memory_target_forget_time") as *const f32).read_unaligned(), 2.5);
            assert_eq!((at("logic_id") as *const i32).read_unaligned(), -5);
            assert_eq!((at("surprise_anim_id") as *const i32).read_unaligned(), 31337);
        }
    }

    /// Every bundle entry names a real field, and its value fits the type.
    #[test]
    fn bundles_resolve() {
        for (key, _, entries) in BUNDLES {
            for &(name, value, _) in entries {
                let Some(&&(_, _, kind)) = THINK_INDEX.get(&normalize(name)) else {
                    panic!("{key}: unknown field {name}");
                };
                let max = match kind {
                    Kind::U8 => 255.0,
                    Kind::U16 => 65535.0,
                    Kind::Bit(_) => 1.0,
                    _ => f64::MAX,
                };
                assert!(value >= 0.0 && value <= max, "{key}: {name}={value} out of range");
            }
        }
    }

    /// `Max` / `Min` only move a value in their direction; `Set` always writes.
    #[test]
    fn modes() {
        // SAFETY: the row type is plain integers, so all zeros is a valid value.
        let mut row: NPC_THINK_PARAM_ST = unsafe { std::mem::zeroed() };
        let ptr = &mut row as *mut _ as *mut u8;
        let o = |name: &'static str, value: f64, mode: Mode| {
            let &&(_, offset, kind) = THINK_INDEX.get(&normalize(name)).unwrap();
            Override { offset, kind, value, mode }
        };
        row.set_battle_start_dist(40);
        row.set_max_backhome_dist(9999);
        row.set_team_attack_effectivity(30);
        row.set_enable_navi_flg_ladder(true);
        // SAFETY: `ptr` points at a whole row.
        unsafe {
            write(ptr, &o("BattleStartDist", 15.0, Max)); // 40 stays
            write(ptr, &o("maxBackhomeDist", 30.0, Min)); // 9999 -> 30
            write(ptr, &o("TeamAttackEffectivity", 100.0, Max)); // 30 -> 100
            write(ptr, &o("enableNaviFlg_Ladder", 1.0, Max)); // stays 1
            write(ptr, &o("enableNaviFlg_Hole", 1.0, Max)); // 0 -> 1
            write(ptr, &o("nose_dist", 7.0, Set));
        }
        assert_eq!(row.battle_start_dist(), 40);
        assert_eq!(row.max_backhome_dist(), 30);
        assert_eq!(row.team_attack_effectivity(), 100);
        assert!(row.enable_navi_flg_ladder() && row.enable_navi_flg_hole());
        assert_eq!(row.nose_dist(), 7);
    }

    /// Bundles only touch rows with `isBuddyAI` set; rows shared with enemies are left alone.
    #[test]
    fn buddy_row_filter() {
        assert_eq!(
            FIELDS.iter().find(|f| f.0 == "is_buddy_ai").map(|f| f.1),
            Some(BUDDY_AI_OFFSET)
        );
        // SAFETY: the row type is plain integers, so all zeros is a valid value.
        let mut row: NPC_THINK_PARAM_ST = unsafe { std::mem::zeroed() };
        let ptr = &row as *const _ as *const u8;
        // SAFETY: `ptr` points at a whole row.
        assert!(!unsafe { is_buddy_row(ptr) });
        row.set_is_buddy_ai(1);
        // SAFETY: `ptr` points at a whole row.
        assert!(unsafe { is_buddy_row(ptr) });
    }

    /// Bit flags: only the named bit changes, neighbours are kept.
    #[test]
    fn bit_flags() {
        // SAFETY: the row type is plain integers, so all zeros is a valid value.
        let mut row: NPC_THINK_PARAM_ST = unsafe { std::mem::zeroed() };
        row.set_enable_navi_flg_edge(true);
        row.set_enable_navi_flg_door(true);
        let ptr = &mut row as *mut _ as *mut u8;
        for (name, value) in [("enableNaviFlg_Edge_Ordinary", 1.0), ("enableNaviFlg_Door", 0.0)] {
            let &&(_, offset, kind) = THINK_INDEX.get(&normalize(name)).unwrap();
            // SAFETY: `ptr` points at a whole row.
            unsafe { write(ptr, &Override { offset, kind, value, mode: Set }) };
        }
        assert!(row.enable_navi_flg_edge() && row.enable_navi_flg_edge_ordinary());
        assert!(!row.enable_navi_flg_door());
    }
}
