//! Auto pickup: a hook on `CS::CSActionButtonManImp::ExecActionButton(this,
//! action_button_id, ...)`, the game's "was this interaction triggered"
//! check. For an interaction of an enabled group the hook answers "yes"
//! (`al = 1`) without running the function, so the game performs the pickup
//! as if the button had been pressed; for everything else (and while paused
//! in combat) the real function runs untouched. Why this function, the
//! pattern, and what was ruled out: `docs/exec_action_button.md`.
//!
//! The hook reads two atomics and nothing else: no allocation, no lock.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

use common::{codepatch, conflict, logger, memscan};

use crate::groups::classify;
use crate::probe;

/// The function's first 61 bytes. Unique in 2.6.2.0, 2.7.0.0, 2.7.1.0 and
/// 2.7.2.0 (`scripts/ida/run.ps1 aob -Version all`).
const EXEC_ACTION_BUTTON_PATTERN: &str = "48 8b c4 55 41 54 41 55 41 56 41 57 48 8d 68 c9 48 81 ec f0 00 00 00 48 c7 45 b7 fe ff ff ff 48 89 58 08 48 89 70 10 48 89 78 18 0f 29 70 c8 0f 29 78 b8 45 0f b6 f1 0f 28 fa 8b f2 4c 8b e9";

/// The pattern minus the first [`RELOCATE_LEN`] bytes: what is left when
/// another mod has replaced the function's start with a jump. Unique in the
/// same four builds, always [`RELOCATE_LEN`] bytes after the function entry.
const TAIL_PATTERN: &str = "48 81 ec f0 00 00 00 48 c7 45 b7 fe ff ff ff 48 89 58 08 48 89 70 10 48 89 78 18 0f 29 70 c8 0f 29 78 b8 45 0f b6 f1 0f 28 fa 8b f2 4c 8b e9";

/// What the mod turns off when the entry is already hooked.
const FEATURE: &str = "Auto pickup hook";

/// Seconds between scans, and how long to keep scanning before giving up
/// (the game code isn't readable until it has unpacked itself).
const SCAN_INTERVAL: Duration = Duration::from_secs(2);
const SCAN_TIMEOUT: Duration = Duration::from_secs(120);

/// The prologue's first 7 instructions (`MOV RAX,RSP`, 5 PUSHes, `LEA RBP`),
/// ending on an instruction boundary before `SUB RSP,0xF0`; none touches an
/// argument register or RIP-relative data, so they relocate verbatim.
const RELOCATE_LEN: usize = 0x10;

/// `mov rax, imm64; jmp rax`: used at the function's entry, where `rax` is
/// free (the relocated `MOV RAX,RSP` sets it again).
const ABS_JMP_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

/// `mov r11, imm64; jmp r11`.
const JMP_R11_LEN: usize = 13;

/// The relocated prologue plus a jump back into the real function, kept in
/// `.data` (made executable by `install`).
const RELOCATED_BUF_LEN: usize = RELOCATE_LEN + JMP_R11_LEN;

/// Jump to `target` without touching `rax`: the function's prologue leaves
/// `rax` = the entry `rsp` and the rest of the function saves registers
/// relative to it (`mov [rax+8], rbx`, `movaps [rax-38h], xmm6`...), so a jump
/// through `rax` would make those writes land in the function's own code
/// (2026-10-26: `illegal instruction` on the first pass-through call).
fn jmp_via_r11(target: u64) -> [u8; JMP_R11_LEN] {
    let mut bytes = [0u8; JMP_R11_LEN];
    bytes[..2].copy_from_slice(&[0x49, 0xBB]);
    bytes[2..10].copy_from_slice(&target.to_le_bytes());
    bytes[10..].copy_from_slice(&[0x41, 0xFF, 0xE3]);
    bytes
}

#[unsafe(no_mangle)]
static mut G_PICKUP_RELOCATED: [u8; RELOCATED_BUF_LEN] = [0u8; RELOCATED_BUF_LEN];

unsafe extern "C" {
    fn pickup_trampoline();
}

std::arch::global_asm!(
    r#"
.global pickup_trampoline
pickup_trampoline:
    # Entered by a JMP at the function's own entry: rsp is exactly what the
    # real caller's `call` left (16-byte aligned + 8).
    push    rbp
    mov     rbp, rsp
    sub     rsp, 0x80

    # Keep the incoming args: the filter call clobbers the volatile
    # registers and the real function still needs them. rcx (this) and edx
    # (id) are the filter's own args, so they stay in place for the call.
    mov     [rsp + 0x20], rcx
    mov     [rsp + 0x28], rdx
    mov     [rsp + 0x30], r8
    mov     [rsp + 0x38], r9
    movups  [rsp + 0x40], xmm0
    movups  [rsp + 0x50], xmm1
    movups  [rsp + 0x60], xmm2
    movups  [rsp + 0x70], xmm3

    call    pickup_filter
    test    eax, eax
    jnz     2f

    mov     rcx, [rsp + 0x20]
    mov     rdx, [rsp + 0x28]
    mov     r8,  [rsp + 0x30]
    mov     r9,  [rsp + 0x38]
    movups  xmm0, [rsp + 0x40]
    movups  xmm1, [rsp + 0x50]
    movups  xmm2, [rsp + 0x60]
    movups  xmm3, [rsp + 0x70]
    mov     rsp, rbp
    pop     rbp

    # rsp is back to its entry value: replay the relocated prologue, then
    # resume the function.
    lea     r11, [rip + G_PICKUP_RELOCATED]
    jmp     r11

2:
    # Answer "triggered" and return to the caller.
    mov     rsp, rbp
    pop     rbp
    mov     eax, 1
    ret
"#
);

