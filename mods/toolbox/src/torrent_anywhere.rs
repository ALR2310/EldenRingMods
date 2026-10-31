//! Summon and ride Torrent anywhere. Copied from
//! `sometweaks::misc::torrent_anywhere` (from `CavaloLivre_V7_Cardoso.dll`);
//! patch details: `mods/sometweaks/docs/misc_patches.md`.
//!
//! 1. `area_list_check` / `direct_ride_check`: `cmp byte[ptr+0x36],0; setne al`
//!    becomes `mov byte[ptr+0x36],0; xor al,al` (same length, in place).
//! 2. `abyssal_forced_dismount_skip`: `jz` -> `jmp` over the forced-dismount
//!    call that runs while SpEffect 19995 is active.
//! 3. SpEffect 19996 ("Remove Forced Torrent Dismount") re-applied every second.
//!    Differs from SomeTweaks: goes through `crate::speffect` (AOB) instead of
//!    `ChrInsExt::apply_speffect`, which panics on 2.7.2.0.

use common::task;
use common::{config, logger, player};
use eldenring::cs::CSTaskGroupIndex;
use eldenring::fd4::FD4TaskData;

use crate::{patch, speffect};

const FEATURE: &str = "TorrentAnywhere";
const REMOVE_FORCED_DISMOUNT_SPEFFECT: i32 = 19996;
const APPLY_INTERVAL_MS: f64 = 1000.0;

const AREA_LIST_CHECK_PATTERN: &str = "48 8B 40 68 80 78 36 00 0F 95 C0 40 B7 01 88 06";
const AREA_LIST_CHECK_PATCHED: [u8; 16] = [
    0x48, 0x8B, 0x40, 0x68, 0xC6, 0x40, 0x36, 0x00, 0x32, 0xC0, 0x40, 0xB7, 0x01, 0x88, 0x06, 0x90,
];

const DIRECT_RIDE_CHECK_PATTERN: &str = "48 8B 48 68 80 79 36 00 0F 95 C0 48 83 C4 28 C3";
const DIRECT_RIDE_CHECK_PATCHED: [u8; 16] = [
    0x48, 0x8B, 0x48, 0x68, 0xC6, 0x41, 0x36, 0x00, 0x32, 0xC0, 0x48, 0x83, 0xC4, 0x28, 0xC3, 0x90,
];

/// `mov edx,0x4E1B(19995); mov rcx,rbx; call HasSpEffect; test al,al; jz +0x10; ...`
/// The `jz` (byte at offset 15) becomes `jmp`.
const ABYSSAL_DISMOUNT_SKIP_PATTERN: &str =
    "BA 1B 4E 00 00 48 8B CB E8 ?? ?? ?? ?? 84 C0 74 10 48 8B 8B A0 06 00 00 48 8B 01 FF 90 B0 00 00 00";
const ABYSSAL_JZ_OFFSET: usize = 15;

pub fn run() {
    if !config::get_bool("TorrentAnywhere", false) {
        logger::log("TorrentAnywhere=false - skipping at startup.");
        return;
    }

    let applied = [
        patch::apply(FEATURE, "area_list_check", AREA_LIST_CHECK_PATTERN, 0, &AREA_LIST_CHECK_PATCHED),
        patch::apply(FEATURE, "direct_ride_check", DIRECT_RIDE_CHECK_PATTERN, 0, &DIRECT_RIDE_CHECK_PATCHED),
        patch::apply(
            FEATURE,
            "abyssal_forced_dismount_skip",
            ABYSSAL_DISMOUNT_SKIP_PATTERN,
            ABYSSAL_JZ_OFFSET,
            &[0xEB],
        ),
    ]
    .into_iter()
    .filter(|ok| *ok)
    .count();
    logger::log(&format!("TorrentAnywhere: {applied}/3 code patch(es) applied."));

    let cs_task = task::wait_for_cs_task();
    let mut elapsed_ms = 0.0;
    task::run_recurring_safe(cs_task, FEATURE, CSTaskGroupIndex::FrameBegin, move |data: &FD4TaskData| {
        elapsed_ms += (data.delta_time.time as f64) * 1000.0;
        if elapsed_ms < APPLY_INTERVAL_MS {
            return;
        }
        elapsed_ms = 0.0;
        // Gate on a real player character (shared/docs/player.md).
        let Some(player) = player::main_player_chr_ins_ptr() else {
            return;
        };
        unsafe { speffect::apply(player as *mut u8, REMOVE_FORCED_DISMOUNT_SPEFFECT, true) };
    });

    patch::park();
}
