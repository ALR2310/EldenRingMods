//! Status buildup tool: triggered from the in-game GUI (one button per status
//! bar, plus per-bar and Clear All buttons).
//!
//! How Elden Ring handles status buildup (`ChrIns::ApplySpEffect`, RVA
//! `0x3e8dc0` -> `sub_1403FB0A0` -> `CSChrResistModule::sub_14043E0E0` ->
//! `sub_14043DE90`):
//! - Each status bar in `CSChrResistModule` (`*(ChrIns + 0x190) + 0x20`) is an
//!   `i32` pair: remaining resistance at `0x10..0x28` and max at `0x2C..0x44`,
//!   counting DOWN from `max` toward `0`.
//! - Simply writing `0` to `0x10..0x28` in memory does NOT show the HUD buildup
//!   gauge (`sub_1404F4C20`) or proc the status ailment, because in Elden Ring
//!   the status `SpEffect` row (`6500` Poison, `6600` Scarlet Rot, `6400`
//!   Bleed, `6800` Death Blight, `6700` Frostbite, `6450` Sleep, `6750`
//!   Madness) is tentatively added to `ChrIns->spEffect` by `sub_1403FB0A0` and
//!   then EITHER removed immediately (when `cur - attack > 0`, leaving only the
//!   reduced `cur` and the HUD gauge on screen) OR kept active on the character
//!   (when `cur - attack <= 0`, triggering the proc, scaling resistance, and
//!   resetting `cur = max`).
//! - Therefore we drive both partial buildup and full proc through
//!   [`crate::speffect::apply`]:
//!   - **Full bar / proc** (`after == 0`): set `cur = 1` and apply
//!     `bar.apply_speffect`, so `sub_14043DE90` sees `1 - attack <= 0` and
//!     procs the status effect + HUD gauge natively.
//!   - **Partial buildup** (`after > 0`): set `cur = max.max(200)`, apply
//!     `bar.apply_speffect` (which pops up the HUD buildup gauge via
//!     `sub_1404F4C20` and immediately removes the SpEffect row because
//!     `cur - attack > 0`), then write `cur = after` so the HUD gauge displays
//!     the exact `max - after` buildup.
//!   - **Clear**: apply the Site of Grace cure SpEffects (`101` Remove All
//!     Status, `102..109` Remove `-99999` buildup per bar) and reset `cur = max`.

use std::sync::Mutex;

use common::input::{is_key_pressed, parse_virtual_key};
use common::task;
use common::{config, logger, overlay, player};
use eldenring::cs::CSTaskGroupIndex;
use eldenring::fd4::FD4TaskData;

use crate::{patch, speffect};

const MODULE_CONTAINER_OFFSET: usize = 0x190;
const RESIST_MODULE_SLOT: usize = 0x20;
const VK_F8: i32 = 0x77;

/// `Grace - Remove All Status` (`stateInfo = 13`): cures active Poison, Scarlet
/// Rot, Frostbite, Sleep, etc. on the player.
const SPEFFECT_CURE_ALL_ACTIVE: i32 = 101;

pub struct Bar {
    pub name: &'static str,
    pub label: &'static str,
    current: usize,
    max: usize,
    /// Vanilla `SpEffectParam` row that inflicts this status buildup / proc.
    apply_speffect: i32,
    /// Vanilla `Grace - Remove ...` `SpEffectParam` row (`-99999` buildup).
    clear_speffect: i32,
}

pub const BARS: [Bar; 7] = [
    Bar { name: "Poison", label: "Poison", current: 0x10, max: 0x2C, apply_speffect: 6500, clear_speffect: 103 },
    Bar { name: "ScarletRot", label: "Scarlet Rot", current: 0x14, max: 0x30, apply_speffect: 6600, clear_speffect: 104 },
    Bar { name: "Bleed", label: "Hemorrhage (Bleed)", current: 0x18, max: 0x34, apply_speffect: 6400, clear_speffect: 102 },
    Bar { name: "Deathblight", label: "Death Blight", current: 0x1C, max: 0x38, apply_speffect: 6800, clear_speffect: 109 },
    Bar { name: "Frost", label: "Frostbite", current: 0x20, max: 0x3C, apply_speffect: 6700, clear_speffect: 105 },
    Bar { name: "Sleep", label: "Sleep", current: 0x24, max: 0x40, apply_speffect: 6450, clear_speffect: 108 },
    Bar { name: "Madness", label: "Madness", current: 0x28, max: 0x44, apply_speffect: 6750, clear_speffect: 107 },
];

