//! Research probes, off by default (`[Logging]` `CancelProbe` / `FlagProbe`,
//! read every frame so a hot reload switches them). They only write to the
//! log, to see why an animation doesn't cancel.
//!
//! - `CancelProbe`: a `WINDOW` line whenever the animation, the cancel windows
//!   or the pressed actions change (animation and progress, its group `grp=`
//!   and the queue id's group `qgrp=` that the mod goes by, what the mod did
//!   `mod=`, the masks, and the queue entries `qin`/`qcan` for the queue id),
//!   plus a `STYLE` line when the grip or the equipped weapons change.
//! - `FlagProbe`: a `FLAG` line for every word of `CSChrActionFlagModule` that
//!   changes (the flags animation events set).
//!
//! Runs at `ChrIns_PostPhysics`, after this frame's windows are final.

use bitfield::BitRange;
use eldenring::cs::{ActionQueueEntry, CSTaskGroupIndex, ChrActions, ChrIns};

use common::anim::{current_anim, format_anim_id, group_of};
use common::logger;

use crate::cancel;
use crate::config;

/// `TaeCancelFlags` bits that last across frames (the rest flicker per frame).
const TAE_FALLING_JUMP_FRAMES: u32 = 1 << 10;
const TAE_CANCEL_DISABLE: u32 = 1 << 11;

/// Size of `CSChrActionFlagModule` (fromsoftware-rs test_size_of).
const ACTION_FLAG_SIZE: usize = 0x258;

/// An action set as one number, for the log.
fn mask(actions: &ChrActions) -> u64 {
    actions.bit_range(63, 0)
}

/// The bits of `entries` for the animation `id`.
fn entry_mask(entries: &[ActionQueueEntry], id: i32) -> u64 {
    entries.iter().find(|e| e.tae_id == id).map_or(0, |e| mask(&e.actions))
}

/// Logs the cancel windows when they (or the animation) change, and every new
/// press (a press is what the windows are for).
#[derive(Default)]
struct WindowProbe {
    last: Option<Snapshot>,
    last_press: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    anim: i32,
    forced: bool,
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
    /// Keys held, and whether guard was held as the animation began.
    held: u64,
    guard_blocked: bool,
}

impl WindowProbe {
    fn tick(&mut self, chr: &ChrIns) {
        let request = &chr.modules.action_request;
        let anim = current_anim(chr);
        let tae: u32 = request.tae_cancels.bit_range(31, 0);
        let id = cancel::queue_id(request);
        let queue = &request.action_request_queue;
        let now = Snapshot {
            anim: anim.anim_id,
            forced: cancel::last_forced(),
            inputs: mask(&request.possible_action_inputs),
            cancels: mask(&request.possible_action_cancels),
            ready: mask(&request.cancel_ready_actions),
            tae: tae & (TAE_FALLING_JUMP_FRAMES | TAE_CANCEL_DISABLE),
            disabled: mask(&request.disabled_action_inputs),
            queue_id: id,
            queue_inputs: entry_mask(&queue.input_entries, id),
            queue_cancels: entry_mask(&queue.cancel_entries, id),
            held: mask(&request.action_requests),
            guard_blocked: cancel::guard_blocked(),
        };
        let press = mask(&request.new_action_presses);
        if self.last.as_ref() == Some(&now) && press == self.last_press {
            return;
        }
        logger::log(&format!(
            "WINDOW {} t={:.2}/{:.2} grp={:?} qgrp={:?} mod={} inputs={:#x} cancels={:#x} queued={:#x} ready={:#x} press={:#x} tae={:#x} disabled={:#x} q={} qin={:#x} qcan={:#x} held={:#x} gblock={}",
            format_anim_id(now.anim),
            anim.play_time,
            anim.anim_length,
            group_of(now.anim),
            cancel::group_for_cancel(now.queue_id),
            if now.forced { "forced" } else { "-" },
            now.inputs,
            now.cancels,
            mask(&request.queued_action_inputs),
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
        self.last = Some(now);
        self.last_press = press;
    }
}

/// Logs what the player holds when it changes: arm style, selected slots and
/// the weapon param IDs (to see what holding two weapons looks like, where
/// guard is an attack).
#[derive(Default)]
struct StyleProbe {
    last: String,
}

impl StyleProbe {
    fn tick(&mut self) {
        let Some(now) = crate::weapon::describe_held() else {
            return;
        };
        if now != self.last {
            logger::log(&format!("STYLE {now}"));
            self.last = now;
        }
    }
}

/// Logs every 4-byte word of the player's `CSChrActionFlagModule` that
/// changed since the last frame (skipping the vftable and owner pointer).
#[derive(Default)]
struct FlagProbe {
    prev: Vec<u8>,
}

impl FlagProbe {
    fn tick(&mut self, chr: &ChrIns) {
        let module = &*chr.modules.action_flag;
        // SAFETY: `ACTION_FLAG_SIZE` is the module's size, and it is read-only here.
        let now = unsafe { std::slice::from_raw_parts(module as *const _ as *const u8, ACTION_FLAG_SIZE) };
        if self.prev.len() == ACTION_FLAG_SIZE {
            let anim = current_anim(chr);
            for off in (0x10..ACTION_FLAG_SIZE).step_by(4) {
                let old = u32::from_le_bytes(self.prev[off..off + 4].try_into().unwrap());
                let new = u32::from_le_bytes(now[off..off + 4].try_into().unwrap());
                if old != new {
                    logger::log(&format!("FLAG {} t={:.2} +{off:#x} {old:#x}->{new:#x}", format_anim_id(anim.anim_id), anim.play_time));
                }
            }
        }
        self.prev.clear();
        self.prev.extend_from_slice(now);
    }

    fn reset(&mut self) {
        self.prev.clear();
    }
}

/// Registers the probe task (returns at once).
pub fn start() {
    let cs_task = common::task::wait_for_cs_task();
    let mut windows = WindowProbe::default();
    let mut style = StyleProbe::default();
    let mut flags = FlagProbe::default();
    common::task::run_recurring_safe(
        cs_task,
        "Probe",
        CSTaskGroupIndex::ChrIns_PostPhysics,
        move |_data: &eldenring::fd4::FD4TaskData| {
            let logging = config::get().logging.clone();
            if !logging.cancel_probe && !logging.flag_probe {
                flags.reset();
                return;
            }
            let Some(chr) = cancel::player() else {
                return;
            };
            if logging.cancel_probe {
                windows.tick(chr);
                style.tick();
            }
            if logging.flag_probe {
                flags.tick(chr);
            } else {
                flags.reset();
            }
        },
    );
}
