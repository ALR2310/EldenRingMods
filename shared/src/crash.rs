//! Logs fatal-looking exceptions (access violation, stack overflow, fast fail,
//! ...) to the mod's log, so a crash the game swallows (no WER dump, no event)
//! still leaves the faulting address and a stack. Installed by
//! [`crate::logger::install_panic_hook`]; details: `shared/docs/crash.md`.
//!
//! It never handles anything: the vectored handler only logs and returns
//! "continue search", so the game's own handlers run as before. These are
//! first-chance exceptions, so a line here is not always the crash: the last
//! lines before the log ends are the interesting ones.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::{Mutex, Once};

use crate::logger;

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
/// `GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | ..._UNCHANGED_REFCOUNT`.
const FROM_ADDRESS_UNCHANGED: u32 = 0x6;
/// Distinct (code, address) pairs logged per process; the rest is noise.
const MAX_LOGGED: usize = 16;
/// Bytes of stack scanned for return addresses.
const STACK_BYTES: usize = 0x400;
const MAX_FRAMES: usize = 12;

#[repr(C)]
struct ExceptionPointers {
    record: *const u8,
    context: *const u8,
}

type Handler = unsafe extern "system" fn(*const ExceptionPointers) -> i32;

unsafe extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: Handler) -> *mut c_void;
    fn GetCurrentProcess() -> *mut c_void;
    fn GetCurrentThreadId() -> u32;
    fn ReadProcessMemory(process: *mut c_void, base: *const c_void, buf: *mut c_void, size: usize, read: *mut usize) -> i32;
    fn GetModuleHandleExW(flags: u32, address: *const c_void, module: *mut *mut c_void) -> i32;
}

static INSTALLED: Once = Once::new();
static SEEN: Mutex<Vec<(u32, usize)>> = Mutex::new(Vec::new());

thread_local! {
    static IN_HANDLER: Cell<bool> = const { Cell::new(false) };
}

/// Registers the handler (once per DLL copy).
pub fn install() {
    INSTALLED.call_once(|| unsafe {
        AddVectoredExceptionHandler(1, handler);
    });
}

fn exception_name(code: u32) -> Option<&'static str> {
    Some(match code {
        0xC000_0005 => "access violation",
        0xC000_00FD => "stack overflow",
        0xC000_001D => "illegal instruction",
        0xC000_0094 => "integer divide by zero",
        0xC000_0096 => "privileged instruction",
        0xC000_0409 => "fast fail",
        0xC000_0374 => "heap corruption",
        _ => return None,
    })
}

/// Reads `len` bytes at `addr` without faulting (false when unreadable).
fn read_bytes(addr: usize, buf: &mut [u8]) -> bool {
    let mut read = 0usize;
    let ok = unsafe { ReadProcessMemory(GetCurrentProcess(), addr as *const c_void, buf.as_mut_ptr().cast(), buf.len(), &mut read) };
    ok != 0 && read == buf.len()
}

/// `name+0xRVA` for an address inside a loaded module, else the bare address.
fn describe(addr: usize) -> String {
    let mut module: *mut c_void = std::ptr::null_mut();
    if unsafe { GetModuleHandleExW(FROM_ADDRESS_UNCHANGED, addr as *const c_void, &mut module) } != 0
        && !module.is_null()
        && let Some(path) = crate::module_path(module)
    {
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
        return format!("{name}+{:#X}", addr - module as usize);
    }
    format!("{addr:#X}")
}

fn in_module(addr: usize) -> bool {
    let mut module: *mut c_void = std::ptr::null_mut();
    unsafe { GetModuleHandleExW(FROM_ADDRESS_UNCHANGED, addr as *const c_void, &mut module) != 0 }
}

/// Whether the 8 bytes before a return address end in a `call` encoding:
/// `E8 rel32`, `FF /2` with a register or `[register]` operand, `FF 15 rel32`,
/// or `FF /2 disp8`.
fn preceded_by_call(before: &[u8; 8]) -> bool {
    before[3] == 0xE8
        || (before[6] == 0xFF && (matches!(before[7], 0xD0..=0xD7) || matches!(before[7], 0x10..=0x17)))
        || (before[2] == 0xFF && before[3] == 0x15)
        || (before[5] == 0xFF && matches!(before[6], 0x50..=0x57))
}

