//! Logs fatal-looking exceptions (access violation, stack overflow, fast fail, ...) to the mod's
//! log, so a crash the game swallows (no WER dump, no event) still leaves the faulting address and
//! a stack. Installed by [`crate::logger::install_panic_hook`].
//!
//! It never handles anything: the vectored handler only logs and returns "continue search", so
//! the game's own handlers run as before. These are first-chance exceptions, so a line here is
//! not always the crash; the last one before the log ends is. A top-level unhandled-exception
//! filter does not work here because the game's CRT replaces it.

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
/// x64 `CONTEXT` size, and the offsets of `Rsp` / `Rip` in it.
const CONTEXT_SIZE: usize = 0x4D0;
const CONTEXT_RSP: usize = 0x98;
const CONTEXT_RIP: usize = 0xF8;
const STACK_OVERFLOW: u32 = 0xC000_00FD;
/// Bytes scanned at the top of the stack for the outermost frames.
const OUTER_BYTES: usize = 0x1000;
/// Bytes shown from the faulting instruction.
const CODE_BYTES: usize = 16;

#[repr(C)]
struct ExceptionPointers {
    record: *const u8,
    context: *const u8,
}

type Handler = unsafe extern "system" fn(*const ExceptionPointers) -> i32;

/// A `CONTEXT` copy; the unwinder needs it 16-byte aligned.
#[repr(C, align(16))]
struct Context([u8; CONTEXT_SIZE]);

unsafe extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: Handler) -> *mut c_void;
    fn RtlLookupFunctionEntry(pc: u64, image_base: *mut u64, history: *mut c_void) -> *mut c_void;
    #[allow(clippy::too_many_arguments)]
    fn RtlVirtualUnwind(
        kind: u32,
        image_base: u64,
        pc: u64,
        entry: *mut c_void,
        context: *mut c_void,
        handler_data: *mut *mut c_void,
        establisher: *mut u64,
        pointers: *mut c_void,
    ) -> *mut c_void;
    fn GetCurrentProcess() -> *mut c_void;
    fn GetCurrentThreadId() -> u32;
    fn ReadProcessMemory(
        process: *mut c_void,
        base: *const c_void,
        buf: *mut c_void,
        size: usize,
        read: *mut usize,
    ) -> i32;
    fn GetModuleHandleExW(flags: u32, address: *const c_void, module: *mut *mut c_void) -> i32;
}

static INSTALLED: Once = Once::new();
static SEEN: Mutex<Vec<(u32, usize)>> = Mutex::new(Vec::new());

thread_local! {
    static IN_HANDLER: Cell<bool> = const { Cell::new(false) };
}

/// Registers the handler (once per DLL copy).
pub fn install() {
    // SAFETY: `handler` has the signature the API expects and stays valid for the life of the process.
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
    // SAFETY: reads this process's own memory into `buf`; an unreadable address makes the call fail instead of faulting.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const c_void,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut read,
        )
    };
    ok != 0 && read == buf.len()
}

/// `name+0xRVA` for an address inside a loaded module, else the bare address.
fn describe(addr: usize) -> String {
    let mut module: *mut c_void = std::ptr::null_mut();
    // SAFETY: the flag only looks the address up, without taking a reference, and `module` is a valid out-pointer.
    if unsafe { GetModuleHandleExW(FROM_ADDRESS_UNCHANGED, addr as *const c_void, &mut module) }
        != 0
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
    // SAFETY: as in `describe`.
    unsafe { GetModuleHandleExW(FROM_ADDRESS_UNCHANGED, addr as *const c_void, &mut module) != 0 }
}

/// Whether the 8 bytes before a return address end in a `call` encoding:
/// `E8 rel32`, `FF /2` with a register or `[register]` operand, `FF 15 rel32`,
/// or `FF /2 disp8`.
fn preceded_by_call(before: &[u8; 8]) -> bool {
    before[3] == 0xE8
        || (before[6] == 0xFF
            && (matches!(before[7], 0xD0..=0xD7) || matches!(before[7], 0x10..=0x17)))
        || (before[2] == 0xFF && before[3] == 0x15)
        || (before[5] == 0xFF && matches!(before[6], 0x50..=0x57))
}

