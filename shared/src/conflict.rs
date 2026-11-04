//! Detects code another mod already hooked and tells the user: an ERROR log line naming the other
//! DLL, plus one message box per mod listing every feature it turned off. A foreign hook is a jump
//! at the patch site (`E9 rel32`, `FF 25` through a pointer, or `mov r64, imm64; jmp r64`) that
//! leaves the game executable; only hooks installed before ours are seen. Conflicts reported close
//! together share one box, shown on its own thread to keep the loader lock free.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;

use crate::{diag, logger};

/// `MEMORY_BASIC_INFORMATION` (x64), for [readable_len].
#[repr(C)]
struct MemoryBasicInformation {
    base_address: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualQuery(
        address: *const c_void,
        buffer: *mut MemoryBasicInformation,
        length: usize,
    ) -> usize;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, flags: u32) -> i32;
}

const MB_OK: u32 = 0x0;
const MB_ICONWARNING: u32 = 0x30;
const MB_SETFOREGROUND: u32 = 0x1_0000;
const MB_TOPMOST: u32 = 0x4_0000;

/// The first conflict waits this long for more, so features that fail one after another share a box.
const COLLECT_WINDOW: Duration = Duration::from_millis(2000);

/// Conflicts waiting for the box: `(feature, other mod's DLL if known)`. One list per mod DLL.
static PENDING: Mutex<Vec<(String, Option<String>)>> = Mutex::new(Vec::new());

/// Bytes read at a hook site: enough for `mov r64, imm64; jmp r64` (12) and the
/// `41 FF E0+r` form (13).
const SITE_READ_LEN: usize = 13;

const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;

/// How much of a stub outside any module is searched for a pointer back to its DLL.
const STUB_SCAN_LEN: usize = 128;

const HEAD_LOG_LEN: usize = 12;

pub struct ForeignHook {
    pub site: usize,
    pub dest: usize,
    /// File name of the DLL that owns `dest`; `None` if it couldn't be found.
    pub owner: Option<String>,
    /// `dest` is a stub outside any module and `owner` is a guess from pointers inside it.
    pub via_stub: bool,
    pub head: Vec<u8>,
}

/// Where the jump at the start of `code` goes, if it is one of the shapes mods hook with. `site`
/// is the address of `code[0]`; `pointer` reads the 8-byte pointer a `FF 25` jumps through.
fn jump_destination(
    site: usize,
    code: &[u8; SITE_READ_LEN],
    pointer: impl Fn(usize) -> Option<usize>,
) -> Option<usize> {
    let rel32 = || i32::from_le_bytes(code[2..6].try_into().unwrap());
    match code[0] {
        0xE9 => Some(
            (site + 5)
                .wrapping_add_signed(i32::from_le_bytes(code[1..5].try_into().unwrap()) as isize),
        ),
        0xFF if code[1] == 0x25 => pointer((site + 6).wrapping_add_signed(rel32() as isize)),
        0x48 | 0x49 if (0xB8..=0xBF).contains(&code[1]) => {
            let jmp_reg = |at: usize| code[at] == 0xFF && (0xE0..=0xE7).contains(&code[at + 1]);
            let jumps = jmp_reg(10) || (code[10] == 0x41 && jmp_reg(11));
            jumps.then(|| usize::from_le_bytes(code[2..10].try_into().unwrap()))
        }
        _ => None,
    }
}

/// The foreign hook at `site`, if its first bytes jump out of the game image (a jump into the
/// game exe is the game's own code).
///
/// # Safety
/// `site..site + 13` must be readable memory of this process.
pub unsafe fn foreign_hook_at(site: *const u8) -> Option<ForeignHook> {
    // SAFETY: `site..site + SITE_READ_LEN` is readable, by this function's contract.
    let code = unsafe { (site as *const [u8; SITE_READ_LEN]).read_unaligned() };
    let dest = jump_destination(site as usize, &code, |ptr| {
        // SAFETY: the slot was checked readable just above.
        (readable_len(ptr) >= size_of::<usize>())
            .then(|| unsafe { (ptr as *const usize).read_unaligned() })
    })?;
    foreign_hook(site as usize, dest)
}

/// A hook at `site` jumping to `dest`, unless `dest` is inside the game exe.
pub fn foreign_hook(site: usize, dest: usize) -> Option<ForeignHook> {
    let owner = diag::module_containing(dest).map(|m| m.name);
    if owner.as_deref().is_some_and(is_game_exe) {
        return None;
    }
    let head_len = readable_len(site).min(HEAD_LOG_LEN);
    // SAFETY: `readable_len` checked `site..site + head_len` is committed, readable memory.
    let head = unsafe { std::slice::from_raw_parts(site as *const u8, head_len) }.to_vec();
    if owner.is_some() {
        return Some(ForeignHook { site, dest, owner, via_stub: false, head });
    }
    let owner = owner_from_stub(dest);
    Some(ForeignHook { site, dest, via_stub: owner.is_some(), owner, head })
}

