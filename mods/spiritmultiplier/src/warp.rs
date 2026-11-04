//! Spirit warp-to-player (experimental). Two parts: the `Warp*` thresholds written every second into
//! the warp manager's runtime copy of the `buddyWarp_*` values, and `WarpWhenFar` (requests a warp for
//! an idle spirit when one of the engine's own conditions holds, see [request_warps]). The warp
//! manager's layout is hard-coded, so [layout_known] checks it first and the features stay off if the
//! game was updated. What the disassembly shows about the manager: `docs/warp_research.md`.

use std::time::Duration;

use eldenring::cs::{CSTaskGroupIndex, WorldChrMan};
use fromsoftware_shared::FromStatic;

use shared::{codepatch, logger, memscan};

use crate::config::{self, Spirit};

const TICK_INTERVAL_MS: f64 = 1000.0;
/// How often `WarpWhenFar` looks at the entries. Walking them checks every
/// node is readable, which adds up with dozens of spirits; 0.1 s is still far
/// quicker than the engine's own 1.5 s conditions.
const REQUEST_INTERVAL_MS: f64 = 100.0;
const MAX_NODES: usize = 64;
/// Bytes of a node the code reads (up to the stuck timers at +132).
const NODE_SIZE: usize = 136;

/// `mov eax,[rbx+60h]; mov ecx,eax; and ecx,7; cmp ecx,7; jnz; test al,8;
/// jz; test al,50h; jz; mov r8b,1; jmp` in `sub_1404C2FD0` (0x1404C309C,
/// unique): reads the flag dword at node +0x60.
const FLAGS_AOB: &str =
    "8B 43 60 8B C8 83 E1 07 83 F9 07 75 0D A8 08 74 09 A8 50 74 05 41 B0 01 EB 08";
/// `cmp byte [rbx+30h],0; jz; cmp byte [rbx+19h],0; jnz ..; mov rax,[rbx+10h];
/// cmp byte [rax+19h],0` in `sub_1404C2FD0` (0x1404C3035): reads the stage
/// byte (+0x30 from the map node's value, +48 from the node) and the isnil
/// byte (+0x19 = 25).
const STAGE_AOB: &str = "80 7B 30 00 74 ?? 80 7B 19 00 0F 85 ?? ?? ?? ?? 48 8B 43 10 80 78 19 00";

/// `cmp qword [r15+0E8h],0; jz ..; call sub_1404B7710; test al,al; jnz ..;
/// mov rdx,[rdi+8]; mov rcx,[r15+0E8h]; call sub_1404C0DD0` at the end of
/// `sub_1404BAEA0` (0x1404BB91B, unique): right after a spirit's chr is
/// created, the game registers it with the warp manager. The `call` to the
/// register function is at +30.
const REGISTER_AOB: &str = "49 83 BF E8 00 00 00 00 74 ?? E8 ?? ?? ?? ?? 84 C0 75 ?? 48 8B 57 08 49 8B 8F E8 00 00 00 E8 ?? ?? ?? ??";
const REGISTER_CALL_OFFSET: usize = 30;

/// `cmp qword [rcx+0E8h],0; mov rdi,rdx; mov rsi,rcx; jz ..; call
/// sub_1404B7710; test al,al; jnz ..; mov rdx,[rdi+8]; mov rcx,[rsi+0E8h];
/// call sub_1404C26F0` in `sub_1404BBAA0` (0x1404BBAAA, unique): when a
/// spirit leaves its group the game removes its warp entry. The `call` to the
/// unregister function is at +36.
const UNREGISTER_AOB: &str = "48 83 B9 E8 00 00 00 00 48 8B FA 48 8B F1 74 ?? E8 ?? ?? ?? ?? 84 C0 75 ?? 48 8B 57 08 48 8B 8E E8 00 00 00 E8 ?? ?? ?? ??";
const UNREGISTER_CALL_OFFSET: usize = 36;

