//! Research probes, off by default (`[Logging]` `CancelProbe` / `FlagProbe`,
//! read every frame so a hot reload turns them on or off). They only write
//! to the log file. Used to work out how the game decides a cancel
//! (docs/animation_cancel.md) and to see why an animation doesn't cancel.
//!
//! - `CancelProbe`: a `WINDOW` line every time the animation, the cancel
//!   windows or the pressed actions change: the animation and how far into
//!   it, its group (`grp=`) and the group of the queue's id (`qgrp=`, the
//!   one the mod goes by: the two differ while a spell shows the legs'
//!   animation), what the mod did this frame (`mod=`: `-` nothing to do,
//!   `forced`), the masks and the queue entries for the queue's id
//!   (`q=<id>`, `qin` = the requests the HKS reads, `qcan` = the permissions
//!   it checks).
//! - `CancelProbe` also writes a `STYLE` line when how the weapons are held or
//!   which weapons are equipped changes (`arm=` the arm style, `slot=` the
//!   selected left/right weapon slots, `weapon=` the param IDs held), to see
//!   what holding two weapons looks like (guard is an attack there).
//! - `FlagProbe`: a `FLAG` line for every word of `CSChrActionFlagModule` that
//!   changes (the flags animation events set).
//!
//! Runs at `ChrIns_PostPhysics`, after this frame's windows are final.

