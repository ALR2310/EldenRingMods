//! Research probe for animation cancels (docs/animation_cancel.md).
//!
//! Two independent pieces, both on the local player only:
//! - **Log** (`LogCancelWindows`, at `ChrIns_PostPhysics`): every time the
//!   animation or the cancel windows of `CSChrActionRequestModule` change,
//!   one line with the masks. Shows which actions each vanilla animation
//!   lets through and when.
//! - **Force** (`[Force]`, toggled with a key): ORs the chosen actions into
//!   `possible_action_inputs` / `possible_action_cancels` every frame, to
//!   test whether the game's HKS then switches state (guard, roll, weapon
//!   swap) in the middle of an animation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::{config, input, logger};

static FORCE: AtomicBool = AtomicBool::new(false);

/// `ChrActions` bit numbers (fromsoftware-rs `action_request.rs`).
const R1: u32 = 0;
const R2: u32 = 1;
const L1: u32 = 2;
const L2: u32 = 3;
const ACTION: u32 = 4;
const SP_MOVE: u32 = 5;
const JUMP: u32 = 6;
const USE_ITEM: u32 = 7;
const CHANGE_WEAPON_R: u32 = 9;
const CHANGE_WEAPON_L: u32 = 10;
/// Queue-only bit (not a real action): the HKS weapon swap also asks for
/// `cancel_entries[id]` bit 37 (`sub_140407CC0`). 2026-10-12 log: the swap press
/// was `ready` from t=0.28 of an attack but only ran at 0.83, when the game
/// opened bit 37.
const SLOT_SWITCH: u32 = 37;
const BACKSTEP: u32 = 16;
const ROLLING: u32 = 17;
/// Not named in fromsoftware-rs. Vanilla opens it together with the roll cancel
/// (2026-10-12 log: 0x40000 appears at t=0.55 of an attack, the roll fires
/// right after), so it goes with Dodge.
const DODGE_18: u32 = 18;
const GUARD: u32 = 24;
const EMERGENCY_STEP: u32 = 25;
const LIGHT_KICK: u32 = 26;
const HEAVY_KICK: u32 = 27;

/// `TaeCancelFlags` bits (fromsoftware-rs `TaeCancelFlags`). The ones below
/// `TAE_FALLING_JUMP_FRAMES` are transient: the game clears them at the start
/// of every frame, so they only count if written before `HavokBehavior`.
const TAE_MOVEMENT_CANCEL: u32 = 1 << 1;
const TAE_MOVEMENT_CANCEL_PREV: u32 = 1 << 2;
const TAE_SLOT_SWITCH: u32 = 1 << 3;
const TAE_RH_ATTACK: u32 = 1 << 4;
const TAE_ACTION_GENERAL: u32 = 1 << 9;

/// `TaeCancelFlags` bits that last across frames (the rest flicker per frame).
const TAE_FALLING_JUMP_FRAMES: u32 = 1 << 10;
const TAE_CANCEL_DISABLE: u32 = 1 << 11;

/// The `ChrActions` / `TaeCancelFlags` bitfield structs keep their value in a
/// private field, so read and write it through a pointer (they are plain
/// `u64` / `u32` newtypes).
fn read<T, U: Copy>(value: &T) -> U {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<U>());
    unsafe { (value as *const T as *const U).read() }
}

fn write<T, U>(value: &mut T, new: U) {
    debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<U>());
    unsafe { (value as *mut T as *mut U).write(new) }
}