/// `lea rdx,[rsp+70h]; mov rcx,[r15+0E8h]; call sub_1404C2890; nop; mov
/// [rsp+70h],rbx; mov [rsp+70h],rdi` in `SummonBuddyManager::Update`
/// (0x1404B8B6D, unique): the per-frame warp manager update (flags, stage
/// 0 -> 1, the stage 1 -> 2 -> 3 move). The `call` is at +12.
const UPDATE_AOB: &str =
    "48 8D 54 24 70 49 8B 8F E8 00 00 00 E8 ?? ?? ?? ?? 90 48 89 5C 24 70 48 89 7C 24 70";
const UPDATE_CALL_OFFSET: usize = 12;

/// The time argument of the update: the engine's `FD4Time` (vftable, then
/// the frame time in seconds at +8); the warp code only reads the float.
#[repr(C)]
struct FrameTime {
    vftable: usize,
    seconds: f32,
    pad: u32,
}

type UpdateFn = unsafe extern "system" fn(manager: usize, time: *const FrameTime) -> i64;

/// `sub_1404C0DD0(warp_manager, field_ins_handle)`: adds an entry for the
/// handle (stage 0) unless there is one. `sub_1404C26F0` (same signature)
/// removes it.
type RegisterFn = unsafe extern "system" fn(manager: usize, handle: u64) -> i64;

/// Both game functions, resolved from the call sites above.
#[derive(Clone, Copy)]
struct Registry {
    register: RegisterFn,
    unregister: RegisterFn,
    update: UpdateFn,
}

/// The four places the warp code asks `sub_1409FA320` "are we in a multiplayer
/// session?" (`call`, then `test al,al; jz`): `sub_1404C36B0` (the player to
/// warp to), `sub_1404C37D0` and `sub_1404C39D0` (that player's position) and
/// `sub_1404C28D0` (the flags). All unique in 2.6.2.0 - 2.7.2.0. (pattern,
/// offset of the `call` to `sub_1409FA320`).
const SESSION_CHECKS: [(&str, &str, usize); 4] = [
    ("target", "48 8B 9B 08 E5 01 00 E8 ?? ?? ?? ?? 48 8B C8 E8 ?? ?? ?? ?? 84 C0 0F 84", 15),
    ("position A", "66 0F 7F 03 E8 ?? ?? ?? ?? 48 8B C8 E8 ?? ?? ?? ?? 84 C0 0F 84", 12),
    (
        "position B",
        "E8 ?? ?? ?? ?? E8 ?? ?? ?? ?? 48 8B C8 E8 ?? ?? ?? ?? 84 C0 0F 84 AC 00 00 00",
        13,
    ),
    ("flags", "0F 29 4C 24 30 E8 ?? ?? ?? ?? 48 8B C8 E8 ?? ?? ?? ?? 84 C0 74 6D", 13),
];

/// Makes the warp code treat the game as not being in a session, so it uses
/// the local player (`WorldChrMan.main_player`) everywhere. In a session
/// (party list non-empty or 2+ players) it uses the player whose handle the
/// session singleton holds (`+144`, the host) instead: a guest's spirits then
/// warped to the host, and the flags (line of sight, distance) were measured
/// to the host, so they were asked to warp again after every warp (a guest's spirits
/// re-warped every ~3 s while the host stood far away, and blinked in and out). Each `call sub_1409FA320` goes to `xor
/// eax,eax; ret`. Returns whether all four were redirected.
fn force_local_warp_target() -> bool {
    let mut all = true;
    for (name, pattern, offset) in SESSION_CHECKS {
        let Some(anchor) = memscan::wait_for_pattern_in_module(
            pattern,
            Duration::from_millis(500),
            Duration::from_secs(30),
        ) else {
            logger::error(&format!("Warp: {name} pattern not found."));
            all = false;
            continue;
        };
        // SAFETY: the offset stays inside the matched game code or the structure it belongs to.
        let site = unsafe { anchor.add(offset) };
        // SAFETY: reads the 5 bytes of a call found in the game image.
        if unsafe { codepatch::rel32_target(site) }.is_none() {
            // SAFETY: `site` is code in the game image, readable for the length checked.
            if !unsafe { crate::conflicts::check(site, &format!("Warp ({name})")) } {
                logger::error(&format!("Warp: {name} site is not a call."));
            }
            all = false;
            continue;
        }
        // SAFETY: `site` is the scanned call; the stub returns 0 like `sub_1409FA320` returning false.
        if (unsafe { codepatch::redirect_rel32(site, &[0x31, 0xC0, 0xC3]) }).is_none() {
            logger::error(&format!("Warp: {name} redirect failed."));
            all = false;
        }
    }
    all
}