/// `[lowest, highest)` address of the current thread's stack, from the TEB
/// (`DeallocationStack` / `StackBase`), so the unwinder never walks outside it.
fn stack_bounds() -> (usize, usize) {
    let (low, high): (usize, usize);
    // SAFETY: gs:[...] is the x64 TEB of the current thread.
    unsafe {
        std::arch::asm!(
            "mov {0}, gs:[0x1478]",
            "mov {1}, gs:[0x8]",
            out(reg) low,
            out(reg) high,
            options(nostack, readonly, preserves_flags)
        );
    }
    (low, high)
}

/// Walks the stack with the `.pdata` unwind tables (exact frames, unlike the
/// scan below). Starts at the faulting instruction. Stops at the first frame
/// whose stack pointer leaves the thread's stack or does not grow.
fn unwind_frames(context: *const u8) -> Vec<String> {
    let mut ctx = Context([0; CONTEXT_SIZE]);
    // SAFETY: the caller passes a CONTEXT record of CONTEXT_SIZE bytes.
    unsafe { std::ptr::copy_nonoverlapping(context, ctx.0.as_mut_ptr(), CONTEXT_SIZE) };
    let (low, high) = stack_bounds();
    let field = |ctx: &Context, offset: usize| {
        u64::from_le_bytes(ctx.0[offset..offset + 8].try_into().unwrap()) as usize
    };
    let mut frames = Vec::new();
    while frames.len() < MAX_FRAMES {
        let (rip, rsp) = (field(&ctx, CONTEXT_RIP), field(&ctx, CONTEXT_RSP));
        if rip < 0x10000 || rsp < low || rsp >= high || rsp % 8 != 0 {
            break;
        }
        frames.push(describe(rip));
        let mut image = 0u64;
        // SAFETY: plain table lookup; a null result means a leaf function.
        let entry = unsafe { RtlLookupFunctionEntry(rip as u64, &mut image, std::ptr::null_mut()) };
        if entry.is_null() {
            let mut ret = [0u8; 8];
            if !read_bytes(rsp, &mut ret) {
                break;
            }
            ctx.0[CONTEXT_RIP..CONTEXT_RIP + 8].copy_from_slice(&ret);
            ctx.0[CONTEXT_RSP..CONTEXT_RSP + 8].copy_from_slice(&((rsp + 8) as u64).to_le_bytes());
        } else {
            let mut handler_data = std::ptr::null_mut();
            let mut establisher = 0u64;
            // SAFETY: rsp is inside the thread's stack and `entry` belongs to `image`;
            // flag 0 = UNW_FLAG_NHANDLER (no handler is looked up or run).
            unsafe {
                RtlVirtualUnwind(
                    0,
                    image,
                    rip as u64,
                    entry,
                    ctx.0.as_mut_ptr().cast(),
                    &mut handler_data,
                    &mut establisher,
                    std::ptr::null_mut(),
                );
            }
        }
        if field(&ctx, CONTEXT_RSP) <= rsp {
            break;
        }
    }
    frames
}

/// Heuristic fallback: values on the first `STACK_BYTES` of the stack that
/// look like a return address. Used when unwinding is not safe (stack
/// overflow) or yields nothing.
fn stack_frames(rsp: usize) -> Vec<String> {
    let mut stack = vec![0u8; STACK_BYTES];
    if !read_bytes(rsp, &mut stack) {
        return Vec::new();
    }
    let mut frames = Vec::new();
    for word in stack.chunks_exact(8) {
        let value = u64::from_le_bytes(word.try_into().unwrap()) as usize;
        if looks_like_return_address(value) {
            frames.push(describe(value));
            if frames.len() == MAX_FRAMES {
                break;
            }
        }
    }
    frames
}

/// Inside a module and right after a `call`.
fn looks_like_return_address(value: usize) -> bool {
    if value < 0x10000 || !in_module(value) {
        return false;
    }
    let mut before = [0u8; 8];
    read_bytes(value - 8, &mut before) && preceded_by_call(&before)
}