use eldenring::cs::{ActionQueueEntry, CSTaskGroupIndex, ChrAsmSlot, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::anim::{current_anim_id, group_of};
use common::logger;

use crate::cancel::{self, queue_id, read};
use crate::config;

/// `TaeCancelFlags` bits that last across frames (the rest flicker per frame).
const TAE_FALLING_JUMP_FRAMES: u32 = 1 << 10;
const TAE_CANCEL_DISABLE: u32 = 1 << 11;

/// Size of `CSChrActionFlagModule` (fromsoftware-rs test_size_of).
const ACTION_FLAG_SIZE: usize = 0x258;

fn fmt_anim(id: i32) -> String {
    if id < 0 {
        return format!("{id}");
    }
    format!("a{:03}_{:06}", id / 1_000_000, id % 1_000_000)
}

fn mod_state(state: u8) -> &'static str {
    match state {
        cancel::FORCED => "forced",
        _ => "-",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Windows {
    anim: i32,
    state: u8,
    inputs: u64,
    cancels: u64,
    ready: u64,
    /// Only the bits that persist across frames.
    tae: u32,
    /// Actions whose presses the game drops before queuing them.
    disabled: u64,
    queue_id: i32,
    queue_inputs: u64,
    queue_cancels: u64,
    /// Keys held (`action_requests`) and whether guard was held as the animation began.
    held: u64,
    guard_blocked: bool,
}

/// Logs the windows when they (or the animation) change, and every new press
/// (a press is what the windows are for).
fn log_windows(chr: &ChrIns, last: &mut Windows, last_press: &mut u64) {
    let request = &chr.modules.action_request;
    let time_act = &chr.modules.time_act;
    let anim = &time_act.anim_queue[(time_act.read_idx as usize) % time_act.anim_queue.len()];
    let tae: u32 = read(&request.tae_cancels);
    let id = queue_id(request);
    let queue = &request.action_request_queue;
    let entry_actions = |entries: &[ActionQueueEntry]| -> u64 {
        entries.iter().find(|e| e.tae_id == id).map_or(0, |e| read(&e.actions))
    };
    let now = Windows {
        anim: anim.anim_id,
        state: cancel::last_state(),
        inputs: read(&request.possible_action_inputs),
        cancels: read(&request.possible_action_cancels),
        ready: read(&request.cancel_ready_actions),
        tae: tae & (TAE_FALLING_JUMP_FRAMES | TAE_CANCEL_DISABLE),
        disabled: read(&request.disabled_action_inputs),
        queue_id: id,
        queue_inputs: entry_actions(&queue.input_entries),
        queue_cancels: entry_actions(&queue.cancel_entries),
        held: read(&request.action_requests),
        guard_blocked: cancel::guard_held_since_start(),
    };
    let press: u64 = read(&request.new_action_presses);
    if now == *last && press == *last_press {
        return;
    }
    *last = now;
    *last_press = press;
    logger::log(&format!(
        "WINDOW {} t={:.2}/{:.2} grp={:?} qgrp={:?} mod={} inputs={:#x} cancels={:#x} queued={:#x} ready={:#x} press={:#x} tae={:#x} disabled={:#x} q={} qin={:#x} qcan={:#x} held={:#x} gblock={}",
        fmt_anim(now.anim),
        anim.play_time,
        anim.anim_length,
        group_of(now.anim),
        cancel::group_for_cancel(now.queue_id),
        mod_state(now.state),
        now.inputs,
        now.cancels,
        read::<_, u64>(&request.queued_action_inputs),
        now.ready,
        press,
        tae,
        now.disabled,
        now.queue_id,
        now.queue_inputs,
        now.queue_cancels,
        now.held,
        now.guard_blocked as u8,
    ));
}

/// The weapon's class and the flags that decide what L1 does with it.
fn weapon_info(id: i32) -> String {
    match crate::weapon::weapon(id) {
        Some(w) => format!(
            "type={} guard={} dual={} motion={}/{} shield={}",
            w.wep_type,
            w.enable_guard as u8,
            w.dual_blade as u8,
            w.motion,
            w.guard_motion,
            crate::weapon::is_shield(&w) as u8
        ),
        None => "no row".to_string(),
    }
}

/// What the player holds: arm style, selected slots and the weapon param IDs.
#[derive(Clone, PartialEq, Eq, Default)]
struct Style(String);

fn read_style() -> Option<Style> {
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let player = world_chr_man.main_player.as_mut()?;
    let data = unsafe { player.player_game_data.as_ref() };
    let asm = &data.equipment.chr_asm;
    let equipment = &asm.equipment;
    let id = |slot: ChrAsmSlot| asm.equipment_param_ids[slot as usize];
    let left = equipment.active_left_weapon_slot();
    let right = equipment.active_right_weapon_slot();
    Some(Style(format!(
        "arm={:?} slot=L{}/R{} weapon=L{}/R{} left[{}] right[{}]",
        equipment.arm_style,
        equipment.selected_slots.left_weapon_slot,
        equipment.selected_slots.right_weapon_slot,
        id(left),
        id(right),
        weapon_info(id(left)),
        weapon_info(id(right))
    )))
}

fn log_style(last: &mut Style) {
    let Some(now) = read_style() else {
        return;
    };
    if now != *last {
        logger::log(&format!("STYLE {}", now.0));
        *last = now;
    }
}

/// Logs every 4-byte word of the player's `CSChrActionFlagModule` that
/// changed since the last frame (skipping the vftable and owner pointer).
fn log_flag_changes(chr: &ChrIns, prev: &mut Vec<u8>) {
    let module = &*chr.modules.action_flag;
    let now = unsafe { std::slice::from_raw_parts(module as *const _ as *const u8, ACTION_FLAG_SIZE) };
    if prev.len() == ACTION_FLAG_SIZE {
        let anim = current_anim_id(chr);
        let time_act = &chr.modules.time_act;
        let play_time = time_act.anim_queue[(time_act.read_idx as usize) % time_act.anim_queue.len()].play_time;
        for off in (0x10..ACTION_FLAG_SIZE).step_by(4) {
            let old = u32::from_le_bytes(prev[off..off + 4].try_into().unwrap());
            let new = u32::from_le_bytes(now[off..off + 4].try_into().unwrap());
            if old != new {
                logger::log(&format!("FLAG {} t={play_time:.2} +{off:#x} {old:#x}->{new:#x}", fmt_anim(anim)));
            }
        }
    }
    prev.clear();
    prev.extend_from_slice(now);
}

/// Registers the probe task (returns at once).
pub fn start() {
    let cs_task = common::task::wait_for_cs_task();
    let mut last = Windows::default();
    let mut last_press = 0u64;
    let mut flag_prev: Vec<u8> = Vec::new();
    let mut last_style = Style::default();
    common::task::run_recurring_safe(
        cs_task,
        "Probe",
        CSTaskGroupIndex::ChrIns_PostPhysics,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let logging = &config::get().logging;
            if !logging.cancel_probe && !logging.flag_probe {
                flag_prev.clear();
                return;
            }
            let Some(chr) = cancel::player() else {
                return;
            };
            if logging.cancel_probe {
                log_windows(chr, &mut last, &mut last_press);
                log_style(&mut last_style);
            }
            if logging.flag_probe {
                log_flag_changes(chr, &mut flag_prev);
            } else {
                flag_prev.clear();
            }
        },
    );
}