/// Whether Seamless Co-op (`ersc.dll`) is loaded.
fn seamless_loaded() -> bool {
    shared::diag::is_module_loaded("ersc.dll")
}

/// The target of the `call` at `offset` in the first match of `pattern`.
fn call_target(pattern: &str, offset: usize) -> Option<usize> {
    let at = memscan::wait_for_pattern_in_module(
        pattern,
        Duration::from_millis(500),
        Duration::from_secs(30),
    )?;
    // SAFETY: reads the 5 bytes of a call/jmp found in the game image.
    unsafe { codepatch::rel32_target(at.add(offset)) }.map(|target| target as usize)
}

unsafe extern "system" {
    fn IsBadReadPtr(ptr: *const u8, size: usize) -> i32;
}

/// Whether `size` bytes at `addr` can be read.
fn readable(addr: usize, size: usize) -> bool {
    addr >= 0x10000
        && addr.is_multiple_of(8)
        // SAFETY: the call only probes the address and reads nothing.
        && unsafe { IsBadReadPtr(addr as *const u8, size) } == 0
}

/// Whether the engine's warp code still reads the node layout this module
/// hard-codes.
fn layout_known() -> bool {
    let found = |pattern: &str| {
        memscan::wait_for_pattern_in_module(
            pattern,
            Duration::from_millis(500),
            Duration::from_secs(30),
        )
        .is_some()
    };
    found(FLAGS_AOB) && found(STAGE_AOB)
}

unsafe fn rd<T: Copy>(base: usize, offset: usize) -> T {
    // SAFETY: the pointer comes from the game's own live structure for this frame.
    unsafe { ((base + offset) as *const T).read_unaligned() }
}

/// In-order successor of `node` in the MSVC red-black tree (nil nodes have
/// the byte at +25 set), or `None` if a link is not readable.
unsafe fn successor(mut node: usize) -> Option<usize> {
    // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
    unsafe {
        let right: usize = rd(node, 16);
        if !readable(right, 32) {
            return None;
        }
        if rd::<u8>(right, 25) == 0 {
            let mut n = right;
            for _ in 0..MAX_NODES {
                let left: usize = rd(n, 0);
                if !readable(left, 32) {
                    return None;
                }
                if rd::<u8>(left, 25) != 0 {
                    return Some(n);
                }
                n = left;
            }
            return None;
        }
        let mut parent: usize = rd(node, 8);
        for _ in 0..MAX_NODES {
            if !readable(parent, 32) {
                return None;
            }
            if rd::<u8>(parent, 25) != 0 || node != rd::<usize>(parent, 16) {
                return Some(parent);
            }
            node = parent;
            parent = rd(node, 8);
        }
        None
    }
}

/// Calls `f(node)` for each entry node of the warp manager at `a1` (at most
/// [MAX_NODES]); stops quietly at the first unreadable link.
unsafe fn for_each_node(a1: usize, mut f: impl FnMut(usize)) {
    // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
    unsafe {
        if !readable(a1, 64) {
            return;
        }
        let sentinel: usize = rd(a1, 8);
        if !readable(sentinel, 32) {
            return;
        }
        let mut node: usize = rd(sentinel, 0);
        let mut count = 0;
        while node != sentinel && count < MAX_NODES {
            if !readable(node, NODE_SIZE) {
                return;
            }
            f(node);
            count += 1;
            match successor(node) {
                Some(next) => node = next,
                None => return,
            }
        }
    }
}

