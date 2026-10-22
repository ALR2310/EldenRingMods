//! Code patching helpers: in-place byte overwrite, a 5-byte `E9 rel32` inline
//! hook with a caller-supplied stub, and redirecting an existing `call`/`jmp`
//! rel32. Which one to use, and why: `shared/docs/codepatch.md`.

use std::ffi::c_void;

unsafe extern "system" {
    fn VirtualAlloc(lp_address: *mut c_void, dw_size: usize, fl_allocation_type: u32, fl_protect: u32) -> *mut c_void;
    fn VirtualFree(lp_address: *mut c_void, dw_size: usize, dw_free_type: u32) -> i32;
    fn VirtualProtect(lp_address: *mut c_void, dw_size: usize, fl_new_protect: u32, lpfl_old_protect: *mut u32) -> i32;
    fn FlushInstructionCache(h_process: *mut c_void, lp_base_address: *const c_void, dw_size: usize) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
}

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;

// A rel32 displacement is a signed 32-bit int. Search only this far from the
// target (well inside 0x7FFFFFFF) so a stub is never barely in range.
const MAX_REL32_RANGE: isize = 0x7000_0000;
const SEARCH_STEP: usize = 0x1_0000;

/// Allocates `size` bytes of executable memory within rel32 reach of
/// `anchor`, searching backward first, then forward. `VirtualAlloc` at an
/// explicit address never relocates, so a hit is always in range.
fn alloc_near(anchor: *mut u8, size: usize) -> Option<*mut u8> {
    let anchor_addr = anchor as isize;
    let try_at = |addr: isize| {
        let mem = unsafe { VirtualAlloc(addr as *mut c_void, size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) };
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

/// Makes `len` bytes at `addr` writable, runs `write`, then restores the
/// protection and flushes the instruction cache. `false` (nothing written)
/// if `VirtualProtect` fails.
unsafe fn with_writable_code(addr: *mut u8, len: usize, write: impl FnOnce()) -> bool {
    let mut old_protect: u32 = 0;
    unsafe {
        if VirtualProtect(addr as *mut c_void, len, PAGE_EXECUTE_READWRITE, &mut old_protect) == 0 {
            return false;
        }
        write();
        let mut unused: u32 = 0;
        VirtualProtect(addr as *mut c_void, len, old_protect, &mut unused);
        FlushInstructionCache(GetCurrentProcess(), addr as *const c_void, len);
    }
    true
}

/// Writes `E9 rel32` at `at`, jumping to `to`. `at` must be writable.
unsafe fn write_jmp_rel32(at: *mut u8, to: *const u8) {
    let rel = (to as isize - at as isize - 5) as i32;
    unsafe {
        *at = 0xE9;
        at.add(1).copy_from_nonoverlapping(rel.to_le_bytes().as_ptr(), 4);
    }
}

/// Length of [absolute_jmp].
pub const ABSOLUTE_JMP_LEN: usize = 12;

/// `mov rax, target; jmp rax`: a jump that reaches any 64-bit address, so
/// the destination can be anywhere (unlike `E9 rel32`). Clobbers `rax`.
pub fn absolute_jmp(target: u64) -> [u8; ABSOLUTE_JMP_LEN] {
    let mut code = [0u8; ABSOLUTE_JMP_LEN];
    code[..2].copy_from_slice(&[0x48, 0xB8]);
    code[2..10].copy_from_slice(&target.to_le_bytes());
    code[10..].copy_from_slice(&[0xFF, 0xE0]);
    code
}

/// Allocates `len` bytes of readable, writable, executable memory anywhere in
/// the address space, for a stub reached by an [absolute_jmp].
pub fn alloc_executable(len: usize) -> Option<*mut u8> {
    let mem = unsafe { VirtualAlloc(std::ptr::null_mut(), len, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) };
    (!mem.is_null()).then_some(mem as *mut u8)
}

/// Marks `len` bytes at `addr` readable, writable and executable for good, for
/// code kept in a data buffer. Returns `false` if `VirtualProtect` fails.
///
/// # Safety
/// `addr..addr + len` must be mapped memory of this process.
pub unsafe fn make_executable(addr: *mut u8, len: usize) -> bool {
    let mut old_protect: u32 = 0;
    unsafe { VirtualProtect(addr as *mut c_void, len, PAGE_EXECUTE_READWRITE, &mut old_protect) != 0 }
}

/// Overwrites `patched.len()` bytes at `addr` in place, for replacements that
/// fit the original byte window exactly (no stub, unlike [install_jmp_hook]).
/// Returns `false`, leaving the original bytes untouched, if `VirtualProtect`
/// fails.
///
/// # Safety
/// `addr..addr + patched.len()` must be mapped code of this process, and
/// `patched` must be valid machine code for the bytes it replaces, since
/// other threads may be running them.
pub unsafe fn overwrite_bytes(addr: *mut u8, patched: &[u8]) -> bool {
    unsafe {
        with_writable_code(addr, patched.len(), || {
            std::ptr::copy_nonoverlapping(patched.as_ptr(), addr, patched.len());
        })
    }
}

/// Allocates a stub near `target_addr`, copies `stub_body` into it, appends a
/// JMP back to `target_addr + original_len`, then overwrites `original_len`
/// bytes at `target_addr` with a JMP into the stub (NOP-padded past 5 bytes).
///
/// `stub_body` must not itself jump/return - the return jump is appended.
/// `original_len` must be >= 5. Returns the stub's address; on any failure the
/// original bytes are left untouched.
///
/// # Safety
/// `target_addr..target_addr + original_len` must be mapped code of this
/// process that ends on an instruction boundary, and `stub_body` must be valid
/// machine code that does what the replaced instructions did.
pub unsafe fn install_jmp_hook(target_addr: *mut u8, original_len: usize, stub_body: &[u8]) -> Option<*mut u8> {
    if original_len < 5 {
        return None;
    }

    let stub_size = stub_body.len() + 5; // +5 for the JMP back
    let stub = alloc_near(target_addr, stub_size)?;

    unsafe {
        std::ptr::copy_nonoverlapping(stub_body.as_ptr(), stub, stub_body.len());
        write_jmp_rel32(stub.add(stub_body.len()), target_addr.add(original_len));

        let hooked = with_writable_code(target_addr, original_len, || {
            std::ptr::write_bytes(target_addr, 0x90, original_len);
            write_jmp_rel32(target_addr, stub);
        });
        if !hooked {
            VirtualFree(stub as *mut c_void, 0, MEM_RELEASE);
            return None;
        }
        FlushInstructionCache(GetCurrentProcess(), stub as *const c_void, stub_size);
    }

    Some(stub)
}

/// Destination of the 5-byte `call rel32` (`E8`) or `jmp rel32` (`E9`) at
/// `site`, or `None` if `site` holds neither.
///
/// # Safety
/// `site..site + 5` must be readable memory of this process.
pub unsafe fn rel32_target(site: *const u8) -> Option<u64> {
    let opcode = unsafe { site.read() };
    if opcode != 0xE8 && opcode != 0xE9 {
        return None;
    }
    let rel = unsafe { (site.add(1) as *const i32).read_unaligned() };
    Some((site as u64 + 5).wrapping_add_signed(rel as i64))
}

/// Points the existing `call rel32` / `jmp rel32` at `site` to a new stub
/// holding `stub_body`, allocated near `site`. Only the 4 displacement bytes
/// change. Prefer this over [install_jmp_hook] when the site is a call (other
/// mods match the opcode and surrounding bytes, not the displacement), and
/// chain with [rel32_target] if another mod already redirected it.
///
/// `stub_body` is entered exactly as the original destination would be, so it
/// must end with its own `ret` / tail jump; nothing is appended. Returns the
/// stub's address; on any failure the original bytes are left untouched.
///
/// # Safety
/// `site` must be a mapped `call rel32` / `jmp rel32` of this process, and
/// `stub_body` must be valid machine code that stands in for its destination.
pub unsafe fn redirect_rel32(site: *mut u8, stub_body: &[u8]) -> Option<*mut u8> {
    unsafe { rel32_target(site) }?;
    let stub = alloc_near(site, stub_body.len())?;
    unsafe {
        std::ptr::copy_nonoverlapping(stub_body.as_ptr(), stub, stub_body.len());
        let rel = (stub as isize - site as isize - 5) as i32;
        let displacement = site.add(1);
        if !with_writable_code(displacement, 4, || (displacement as *mut i32).write_unaligned(rel)) {
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
        unsafe { mem.write(0xC3) };
        assert_eq!(unsafe { mem.read() }, 0xC3);
    }
}