/// The outermost frames (thread start up to where a runaway recursion began),
/// scanned from the top of the stack downwards. After a stack overflow the
/// innermost frames are all the same loop; these show who entered it.
fn outer_frames() -> Vec<String> {
    let (low, high) = stack_bounds();
    if high <= low || high - low < OUTER_BYTES {
        return Vec::new();
    }
    let mut stack = vec![0u8; OUTER_BYTES];
    if !read_bytes(high - OUTER_BYTES, &mut stack) {
        return Vec::new();
    }
    let mut frames = Vec::new();
    for word in stack.chunks_exact(8).rev() {
        let value = u64::from_le_bytes(word.try_into().unwrap()) as usize;
        if looks_like_return_address(value) {
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

/// `name=0xVALUE`, plus `(module+RVA)` when the value points into a module.
fn reg_text(name: &str, value: u64) -> String {
    let value = value as usize;
    if value >= 0x10000 && in_module(value) {
        format!("{name}={value:#X}({})", describe(value))
    } else {
        format!("{name}={value:#X}")
    }
}

fn log_exception(record: *const u8, context: *const u8) {
    // SAFETY: EXCEPTION_RECORD / CONTEXT (x64) field offsets.
    let code = unsafe { (record as *const u32).read_unaligned() };
    let Some(name) = exception_name(code) else {
        return;
    };
    // SAFETY: `record` is the EXCEPTION_RECORD the OS passed to the handler; `ExceptionAddress` is at +0x10.
    let address = unsafe { (record.add(0x10) as *const usize).read_unaligned() };
    {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        if seen.len() >= MAX_LOGGED || seen.contains(&(code, address)) {
            return;
        }
        seen.push((code, address));
    }

    // SAFETY: `GetCurrentThreadId` has no preconditions.
    let mut text = format!("Exception: {name} at {} (thread {})", describe(address), unsafe {
        GetCurrentThreadId()
    });
    if code == 0xC000_0005 {
        // SAFETY: for an access violation `ExceptionInformation[0..2]` (kind, target address) sit at +0x20 and +0x28 of the record.
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
        text = format!(
            "Exception: access violation, {kind} {target:#X}, at {} (thread {})",
            describe(address),
            // SAFETY: `GetCurrentThreadId` has no preconditions.
            unsafe { GetCurrentThreadId() }
        );
    }
    logger::error(&text);

    let mut code_bytes = [0u8; CODE_BYTES];
    if read_bytes(address, &mut code_bytes) {
        let hex: Vec<String> = code_bytes.iter().map(|b| format!("{b:02X}")).collect();
        logger::error(&format!("  code: {}", hex.join(" ")));
    }

    // SAFETY: `context` is the CONTEXT the OS passed to the handler and every offset used is inside it.
    let reg = |offset: usize| unsafe { (context.add(offset) as *const u64).read_unaligned() };
    let rsp = reg(0x98) as usize;
    let line = |regs: &[(&str, usize)]| {
        let parts: Vec<String> =
            regs.iter().map(|&(name, offset)| reg_text(name, reg(offset))).collect();
        logger::error(&format!("  {}", parts.join(" ")));
    };
    line(&[
        ("rax", 0x78),
        ("rcx", 0x80),
        ("rdx", 0x88),
        ("rbx", 0x90),
        ("rsp", CONTEXT_RSP),
        ("rbp", 0xA0),
        ("rsi", 0xA8),
        ("rdi", 0xB0),
    ]);
    line(&[
        ("r8", 0xB8),
        ("r9", 0xC0),
        ("r10", 0xC8),
        ("r11", 0xD0),
        ("r12", 0xD8),
        ("r13", 0xE0),
        ("r14", 0xE8),
        ("r15", 0xF0),
    ]);
    let mut frames = if code == STACK_OVERFLOW { Vec::new() } else { unwind_frames(context) };
    if frames.len() < 2 {
        frames = stack_frames(rsp);
    }
    if !frames.is_empty() {
        logger::error(&format!("  stack: {}", frames.join(" < ")));
    }
    if code == STACK_OVERFLOW {
        let outer = outer_frames();
        if !outer.is_empty() {
            logger::error(&format!("  outermost: {}", outer.join(" > ")));
        }
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
    fn outer_frames_reach_the_thread_start() {
        let frames = outer_frames();
        assert!(!frames.is_empty(), "no outer frames");
        assert!(
            frames.iter().any(|f| f.starts_with("KERNEL32") || f.starts_with("ntdll")),
            "frames: {frames:?}"
        );
    }

    #[test]
    fn unwinds_the_current_thread() {
        unsafe extern "system" {
            fn RtlCaptureContext(context: *mut c_void);
        }
        let mut ctx = Context([0; CONTEXT_SIZE]);
        // SAFETY: `ctx` is a writable, CONTEXT-sized buffer.
        unsafe { RtlCaptureContext(ctx.0.as_mut_ptr().cast()) };
        let frames = unwind_frames(ctx.0.as_ptr());
        assert!(frames.len() >= 3, "frames: {frames:?}");
        assert!(frames[0].contains('+'), "first frame has module+RVA: {frames:?}");
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