/// `FieldInsHandle`s (as the raw 8 bytes the map keys use) of the spirits that
/// belong to the local player - the entries of other players' spirits (co-op)
/// must not be touched.
fn local_handles(world_chr_man: &WorldChrMan) -> Vec<u64> {
    world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|pair| pair.second.iter())
        .filter(|g| !g.is_remote && crate::multi_spirit::group_alive(g))
        // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
        .map(|g| unsafe {
            (&g.chr_ins.as_ref().field_ins_handle as *const _ as *const u64).read_unaligned()
        })
        .collect()
}

/// Whether the game can find the chr of `handle`. The warp update looks each
/// entry's chr up by handle and uses the result without a null check, so an
/// entry whose chr is missing (or never findable, as it seems for Seamless's
/// Mimic Tear: crash right after summoning it) takes the game down.
///
/// Calls the ChrSet's `get_chr_ins_by_handle` (vtable slot 3) directly:
/// `WorldChrMan::chr_ins_by_handle` also type-checks the chr through
/// `rva::get()`, which panics on game builds newer than the pinned
/// `fromsoftware-rs` (on 2.7.2.0 the Warp tick panicked every frame).
fn resolvable(world_chr_man: &WorldChrMan, handle: u64) -> bool {
    use eldenring::cs::{ChrIns, ChrSet, FieldInsHandle};
    // SAFETY: the pointer comes from the game's own live structure for this frame.
    let handle = unsafe { (&handle as *const u64 as *const FieldInsHandle).read_unaligned() };
    let Some(Some(chr_set)) = world_chr_man.chr_sets.get(handle.selector.container() as usize)
    else {
        return false;
    };
    let chr_set: &ChrSet<ChrIns> = chr_set;
    type GetByHandle =
        extern "C" fn(&ChrSet<ChrIns>, FieldInsHandle) -> Option<std::ptr::NonNull<ChrIns>>;
    // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
    unsafe {
        let vtable = *(chr_set as *const ChrSet<ChrIns> as *const *const usize);
        let get_by_handle: GetByHandle = std::mem::transmute(*vtable.add(3));
        get_by_handle(chr_set, handle).is_some()
    }
}

/// Registers with the warp manager every spirit of the local player that has
/// no entry. The game does it when it creates the spirit's chr, but with
/// Seamless Co-op the spirits are created by Seamless's own code, which skips
/// that call: no entry, so the engine never warps them (the same config with and
/// without Seamless: `entries: none` with 3 spirits out).
/// Does nothing when every spirit already has its entry (vanilla).
///
/// `ours` = the handles registered here: when such a spirit is gone (dead,
/// sent back, despawned) its entry is removed again, as the game would have
/// done - the update looks the chr up by handle and must not find a stale
/// entry.
unsafe fn register_missing(
    a1: usize,
    world_chr_man: &WorldChrMan,
    registry: Registry,
    ours: &mut Vec<u64>,
) {
    let mut have: Vec<u64> = Vec::new();
    // SAFETY: `a1` is the live warp manager, and the node walk checks every link is readable.
    unsafe { for_each_node(a1, |node| have.push(rd(node, 32))) };
    let local = local_handles(world_chr_man);
    for &handle in &local {
        if !have.contains(&handle) && resolvable(world_chr_man, handle) {
            // SAFETY: the function was found by its pattern in the game image and `a1` is the live warp manager.
            unsafe { (registry.register)(a1, handle) };
            ours.push(handle);
            logger::log(&format!("Warp: registered spirit {handle:#x} with the warp manager."));
        }
    }
    ours.retain(|handle| {
        if local.contains(handle) && resolvable(world_chr_man, *handle) {
            return true;
        }
        if have.contains(handle) {
            // SAFETY: the function was found by its pattern in the game image and `a1` is the live warp manager.
            unsafe { (registry.unregister)(a1, *handle) };
            logger::log(&format!("Warp: removed spirit {handle:#x} from the warp manager."));
        }
        false
    });
}