fn is_game_exe(name: &str) -> bool {
    name.to_lowercase().ends_with(".exe")
}

/// Bytes readable from `addr` to the end of its committed region, 0 if unreadable.
pub(crate) fn readable_len(addr: usize) -> usize {
    // SAFETY: `MemoryBasicInformation` holds only integers, for which all-zero is valid.
    let mut info = unsafe { std::mem::zeroed::<MemoryBasicInformation>() };
    // SAFETY: `info` is a writable out-struct and its size is passed along.
    let got = unsafe {
        VirtualQuery(addr as *const c_void, &mut info, size_of::<MemoryBasicInformation>())
    };
    if got == 0 || info.state != MEM_COMMIT || info.protect & (PAGE_NOACCESS | PAGE_GUARD) != 0 {
        return 0;
    }
    (info.base_address + info.region_size).saturating_sub(addr)
}

/// Names the mod that owns a stub allocated outside any module: the first non-game, non-system
/// module that a pointer or `E8`/`E9 rel32` in the stub lands in. A guess.
fn owner_from_stub(stub: usize) -> Option<String> {
    let len = readable_len(stub).min(STUB_SCAN_LEN);
    // SAFETY: `readable_len` checked `stub..stub + len` is committed, readable memory.
    let code = unsafe { std::slice::from_raw_parts(stub as *const u8, len) };
    let candidates = (0..len.saturating_sub(8)).flat_map(|i| {
        let pointer = usize::from_le_bytes(code[i..i + 8].try_into().unwrap());
        let rel32 = matches!(code[i], 0xE8 | 0xE9).then(|| {
            (stub + i + 5).wrapping_add_signed(i32::from_le_bytes(
                code[i + 1..i + 5].try_into().unwrap(),
            ) as isize)
        });
        [Some(pointer), rel32]
    });
    candidates.flatten().find_map(|addr| {
        diag::module_containing(addr)
            .filter(|m| !is_game_exe(&m.name) && !diag::is_system_module(m))
            .map(|m| m.name)
    })
}

/// Logs `hook` and queues it for the mod's message box; `feature` is what the mod turns off and
/// `log_file` the file the user should look at.
pub fn report(mod_name: &str, feature: &str, log_file: &str, hook: &ForeignHook) {
    let owner = match (&hook.owner, hook.via_stub) {
        (Some(name), true) => format!("{name}, from its stub"),
        (Some(name), false) => name.clone(),
        (None, _) => "unknown, stub outside any module".to_string(),
    };
    let head: Vec<String> = hook.head.iter().map(|b| format!("{b:02X}")).collect();
    logger::error(&format!(
        "{feature}: hooked by another mod at 0x{:X} (jmp to 0x{:X}, owner: {owner}, bytes: {}), disabled",
        hook.site,
        hook.dest,
        head.join(" ")
    ));
    queue_box(mod_name, log_file, feature, hook.owner.clone());
}

fn pending() -> std::sync::MutexGuard<'static, Vec<(String, Option<String>)>> {
    PENDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The first conflict starts the thread that shows everything collected after [COLLECT_WINDOW] in
/// one box. A modal box on the caller's thread could stall the game.
fn queue_box(mod_name: &str, log_file: &str, feature: &str, owner: Option<String>) {
    let first = {
        let mut queue = pending();
        queue.push((feature.to_string(), owner));
        queue.len() == 1
    };
    if !first {
        return;
    }
    let (mod_name, log_file) = (mod_name.to_string(), log_file.to_string());
    std::thread::spawn(move || {
        std::thread::sleep(COLLECT_WINDOW);
        let entries = std::mem::take(&mut *pending());
        show(&format!("{mod_name} conflict"), &box_text(&mod_name, &entries, &log_file));
    });
}

