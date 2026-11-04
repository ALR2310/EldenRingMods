//! Heal-on-hit: a hook at the start of the game's hit-resolution function. It runs before the real
//! function and only observes: it marks combat activity and, for the player's own hits, restores
//! HP, FP and Stamina per `[PerHit]`. Nothing in it allocates or locks. See `docs/per_hit.md`.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use shared::{codepatch, logger, memscan};

use crate::config::PerHit;
use crate::heal::{HealField, heal_main_player};
use crate::trigger;

/// The function's prologue, matched in full to avoid other functions with the same MSVC shape.
/// Verified on 2.7.1.0 only.
const HIT_APPLY_PATTERN: &str = "4c 8b dc 55 53 56 57 41 56 41 57 49 8d 6b 88 48 81 ec 48 01 00 00 48 8b 05 ?? ?? ?? ?? 48 33 c4 48 89 45 20 4d 89 63 20";

/// The first 9 instructions (`MOV R11,RSP`, 6 PUSHes, `LEA`) end on an instruction boundary before
/// `SUB RSP,0x148` and touch neither the argument registers nor RIP-relative data, so they
/// relocate verbatim.
const RELOCATE_LEN: usize = 0xF;

/// `mov rax, imm64; jmp rax` at the entry and after the relocated prologue. `rax` is free in both
/// places: the real function overwrites it right after.
const ABS_JMP_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

const RELOCATED_BUF_LEN: usize = RELOCATE_LEN + ABS_JMP_LEN;

// Address of the relocated prologue, set by `install`. The trampoline reads it through `sym`, so
// neither this nor `on_hit_pre` is exported from the DLL.
static RELOCATED_ADDR: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn hit_trampoline();
}

std::arch::global_asm!(
    r#"
.global hit_trampoline
hit_trampoline:
    # Entered by a JMP at the function's entry: rsp is what the caller's `call` left.
    push    r12
    push    r13
    mov     r12, rsp

    # The call below clobbers the 4 argument registers; the real function still needs them.
    push    rcx
    push    rdx
    push    r8
    push    r9
    mov     r13, rsp

    and     rsp, -16
    sub     rsp, 0x20
    call    {on_hit}

    mov     rsp, r13
    pop     r9
    pop     r8
    pop     rdx
    pop     rcx
    pop     r13
    pop     r12

    # rsp is back to its entry value: replay the relocated prologue, which resumes the function.
    mov     r11, [rip + {relocated}]
    jmp     r11
"#,
    on_hit = sym on_hit_pre,
    relocated = sym RELOCATED_ADDR,
);

const HITINFO_DAMAGE_OFFSET: isize = 0x228;
const HITINFO_SOURCE_OBJECT_OFFSET: isize = 0x1D8;
const CTX_TARGET_OFFSET: isize = 0x8;

fn looks_like_pointer(p: *const c_void) -> bool {
    !p.is_null() && (p as usize) >= 0x10000
}