/// Distance to the player of each live spirit of the local player, by handle.
fn local_distances(world_chr_man: &WorldChrMan) -> Vec<(u64, f32)> {
    let Some(player) = world_chr_man.main_player.as_ref() else {
        return Vec::new();
    };
    let at = player.chr_ins.modules.physics.position;
    world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|pair| pair.second.iter())
        .filter(|g| !g.is_remote && crate::multi_spirit::group_alive(g))
        .map(|g| {
            // SAFETY: the pointer comes from the game's own live structure for this frame.
            let chr = unsafe { g.chr_ins.as_ref() };
            let handle =
                // SAFETY: the pointer comes from the game's own live structure for this frame.
                unsafe { (&chr.field_ins_handle as *const _ as *const u64).read_unaligned() };
            let p = chr.modules.physics.position;
            let (dx, dy, dz) = (p.0 - at.0, p.1 - at.1, p.2 - at.2);
            (handle, (dx * dx + dy * dy + dz * dz).sqrt())
        })
        .collect()
}

/// How long a spirit must stay farther than the warp distance (the engine's
/// own "far" flag uses the same 1.5 s).
const FAR_SECONDS: f32 = 1.5;

/// For the spirits registered by this module (`ours`): the engine's own
/// conditions are not met for them (under Seamless every flag
/// stayed 0, even far away), so the distance is measured here and a spirit
/// that stays farther than the manager's warp distance for [FAR_SECONDS] gets
/// its stage set to 1 (RequestWarp). `far_for` = seconds each has been far.
unsafe fn request_far_ours(
    a1: usize,
    world_chr_man: &WorldChrMan,
    ours: &[u64],
    far_for: &mut std::collections::HashMap<u64, f32>,
    dt: f32,
) {
    let limit = world_chr_man.summon_buddy_manager.warp_manager.trigger_dist_to_player;
    let mut due: Vec<(u64, f32)> = Vec::new();
    far_for.retain(|handle, _| ours.contains(handle));
    for (handle, dist) in local_distances(world_chr_man) {
        if !ours.contains(&handle) {
            continue;
        }
        if dist >= limit {
            let time = far_for.entry(handle).or_insert(0.0);
            *time += dt;
            if *time >= FAR_SECONDS {
                due.push((handle, dist));
            }
        } else {
            far_for.remove(&handle);
        }
    }
    if due.is_empty() {
        return;
    }
    // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
    unsafe {
        for_each_node(a1, |node| {
            let key: u64 = rd(node, 32);
            if let Some(&(_, dist)) = due.iter().find(|(handle, _)| *handle == key)
                && rd::<u8>(node, 48) == 0
            {
                ((node + 48) as *mut u8).write(1);
                far_for.remove(&key);
                logger::log(&format!("Warp: spirit {key:#x} is {dist:.0} m away: warp requested"));
            }
        });
    }
}

/// `WarpWhenFar`: sets the stage of an idle spirit (stage 0) of the local
/// player to 1 (RequestWarp) when one of these holds (flag bits, see the
/// module doc): far for 1.5 s (0x100, which nothing in the engine reads on its
/// own), blocked longer than `WarpBlockedTime` (0x20), or stuck (0x8) while
/// far / blocked (0x50). The engine also wants bits 0-2 all set for the last
/// two; bit 2 is often 0 (blocked for 4.8 s without a warp), so ask
/// regardless. The engine then picks a spot near the player and moves the
/// spirit (stage 2, 3).
unsafe fn request_warps(a1: usize, world_chr_man: &WorldChrMan) {
    let mut local: Option<Vec<u64>> = None;
    // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
    unsafe {
        for_each_node(a1, |node| {
            let stage: u8 = rd(node, 48);
            let flags: u32 = rd(node, 96);
            let wanted =
                flags & 0x100 != 0 || flags & 0x20 != 0 || (flags & 0x8 != 0 && flags & 0x50 != 0);
            if stage != 0 || !wanted {
                return;
            }
            let key: u64 = rd(node, 32);
            let local = local.get_or_insert_with(|| local_handles(world_chr_man));
            if local.contains(&key) {
                ((node + 48) as *mut u8).write(1);
                logger::log(&format!(
                    "Warp: spirit {key:#x} (flags {flags:#05x}): warp requested."
                ));
            }
        });
    }
}