/// Mask of the actions named in `Actions` (unknown names are logged and skipped).
fn parse_actions(list: &str) -> u64 {
    let mut mask = 0u64;
    for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let bits: &[u32] = match name.to_ascii_lowercase().as_str() {
            // Guard is L1 in game: ACTION_ARM_GUARD (24) alone left the 2026-10-12 guard
            // test without a queued press (the L1 press is bit 2).
            "guard" => &[GUARD, L1],
            "dodge" => &[SP_MOVE, ROLLING, BACKSTEP, DODGE_18, EMERGENCY_STEP],
            // ACTION (4) too: on keyboard the swap key is the pouch/swap button.
            "weapon" => &[ACTION, CHANGE_WEAPON_R, CHANGE_WEAPON_L, SLOT_SWITCH],
            "item" => &[USE_ITEM],
            "attack" => &[R1, R2, L1, L2, LIGHT_KICK, HEAVY_KICK],
            "jump" => &[JUMP],
            _ => {
                logger::warn(&format!("AnimCancel.ini: unknown action '{name}' in Actions, skipped."));
                continue;
            }
        };
        for bit in bits {
            mask |= 1 << bit;
        }
    }
    mask
}

/// Mask of the `TaeCancelFlags` named in `TaeFlags`.
fn parse_tae_flags(list: &str) -> u32 {
    let mut mask = 0u32;
    for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        mask |= match name.to_ascii_lowercase().as_str() {
            "general" => TAE_ACTION_GENERAL,
            "slotswitch" => TAE_SLOT_SWITCH,
            "attack" => TAE_RH_ATTACK,
            "movement" => TAE_MOVEMENT_CANCEL | TAE_MOVEMENT_CANCEL_PREV,
            _ => {
                logger::warn(&format!("AnimCancel.ini: unknown flag '{name}' in TaeFlags, skipped."));
                continue;
            }
        };
    }
    mask
}

fn parse_stage(name: &str) -> CSTaskGroupIndex {
    match name.trim().to_ascii_lowercase().as_str() {
        "prebehavior" => CSTaskGroupIndex::ChrIns_PreBehavior,
        "prebehaviorsafe" => CSTaskGroupIndex::ChrIns_PreBehaviorSafe,
        "prephysics" => CSTaskGroupIndex::ChrIns_PrePhysics,
        "postphysics" => CSTaskGroupIndex::ChrIns_PostPhysics,
        "behaviorsafe" => CSTaskGroupIndex::ChrIns_BehaviorSafe,
        other => {
            logger::warn(&format!(
                "AnimCancel.ini: unknown Stage '{other}', using BehaviorSafe."
            ));
            CSTaskGroupIndex::ChrIns_BehaviorSafe
        }
    }
}

fn stage_name(stage: CSTaskGroupIndex) -> String {
    format!("{stage:?}")
}

/// The local player's `ChrIns`, only once actually in game (see
/// `common::player::main_player_chr_ins_ptr`).
fn player<'a>() -> Option<&'a mut ChrIns> {
    common::player::main_player_chr_ins_ptr()?;
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    world_chr_man.main_player.as_mut().map(|p| &mut p.chr_ins)
}