/// The attacker's `sourceType` (1 melee, 3 projectile), read through the source object's vtable;
/// -1 if anything looks invalid.
fn read_source_type(hit_info: *const c_void) -> i32 {
    if !looks_like_pointer(hit_info) {
        return -1;
    }
    // SAFETY: `hit_info` is the function's own argument; each pointer is checked before it is
    // followed, and the type getter is a vtable entry of the live source object.
    unsafe {
        let source_object =
            *(hit_info.byte_offset(HITINFO_SOURCE_OBJECT_OFFSET) as *const *const c_void);
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
    // SAFETY: `hit_info` is the function's own argument, checked to be a plausible pointer.
    looks_like_pointer(hit_info)
        .then(|| unsafe { *(hit_info.byte_offset(HITINFO_DAMAGE_OFFSET) as *const i32) })
}

fn read_target(ctx: *const c_void) -> Option<*const c_void> {
    // SAFETY: `ctx` is the function's own argument, checked to be a plausible pointer.
    looks_like_pointer(ctx)
        .then(|| unsafe { *(ctx.byte_offset(CTX_TARGET_OFFSET) as *const *const c_void) })
}

/// `damage_type`: 0 melee only, 1 ranged and spells only, 2 both.
fn matches_damage_type(hit_info: *const c_void, damage_type: i32) -> bool {
    match damage_type {
        2 => true,
        1 => read_source_type(hit_info) == 3,
        _ => read_source_type(hit_info) == 1,
    }
}

/// `[PerHit]` as the hook reads it.
#[derive(Clone, Copy)]
pub struct OnHitParams {
    pub enabled: bool,
    /// 0 flat points, 1 percent of max, 2 percent of the damage dealt.
    pub mode: i32,
    pub damage_type: i32,
    pub exclude_aow: bool,
    pub hp: f64,
    pub fp: f64,
    pub sp: f64,
}

impl OnHitParams {
    pub fn from_config(per_hit: &PerHit) -> Self {
        OnHitParams {
            enabled: per_hit.enabled,
            mode: per_hit.mode,
            damage_type: per_hit.damage_type,
            exclude_aow: per_hit.exclude_aow,
            hp: per_hit.hp,
            fp: per_hit.fp,
            sp: per_hit.sp,
        }
    }

    pub fn wants_heal(&self) -> bool {
        self.enabled && (self.hp > 0.0 || self.fp > 0.0 || self.sp > 0.0)
    }
}

/// [`OnHitParams`] in atomics: the hook cannot take a lock.
struct SharedParams {
    enabled: AtomicBool,
    mode: AtomicI32,
    damage_type: AtomicI32,
    exclude_aow: AtomicBool,
    hp: AtomicU64,
    fp: AtomicU64,
    sp: AtomicU64,
}

static PARAMS: SharedParams = SharedParams {
    enabled: AtomicBool::new(false),
    mode: AtomicI32::new(0),
    damage_type: AtomicI32::new(0),
    exclude_aow: AtomicBool::new(false),
    hp: AtomicU64::new(0),
    fp: AtomicU64::new(0),
    sp: AtomicU64::new(0),
};

pub fn update_params(params: OnHitParams) {
    PARAMS.enabled.store(params.enabled, Ordering::Relaxed);
    PARAMS.mode.store(params.mode, Ordering::Relaxed);
    PARAMS.damage_type.store(params.damage_type, Ordering::Relaxed);
    PARAMS.exclude_aow.store(params.exclude_aow, Ordering::Relaxed);
    PARAMS.hp.store(params.hp.to_bits(), Ordering::Relaxed);
    PARAMS.fp.store(params.fp.to_bits(), Ordering::Relaxed);
    PARAMS.sp.store(params.sp.to_bits(), Ordering::Relaxed);
}

fn params() -> OnHitParams {
    OnHitParams {
        enabled: PARAMS.enabled.load(Ordering::Relaxed),
        mode: PARAMS.mode.load(Ordering::Relaxed),
        damage_type: PARAMS.damage_type.load(Ordering::Relaxed),
        exclude_aow: PARAMS.exclude_aow.load(Ordering::Relaxed),
        hp: f64::from_bits(PARAMS.hp.load(Ordering::Relaxed)),
        fp: f64::from_bits(PARAMS.fp.load(Ordering::Relaxed)),
        sp: f64::from_bits(PARAMS.sp.load(Ordering::Relaxed)),
    }
}

/// Amounts restored since the tick last looked: file I/O inside hit resolution can disturb tightly
/// timed animations, so the tick writes the log line.
static RESTORED: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

/// `(hp, fp, sp)` restored since the previous call, `None` if nothing was.
pub fn take_restored() -> Option<(u32, u32, u32)> {
    let [hp, fp, sp] = [0, 1, 2].map(|i| RESTORED[i].swap(0, Ordering::Relaxed));
    (hp | fp | sp != 0).then_some((hp, fp, sp))
}

/// Called by `hit_trampoline` with the hit function's own arguments, before the real function.
extern "C" fn on_hit_pre(
    ctx: *mut c_void,
    attacker: *mut c_void,
    hit_info: *mut c_void,
    _flag: u64,
) {
    let _ = std::panic::catch_unwind(|| apply_hit_heal(ctx, attacker, hit_info));
}

fn apply_hit_heal(ctx: *mut c_void, attacker: *mut c_void, hit_info: *mut c_void) {
    let Some(player) = shared::player::main_player_chr_ins_ptr() else {
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
    let restored = [
        restore(HealField::Hp, params.hp),
        restore(HealField::Fp, params.fp),
        restore(HealField::Stamina, params.sp),
    ];
    for (total, amount) in RESTORED.iter().zip(restored) {
        total.fetch_add(amount.max(0) as u32, Ordering::Relaxed);
    }
}

/// Scans for the prologue and installs the hook. `false` (logged) if the pattern is not found: the
/// mod keeps running without heal-on-hit.
pub fn install(params: OnHitParams) -> bool {
    update_params(params);
    let Some(entry) = memscan::find_pattern_in_module(HIT_APPLY_PATTERN) else {
        logger::error("Hit hook: pattern not found, heal-on-hit disabled");
        return false;
    };
    let Some(relocated) = codepatch::alloc_executable(RELOCATED_BUF_LEN) else {
        logger::error("Hit hook: VirtualAlloc failed, heal-on-hit disabled");
        return false;
    };

    // SAFETY: `entry` is the matched function start in the game image with `RELOCATE_LEN` bytes
    // of code, and `relocated` is a fresh allocation of `RELOCATED_BUF_LEN` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(entry, relocated, RELOCATE_LEN);
        let back = codepatch::absolute_jmp((entry as usize + RELOCATE_LEN) as u64);
        std::ptr::copy_nonoverlapping(back.as_ptr(), relocated.add(RELOCATE_LEN), ABS_JMP_LEN);
        RELOCATED_ADDR.store(relocated as usize, Ordering::Release);

        // The jump takes 12 of the 15 relocated bytes; the rest stay NOPs.
        let mut patch = [0x90u8; RELOCATE_LEN];
        patch[..ABS_JMP_LEN]
            .copy_from_slice(&codepatch::absolute_jmp(hit_trampoline as *const () as usize as u64));
        if !codepatch::overwrite_bytes(entry, &patch) {
            logger::error("Hit hook: VirtualProtect failed, heal-on-hit disabled");
            return false;
        }
    }
    logger::log("Hit hook: installed");
    true
}

/// The tick retries every frame until the install works (another mod may still be patching the
/// same code); these throttle the attempts and turn a long failure into one clear message.
const RETRY_THROTTLE: Duration = Duration::from_secs(10);
const WARN_AFTER: Duration = Duration::from_secs(15);

struct Attempts {
    first: Option<Instant>,
    last: Option<Instant>,
    warned: bool,
}

static ATTEMPTS: Mutex<Attempts> = Mutex::new(Attempts { first: None, last: None, warned: false });

/// A throttled [install]: call it every tick while the hook is missing.
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
                "Hit hook: pattern not found after {}s, the game may need a mod update",
                WARN_AFTER.as_secs()
            ));
        }
    }
    install(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFF: OnHitParams = OnHitParams {
        enabled: false,
        mode: 0,
        damage_type: 0,
        exclude_aow: false,
        hp: 0.0,
        fp: 0.0,
        sp: 0.0,
    };

    #[test]
    fn only_an_enabled_nonzero_heal_wants_the_hook() {
        let on = OnHitParams { enabled: true, hp: 5.0, ..OFF };
        assert!(on.wants_heal());
        assert!(!OnHitParams { enabled: false, ..on }.wants_heal());
        assert!(!OnHitParams { hp: 0.0, ..on }.wants_heal());
        assert!(OnHitParams { hp: 0.0, sp: 1.0, ..on }.wants_heal());
    }

    #[test]
    fn params_survive_the_atomics() {
        let written = OnHitParams {
            enabled: true,
            mode: 2,
            damage_type: 1,
            exclude_aow: true,
            hp: 7.5,
            fp: 0.25,
            sp: 3.0,
        };
        update_params(written);
        let read = params();
        assert_eq!(
            (read.mode, read.damage_type, read.hp, read.fp, read.sp),
            (2, 1, 7.5, 0.25, 3.0)
        );
        assert!(read.enabled && read.exclude_aow);
    }

    #[test]
    fn restored_amounts_are_taken_once() {
        take_restored();
        RESTORED[0].fetch_add(5, Ordering::Relaxed);
        RESTORED[2].fetch_add(2, Ordering::Relaxed);
        assert_eq!(take_restored(), Some((5, 0, 2)));
        assert_eq!(take_restored(), None);
    }
}
