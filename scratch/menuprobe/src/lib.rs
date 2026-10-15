#![allow(non_snake_case)] // the crate is named "MenuProbe" to control the DLL's filename

//! Throwaway PROBE (not a mod, never published): logs how the game's menu list
//! controls behave, to decide how to build a scrolling menu page. It changes
//! nothing: every hook only logs, then runs the original instructions.
//! Research and the questions this answers: `docs/ingame_menu_newpage.md`,
//! how to run it: `scratch/menuprobe/README.md`.
//!
//! Game build 2.7.1.0 only (hard-coded RVAs, every site is checked byte for
//! byte first and skipped on a mismatch).

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use common::{codepatch, dll_dir, logger};

unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut u8;
}

/// One function entry to hook: the bytes that must be there, and the Rust
/// function called (with the original rcx, rdx, r8, r9) before they run.
struct Site {
    name: &'static str,
    rva: usize,
    /// Whole instructions at the entry, position independent, at least 5 bytes.
    expect: &'static [u8],
    log: usize,
}

fn sites() -> Vec<Site> {
    vec![
        // CS::GridControl vtable slot 2: runs every frame for every grid.
        Site { name: "grid_update", rva: 0x73A140, expect: &[0x40, 0x55, 0x53, 0x56, 0x57], log: grid_update as *const () as usize },
        // Selection moved to an item (arrows, mouse, wheel).
        Site { name: "grid_select", rva: 0x73C240, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10], log: grid_select as *const () as usize },
        // Calls the grid's "refresh the visible rows" callback.
        Site {
            name: "grid_refresh",
            rva: 0x73D0D0,
            expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x60],
            log: grid_refresh as *const () as usize,
        },
        // Key config screen: draws one item into a row position.
        Site { name: "keycfg_draw", rva: 0x8684D0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x57], log: keycfg_draw as *const () as usize },
        // Options pages: adds one property (row), 16 at most.
        Site {
            name: "prop_push",
            rva: 0x928630,
            expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30],
            log: prop_push as *const () as usize,
        },
    ]
}

/// Code run at the function entry: saves every register the callee may change,
/// calls `log` (a Win64 function taking the original rcx, rdx, r8, r9), and
/// restores them. The original instructions are appended after it.
fn stub_prefix(log: usize) -> Vec<u8> {
    // push rax, rcx, rdx, r8, r9, r10, r11: 56 bytes, so with the 0x80 below
    // rsp is 16-aligned at the call (it is 8 mod 16 at a function entry).
    let mut code = vec![0x50, 0x51, 0x52, 0x41, 0x50, 0x41, 0x51, 0x41, 0x52, 0x41, 0x53];
    code.extend([0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00]); // sub rsp, 0x80 (0x20 shadow + xmm0-5)
    for i in 0..6u8 {
        code.extend([0x0F, 0x11, 0x44 | (i << 3), 0x24, 0x20 + 0x10 * i]); // movups [rsp+0x20+16i], xmm_i
    }
    code.extend([0x48, 0xB8]); // mov rax, log
    code.extend(log.to_le_bytes());
    code.extend([0xFF, 0xD0]); // call rax
    for i in 0..6u8 {
        code.extend([0x0F, 0x10, 0x44 | (i << 3), 0x24, 0x20 + 0x10 * i]); // movups xmm_i, [rsp+0x20+16i]
    }
    code.extend([0x48, 0x81, 0xC4, 0x80, 0x00, 0x00, 0x00]); // add rsp, 0x80
    code.extend([0x41, 0x5B, 0x41, 0x5A, 0x41, 0x59, 0x41, 0x58, 0x5A, 0x59, 0x58]); // pop r11..rax
    code
}

/// # Safety
/// `base + site.rva` must be readable.
unsafe fn bytes_match(base: usize, site: &Site) -> bool {
    let at = (base + site.rva) as *const u8;
    site.expect.iter().enumerate().all(|(i, b)| unsafe { at.add(i).read() } == *b)
}

// `install_jmp_hook` is a safe fn until the codepatch "unsafe" change lands, an
// unsafe fn after it; the block must compile (without a warning) on both.
#[allow(unused_unsafe)]
fn install(base: usize, site: &Site) -> bool {
    let mut body = stub_prefix(site.log);
    body.extend_from_slice(site.expect);
    let target = (base + site.rva) as *mut u8;
    match unsafe { codepatch::install_jmp_hook(target, site.expect.len(), &body) } {
        Some(stub) => {
            logger::log(&format!("hooked {} at exe+0x{:X} (stub {:p})", site.name, site.rva, stub));
            true
        }
        None => {
            logger::log(&format!("could not hook {} at exe+0x{:X}", site.name, site.rva));
            false
        }
    }
}

