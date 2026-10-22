//! Heal-on-hit: a hook on the game's hit-resolution function
//! (`FUN_140448910(ctx, attacker, hit_info, _, flag)`). It runs before the
//! real function, so it only observes: it marks combat activity and, for the
//! player's own hits, heals per `Regen.PerHit`. How the function was found,
//! the hit_info layout and the incidents behind the design: `docs/per_hit.md`.

use std::ffi::c_void;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use common::{codepatch, config, logger, memscan};

use crate::heal::{HealField, heal_main_player};
use crate::trigger;

/// The hit function's own prologue, anchored on the full 40-byte window to cut
/// collisions with other functions sharing the generic MSVC prologue shape.
/// Only verified on one game build (2.7.1.0 / 1.17.1).
const HIT_APPLY_PATTERN: &str = "4c 8b dc 55 53 56 57 41 56 41 57 49 8d 6b 88 48 81 ec 48 01 00 00 48 8b 05 ?? ?? ?? ?? 48 33 c4 48 89 45 20 4d 89 63 20";

/// The prologue's first 9 instructions (`MOV R11,RSP`, 6 PUSHes, `LEA`) end on
/// an instruction boundary just before `SUB RSP,0x148` and touch neither the
/// argument registers nor RIP-relative data, so they relocate verbatim.
const RELOCATE_LEN: usize = 0xF;

/// `mov rax, imm64; jmp rax`.
const ABS_JMP_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

/// The relocated prologue plus a jump back into the real function, kept in
/// `.data` (made executable by `install`). Without the trailing jump execution
/// would fall off the end into whatever static data follows.
const RELOCATED_BUF_LEN: usize = RELOCATE_LEN + ABS_JMP_LEN;

#[unsafe(no_mangle)]
static mut G_RELOCATED_BYTES: [u8; RELOCATED_BUF_LEN] = [0u8; RELOCATED_BUF_LEN];

unsafe extern "C" {
    fn hit_trampoline();
}

std::arch::global_asm!(
    r#"
.global hit_trampoline
hit_trampoline:
    # Entered by a JMP at the function's own entry: rsp is exactly what the
    # real caller's `call` left.
    push    r12
    push    r13
    mov     r12, rsp

    # Save the 4 incoming args (rcx=ctx, rdx=attacker, r8=hit_info, r9=flag):
    # our call below clobbers them, the real function still needs them.
    push    rcx
    push    rdx
    push    r8
    push    r9
    mov     r13, rsp

    and     rsp, -16
    sub     rsp, 0x20
    call    on_hit_pre

    mov     rsp, r13
    pop     r9
    pop     r8
    pop     rdx
    pop     rcx
    pop     r13
    pop     r12

    # rsp is back to its entry value: replay the relocated prologue, then
    # resume the function.
    lea     r11, [rip + G_RELOCATED_BYTES]
    jmp     r11
"#
);

const HITINFO_DAMAGE_OFFSET: isize = 0x228;
const HITINFO_SOURCE_OBJECT_OFFSET: isize = 0x1D8;
const CTX_TARGET_OFFSET: isize = 0x8;

fn looks_like_pointer(p: *const c_void) -> bool {
    !p.is_null() && (p as usize) >= 0x10000
}

/// The attacker's `sourceType` (1 = direct / melee, 3 = projectile), read
/// through the source object's vtable, or -1 if anything looks invalid.
fn read_source_type(hit_info: *const c_void) -> i32 {
    if !looks_like_pointer(hit_info) {
        return -1;
    }
    unsafe {
        let source_object = *(hit_info.byte_offset(HITINFO_SOURCE_OBJECT_OFFSET) as *const *const c_void);
        if !looks_like_pointer(source_object) {
            return -1;
        }
        let vtable = *(source_object as *const *const c_void);
        if !looks_like_pointer(vtable) {
            return -1;
        }
        let type_fn_ptr = *(vtable.byte_add(0x10) as *const *const c_void);
        if !looks_like_pointer(type_fn_ptr) {
            return -1;
        }
        let type_fn: extern "C" fn(*const c_void) -> i32 = std::mem::transmute(type_fn_ptr);
        type_fn(source_object)
    }
}