#[derive(Clone, Copy)]
pub enum GuiAction {
    /// Fill/advance `BARS[index]` by `BuildupAmount` (0 = full bar / proc).
    Apply(usize),
    /// Reset `BARS[index]` back to its max (0 buildup).
    ClearOne(usize),
    /// Cure active status effects and reset every bar back to its max.
    ClearAll,
}

/// Snapshot of the player's 7 bars (current remaining resistance, max) plus the
/// last status message, updated every frame on `FrameBegin` for the GUI.
#[derive(Clone)]
pub struct Snapshot {
    pub in_game: bool,
    pub bars: [(i32, i32); 7],
    pub status: String,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            in_game: false,
            bars: [(0, 0); 7],
            status: String::new(),
        }
    }
}

static SNAPSHOT: Mutex<Snapshot> = Mutex::new(Snapshot {
    in_game: false,
    bars: [(0, 0); 7],
    status: String::new(),
});

static PENDING: Mutex<Vec<GuiAction>> = Mutex::new(Vec::new());

/// Queues a buildup action from the GUI render thread to run on the next
/// `FrameBegin` tick.
pub fn queue(action: GuiAction) {
    PENDING.lock().unwrap().push(action);
}

/// Latest bar snapshot for the GUI.
pub fn snapshot() -> Snapshot {
    SNAPSHOT.lock().unwrap().clone()
}

pub fn set_status(msg: impl Into<String>) {
    SNAPSHOT.lock().unwrap().status = msg.into();
}

unsafe fn resist_module(player: *const u8) -> Option<*mut u8> {
    let container = unsafe { *(player.add(MODULE_CONTAINER_OFFSET) as *const usize) };
    if container == 0 {
        return None;
    }
    let module = unsafe { *((container + RESIST_MODULE_SLOT) as *const usize) };
    (module != 0).then_some(module as *mut u8)
}

fn apply_bar(player: *const u8, index: usize) {
    let Some(bar) = BARS.get(index) else {
        return;
    };
    let Some(module) = (unsafe { resist_module(player) }) else {
        logger::warn("Buildup: resist module not resolved, nothing applied.");
        set_status("Player resist module not ready.");
        return;
    };
    let cur = unsafe { module.add(bar.current) } as *mut i32;
    let max_ptr = unsafe { module.add(bar.max) } as *mut i32;
    let max = unsafe { std::ptr::read_volatile(max_ptr) };
    let before = unsafe { std::ptr::read_volatile(cur) };
    let amount = config::get_int("BuildupAmount", 0);
    let after = if amount <= 0 { 0 } else { (before - amount).max(0) };

    if after == 0 {
        // Full bar -> proc the status effect natively via ChrIns::ApplySpEffect.
        unsafe { std::ptr::write_volatile(cur, 1) };
        unsafe { speffect::apply(player as *mut u8, bar.apply_speffect, true) };
        // If `cur` is still 1, the status was already active on the player
        // (`sub_1404FAE40` blocked buildup); cure active status and re-apply.
        if unsafe { std::ptr::read_volatile(cur) } == 1 {
            unsafe {
                speffect::apply(player as *mut u8, SPEFFECT_CURE_ALL_ACTIVE, true);
                std::ptr::write_volatile(cur, 1);
                speffect::apply(player as *mut u8, bar.apply_speffect, true);
            }
        }
        let new_max = unsafe { std::ptr::read_volatile(max_ptr) };
        let line = format!("Proc'd {} (bar 100%, max {max} -> {new_max})", bar.label);
        logger::log(&line);
        set_status(line);
    } else {
        // Partial buildup -> notify HUD gauge via ApplySpEffect without proc'ing,
        // then set exact remaining resistance `after`.
        let safe_cap = max.max(200);
        unsafe {
            std::ptr::write_volatile(max_ptr, safe_cap);
            std::ptr::write_volatile(cur, safe_cap);
            speffect::apply(player as *mut u8, bar.apply_speffect, true);
            if std::ptr::read_volatile(cur) == safe_cap {
                // Status was already active; clear it so the buildup bar can show.
                speffect::apply(player as *mut u8, SPEFFECT_CURE_ALL_ACTIVE, true);
                speffect::apply(player as *mut u8, bar.apply_speffect, true);
            }
            std::ptr::write_volatile(max_ptr, max);
            std::ptr::write_volatile(cur, after);
        }
        let filled = (max - after).max(0);
        let line = format!("Buildup {}: {filled} / {max} (rem {before} -> {after})", bar.label);
        logger::log(&line);
        set_status(line);
    }
}