fn stack_frames(rsp: usize) -> Vec<String> {
    let mut stack = vec![0u8; STACK_BYTES];
    if !read_bytes(rsp, &mut stack) {
        return Vec::new();
    }
    let mut frames = Vec::new();
    for word in stack.chunks_exact(8) {
        let value = u64::from_le_bytes(word.try_into().unwrap()) as usize;
        if value < 0x10000 || !in_module(value) {
            continue;
        }
        let mut before = [0u8; 8];
        if read_bytes(value - 8, &mut before) && preceded_by_call(&before) {
            frames.push(describe(value));
            if frames.len() == MAX_FRAMES {
                break;
            }
        }
    }
    frames
}

unsafe extern "system" fn handler(info: *const ExceptionPointers) -> i32 {
    if info.is_null() || IN_HANDLER.with(|f| f.replace(true)) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    // SAFETY: the OS passes valid exception and context records.
    let (record, context) = unsafe { ((*info).record, (*info).context) };
    if !record.is_null() && !context.is_null() {
        log_exception(record, context);
    }
    IN_HANDLER.with(|f| f.set(false));
    EXCEPTION_CONTINUE_SEARCH
}

fn log_exception(record: *const u8, context: *const u8) {
    // SAFETY: EXCEPTION_RECORD / CONTEXT (x64) field offsets.
    let code = unsafe { (record as *const u32).read_unaligned() };
    let Some(name) = exception_name(code) else {
        return;
    };
    let address = unsafe { (record.add(0x10) as *const usize).read_unaligned() };
    {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        if seen.len() >= MAX_LOGGED || seen.contains(&(code, address)) {
            return;
        }
        seen.push((code, address));
    }

    let mut text = format!("Exception: {name} at {} (thread {})", describe(address), unsafe { GetCurrentThreadId() });
    if code == 0xC000_0005 {
        let (kind, target) = unsafe {
            (
                (record.add(0x20) as *const u64).read_unaligned(),
                (record.add(0x28) as *const u64).read_unaligned(),
            )
        };
        let kind = match kind {
            0 => "read",
            1 => "write",
            _ => "execute",
        };
        text = format!("Exception: access violation, {kind} {target:#X}, at {} (thread {})", describe(address), unsafe { GetCurrentThreadId() });
    }
    logger::error(&text);

    let reg = |offset: usize| unsafe { (context.add(offset) as *const u64).read_unaligned() };
    let rsp = reg(0x98) as usize;
    logger::error(&format!(
        "  rax={:#X} rcx={:#X} rdx={:#X} rbx={:#X} rsp={:#X} rbp={:#X} rsi={:#X} rdi={:#X}",
        reg(0x78), reg(0x80), reg(0x88), reg(0x90), rsp, reg(0xA0), reg(0xA8), reg(0xB0)
    ));
    logger::error(&format!(
        "  r8={:#X} r9={:#X} r10={:#X} r11={:#X} r12={:#X} r13={:#X} r14={:#X} r15={:#X}",
        reg(0xB8), reg(0xC0), reg(0xC8), reg(0xD0), reg(0xD8), reg(0xE0), reg(0xE8), reg(0xF0)
    ));
    let frames = stack_frames(rsp);
    if !frames.is_empty() {
        logger::error(&format!("  stack: {}", frames.join(" < ")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fatal_codes_are_named() {
        assert_eq!(exception_name(0xC000_0005), Some("access violation"));
        assert_eq!(exception_name(0xE06D_7363), None); // C++ exception
        assert_eq!(exception_name(0x8000_0003), None); // breakpoint
    }

    /// A fake access violation with an unreadable stack must log and return
    /// without panicking (the handler runs inside a crash).
    #[test]
    fn fake_access_violation_does_not_panic() {
        let mut record = [0u8; 0x98];
        record[..4].copy_from_slice(&0xC000_0005u32.to_le_bytes());
        record[0x10..0x18].copy_from_slice(&0x1234usize.to_le_bytes());
        let context = [0u8; 0x4D0];
        log_exception(record.as_ptr(), context.as_ptr());
    }

    #[test]
    fn call_encodings_are_recognised() {
        // ... E8 rel32 | return address
        assert!(preceded_by_call(&[0, 0, 0, 0xE8, 1, 2, 3, 4]));
        // FF D0 (call rax)
        assert!(preceded_by_call(&[0, 0, 0, 0, 0, 0, 0xFF, 0xD0]));
        assert!(!preceded_by_call(&[0x48, 0x89, 0xC1, 0x90, 0x90, 0x90, 0x90, 0x90]));
    }
}
