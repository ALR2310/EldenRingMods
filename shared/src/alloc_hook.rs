//! Resolves the game's global runtime heap allocator by AOB instead of `eldenring::rva::get()`,
//! which panics on game versions `fromsoftware-rs` has no table for.

use eldenring::dlkr::DLAllocator;

use crate::logger;
use crate::memscan::{self, CachedAddr};

// Address of the global variable holding the allocator pointer, not its value: the game may not
// have constructed the allocator yet, so a null instance must be re-read on the next call
// instead of cached.
static GLOBAL_PTR_ADDR: CachedAddr = CachedAddr::new();

// The small "read or lazily construct" getter of the allocator global. The `mov rax,[rip+X]` at
// `LOAD_INSTR_OFFSET` loads it; the rest of the pattern only anchors the match to this function.
const GETTER_PATTERN: &str = "48 83 ec 28 e8 ?? ?? ?? ?? 48 8b 05 ?? ?? ?? ?? 48 85 c0 75 0c e8 ?? ?? ?? ?? 48 89 05 ?? ?? ?? ?? 48 89 05 ?? ?? ?? ?? 48 83 c4 28 c3";
const LOAD_INSTR_OFFSET: usize = 9;

/// Resolves `DLAllocator::runtime_heap_allocator()` via AOB. Returns `None` (logged) if the
/// pattern isn't found or the instance is still null.
pub fn runtime_heap_allocator() -> Option<&'static DLAllocator> {
    let global_ptr_addr = GLOBAL_PTR_ADDR.get_or_resolve(|| {
        let Some(matched) = memscan::find_pattern_in_module(GETTER_PATTERN) else {
            logger::log("AllocHook: getter pattern not found");
            return None;
        };
        logger::log("AllocHook: getter found");
        // SAFETY: the pattern guarantees a `mov rax,[rip+disp32]` at `LOAD_INSTR_OFFSET`, so the
        // four displacement bytes after its 3-byte opcode lie inside the matched code.
        unsafe {
            let load_instr = matched.add(LOAD_INSTR_OFFSET);
            // 3-byte opcode, disp32, rip-relative from the end of the 7-byte instruction.
            let disp32 = i32::from_le_bytes(*(load_instr.add(3) as *const [u8; 4]));
            Some(load_instr.add(7).offset(disp32 as isize) as usize)
        }
    })? as *const *const DLAllocator;

    // SAFETY: `global_ptr_addr` is the address of a pointer-sized global in the game's data; once
    // the game has set it, the allocator it points to lives for the rest of the process.
    unsafe {
        let instance_ptr = *global_ptr_addr;
        if instance_ptr.is_null() {
            logger::log("AllocHook: allocator resolved but its instance is still null");
            return None;
        }
        Some(&*instance_ptr)
    }
}
