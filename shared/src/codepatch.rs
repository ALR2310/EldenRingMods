//! Byte patching, and the pieces a hook needs to jump to a stub anywhere in the address space.
//! Keep stubs in [alloc_executable] memory, never in a `static` made executable: antivirus engines
//! flag a DLL that jumps into its own data section.

use std::ffi::c_void;

unsafe extern "system" {
    fn VirtualAlloc(
        lp_address: *mut c_void,
        dw_size: usize,
        fl_allocation_type: u32,
        fl_protect: u32,
    ) -> *mut c_void;
    fn VirtualProtect(
        lp_address: *mut c_void,
        dw_size: usize,
        fl_new_protect: u32,
        lpfl_old_protect: *mut u32,
    ) -> i32;
    fn VirtualFree(lp_address: *mut c_void, dw_size: usize, dw_free_type: u32) -> i32;
    fn FlushInstructionCache(
        h_process: *mut c_void,
        lp_base_address: *const c_void,
        dw_size: usize,
    ) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
}

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;

// A rel32 displacement is a signed 32-bit int; stay well inside it so a stub is never barely in range.
const MAX_REL32_RANGE: isize = 0x7000_0000;
const SEARCH_STEP: usize = 0x1_0000;

pub const ABSOLUTE_JMP_LEN: usize = 12;

/// `mov rax, target; jmp rax`: a jump that reaches any 64-bit address (unlike `E9 rel32`).
/// Clobbers `rax`.
pub fn absolute_jmp(target: u64) -> [u8; ABSOLUTE_JMP_LEN] {
    let mut code = [0u8; ABSOLUTE_JMP_LEN];
    code[..2].copy_from_slice(&[0x48, 0xB8]);
    code[2..10].copy_from_slice(&target.to_le_bytes());
    code[10..].copy_from_slice(&[0xFF, 0xE0]);
    code
}

