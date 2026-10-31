//! Open the map and fast travel from anywhere, including mid-boss-fight and
//! inside caves/dungeons. Copied from `sometweaks::misc::warp_anywhere`
//! (itself from `Zibinha_FastTravel.dll`); patch details and the Ghidra
//! derivation: `mods/sometweaks/docs/misc_patches.md`.
//!
//! 1. `map_check`: the `call` that reports "map blocked in combat" becomes
//!    `xor rax,rax; nop; nop`, so its result is always 0.
//! 2. `warp_block`: `je` -> `jmp` on the branch taken when warping is allowed.
//! 3. `field_area_unlock`: zero `FieldArea + 0xA0` every frame (the flag that
//!    blocks fast travel inside a dungeon/cave).

use common::task;
use common::{config, logger, memscan};
use eldenring::cs::CSTaskGroupIndex;
use eldenring::fd4::FD4TaskData;

use crate::patch;

const FEATURE: &str = "MapAnywhere";

const MAP_CHECK_PATTERN: &str = "E8 ?? ?? ?? ?? 84 C0 75 11 38 83 C2 3E 00 00 75 09 83 E6 FE 89 74 24 40";
const MAP_CHECK_PATCHED: [u8; 5] = [0x48, 0x31, 0xC0, 0x90, 0x90];

const WARP_BLOCK_PATTERN: &str = "74 2E C7 45 50 58 02 00 00 C7 45 54 02 00 00 00 C7 45 58 01 00 00 00";
const WARP_BLOCK_PATCHED: [u8; 2] = [0xEB, 0x2E];

/// Only the `mov rcx,[rip+disp32]` head (bytes 0..7) is used, to resolve the
/// global slot holding `FieldArea`.
const FIELD_AREA_PATTERN: &str =
    "48 8B 0D ?? ?? ?? ?? 48 ?? ?? ?? 44 0F B6 61 ?? E8 ?? ?? ?? ?? 48 63 87 ?? ?? ?? ?? 48 ?? ?? ?? 48 85 C0";
const FIELD_AREA_INSTANCE_OFFSET: usize = 0xA0;

/// Address of the global slot that holds the `FieldArea` pointer
/// (`match + 7 + disp32`). The slot is fixed; the pointer inside it changes,
/// so it is re-read every tick.
fn resolve_field_area_slot() -> Option<usize> {
    let addr = memscan::wait_for_pattern_in_module(
        FIELD_AREA_PATTERN,
        std::time::Duration::from_millis(500),
        std::time::Duration::from_secs(60),
    )?;
    let disp = unsafe { std::ptr::read_unaligned(addr.add(3) as *const i32) };
    let slot = unsafe { addr.add(7).offset(disp as isize) };
    logger::log(&format!("{FEATURE}: field_area AOB at {addr:p}, global slot at {slot:p}."));
    Some(slot as usize)
}

pub fn run() {
    if !config::get_bool("MapAnywhere", false) {
        logger::log("MapAnywhere=false - skipping at startup.");
        return;
    }

    patch::apply(FEATURE, "map_check", MAP_CHECK_PATTERN, 0, &MAP_CHECK_PATCHED);
    patch::apply(FEATURE, "warp_block", WARP_BLOCK_PATTERN, 0, &WARP_BLOCK_PATCHED);

    let Some(slot) = resolve_field_area_slot() else {
        logger::error("MapAnywhere: field_area pattern not found, dungeon/cave unlock off for this session.");
        patch::park();
    };

    let cs_task = task::wait_for_cs_task();
    let mut logged_once = false;
    task::run_recurring_safe(cs_task, FEATURE, CSTaskGroupIndex::FrameBegin, move |_: &FD4TaskData| {
        let instance = unsafe { std::ptr::read(slot as *const usize) };
        if instance == 0 {
            return;
        }
        unsafe { *((instance + FIELD_AREA_INSTANCE_OFFSET) as *mut i32) = 0 };
        if !logged_once {
            logged_once = true;
            logger::log("MapAnywhere: FieldArea instance found, dungeon/cave unlock applied.");
        }
    });

    patch::park();
}