fn clear_bar(player: *const u8, index: usize) {
    let Some(bar) = BARS.get(index) else {
        return;
    };
    let Some(module) = (unsafe { resist_module(player) }) else {
        return;
    };
    unsafe { speffect::apply(player as *mut u8, bar.clear_speffect, true) };
    let cur = unsafe { module.add(bar.current) } as *mut i32;
    let max = unsafe { *(module.add(bar.max) as *const i32) };
    unsafe { std::ptr::write_volatile(cur, max) };
    let line = format!("Cleared {} (0 / {max})", bar.label);
    logger::log(&line);
    set_status(line);
}

fn clear_all(player: *const u8) {
    let Some(module) = (unsafe { resist_module(player) }) else {
        return;
    };
    unsafe { speffect::apply(player as *mut u8, SPEFFECT_CURE_ALL_ACTIVE, true) };
    for bar in &BARS {
        unsafe { speffect::apply(player as *mut u8, bar.clear_speffect, true) };
        let cur = unsafe { module.add(bar.current) } as *mut i32;
        let max = unsafe { *(module.add(bar.max) as *const i32) };
        unsafe { std::ptr::write_volatile(cur, max) };
    }
    logger::log("Buildup: cured active statuses and cleared all bars to max.");
    set_status("Cured statuses and cleared all buildup bars.");
}

fn refresh_snapshot(player: Option<*const u8>) {
    let mut snap = SNAPSHOT.lock().unwrap();
    let Some(p) = player else {
        snap.in_game = false;
        return;
    };
    let Some(module) = (unsafe { resist_module(p) }) else {
        snap.in_game = false;
        return;
    };
    snap.in_game = true;
    for (i, bar) in BARS.iter().enumerate() {
        let cur = unsafe { *(module.add(bar.current) as *const i32) };
        let max = unsafe { *(module.add(bar.max) as *const i32) };
        snap.bars[i] = (cur, max);
    }
}

pub fn run() {
    let cs_task = task::wait_for_cs_task();
    task::run_recurring_safe(cs_task, "Buildup", CSTaskGroupIndex::FrameBegin, move |_: &FD4TaskData| {
        // Menu toggle hotkey (default F8).
        let menu_vk = parse_virtual_key(&config::get_string("MenuKey", "F8"), VK_F8);
        if is_key_pressed(menu_vk) {
            overlay::toggle();
        }

        let player_ptr = player::main_player_chr_ins_ptr();

        // Drain GUI button actions first.
        let actions: Vec<GuiAction> = std::mem::take(&mut *PENDING.lock().unwrap());
        if !actions.is_empty() {
            if let Some(p) = player_ptr {
                for act in actions {
                    match act {
                        GuiAction::Apply(i) => apply_bar(p, i),
                        GuiAction::ClearOne(i) => clear_bar(p, i),
                        GuiAction::ClearAll => clear_all(p),
                    }
                }
            } else {
                set_status("Not in the game world yet.");
            }
        }

        if overlay::is_open() {
            refresh_snapshot(player_ptr);
        }
    });

    patch::park();
}