/// One threshold of the warp manager: its config key and value, how to reach the field, and the
/// bookkeeping to restore the game's value.
struct Threshold {
    key: &'static str,
    value: fn(&Spirit) -> f64,
    field: fn(&mut eldenring::cs::SummonBuddyWarpManager) -> &mut f32,
    /// The game's own value (last seen not written by us).
    vanilla: Option<f32>,
    /// What we last wrote.
    written: Option<f32>,
}

impl Threshold {
    fn new(
        key: &'static str,
        value: fn(&Spirit) -> f64,
        field: fn(&mut eldenring::cs::SummonBuddyWarpManager) -> &mut f32,
    ) -> Self {
        Threshold { key, value, field, vanilla: None, written: None }
    }

    /// Writes / restores this threshold in `manager`.
    fn sync(&mut self, manager: &mut eldenring::cs::SummonBuddyWarpManager) {
        let wanted = (self.value)(&config::get().spirit).max(0.0) as f32;
        let at = (self.field)(manager);
        if Some(*at) != self.written {
            // The game (re)set it: that is the vanilla value.
            self.vanilla = Some(*at);
            self.written = None;
        }
        if wanted > 0.0 {
            if *at != wanted {
                logger::log(&format!("Warp: {} {} -> {wanted}.", self.key, *at));
                *at = wanted;
            }
            self.written = Some(wanted);
        } else if let (Some(_), Some(vanilla)) = (self.written, self.vanilla) {
            *at = vanilla;
            logger::log(&format!("Warp: {} back to {vanilla}.", self.key));
            self.written = None;
        }
    }
}