fn read_damage(hit_info: *const c_void) -> Option<i32> {
    looks_like_pointer(hit_info).then(|| unsafe { *(hit_info.byte_offset(HITINFO_DAMAGE_OFFSET) as *const i32) })
}

fn read_target(ctx: *const c_void) -> Option<*const c_void> {
    looks_like_pointer(ctx).then(|| unsafe { *(ctx.byte_offset(CTX_TARGET_OFFSET) as *const *const c_void) })
}

/// `damage_type`: 0 melee only, 1 ranged / spells only, 2 both.
fn matches_damage_type(hit_info: *const c_void, damage_type: i32) -> bool {
    match damage_type {
        2 => true,
        1 => read_source_type(hit_info) == 3,
        _ => read_source_type(hit_info) == 1,
    }
}

/// `Regen.PerHit.*`, shared with the hook through [PARAMS].
#[derive(Clone, Copy)]
pub struct OnHitParams {
    pub enabled: bool,
    /// 0 flat points, 1 % of max, 2 % of the damage dealt.
    pub mode: i32,
    pub damage_type: i32,
    pub exclude_aow: bool,
    pub hp: f64,
    pub fp: f64,
    pub stamina: f64,
}

impl OnHitParams {
    const OFF: OnHitParams =
        OnHitParams { enabled: false, mode: 0, damage_type: 0, exclude_aow: false, hp: 0.0, fp: 0.0, stamina: 0.0 };

    pub fn from_config() -> Self {
        OnHitParams {
            enabled: config::get_bool("Regen.PerHit.Enabled", false),
            mode: config::get_int("Regen.PerHit.Mode", 0),
            damage_type: config::get_int("Regen.PerHit.DamageType", 0),
            exclude_aow: config::get_bool("Regen.PerHit.ExcludeAow", false),
            hp: config::get_double("Regen.PerHit.HP", 0.0),
            fp: config::get_double("Regen.PerHit.FP", 0.0),
            stamina: config::get_double("Regen.PerHit.SP", 0.0),
        }
    }

    pub fn wants_heal(&self) -> bool {
        self.enabled && (self.hp > 0.0 || self.fp > 0.0 || self.stamina > 0.0)
    }
}

static PARAMS: RwLock<OnHitParams> = RwLock::new(OnHitParams::OFF);

pub fn update_params(params: OnHitParams) {
    *PARAMS.write().unwrap_or_else(|e| e.into_inner()) = params;
}

fn params() -> OnHitParams {
    *PARAMS.read().unwrap_or_else(|e| e.into_inner())
}

/// Log lines queued from the hit path and written by the tick: synchronous
/// file I/O inside hit resolution can disturb tightly timed animations.
static PENDING_LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn queue_log(message: String) {
    if let Ok(mut queue) = PENDING_LOGS.lock() {
        queue.push(message);
    }
}

pub fn flush_pending_logs() {
    let messages = match PENDING_LOGS.lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(_) => return,
    };
    for message in messages {
        logger::log(&message);
    }
}

/// Called by `hit_trampoline` with the hit function's own arguments, before
/// the real function runs.
#[unsafe(no_mangle)]
extern "C" fn on_hit_pre(ctx: *mut c_void, attacker: *mut c_void, hit_info: *mut c_void, _flag: u64) {
    let _ = std::panic::catch_unwind(|| apply_hit_heal(ctx, attacker, hit_info));
}

fn apply_hit_heal(ctx: *mut c_void, attacker: *mut c_void, hit_info: *mut c_void) {
    let Some(player) = common::player::main_player_chr_ins_ptr() else {
        return;
    };
    let player_attacks = attacker as *const u8 == player;
    let player_is_hit = read_target(ctx).is_some_and(|target| target as *const u8 == player);
    if player_attacks || player_is_hit {
        trigger::mark_combat_activity();
    }
    if !player_attacks {
        return;
    }

    let params = params();
    if !params.enabled
        || !matches_damage_type(hit_info, params.damage_type)
        || (params.exclude_aow && (trigger::is_last_attack_skill() || trigger::is_skill_anim()))
    {
        return;
    }
    heal_for_hit(&params, hit_info);
}

