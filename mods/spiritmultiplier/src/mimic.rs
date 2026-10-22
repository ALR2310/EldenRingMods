//! `InstantMimic` (default `true`): the Mimic Tear skips the "Blob" stage. Analysis and the er10x origin: `docs/mimic_tear.md`.
//!
//! One param edit, from a snapshot so F5 works both ways: the Blob `BuddyParam` row (NPC in [crate::chain]'s `TRANSIENT_NPCS`)
//! takes the real Mimic's spawn fields from the row with the same
//! `triggerSpEffectId`. The row ID stays: the game tests that ID to decide
//! "this request is a Mimic Tear" and copy the player's build. The Mimic's
//! fade-in (NpcParam slot 24, SpEffect 16316) is left as vanilla.
//!
//! The chain side (one Mimic per cast instead of Blob + Mimic) is in
//! `chain.rs`, gated on [blob_replaced].

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

use eldenring::cs::{BuddyParam, CSTaskGroupIndex, SoloParamRepository};
use eldenring::param::BUDDY_PARAM_ST;
use fromsoftware_shared::FromStatic;

use common::{codepatch, config, logger, memscan};

const TICK_INTERVAL_MS: f64 = 1000.0;

/// Experiment switches (`docs/mimic_tear.md`, tests 2026-10-22). Both on =
/// er10x's result: instant, but the Blob-row Mimic has no HUD HP bar. Both off
/// = the first version: the Mimic rises out of the ground, and the Blob-row
/// copy stands still (no follow flags).
const COPY_FLAGS: bool = true;
const PLAIN_ANIM: bool = true;

static BLOB_REPLACED: AtomicBool = AtomicBool::new(false);

/// 1 while the HUD stub below answers "not a Blob". Read by the stub.
static HUD_FLAG: AtomicU8 = AtomicU8::new(0);

/// `test al,al; jz; ...; call sub_1404B7A10; test al,al; jnz; cmp edi,5` in
/// the buddy HUD's list builder (`sub_140771ED0` on 2.7.2.0, RVA 0x7722BE;
/// unique on 2.6.2 - 2.7.2). For each summoned chr it asks `sub_1404B7A10`
/// "is this the Mimic Blob" (a chr whose group entry has BuddyParam id
/// 20700000) and leaves a Blob out of the HP bars (at most 5 slots). The
/// `call` is at +10.
const HUD_AOB: &str = "84 C0 74 ?? 48 8B D6 49 8B CE E8 ?? ?? ?? ?? 84 C0 75 ?? 83 FF 05";
const HUD_CALL_OFFSET: usize = 10;

/// Destination of that `call` (rcx/rdx pass through untouched): while
/// [HUD_FLAG] is 1 answer `al = 0` ("not a Blob", so the Mimic that now sits
/// on the Blob row gets its HP bar), else jump to the real predicate.
fn build_hud_stub(flag: u64, original: u64) -> Vec<u8> {
    let mut c = Vec::with_capacity(40);
    c.extend_from_slice(&[0x49, 0xBB]); // mov r11, &HUD_FLAG
    c.extend_from_slice(&flag.to_le_bytes());
    c.extend_from_slice(&[0x41, 0x80, 0x3B, 0x00]); // cmp byte [r11], 0
    c.extend_from_slice(&[0x74, 0x03]); // je original
    c.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
    c.push(0xC3); // ret
    c.extend_from_slice(&[0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]); // jmp [rip+0]
    c.extend_from_slice(&original.to_le_bytes());
    c
}

fn install_hud_patch() -> bool {
    let Some(anchor) = memscan::wait_for_pattern_in_module(HUD_AOB, Duration::from_millis(500), Duration::from_secs(60)) else {
        logger::error("InstantMimic: HUD pattern not found, the Blob-row Mimic gets no HP bar");
        return false;
    };
    let site = unsafe { anchor.add(HUD_CALL_OFFSET) };
    // SAFETY: reads the 5 bytes of a call found in the game image.
    let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
        if !unsafe { crate::conflicts::check(site, "Mimic HP bar") } {
            logger::error("InstantMimic: HUD site is not a call");
        }
        return false;
    };
    let stub = build_hud_stub(HUD_FLAG.as_ptr() as u64, original);
    // SAFETY: `site` is the scanned call; the stub stands in for its destination.
    if (unsafe { codepatch::redirect_rel32(site, &stub) }).is_none() {
        logger::error("InstantMimic: HUD redirect failed, the Blob-row Mimic gets no HP bar");
        return false;
    }
    logger::log("InstantMimic: HUD patched");
    true
}

