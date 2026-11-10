//! Debug-only `PersistProbe` (off by default): passive log for the "keep
//! spirits through resting at a grace / fast travel" research
//! (`docs/persist_spirits.md`). No hook, no gameplay change.
//!
//! Every frame it snapshots the local player's spirit groups (key of the
//! group map, buddy param, `disappear_requested`, HP, whether the chr can be
//! looked up), the block the player stands in and the player's SpEffects, and
//! logs each change. Reading the log around a grace rest tells which path
//! removed the spirits: groups first flagged `disappear` (a `DisappearAll`
//! call, so something asked for a dismissal) or groups vanishing with no flag
//! (the unload path, `sub_1404BBAA0`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, WorldChrMan};
use fromsoftware_shared::FromStatic;

use shared::{codepatch, logger, memscan};

use crate::config;

/// Calls of `sub_1404BBB80` (destroys one spirit group and its chr) by site:
/// 0 = owner-gone loop, 1 = delay-ended loop, 2 = flagged-group loop (all in
/// the manager update), 3 = `sub_1404BBA80` (all groups of a key). Counted by a
/// stub at each `call` (`lock inc`, then the original), so no Rust runs in the
/// hook and the log shows which path destroyed the spirits.
static DESTROY_CALLS: [AtomicU64; 4] =
    [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

/// (pattern, offset of the `call`), unique in 2.6.2.0 - 2.7.2.0.
const DESTROY_SITES: [(&str, usize); 4] = [
    ("48 8B 56 10 49 8B CF E8 ?? ?? ?? ?? 48 8B 46 08 48 8B D6 48 8B 1E", 7),
    ("F3 0F 11 47 34 72 ?? 48 8B 57 10 49 8B CF E8 ?? ?? ?? ?? 48 8B 47 08", 14),
    ("80 7F 30 00 74 ?? 48 8B 57 10 49 8B CF E8 ?? ?? ?? ?? 48 8B 47 08", 13),
    ("48 8B 56 10 48 8B CD E8 ?? ?? ?? ?? 48 8B 46 08 48 8B D6 48 8B 1E", 7),
];

/// `mov rax,counter; lock inc [rax]; mov rax,original; jmp rax`: rcx/rdx keep
/// the arguments, rax is free at a call.
fn counting_stub(counter: usize, original: u64) -> Vec<u8> {
    let mut c = vec![0x48, 0xB8];
    c.extend_from_slice(&(counter as u64).to_le_bytes());
    c.extend_from_slice(&[0xF0, 0x48, 0xFF, 0x00]);
    c.extend_from_slice(&codepatch::absolute_jmp(original));
    c
}

/// Calls of `DisappearAll` by site: 0 = player has SpEffect 202, 1 = dismiss
/// request (counted by `multi_spirit::disappear_all_hook`), 2 = "left the stone
/// area" (`+0x20 < 0` and both range flags off), 3 = player dead, 4 = the
/// session-reset routine `sub_140C98CA0`. Sites 1 is not patched here.
static DISAPPEAR_CALLS: [AtomicU64; 4] =
    [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

/// (pattern, offset of the `call`), unique in 2.6.2.0 - 2.7.2.0; counter index
/// into [DISAPPEAR_CALLS] (cond 1 / 3 / 4 / session reset = 0 / 1 / 2 / 3).
const DISAPPEAR_SITES: [(&str, usize); 4] = [
    ("48 8B 89 08 E5 01 00 BA CA 00 00 00 E8 ?? ?? ?? ?? 84 C0 74 08 49 8B CF E8", 24),
    ("41 80 BF B5 00 00 00 00 75 08 49 8B CF E8 ?? ?? ?? ?? E8", 13),
    ("84 C0 74 08 49 8B CF E8 ?? ?? ?? ?? 48 8D 54 24 60 48 8B CD E8", 7),
    (
        "48 8B C8 E8 ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 8B 48 08 48 85 C9 74 08 38 99 E1 00 00 00",
        3,
    ),
];

fn install_counters(name: &str, sites: &[(&str, usize)], counters: &'static [AtomicU64; 4]) {
    for (i, (pattern, offset)) in sites.iter().enumerate() {
        let Some(anchor) = memscan::wait_for_pattern_in_module(
            pattern,
            Duration::from_millis(500),
            Duration::from_secs(30),
        ) else {
            logger::error(&format!("PersistProbe: {name} site {i} not found"));
            continue;
        };
        // SAFETY: the offset stays inside the matched game code.
        let site = unsafe { anchor.add(*offset) };
        // SAFETY: reads the 5 bytes of a call found in the game image.
        let Some(original) = (unsafe { codepatch::rel32_target(site) }) else {
            logger::error(&format!("PersistProbe: {name} site {i} is not a call"));
            continue;
        };
        let stub = counting_stub(counters[i].as_ptr() as usize, original);
        // SAFETY: `site` is the scanned call; the stub counts then tail-jumps to the original destination.
        if (unsafe { codepatch::redirect_rel32(site, &stub) }).is_none() {
            logger::error(&format!("PersistProbe: {name} site {i} redirect failed"));
        }
    }
}

fn install_destroy_counters() {
    install_counters("destroy", &DESTROY_SITES, &DESTROY_CALLS);
    install_counters("disappear", &DISAPPEAR_SITES, &DISAPPEAR_CALLS);
}

/// The players of `WorldChrMan.player_chr_set` as the engine's owner-gone
/// cleanup walks them (`sub_140509480`: vtable slot 0 = count, slot 2 = entry
/// i), each as the owner id it compares with the group key (`-` = none).
fn list_players(world_chr_man: &WorldChrMan) -> String {
    type Count = unsafe extern "system" fn(usize) -> i32;
    type Entry = unsafe extern "system" fn(usize, i32) -> usize;
    let set = &world_chr_man.player_chr_set as *const _ as usize;
    // SAFETY: the same vtable calls the game makes on this ChrSet every frame.
    unsafe {
        let vtable = *(set as *const *const usize);
        let count: Count = std::mem::transmute(*vtable);
        let entry: Entry = std::mem::transmute(*vtable.add(2));
        let n = count(set).clamp(0, 16);
        let ids: Vec<String> = (0..n)
            .map(|i| {
                let player = entry(set, i);
                if player < 0x10000 {
                    return "null".to_string();
                }
                let data = ((player + 1408) as *const usize).read_unaligned();
                if data < 0x10000 || !data.is_multiple_of(8) {
                    "-".to_string()
                } else {
                    format!("{}", ((data + 8) as *const i32).read_unaligned())
                }
            })
            .collect();
        format!("{n}{ids:?}")
    }
}

pub fn run() {
    if !config::get().logging.persist_probe {
        logger::log("PersistProbe=false - probe disabled.");
        return;
    }
    install_destroy_counters();
    let cs_task = shared::task::wait_for_cs_task();
    let mut last = String::new();
    let mut last_effects: Vec<i32> = Vec::new();

    let _handle = shared::task::run_recurring_safe(
        cs_task,
        "PersistProbe",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| {
            if shared::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            // SAFETY: the accessor returns `Err` while the singleton does not exist.
            let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
                return;
            };
            let Some(player) = world_chr_man.main_player.as_ref() else {
                return;
            };

            let mut effects: Vec<i32> =
                player.chr_ins.special_effect.entries().map(|e| e.param_id).collect();
            effects.sort_unstable();
            effects.dedup();
            let effects_changed = effects != last_effects;
            if effects_changed {
                last_effects = effects;
            }

            // The id the engine compares with the group keys (`sub_1406576E0`):
            // `*(PlayerIns+1408 -> +8)`; if it changes while the groups keep
            // their old key, the owner-gone cleanup removes them.
            let player_addr = &**player as *const eldenring::cs::PlayerIns as usize;
            // SAFETY: reads PlayerIns +1408 (a pointer, or null) and, if it is non-null, 4 bytes at +8 of it.
            let (data_ptr, owner_id) = unsafe {
                let data = ((player_addr + 1408) as *const usize).read_unaligned();
                (
                    data,
                    if data >= 0x10000 && data.is_multiple_of(8) {
                        Some(((data + 8) as *const i32).read_unaligned())
                    } else {
                        None
                    },
                )
            };
            let mut text = format!(
                "block {:#x}; local owner {owner_id:?} ({data_ptr:#x}); players {}; groups (key = owner id):",
                player.current_block_id.0,
                list_players(world_chr_man)
            );
            let mut count = 0;
            let mut raw_groups: Vec<String> = Vec::new();
            // SpEffects of the first spirit's chr: what makes it play its vanish animation?
            let mut spirit_effects = String::new();
            for pair in world_chr_man.summon_buddy_manager.groups.iter() {
                let (mut n, mut flagged, mut dead) = (0, 0, 0);
                for g in pair.second.iter() {
                    // Raw bytes +0x08..+0x2E of the group (the flag bytes), so a
                    // flip right before the group disappears shows up in the log.
                    // SAFETY: `g` is a live group (the struct is 0x2E bytes).
                    let raw = unsafe {
                        std::slice::from_raw_parts((g as *const _ as *const u8).add(8), 0x26)
                    };
                    let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
                    raw_groups.push(hex);
                    if spirit_effects.is_empty() {
                        // SAFETY: only live characters of the manager are read, this frame.
                        let mut ids: Vec<i32> = unsafe { g.chr_ins.as_ref() }
                            .special_effect
                            .entries()
                            .map(|e| e.param_id)
                            .collect();
                        ids.sort_unstable();
                        ids.dedup();
                        spirit_effects = format!("{ids:?}");
                    }
                    n += 1;
                    flagged += g.disappear_requested as u32;
                    // SAFETY: only live characters of the manager are read, this frame.
                    dead += (unsafe { g.chr_ins.as_ref() }.modules.data.hp <= 0) as u32;
                    count += 1;
                }
                text.push_str(&format!(
                    " [key {:#x}: {n} groups, {flagged} disappear, {dead} dead]",
                    pair.first
                ));
            }
            if count == 0 {
                text.push_str(" none");
            }
            let destroyed: Vec<u64> =
                DESTROY_CALLS.iter().map(|c| c.load(Ordering::Relaxed)).collect();
            text.push_str(&format!("; raw {raw_groups:?}; spirit fx {spirit_effects}; destroy calls {destroyed:?}; disappear calls (202/dismiss/left-zone/dead/reset) [{}, {}, {}, {}, {}]",
                DISAPPEAR_CALLS[0].load(Ordering::Relaxed),
                crate::multi_spirit::DISAPPEAR_HOOK_CALLS.load(Ordering::Relaxed),
                DISAPPEAR_CALLS[1].load(Ordering::Relaxed),
                DISAPPEAR_CALLS[2].load(Ordering::Relaxed),
                DISAPPEAR_CALLS[3].load(Ordering::Relaxed)
            ));
            // The manager's request/state fields (offsets of `docs/persist_spirits.md`):
            // +0x20 requested summon SpEffect, +0x24 active one, +0x28 dismiss
            // request, +0x38/+0x3C stone, +0x88 cooldown int, +0xB4..+0xB7 flags.
            let m = &*world_chr_man.summon_buddy_manager as *const eldenring::cs::SummonBuddyManager
                as usize;
            // SAFETY: plain reads inside the SummonBuddyManager struct (size > 0x100).
            let (req, act, dis, stone, active_stone, cd, flags) = unsafe {
                (
                    ((m + 0x20) as *const i32).read_unaligned(),
                    ((m + 0x24) as *const i32).read_unaligned(),
                    ((m + 0x28) as *const u8).read_unaligned(),
                    ((m + 0x38) as *const i32).read_unaligned(),
                    ((m + 0x3C) as *const i32).read_unaligned(),
                    ((m + 0x88) as *const i32).read_unaligned(),
                    ((m + 0xB4) as *const [u8; 4]).read_unaligned(),
                )
            };
            text.push_str(&format!(
                "; mgr req {req} active {act} dismiss {dis} stone {stone}/{active_stone} cd {cd} flags {flags:?}"
            ));
            if text != last || effects_changed {
                logger::log(&format!("PersistProbe: {text}; player SpEffects {last_effects:?}"));
                last = text;
            }
        },
    );
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