fn heal_for_hit(params: &OnHitParams, hit_info: *const c_void) {
    let damage = if params.mode == 2 {
        match read_damage(hit_info) {
            Some(damage) if damage > 0 => damage,
            _ => return,
        }
    } else {
        0
    };
    let restore = |field: HealField, value: f64| match params.mode {
        2 => heal_main_player(field, (damage as f64 * value / 100.0) as i32, 0.0),
        1 => heal_main_player(field, 0, value / 100.0),
        _ => heal_main_player(field, value.round() as i32, 0.0),
    };
    let (hp, fp, stamina) =
        (restore(HealField::Hp, params.hp), restore(HealField::Fp, params.fp), restore(HealField::Stamina, params.stamina));
    if (hp > 0 || fp > 0 || stamina > 0) && config::get_bool("LogFile", false) {
        queue_log(format!("Hit: +{hp} HP, +{fp} FP, +{stamina} SP"));
    }
}

/// Scans for the hit function's prologue and installs the hook. Returns
/// `false` (logged) if the pattern isn't found: the mod keeps running without
/// heal-on-hit.
pub fn install(params: OnHitParams) -> bool {
    update_params(params);
    let Some(entry) = memscan::find_pattern_in_module(HIT_APPLY_PATTERN) else {
        logger::error("HitHook: pattern not found, heal-on-hit disabled");
        return false;
    };

    unsafe {
        let relocated = (&raw mut G_RELOCATED_BYTES) as *mut u8;
        std::ptr::copy_nonoverlapping(entry, relocated, RELOCATE_LEN);
        let back = codepatch::absolute_jmp((entry as usize + RELOCATE_LEN) as u64);
        std::ptr::copy_nonoverlapping(back.as_ptr(), relocated.add(RELOCATE_LEN), ABS_JMP_LEN);
        if !codepatch::make_executable(relocated, RELOCATED_BUF_LEN) {
            logger::error("HitHook: VirtualProtect failed, heal-on-hit disabled");
            return false;
        }

        // The jump takes 12 of the 15 relocated bytes; the rest stay NOPs so a
        // disassembler reads cleanly.
        let mut patch = [0x90u8; RELOCATE_LEN];
        patch[..ABS_JMP_LEN].copy_from_slice(&codepatch::absolute_jmp(hit_trampoline as *const () as usize as u64));
        if !codepatch::overwrite_bytes(entry, &patch) {
            logger::error("HitHook: VirtualProtect failed, heal-on-hit disabled");
            return false;
        }
    }
    logger::log("HitHook: installed");
    true
}

/// The tick retries `try_install` every frame until it succeeds (another mod
/// may not have finished patching the same site yet); this throttles the real
/// attempts and escalates to one clear message instead of a wall of retries.
const RETRY_THROTTLE: Duration = Duration::from_secs(10);
const WARN_AFTER: Duration = Duration::from_secs(15);

struct Attempts {
    first: Option<Instant>,
    last: Option<Instant>,
    warned: bool,
}

static ATTEMPTS: Mutex<Attempts> = Mutex::new(Attempts { first: None, last: None, warned: false });

/// Throttled [install]: call it every tick while the hook isn't installed.
pub fn try_install(params: OnHitParams) -> bool {
    {
        let mut attempts = ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let first = *attempts.first.get_or_insert(now);
        if attempts.last.is_some_and(|last| now - last < RETRY_THROTTLE) {
            return false;
        }
        attempts.last = Some(now);
        if now - first >= WARN_AFTER && !attempts.warned {
            attempts.warned = true;
            logger::error(&format!(
                "HitHook: pattern not found after {}s - game may need a mod update, check Nexus.",
                WARN_AFTER.as_secs()
            ));
        }
    }
    install(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_enabled_nonzero_heal_wants_the_hook() {
        let on = OnHitParams { enabled: true, hp: 5.0, ..OnHitParams::OFF };
        assert!(on.wants_heal());
        assert!(!OnHitParams { enabled: false, ..on }.wants_heal());
        assert!(!OnHitParams { hp: 0.0, ..on }.wants_heal());
        assert!(OnHitParams { hp: 0.0, stamina: 1.0, ..on }.wants_heal());
    }
}