/// Readable, writable, executable memory for a stub reached by an [absolute_jmp] or trampoline.
pub fn alloc_executable(len: usize) -> Option<*mut u8> {
    // SAFETY: a null address lets the OS pick fresh memory; nothing existing is touched.
    let mem = unsafe {
        VirtualAlloc(std::ptr::null_mut(), len, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    };
    (!mem.is_null()).then_some(mem as *mut u8)
}

/// Overwrites bytes in place and flushes the instruction cache. `false`, with the original bytes
/// untouched, if `VirtualProtect` fails.
///
/// # Safety
/// `addr..addr + patched.len()` must be mapped code of this process, and `patched` must be valid
/// machine code for the bytes it replaces, since other threads may be running them.
pub unsafe fn overwrite_bytes(addr: *mut u8, patched: &[u8]) -> bool {
    let len = patched.len();
    let mut old_protect: u32 = 0;
    // SAFETY: the range is mapped code, by this function's contract.
    unsafe {
        if VirtualProtect(addr as *mut c_void, len, PAGE_EXECUTE_READWRITE, &mut old_protect) == 0 {
            return false;
        }
        std::ptr::copy_nonoverlapping(patched.as_ptr(), addr, len);
        let mut unused: u32 = 0;
        VirtualProtect(addr as *mut c_void, len, old_protect, &mut unused);
        FlushInstructionCache(GetCurrentProcess(), addr as *const c_void, len);
    }
    true
}

/// Executable memory within rel32 reach of `anchor`, searched backward first, then forward.
/// `VirtualAlloc` at an explicit address never relocates, so a hit is always in range.
fn alloc_near(anchor: *mut u8, size: usize) -> Option<*mut u8> {
    let anchor_addr = anchor as isize;
    let try_at = |addr: isize| {
        // SAFETY: an explicit address either yields fresh memory there or fails; nothing existing is touched.
        let mem = unsafe {
            VirtualAlloc(
                addr as *mut c_void,
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };
        (!mem.is_null()).then_some(mem as *mut u8)
    };
    let offsets = || (0..MAX_REL32_RANGE).step_by(SEARCH_STEP);
    for offset in offsets() {
        let addr = anchor_addr - offset;
        if addr <= 0x10000 {
            break;
        }
        if let Some(mem) = try_at(addr) {
            return Some(mem);
        }
    }
    offsets().find_map(|offset| try_at(anchor_addr + offset))
}

/// Destination of the `call rel32` (`E8`) or `jmp rel32` (`E9`) at `site`, `None` if it is neither.
///
/// # Safety
/// `site..site + 5` must be readable memory of this process.
pub unsafe fn rel32_target(site: *const u8) -> Option<u64> {
    // SAFETY: readable, by this function's contract.
    let (opcode, rel) = unsafe { (site.read(), (site.add(1) as *const i32).read_unaligned()) };
    if opcode != 0xE8 && opcode != 0xE9 {
        return None;
    }
    Some((site as u64 + 5).wrapping_add_signed(rel as i64))
}

/// Points the `call rel32` / `jmp rel32` at `site` to a new stub holding `stub_body`, allocated near
/// `site`. Only the 4 displacement bytes change, so mods that match the opcode and the bytes around
/// it still find the site. The stub is entered as the original destination would be and must end
/// with its own `ret` or tail jump. Returns the stub's address; on failure the site is untouched.
///
/// # Safety
/// `site` must be a mapped `call rel32` / `jmp rel32` of this process, and `stub_body` valid machine
/// code that stands in for its destination.
pub unsafe fn redirect_rel32(site: *mut u8, stub_body: &[u8]) -> Option<*mut u8> {
    // SAFETY: `site` is mapped code, by this function's contract.
    unsafe { rel32_target(site) }?;
    let stub = alloc_near(site, stub_body.len())?;
    // SAFETY: `stub` is a fresh allocation of `stub_body.len()` bytes; the displacement lies inside `site`'s instruction.
    unsafe {
        std::ptr::copy_nonoverlapping(stub_body.as_ptr(), stub, stub_body.len());
        let rel = (stub as isize - site as isize - 5) as i32;
        if !overwrite_bytes(site.add(1), &rel.to_le_bytes()) {
            VirtualFree(stub as *mut c_void, 0, MEM_RELEASE);
            return None;
        }
        FlushInstructionCache(GetCurrentProcess(), stub as *const c_void, stub_body.len());
    }
    Some(stub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_jmp_encodes_mov_rax_imm64_jmp_rax() {
        let code = absolute_jmp(0x1122_3344_5566_7788);
        assert_eq!(code[..2], [0x48, 0xB8]);
        assert_eq!(code[2..10], 0x1122_3344_5566_7788u64.to_le_bytes());
        assert_eq!(code[10..], [0xFF, 0xE0]);
    }

    #[test]
    fn alloc_executable_returns_usable_memory() {
        let mem = alloc_executable(64).expect("alloc");
        // SAFETY: `mem` is a fresh 64-byte allocation owned by this test.
        unsafe { mem.write(0xC3) };
        // SAFETY: as above.
        assert_eq!(unsafe { mem.read() }, 0xC3);
    }

    #[test]
    fn rel32_target_reads_calls_and_jumps() {
        let call = [0xE8, 0x10, 0, 0, 0];
        // SAFETY: `call` is readable memory owned by this test.
        let target = unsafe { rel32_target(call.as_ptr()) };
        assert_eq!(target, Some(call.as_ptr() as u64 + 5 + 0x10));
        let other = [0x90u8; 5];
        // SAFETY: as above.
        assert_eq!(unsafe { rel32_target(other.as_ptr()) }, None);
    }

    #[test]
    fn overwrite_bytes_patches_a_buffer() {
        let mut buffer = [1u8, 2, 3, 4];
        // SAFETY: `buffer` is mapped memory owned by this test.
        let patched = unsafe { overwrite_bytes(buffer.as_mut_ptr().add(1), &[9, 9]) };
        assert!(patched);
        assert_eq!(buffer, [1, 9, 9, 4]);
    }
}