pub fn run() {
    if !layout_known() {
        logger::error(
            "Warp: the engine's warp code was not recognised (game updated?) - warp features off.",
        );
        return;
    }
    let cs_task = shared::task::wait_for_cs_task();
    // Only with Seamless Co-op: the game registers vanilla spirits itself (and
    // not on purpose in the arena), so there is nothing to fix without it.
    // Checked after `CSTaskImp` exists, when every mod loader has loaded its DLLs.
    let registry = if seamless_loaded() {
        match (
            call_target(REGISTER_AOB, REGISTER_CALL_OFFSET),
            call_target(UNREGISTER_AOB, UNREGISTER_CALL_OFFSET),
            call_target(UPDATE_AOB, UPDATE_CALL_OFFSET),
        ) {
            (Some(register), Some(unregister), Some(update)) => {
                logger::log(
                    "Warp: Seamless Co-op found - its spirits are registered for warping here.",
                );
                if force_local_warp_target() {
                    logger::log("Warp: spirits warp to their own owner, not the session's host.");
                }
                // SAFETY: `a1` and the addresses read from it belong to the live warp manager, and every link is checked readable first.
                Some(unsafe {
                    Registry {
                        register: std::mem::transmute::<usize, RegisterFn>(register),
                        unregister: std::mem::transmute::<usize, RegisterFn>(unregister),
                        update: std::mem::transmute::<usize, UpdateFn>(update),
                    }
                })
            }
            _ => {
                logger::error(
                    "Warp: the game's spirit registration was not found - Seamless spirits will not warp.",
                );
                None
            }
        }
    } else {
        None
    };
    let mut ours: Vec<u64> = Vec::new();
    let mut far_for: std::collections::HashMap<u64, f32> = std::collections::HashMap::new();
    let mut elapsed_ms: f64 = TICK_INTERVAL_MS;
    let mut request_ms: f64 = 0.0;
    let mut when_far = false;
    let mut thresholds = [
        Threshold::new("WarpDistance", |s| s.warp_distance, |m| &mut m.trigger_dist_to_player),
        Threshold::new(
            "WarpBlockedTime",
            |s| s.warp_blocked_time,
            |m| &mut m.trigger_time_ray_block,
        ),
        Threshold::new(
            "WarpStuckTime",
            |s| s.warp_stuck_time,
            |m| &mut m.trigger_threshold_time_path_stacked,
        ),
        Threshold::new(
            "WarpStuckRange",
            |s| s.warp_stuck_range,
            |m| &mut m.trigger_threshold_range_path_stacked,
        ),
    ];

    let _handle = shared::task::run_recurring_safe(
        cs_task,
        "Warp",
        CSTaskGroupIndex::FrameBegin,
        move |data: &eldenring::fd4::FD4TaskData| {
            if shared::player::main_player_chr_ins_ptr().is_none() {
                return;
            }
            // SAFETY: called from a game task; the accessor returns `Err` while the singleton does not exist.
            let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
                return;
            };
            let a1 = &*world_chr_man.summon_buddy_manager.warp_manager as *const _ as usize;
            if a1 == 0 {
                return;
            }

            let frame_ms = (data.delta_time.time as f64) * 1000.0;

            // The engine does not run the warp update for the spirits this
            // module registered (under Seamless every flag stayed 0 and a
            // requested warp stayed at stage 1), so run it here, once a
            // frame, as `SummonBuddyManager::Update` would.
            if let (Some(registry), false) = (registry, ours.is_empty()) {
                // The engine's update must never meet an entry whose chr is
                // gone: drop it first, not at the next 100 ms tick.
                ours.retain(|&handle| {
                    if resolvable(world_chr_man, handle) {
                        return true;
                    }
                    // SAFETY: the function was found by its pattern in the game image and `a1` is the live warp manager.
                    unsafe { (registry.unregister)(a1, handle) };
                    logger::log(&format!(
                        "Warp: removed spirit {handle:#x} from the warp manager (chr not found)."
                    ));
                    false
                });
            }
            if let (Some(registry), false) = (registry, ours.is_empty()) {
                let time = FrameTime { vftable: 0, seconds: data.delta_time.time, pad: 0 };
                // SAFETY: the function was found by its pattern in the game image and `a1` is the live warp manager.
                unsafe { (registry.update)(a1, &time) };
            }

            // Once a second: the settings and the thresholds.
            elapsed_ms += frame_ms;
            if elapsed_ms >= TICK_INTERVAL_MS {
                elapsed_ms = 0.0;
                when_far = config::get().spirit.warp_when_far;
                for threshold in thresholds.iter_mut() {
                    threshold.sync(&mut world_chr_man.summon_buddy_manager.warp_manager);
                }
            }

            // Ten times a second: a warp request should not wait a whole second.
            request_ms += frame_ms;
            if request_ms >= REQUEST_INTERVAL_MS {
                let dt = (request_ms / 1000.0) as f32;
                request_ms = 0.0;
                // Often, so a spirit that just left is dropped before the engine's
                // update looks for its chr again.
                if let Some(registry) = registry {
                    // SAFETY: `a1` is the live warp manager, and the node walk checks every link is readable.
                    unsafe { register_missing(a1, world_chr_man, registry, &mut ours) };
                    // SAFETY: `a1` is the live warp manager, and the node walk checks every link is readable.
                    unsafe { request_far_ours(a1, world_chr_man, &ours, &mut far_for, dt) };
                }
                if when_far {
                    // SAFETY: `a1` is the live warp manager, and the node walk checks every link is readable.
                    unsafe { request_warps(a1, world_chr_man) };
                }
            }
        },
    );
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}