/// One sentence for a single conflict, a list for several; details are in the log.
fn box_text(mod_name: &str, entries: &[(String, Option<String>)], log_file: &str) -> String {
    let who = |owner: &Option<String>| owner.clone().unwrap_or_else(|| "another mod".to_string());
    match entries {
        [(feature, owner)] => {
            format!(
                "{feature} of {mod_name} conflicts with {} and has been turned off. See {log_file} for details.",
                who(owner)
            )
        }
        _ => {
            let lines: Vec<String> = entries
                .iter()
                .map(|(feature, owner)| format!("- {feature}: {}", who(owner)))
                .collect();
            format!(
                "{} features of {mod_name} conflict with other mods and have been turned off:\n\n{}\n\nSee {log_file} for details.",
                entries.len(),
                lines.join("\n")
            )
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn show(title: &str, text: &str) {
    let (title, text) = (wide(title), wide(text));
    // SAFETY: `title` and `text` are NUL-terminated UTF-16 strings that outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(bytes: &[u8]) -> [u8; SITE_READ_LEN] {
        let mut out = [0x90; SITE_READ_LEN];
        out[..bytes.len()].copy_from_slice(bytes);
        out
    }

    #[test]
    fn box_text_for_one_conflict_is_a_sentence() {
        let one = [("Equip load hook".to_string(), Some("Other.dll".to_string()))];
        assert_eq!(
            box_text("ExampleMod", &one, "ExampleMod.log"),
            "Equip load hook of ExampleMod conflicts with Other.dll and has been turned off. See ExampleMod.log for details."
        );
        let unnamed = [("F".to_string(), None)];
        assert!(box_text("X", &unnamed, "X.log").contains("conflicts with another mod"));
    }

    #[test]
    fn box_text_for_several_conflicts_is_a_list() {
        let many = [
            ("Summon limit".to_string(), Some("Other.dll".to_string())),
            ("Spirit warp".to_string(), None),
        ];
        assert_eq!(
            box_text("ExampleMod", &many, "ExampleMod.log"),
            "2 features of ExampleMod conflict with other mods and have been turned off:\n\n\
             - Summon limit: Other.dll\n- Spirit warp: another mod\n\nSee ExampleMod.log for details."
        );
    }

    #[test]
    fn rel32_jump() {
        // jmp +0x10 from 0x1000: lands at 0x1000 + 5 + 0x10
        assert_eq!(jump_destination(0x1000, &code(&[0xE9, 0x10, 0, 0, 0]), |_| None), Some(0x1015));
        // backward jump
        assert_eq!(
            jump_destination(0x1000, &code(&[0xE9, 0xF0, 0xFF, 0xFF, 0xFF]), |_| None),
            Some(0x0FF5)
        );
    }

    #[test]
    fn jump_through_pointer() {
        // jmp [rip+0x20] at 0x1000: pointer at 0x1000 + 6 + 0x20
        let at = |ptr: usize| (ptr == 0x1026).then_some(0xDEAD_0000);
        assert_eq!(
            jump_destination(0x1000, &code(&[0xFF, 0x25, 0x20, 0, 0, 0]), at),
            Some(0xDEAD_0000)
        );
    }

    #[test]
    fn absolute_jump_through_register() {
        let imm = 0x1122_3344_5566_7788u64.to_le_bytes();
        let mut mov_rax = vec![0x48, 0xB8];
        mov_rax.extend_from_slice(&imm);
        mov_rax.extend_from_slice(&[0xFF, 0xE0]); // jmp rax
        assert_eq!(
            jump_destination(0x1000, &code(&mov_rax), |_| None),
            Some(0x1122_3344_5566_7788)
        );

        let mut mov_r11 = vec![0x49, 0xBB];
        mov_r11.extend_from_slice(&imm);
        mov_r11.extend_from_slice(&[0x41, 0xFF, 0xE3]); // jmp r11
        assert_eq!(
            jump_destination(0x1000, &code(&mov_r11), |_| None),
            Some(0x1122_3344_5566_7788)
        );
    }

    #[test]
    fn ordinary_code_is_not_a_jump() {
        // movaps xmm0,xmm6; mov rbx,[r11+0x10]: a real hook site
        assert_eq!(
            jump_destination(0x1000, &code(&[0x0F, 0x28, 0xC6, 0x49, 0x8B, 0x5B, 0x10]), |_| None),
            None
        );
        // mov rax, imm64 not followed by a jump
        let mut plain = vec![0x48, 0xB8];
        plain.extend_from_slice(&[0; 8]);
        plain.extend_from_slice(&[0x48, 0x89]);
        assert_eq!(jump_destination(0x1000, &code(&plain), |_| None), None);
    }

    #[test]
    fn stub_scan_skips_the_game_and_system_modules() {
        assert_eq!(readable_len(0x10), 0);
        assert!(owner_from_stub(0x10).is_none());

        // A stub whose only pointers go back into the exe and into kernel32
        // names no owner.
        let exe_ptr = wide as *const () as usize;
        let kernel32 = diag::loaded_modules()
            .into_iter()
            .find(|m| m.name.eq_ignore_ascii_case("kernel32.dll"))
            .unwrap();
        let mut stub = vec![0x90u8; 4];
        stub.extend_from_slice(&exe_ptr.to_le_bytes());
        stub.extend_from_slice(&(kernel32.base + 0x100).to_le_bytes());
        stub.extend_from_slice(&[0x90; 16]);
        assert!(owner_from_stub(stub.as_ptr() as usize).is_none());
    }

    #[test]
    fn a_jump_into_the_game_exe_is_not_foreign() {
        // An address inside this test binary resolves to the .exe module.
        let here = wide as *const () as usize;
        assert!(foreign_hook(0x1000, here).is_none());
        // Memory outside every module is a foreign stub with no owner name.
        assert!(foreign_hook(0x1000, 0x10).is_some_and(|h| h.owner.is_none()));
    }
}
