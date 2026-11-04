//! Code patching helpers: an in-place byte overwrite and the pieces a hook needs to jump to a
//! stub anywhere in the address space.
//!
//! Keep hook stubs in memory from [alloc_executable], not in a `static` that is made executable
//! later: a DLL that jumps into its own data section is flagged by several antivirus engines.

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
    fn FlushInstructionCache(
        h_process: *mut c_void,
        lp_base_address: *const c_void,
        dw_size: usize,
    ) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
}

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;

/// Length of [absolute_jmp].
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

/// Allocates `len` bytes of readable, writable, executable memory anywhere in the address space,
/// for a stub reached by an [absolute_jmp] or a trampoline.
pub fn alloc_executable(len: usize) -> Option<*mut u8> {
    // SAFETY: a null address lets the OS pick fresh memory; nothing existing is touched.
    let mem = unsafe {
        VirtualAlloc(std::ptr::null_mut(), len, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    };
    (!mem.is_null()).then_some(mem as *mut u8)
}

/// Overwrites `patched.len()` bytes at `addr` in place. Returns `false`, leaving the original
/// bytes untouched, if `VirtualProtect` fails. The protection is restored and the instruction
/// cache flushed afterwards.
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
    fn overwrite_bytes_patches_a_buffer() {
        let mut buffer = [1u8, 2, 3, 4];
        // SAFETY: `buffer` is mapped memory owned by this test.
        let patched = unsafe { overwrite_bytes(buffer.as_mut_ptr().add(1), &[9, 9]) };
        assert!(patched);
        assert_eq!(buffer, [1, 9, 9, 4]);
    }
}
