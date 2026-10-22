//! Keeps the engine's cap on how many open-field characters get activated
//! at once from being eaten by our spirits: every tick the cap becomes the
//! game's own value plus the number of spirits out (no extra cost with none),
//! so enemies keep the slots they have in vanilla. Always on. The IDA findings,
//! the singleton layout and the decision history: `docs/activate_limit.md`.

use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::{config, logger, memscan};

/// `mov rax,[rip+X]; cmp byte [rax+0xE8],0; jnz +0xD; cmp byte
/// [rax+0xF4],0; jz ...` in `sub_14050F9E0` (0x14050FAEC in 2.7.1.0).
/// Unique in 2.7.1.0.
const LIMIT_READ_AOB: &str = "48 8B 05 ?? ?? ?? ?? 80 B8 E8 00 00 00 00 75 0D 80 B8 F4 00 00 00 00 0F 84";

const LIMIT_OFFSETS: [usize; 4] = [0xEC, 0xF0, 0xF8, 0xFC];

const TICK_INTERVAL_MS: f64 = 1000.0;
const SCAN_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);

/// Spirits currently occupying the summon ChrSet (everyone's - a co-op
/// partner's spirits shown on this machine take activation slots too).
/// Slots below [crate::band::BAND_START] hold Torrents and are skipped.
/// Only entry pointers are read, never the `ChrIns` behind them, so a
/// spirit mid-load/despawn can't be a stale-pointer read (the lesson from
/// `enemy_probe.rs`'s crashes); such a spirit may be counted for a second
/// or two, which only makes the cap briefly generous.
fn spirits_out() -> u32 {
    if common::player::main_player_chr_ins_ptr().is_none() {
        return 0;
    }
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return 0;
    };
    let chr_set = &world_chr_man.summon_buddy_chr_set;
    (crate::band::BAND_START..chr_set.capacity)
        .filter(|&slot| unsafe { chr_set.entries.add(slot as usize).as_ref() }.chr_ins.is_some())
        .count() as u32
}

/// Address of the `qword_143D6A208` global (a pointer to the singleton).
fn resolve_singleton_slot() -> Option<*const *mut u8> {
    let site = memscan::wait_for_pattern_in_module(LIMIT_READ_AOB, SCAN_RETRY_INTERVAL, SCAN_TIMEOUT)?;
    let rel = unsafe { (site.add(3) as *const i32).read_unaligned() };
    Some(unsafe { site.add(7).offset(rel as isize) } as *const *mut u8)
}

pub fn run() {
    let Some(slot) = resolve_singleton_slot() else {
        logger::error("ActiveLimit: pattern not found. Game may have been updated - re-check LIMIT_READ_AOB.");
        return;
    };
    let slot_addr = slot as usize;

    let cs_task = common::task::wait_for_cs_task();
    let mut elapsed_ms: f64 = 0.0;
    // The game's own values, captured the first time the singleton is
    // seen (before this module ever writes) - restored when the ini key
    // goes back to 0, so a hot reload can switch the experiment off.
    let mut original: Option<[u32; 4]> = None;
    let mut last_applied: Option<[u32; 4]> = None;

    let _handle = common::task::run_recurring_safe(
        cs_task,
        "ActiveLimit",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            elapsed_ms += (data.delta_time.time as f64) * 1000.0;
            if elapsed_ms < TICK_INTERVAL_MS {
                return;
            }
            elapsed_ms = 0.0;

            let singleton = unsafe { *(slot_addr as *const *mut u8) };
            if singleton.is_null() {
                return;
            }
            let field = |offset: usize| unsafe { singleton.add(offset) as *mut u32 };
            let original = *original.get_or_insert_with(|| {
                let values = LIMIT_OFFSETS.map(|o| unsafe { *field(o) });
                let enabled = unsafe { *singleton.add(0xE8) };
                let alt = unsafe { *singleton.add(0xF4) };
                logger::log(&format!(
                    "ActiveLimit: singleton at {singleton:p}, enabled={enabled} alt_mode={alt}, game's limits +EC/+F0/+F8/+FC = {values:?}"
                ));
                values
            });

            // Re-read every tick so F5 (common::reload) applies it live.
            let limit = config::get_int("ActiveCharacterLimit", 0);
            let spirits = spirits_out();
            let target = if limit > 0 {
                [limit as u32; 4]
            } else {
                original.map(|value| value + spirits)
            };
            // Only write what differs - normally nothing, once settled.
            for (offset, value) in LIMIT_OFFSETS.iter().zip(target) {
                let ptr = field(*offset);
                if unsafe { *ptr } != value {
                    unsafe { *ptr = value };
                }
            }
            if last_applied != Some(target) {
                logger::log(&format!(
                    "ActiveLimit: limits now {target:?} (spirits out={spirits}, ActiveCharacterLimit={limit})."
                ));
                last_applied = Some(target);
            }
        },
    );

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
