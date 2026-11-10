//! Auto pickup: a hook on `CSActionButtonManImp::ExecActionButton`, the game's "was this
//! interaction triggered" check. For an enabled group it answers yes, so the game picks the item
//! up; everything else runs the real function. The hook reads two atomics: no allocation, no lock.
//! See `docs/exec_action_button.md`.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::time::Duration;

use shared::{codepatch, conflict, logger, memscan};

use crate::groups::classify;
use crate::probe;

/// The function's first 61 bytes, unique on every supported build.
const EXEC_ACTION_BUTTON_PATTERN: &str = "48 8b c4 55 41 54 41 55 41 56 41 57 48 8d 68 c9 48 81 ec f0 00 00 00 48 c7 45 b7 fe ff ff ff 48 89 58 08 48 89 70 10 48 89 78 18 0f 29 70 c8 0f 29 78 b8 45 0f b6 f1 0f 28 fa 8b f2 4c 8b e9";

/// The pattern minus the first [`RELOCATE_LEN`] bytes: what remains when another mod has replaced
/// the function's start with a jump.
const TAIL_PATTERN: &str = "48 81 ec f0 00 00 00 48 c7 45 b7 fe ff ff ff 48 89 58 08 48 89 70 10 48 89 78 18 0f 29 70 c8 0f 29 78 b8 45 0f b6 f1 0f 28 fa 8b f2 4c 8b e9";

const FEATURE: &str = "Auto pickup hook";

/// The game code is not readable until it has unpacked itself, so the scan retries.
const SCAN_INTERVAL: Duration = Duration::from_secs(2);
const SCAN_TIMEOUT: Duration = Duration::from_secs(120);

/// The prologue's first 7 instructions, ending before `SUB RSP,0xF0`. None touches an argument
/// register or RIP-relative data, so they relocate verbatim.
const RELOCATE_LEN: usize = 0x10;

/// `mov rax, imm64; jmp rax` at the entry, where `rax` is free (the relocated prologue sets it).
const ABS_JMP_LEN: usize = codepatch::ABSOLUTE_JMP_LEN;

const JMP_R11_LEN: usize = 13;

const RELOCATED_BUF_LEN: usize = RELOCATE_LEN + JMP_R11_LEN;

/// Jump to `target` without touching `rax`: the prologue leaves `rax` = the entry `rsp` and the
/// rest of the function saves registers relative to it, so a jump through `rax` would make those
/// writes land in the function's own code.
fn jmp_via_r11(target: u64) -> [u8; JMP_R11_LEN] {
    let mut bytes = [0u8; JMP_R11_LEN];
    bytes[..2].copy_from_slice(&[0x49, 0xBB]);
    bytes[2..10].copy_from_slice(&target.to_le_bytes());
    bytes[10..].copy_from_slice(&[0x41, 0xFF, 0xE3]);
    bytes
}

// Address of the relocated prologue, set by `install`. The trampoline below reads it through
// `sym`, so neither this nor `pickup_filter` is exported from the DLL.
static RELOCATED_ADDR: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn pickup_trampoline();
}

std::arch::global_asm!(
    r#"
.global pickup_trampoline
pickup_trampoline:
    push    rbp
    mov     rbp, rsp
    sub     rsp, 0x80

    # The real function still needs the incoming arguments; rcx and edx double as the filter's.
    mov     [rsp + 0x20], rcx
    mov     [rsp + 0x28], rdx
    mov     [rsp + 0x30], r8
    mov     [rsp + 0x38], r9
    movups  [rsp + 0x40], xmm0
    movups  [rsp + 0x50], xmm1
    movups  [rsp + 0x60], xmm2
    movups  [rsp + 0x70], xmm3

    call    {filter}
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

    mov     r11, [rip + {relocated}]
    jmp     r11

2:
    mov     rsp, rbp
    pop     rbp
    mov     eax, 1
    ret
"#,
    filter = sym pickup_filter,
    relocated = sym RELOCATED_ADDR,
);

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
extern "C" fn pickup_filter(_this: *mut c_void, id: u32) -> i32 {
    std::panic::catch_unwind(|| {
        let answer = decide(ENABLED.load(Ordering::Relaxed), PAUSED.load(Ordering::Relaxed), id);
        probe::record(id, answer == 1);
        answer
    })
    .unwrap_or(0)
}

enum Found {
    Entry(*mut u8),
    Hooked,
    No,
}

/// The full pattern, else the tail, to tell "another mod hooked the entry" from "the game code
/// isn't there yet or changed".
fn scan(name: &str) -> Found {
    if let Some(entry) = memscan::find_pattern_in_module(EXEC_ACTION_BUTTON_PATTERN) {
        return Found::Entry(entry);
    }
    let Some(tail) = memscan::find_pattern_in_module(TAIL_PATTERN) else {
        return Found::No;
    };
    // SAFETY: the tail matched `RELOCATE_LEN` bytes after the function entry, so the subtraction
    // stays inside the game image.
    let entry = unsafe { tail.sub(RELOCATE_LEN) };
    // SAFETY: `entry` is in the game image just before the matched tail, one readable code region.
    match unsafe { conflict::foreign_hook_at(entry) } {
        Some(hook) => {
            conflict::report(name, FEATURE, &format!("{name}.log"), &hook);
            Found::Hooked
        }
        None => Found::No,
    }
}

/// Waits for the prologue and patches it. Returns `false` (logged) if it isn't found or another
/// mod already hooked it; the mod then runs without pickup.
pub fn install(name: &str) -> bool {
    if INSTALLED.load(Ordering::Relaxed) {
        return true;
    }
    let start = std::time::Instant::now();
    let entry = loop {
        match scan(name) {
            Found::Entry(entry) => break entry,
            Found::Hooked => return false,
            Found::No if start.elapsed() >= SCAN_TIMEOUT => {
                logger::error("Hook: pattern not found - game may need a mod update, check Nexus.");
                return false;
            }
            Found::No => std::thread::sleep(SCAN_INTERVAL),
        }
    };

    let Some(relocated) = codepatch::alloc_executable(RELOCATED_BUF_LEN) else {
        logger::error("Hook: VirtualAlloc failed (relocated bytes)");
        return false;
    };
    // SAFETY: `entry` is the matched function start in the game image, with `RELOCATE_LEN` bytes
    // of code, and `relocated` is a fresh allocation of `RELOCATED_BUF_LEN` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(entry, relocated, RELOCATE_LEN);
        let back = jmp_via_r11((entry as usize + RELOCATE_LEN) as u64);
        std::ptr::copy_nonoverlapping(back.as_ptr(), relocated.add(RELOCATE_LEN), JMP_R11_LEN);
        RELOCATED_ADDR.store(relocated as usize, Ordering::Release);

        let mut patch = [0x90u8; RELOCATE_LEN];
        patch[..ABS_JMP_LEN].copy_from_slice(&codepatch::absolute_jmp(
            pickup_trampoline as *const () as usize as u64,
        ));
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
        assert_eq!(&bytes[..2], &[0x49, 0xBB]);
        assert_eq!(&bytes[2..10], &0x1122_3344_5566_7788u64.to_le_bytes());
        assert_eq!(&bytes[10..], &[0x41, 0xFF, 0xE3]);
    }

    #[test]
    fn relocated_length_matches_the_prologue() {
        // MOV RAX,RSP (3) + PUSH RBP (1) + 4 x PUSH r12..r15 (2 each) + LEA RBP,[RAX-37h] (4)
        assert_eq!(3 + 1 + 4 * 2 + 4, RELOCATE_LEN);
    }
}