/// True while every Blob row carries its Mimic's fields (chain.rs then
/// counts one Mimic per link).
pub fn blob_replaced() -> bool {
    BLOB_REPLACED.load(Ordering::Relaxed)
}

/// The spawn fields of a `BuddyParam` row, everything but its trigger.
#[derive(Clone, Copy)]
struct Spawn {
    npc: [i32; 4],
    placement: [f32; 3],
    doping: [i32; 11],
    init: i32,
    anim: i32,
    /// appearOnAroundSekihi, disablePCTargetShare, pcFollowType
    flags: [u8; 3],
}

impl Spawn {
    fn read(row: &BUDDY_PARAM_ST) -> Self {
        Self {
            npc: [
                row.npc_param_id(),
                row.npc_think_param_id(),
                row.npc_param_id_ridden(),
                row.npc_think_param_id_ridden(),
            ],
            placement: [row.x_offset(), row.z_offset(), row.y_angle()],
            doping: [
                row.doping_sp_effect_lv0(),
                row.doping_sp_effect_lv1(),
                row.doping_sp_effect_lv2(),
                row.doping_sp_effect_lv3(),
                row.doping_sp_effect_lv4(),
                row.doping_sp_effect_lv5(),
                row.doping_sp_effect_lv6(),
                row.doping_sp_effect_lv7(),
                row.doping_sp_effect_lv8(),
                row.doping_sp_effect_lv9(),
                row.doping_sp_effect_lv10(),
            ],
            init: row.npc_player_init_param_id(),
            anim: row.generate_anim_id(),
            flags: [row.appear_on_around_sekihi(), row.disable_pc_target_share(), row.pc_follow_type()],
        }
    }

    fn write(&self, row: &mut BUDDY_PARAM_ST) {
        row.set_npc_param_id(self.npc[0]);
        row.set_npc_think_param_id(self.npc[1]);
        row.set_npc_param_id_ridden(self.npc[2]);
        row.set_npc_think_param_id_ridden(self.npc[3]);
        row.set_x_offset(self.placement[0]);
        row.set_z_offset(self.placement[1]);
        row.set_y_angle(self.placement[2]);
        row.set_doping_sp_effect_lv0(self.doping[0]);
        row.set_doping_sp_effect_lv1(self.doping[1]);
        row.set_doping_sp_effect_lv2(self.doping[2]);
        row.set_doping_sp_effect_lv3(self.doping[3]);
        row.set_doping_sp_effect_lv4(self.doping[4]);
        row.set_doping_sp_effect_lv5(self.doping[5]);
        row.set_doping_sp_effect_lv6(self.doping[6]);
        row.set_doping_sp_effect_lv7(self.doping[7]);
        row.set_doping_sp_effect_lv8(self.doping[8]);
        row.set_doping_sp_effect_lv9(self.doping[9]);
        row.set_doping_sp_effect_lv10(self.doping[10]);
        row.set_npc_player_init_param_id(self.init);
        row.set_generate_anim_id(self.anim);
        if COPY_FLAGS {
            row.set_appear_on_around_sekihi(self.flags[0]);
            row.set_disable_pc_target_share(self.flags[1]);
            row.set_pc_follow_type(self.flags[2]);
        }
    }
}

struct Plan {
    /// (BuddyParam row index, original fields, fields while `InstantMimic`):
    /// the Blob rows, and the real Mimic rows next to them.
    rows: Vec<(usize, Spawn, Spawn)>,
    blobs: usize,
}