fn run() {
    let base = unsafe { GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(base as u64, Ordering::Relaxed);
    logger::log(&format!("MenuProbe: exe base 0x{base:X}. Open the Options pages and the key config screen, then send this log."));

    // The exe's code is unpacked a little after start: wait until the first
    // site has the expected bytes (at most a minute), then hook what matches.
    let list = sites();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !unsafe { bytes_match(base, &list[0]) } {
        if Instant::now() >= deadline {
            logger::log("MenuProbe: the first site never matched (another game build?), nothing hooked.");
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    for site in &list {
        if unsafe { bytes_match(base, site) } {
            install(base, site);
        } else {
            logger::log(&format!("skipped {}: bytes at exe+0x{:X} differ (already hooked, or another build)", site.name, site.rva));
        }
    }
}

/// # Safety
/// Called by the library loader; do not call it yourself.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: u64, reason: u32) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason != DLL_PROCESS_ATTACH {
        return true;
    }
    std::thread::spawn(move || {
        let dir = dll_dir(hmodule);
        logger::init(&dir, "MenuProbe.log");
        logger::set_enabled(true);
        logger::install_panic_hook();
        run();
    });
    true
}

// ---------------------------------------------------------------------------
// What the hooks log. They run on game threads: keep them short, never panic.
// ---------------------------------------------------------------------------

static BASE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Reads a `T` at `base + offset`; `None` for an obviously bad pointer.
fn read<T: Copy>(base: usize, offset: usize) -> Option<T> {
    (base > 0x10000).then(|| unsafe { ((base + offset) as *const T).read_unaligned() })
}

fn rva(address: usize) -> usize {
    address.wrapping_sub(BASE.load(Ordering::Relaxed) as usize)
}

/// A grid's counters: item count (+208), selected index (+212), first visible
/// row (+840). Offsets from the decompile of `GridControl`, unverified.
fn grid_state(grid: usize) -> [i32; 3] {
    [read(grid, 208).unwrap_or(-1), read(grid, 212).unwrap_or(-1), read(grid, 840).unwrap_or(-1)]
}

fn describe(grid: usize) -> String {
    let vtable: usize = read(grid, 0).unwrap_or(0);
    let state = grid_state(grid);
    // Slots found in the movie (+216 rows, +220 columns, unverified) and the
    // pointers that say whether the grid has a row-refresh callback (+1816) and
    // a "can select" check (+1752).
    let rows: i32 = read(grid, 216).unwrap_or(-1);
    let cols: i32 = read(grid, 220).unwrap_or(-1);
    let refresh: usize = read(grid, 1816).unwrap_or(0);
    let can_select: usize = read(grid, 1752).unwrap_or(0);
    format!(
        "grid {grid:#X} vtable exe+0x{:X} count={} index={} scroll={} slots={rows}x{cols} refresh_cb={} select_check={}",
        rva(vtable),
        state[0],
        state[1],
        state[2],
        if refresh != 0 { "set" } else { "null" },
        if can_select != 0 { "set" } else { "null" },
    )
}

/// Logs the first `first` calls, then every `every`-th.
fn sampled(counter: &AtomicU32, first: u32, every: u32) -> Option<u32> {
    let n = counter.fetch_add(1, Ordering::Relaxed);
    (n < first || n.is_multiple_of(every)).then_some(n)
}

static GRIDS: Mutex<Vec<(usize, [i32; 3])>> = Mutex::new(Vec::new());

extern "C" fn grid_update(this: usize, _b: usize, _c: usize, _d: usize) {
    let _ = std::panic::catch_unwind(|| {
        let state = grid_state(this);
        let change = {
            let mut grids = GRIDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some((_, last)) = grids.iter_mut().find(|(grid, _)| *grid == this) {
                if *last == state {
                    None
                } else {
                    *last = state;
                    Some("changed")
                }
            } else if grids.len() < 64 {
                grids.push((this, state));
                Some("new")
            } else {
                None
            }
        };
        if let Some(what) = change {
            logger::log(&format!("grid_update {what}: {}", describe(this)));
        }
    });
}

static SELECT: AtomicU32 = AtomicU32::new(0);

extern "C" fn grid_select(this: usize, index: usize, r8: usize, r9: usize) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(n) = sampled(&SELECT, 200, 20) {
            logger::log(&format!(
                "grid_select #{n}: index={} r8={} r9={} | {}",
                index as i32,
                r8 as i32,
                r9 as u8,
                describe(this)
            ));
        }
    });
}

static REFRESH: AtomicU32 = AtomicU32::new(0);

extern "C" fn grid_refresh(this: usize, _b: usize, _c: usize, _d: usize) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(n) = sampled(&REFRESH, 60, 300) {
            logger::log(&format!("grid_refresh #{n}: {}", describe(this)));
        }
    });
}

static DRAW: AtomicU32 = AtomicU32::new(0);

extern "C" fn keycfg_draw(item: usize, row: usize, _c: usize, _d: usize) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(n) = sampled(&DRAW, 40, 200) {
            let vtable: usize = read(item, 0).unwrap_or(0);
            logger::log(&format!("keycfg_draw #{n}: item={item:#X} (vtable exe+0x{:X}) row={row:#X}", rva(vtable)));
        }
    });
}

static PUSH: AtomicU32 = AtomicU32::new(0);

extern "C" fn prop_push(page: usize, prop: usize, _c: usize, _d: usize) {
    let _ = std::panic::catch_unwind(|| {
        // The page's property count before this one is added (+0x888).
        let count: u64 = read(page, 0x888).unwrap_or(u64::MAX);
        let n = PUSH.fetch_add(1, Ordering::Relaxed);
        if n < 120 || count >= 12 {
            let vtable: usize = read(page, 0).unwrap_or(0);
            logger::log(&format!(
                "prop_push #{n}: page={page:#X} (vtable exe+0x{:X}) count_before={count} prop={prop:#X}{}",
                rva(vtable),
                if count >= 15 { "  <- 16th property: the next one fails" } else { "" }
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_dump() {
        let hex: String = stub_prefix(0x1122334455667788).iter().map(|b| format!("{b:02x}")).collect();
        println!("STUB {hex}");
        assert_eq!(stub_prefix(0).len(), 11 + 7 + 6 * 5 + 10 + 2 + 6 * 5 + 7 + 11);
    }
}