/// Bit per enabled [`crate::groups::Group`].
static ENABLED: AtomicU8 = AtomicU8::new(0);
/// The hook stands down while this is set (combat, with `PauseInCombat`).
static PAUSED: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(mask: u8) {
    ENABLED.store(mask, Ordering::Relaxed);
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

/// 1 = answer "triggered", 0 = run the real function.
fn decide(enabled: u8, paused: bool, id: u32) -> i32 {
    if paused {
        return 0;
    }
    match classify(id) {
        Some(group) if enabled & group.bit() != 0 => 1,
        _ => 0,
    }
}

/// Called by `pickup_trampoline` with the function's own first two arguments.
#[unsafe(no_mangle)]
extern "C" fn pickup_filter(_this: *mut c_void, id: u32) -> i32 {
    std::panic::catch_unwind(|| {
        let answer = decide(ENABLED.load(Ordering::Relaxed), PAUSED.load(Ordering::Relaxed), id);
        probe::record(id, answer == 1);
        answer
    })
    .unwrap_or(0)
}

/// What a scan of the game code found.
enum Found {
    /// The function with its prologue intact: safe to patch.
    Entry(*mut u8),
    /// Another mod has hooked the entry (reported).
    Hooked,
    /// Nothing yet.
    No,
}

/// One scan: the full pattern, else the tail (the entry is [`RELOCATE_LEN`]
/// bytes before it) to tell "another mod hooked the entry" from "the game code
/// isn't there (yet) or changed".
fn scan() -> Found {
    if let Some(entry) = memscan::find_pattern_in_module(EXEC_ACTION_BUTTON_PATTERN) {
        return Found::Entry(entry);
    }
    let Some(tail) = memscan::find_pattern_in_module(TAIL_PATTERN) else {
        return Found::No;
    };
    let entry = unsafe { tail.sub(RELOCATE_LEN) };
    // SAFETY: `entry` is in the game image just before the matched tail, one readable code region.
    match unsafe { conflict::foreign_hook_at(entry) } {
        Some(hook) => {
            conflict::report("AutoPickup", FEATURE, "AutoPickup.log", &hook);
            Found::Hooked
        }
        None => Found::No,
    }
}

/// Waits for the function's prologue to show up and patches it. Returns
/// `false` (logged) if it isn't found or another mod already hooked it: the
/// mod keeps running without pickup.
pub fn install() -> bool {
    if INSTALLED.load(Ordering::Relaxed) {
        return true;
    }
    let start = std::time::Instant::now();
    let entry = loop {
        match scan() {
            Found::Entry(entry) => break entry,
            Found::Hooked => return false,
            Found::No if start.elapsed() >= SCAN_TIMEOUT => {
                logger::error("Hook: pattern not found - game may need a mod update, check Nexus.");
                return false;
            }
            Found::No => std::thread::sleep(SCAN_INTERVAL),
        }
    };

    unsafe {
        let relocated = (&raw mut G_PICKUP_RELOCATED) as *mut u8;
        std::ptr::copy_nonoverlapping(entry, relocated, RELOCATE_LEN);
        let back = jmp_via_r11((entry as usize + RELOCATE_LEN) as u64);
        std::ptr::copy_nonoverlapping(back.as_ptr(), relocated.add(RELOCATE_LEN), JMP_R11_LEN);
        if !codepatch::make_executable(relocated, RELOCATED_BUF_LEN) {
            logger::error("Hook: VirtualProtect failed (relocated bytes)");
            return false;
        }

        // The jump takes 12 of the 16 relocated bytes; the rest stay NOPs so a
        // disassembler reads cleanly.
        let mut patch = [0x90u8; RELOCATE_LEN];
        patch[..ABS_JMP_LEN].copy_from_slice(&codepatch::absolute_jmp(pickup_trampoline as *const () as usize as u64));
        if !codepatch::overwrite_bytes(entry, &patch) {
            logger::error("Hook: VirtualProtect failed (entry patch)");
            return false;
        }
    }
    INSTALLED.store(true, Ordering::Relaxed);
    logger::log("Hook: installed");
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groups::Group;

    const ALL: u8 = 0b11111;

    #[test]
    fn enabled_group_is_answered() {
        assert_eq!(decide(ALL, false, 7800), 1);
        assert_eq!(decide(ALL, false, 4100), 1);
        assert_eq!(decide(ALL, false, 1000), 1);
    }

    #[test]
    fn disabled_group_or_unknown_row_runs_the_real_function() {
        assert_eq!(decide(ALL & !Group::Items.bit(), false, 4000), 0);
        assert_eq!(decide(ALL, false, 5000), 0);
        assert_eq!(decide(0, false, 7800), 0);
    }

    #[test]
    fn pause_stands_the_hook_down() {
        assert_eq!(decide(ALL, true, 7800), 0);
    }

    #[test]
    fn the_jump_back_uses_r11_and_leaves_rax_alone() {
        let bytes = jmp_via_r11(0x1122_3344_5566_7788);
        // mov r11, imm64
        assert_eq!(&bytes[..2], &[0x49, 0xBB]);
        assert_eq!(&bytes[2..10], &0x1122_3344_5566_7788u64.to_le_bytes());
        // jmp r11
        assert_eq!(&bytes[10..], &[0x41, 0xFF, 0xE3]);
    }

    #[test]
    fn relocated_length_matches_the_prologue() {
        // MOV RAX,RSP (3) + PUSH RBP (1) + 4 x PUSH r12..r15 (2 each) + LEA RBP,[RAX-37h] (4)
        assert_eq!(3 + 1 + 4 * 2 + 4, RELOCATE_LEN);
    }
}