fn capture(repo: &mut SoloParamRepository) -> Option<Plan> {
    if let Err(err) = common::params::check::<BuddyParam>(repo) {
        logger::error(&format!("InstantMimic: {err} - not touching params."));
        return None;
    }
    let Some(ids) = common::params::row_ids::<BuddyParam>(repo) else {
        logger::error("InstantMimic: param row IDs unreadable - not touching params.");
        return None;
    };
    // Cached before any write: after the rewrite the Blob rows no longer
    // show their NPC.
    let transient = crate::chain::transient_buddies();

    // trigger -> (Blob row indexes, first non-Blob row's index)
    let mut groups: HashMap<i32, (Vec<usize>, Option<usize>)> = HashMap::new();
    let mut spawns: Vec<Spawn> = Vec::new();
    common::params::for_each_row_mut::<BuddyParam>(repo, |index, row| {
        spawns.push(Spawn::read(row));
        let entry = groups.entry(row.trigger_sp_effect_id()).or_default();
        if transient.contains(&(ids[index] as i32)) {
            entry.0.push(index);
        } else if entry.1.is_none() {
            entry.1 = Some(index);
        }
    });

    // What er10x does (decompiled, `docs/mimic_tear.md`): the Blob row becomes
    // a copy of the Mimic row, and every spirit's rise-out-of-the-ground spawn
    // animation (the Mimic's 60500) becomes the plain one all ordinary spirits
    // share (1830, found as the most common non-Blob value). The three flag
    // bytes matter too: a Blob row's follow type is what left the copy standing
    // still (test 2026-10-22).
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for (index, spawn) in spawns.iter().enumerate() {
        if !transient.contains(&(ids[index] as i32)) {
            *counts.entry(spawn.anim).or_default() += 1;
        }
    }
    let plain = counts.into_iter().max_by_key(|&(anim, count)| (count, anim)).map(|(anim, _)| anim);

    let mut rows: Vec<(usize, Spawn, Spawn)> = Vec::new();
    let mut blobs = 0;
    for (blob_rows, mimic) in groups.into_values() {
        let Some(mimic_index) = mimic else { continue };
        let with_plain = |spawn: Spawn| Spawn { anim: if PLAIN_ANIM { plain.unwrap_or(spawn.anim) } else { spawn.anim }, ..spawn };
        let mimic = spawns[mimic_index];
        for index in blob_rows {
            rows.push((index, spawns[index], with_plain(mimic)));
            blobs += 1;
        }
        if !rows.iter().any(|&(index, ..)| index == mimic_index) {
            rows.push((mimic_index, mimic, with_plain(mimic)));
        }
    }
    Some(Plan { rows, blobs })
}

pub fn run() {
    install_hud_patch();
    let cs_task = common::task::wait_for_cs_task();

    let mut elapsed_ms: f64 = 0.0;
    // None = not captured yet; Some(None) = capture failed, stay off.
    let mut plan: Option<Option<Plan>> = None;
    let mut applied: Option<bool> = None;

    let _handle = common::task::run_recurring_safe(
        cs_task,
        "InstantMimic",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < TICK_INTERVAL_MS {
                return;
            }
            elapsed_ms = 0.0;

            // Same in-world gate as every other param-touching feature.
            if common::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            let Ok(repo) = (unsafe { SoloParamRepository::instance_mut() }) else {
                return;
            };

            let instant = config::get_bool("InstantMimic", true);
            let Some(plan) = plan.get_or_insert_with(|| capture(repo)) else {
                return;
            };
            if applied == Some(instant) {
                return;
            }

            for &(index, original, mimic) in &plan.rows {
                if let Some(row) = repo.get_row_by_index_mut::<BuddyParam>(index) {
                    if instant { mimic } else { original }.write(row);
                }
            }
            BLOB_REPLACED.store(instant && plan.blobs > 0, Ordering::Relaxed);
            HUD_FLAG.store(blob_replaced() as u8, Ordering::Relaxed);
            logger::log(&format!(
                "InstantMimic={instant}: {} blob row(s), {} row(s) written.",
                plan.blobs,
                plan.rows.len()
            ));
            applied = Some(instant);
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