fn fmt_anim(id: i32) -> String {
    if id < 0 {
        return format!("{id}");
    }
    format!("a{:03}_{:06}", id / 1_000_000, id % 1_000_000)
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Windows {
    anim: i32,
    inputs: u64,
    cancels: u64,
    ready: u64,
    /// Only the bits that persist across frames.
    tae: u32,
    /// Queue mode: the game keeps per-state input/cancel entries too, and the
    /// HKS getters read those instead of the masks when it is on.
    queue_mode: bool,
    /// Actions whose presses the game drops before queuing them.
    disabled: u64,
    queue_id: i32,
    queue_inputs: u64,
    queue_cancels: u64,
}

/// TAE id the queue entries are looked up by (`queue_tae_id_override` wins).
fn queue_id(request: &eldenring::cs::CSChrActionRequestModule) -> i32 {
    if request.queue_tae_id_override >= 0 {
        request.queue_tae_id_override
    } else {
        request.action_request_queue.current_tae_id
    }
}

/// One frame's force: widens the windows of the current animation.
fn force(chr: &mut ChrIns, mask: u64, tae_mask: u32, clear_disable: bool) {
    let request = &mut chr.modules.action_request;
    let inputs: u64 = read(&request.possible_action_inputs);
    write(&mut request.possible_action_inputs, inputs | mask);
    let cancels: u64 = read(&request.possible_action_cancels);
    write(&mut request.possible_action_cancels, cancels | mask);
    // `UpdateFromManipulator` drops `action_requests AND disabled_action_inputs`
    // before queuing: let the forced actions through.
    let disabled: u64 = read(&request.disabled_action_inputs);
    write(&mut request.disabled_action_inputs, disabled & !mask);
    // Queue mode (always on for the player, 2026-10-12 log): the HKS getters
    // read `input_entries[id]`, which the game fills each frame with
    // `queued AND cancel_entries[id]` (`sub_140408420`). So the permission to
    // widen is `cancel_entries[id]` - an entry that only exists once an animation
    // event wrote it, hence created here when missing. `input_entries` is the
    // *requests* list: writing the mask there made every action fire
    // (2026-10-12 test: attack -> weapon swap -> weapon swap -> roll).
    let id = queue_id(request);
    if id >= 0 {
        let cancel_entries = &mut request.action_request_queue.cancel_entries;
        if let Some(entry) = cancel_entries.iter_mut().find(|e| e.tae_id == id) {
            let actions: u64 = read(&entry.actions);
            write(&mut entry.actions, actions | mask);
        } else {
            // SAFETY: plain integers; an all-zero entry is valid.
            let mut entry: eldenring::cs::ActionQueueEntry = unsafe { std::mem::zeroed() };
            entry.tae_id = id;
            write(&mut entry.actions, mask);
            cancel_entries.push_back(entry);
        }
    }
    let mut tae: u32 = read(&request.tae_cancels);
    tae |= tae_mask;
    if clear_disable {
        tae &= !TAE_CANCEL_DISABLE;
    }
    write(&mut request.tae_cancels, tae);
}

/// Logs the windows when they (or the animation) change, and every new press
/// that matters (a press is what the windows are for).
fn log_windows(chr: &ChrIns, last: &mut Windows, last_press: &mut u64) {
    let request = &chr.modules.action_request;
    let time_act = &chr.modules.time_act;
    let idx = (time_act.read_idx as usize) % time_act.anim_queue.len();
    let anim = &time_act.anim_queue[idx];
    let tae: u32 = read(&request.tae_cancels);
    let id = queue_id(request);
    let queue = &request.action_request_queue;
    let entry_actions = |entries: &[eldenring::cs::ActionQueueEntry]| -> u64 {
        entries.iter().find(|e| e.tae_id == id).map_or(0, |e| read(&e.actions))
    };
    let now = Windows {
        queue_mode: request.queue_mode_enabled,
        disabled: read(&request.disabled_action_inputs),
        queue_id: id,
        queue_inputs: entry_actions(&queue.input_entries),
        queue_cancels: entry_actions(&queue.cancel_entries),
        anim: anim.anim_id,
        inputs: read(&request.possible_action_inputs),
        cancels: read(&request.possible_action_cancels),
        ready: read(&request.cancel_ready_actions),
        tae: tae & (TAE_FALLING_JUMP_FRAMES | TAE_CANCEL_DISABLE),
    };
    let press: u64 = read(&request.new_action_presses);
    if now == *last && press == *last_press {
        return;
    }
    *last = now;
    *last_press = press;
    logger::log(&format!(
        "WINDOW {} t={:.2}/{:.2} inputs={:#x} cancels={:#x} queued={:#x} ready={:#x} press={:#x} tae={:#x} disabled={:#x} q={}/{} qin={:#x} qcan={:#x} force={}",
        fmt_anim(now.anim),
        anim.play_time,
        anim.anim_length,
        now.inputs,
        now.cancels,
        read::<_, u64>(&request.queued_action_inputs),
        now.ready,
        press,
        tae,
        now.disabled,
        now.queue_mode,
        now.queue_id,
        now.queue_inputs,
        now.queue_cancels,
        FORCE.load(Ordering::Relaxed),
    ));
}

/// Size of `CSChrActionFlagModule` (fromsoftware-rs test_size_of).
const ACTION_FLAG_SIZE: usize = 0x258;

/// Logs every 4-byte word of the player's `CSChrActionFlagModule` that
/// changed since the last frame (skipping the vftable and owner pointer).
/// Those flags are what animation events set; the point is to see which one
/// flips when the game finally lets a cancel through.
fn log_flag_changes(chr: &ChrIns, prev: &mut Vec<u8>) {
    let module = &*chr.modules.action_flag;
    let now = unsafe { std::slice::from_raw_parts(module as *const _ as *const u8, ACTION_FLAG_SIZE) };
    if prev.len() == ACTION_FLAG_SIZE {
        let time_act = &chr.modules.time_act;
        let anim = &time_act.anim_queue[(time_act.read_idx as usize) % time_act.anim_queue.len()];
        for off in (0x10..ACTION_FLAG_SIZE).step_by(4) {
            let old = u32::from_le_bytes(prev[off..off + 4].try_into().unwrap());
            let new = u32::from_le_bytes(now[off..off + 4].try_into().unwrap());
            if old != new {
                logger::log(&format!(
                    "FLAG {} t={:.2} +{off:#x} {old:#x}->{new:#x}",
                    fmt_anim(anim.anim_id),
                    anim.play_time,
                ));
            }
        }
    }
    prev.clear();
    prev.extend_from_slice(now);
}

pub fn run() {
    let cs_task = common::task::wait_for_cs_task();

    if config::get_bool("LogCancelWindows", true) {
        let mut last = Windows::default();
        let mut last_press = 0u64;
        let log_flags = config::get_bool("LogFlagChanges", true);
        let mut flag_prev: Vec<u8> = Vec::new();
        // PostPhysics: after this frame's windows and forced bits are final.
        common::task::run_recurring_safe(
            cs_task,
            "Log",
            CSTaskGroupIndex::ChrIns_PostPhysics,
            move |_data: &eldenring::fd4::FD4TaskData| {
                if let Some(chr) = player() {
                    log_windows(chr, &mut last, &mut last_press);
                    if log_flags {
                        log_flag_changes(chr, &mut flag_prev);
                    }
                }
            },
        );
    }

    let mask = parse_actions(&config::get_string("Actions", "Guard,Dodge,Weapon"));
    let tae_mask = parse_tae_flags(&config::get_string("TaeFlags", "General,SlotSwitch,Attack"));
    let stages: Vec<CSTaskGroupIndex> = config::get_string("Stage", "PreBehaviorSafe,BehaviorSafe")
        .split(',')
        .map(parse_stage)
        .collect();
    let clear_disable = config::get_bool("ClearCancelDisable", true);
    FORCE.store(config::get_bool("StartEnabled", false), Ordering::Relaxed);
    // One task per stage: the same write at several points of the frame.
    for &stage in &stages {
        common::task::run_recurring_safe(
            cs_task,
            "Force",
            stage,
            move |_data: &eldenring::fd4::FD4TaskData| {
                if !FORCE.load(Ordering::Relaxed) {
                    return;
                }
                if let Some(chr) = player() {
                    force(chr, mask, tae_mask, clear_disable);
                }
            },
        );
    }
    logger::log(&format!(
        "Force: actions mask {mask:#x}, tae flags {tae_mask:#x}, stages {}, clear cancel_disable {clear_disable}, {}.",
        stages.iter().map(|s| stage_name(*s)).collect::<Vec<_>>().join("+"),
        if FORCE.load(Ordering::Relaxed) { "on" } else { "off" }
    ));

    let key = input::parse_virtual_key(&config::get_string("Key", "F7"), 0x76);
    loop {
        std::thread::sleep(Duration::from_millis(50));
        if input::is_key_pressed(key) {
            let on = !FORCE.fetch_xor(true, Ordering::Relaxed);
            logger::log(&format!("Force: {}.", if on { "ON" } else { "OFF" }));
        }
    }
}
