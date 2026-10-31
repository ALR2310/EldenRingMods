//! Calls the game's `ChrIns::ApplySpEffect` without `eldenring::rva::get()`,
//! whose RVA table panics on 2.7.2.0 (AGENTS.md): the function is found by AOB
//! over its prologue (unique on 2.6.2.0, 2.7.0.0, 2.7.1.0, 2.7.2.0, checked with
//! `scripts/ida/run.ps1 aob -Version all`). Signature copied from
//! `ChrInsExt::apply_speffect`: `(chr_ins, speffect_id, dont_sync) -> u64`.

use std::sync::LazyLock;

use common::{logger, memscan};

const APPLY_SPEFFECT_PATTERN: &str = "48 8B C4 48 89 58 08 48 89 70 10 57 48 81 EC C0 00 00 00 0F 28 05 ?? ?? ?? ?? 48 8B F1 \
     0F 28 0D ?? ?? ?? ?? 48 8D 48 88 0F 29 40 A8 41 0F B6 D8";

type ApplySpEffect = unsafe extern "C" fn(*mut u8, i32, bool) -> u64;

// Resolved once; a miss is logged once and stays a miss.
static APPLY: LazyLock<Option<ApplySpEffect>> = LazyLock::new(|| {
    let addr = memscan::find_pattern_in_module(APPLY_SPEFFECT_PATTERN);
    match addr {
        Some(p) => {
            logger::log(&format!("ApplySpEffect found at {p:p}."));
            Some(unsafe { std::mem::transmute::<*mut u8, ApplySpEffect>(p) })
        }
        None => {
            logger::error("ApplySpEffect pattern not found, SpEffect features off.");
            None
        }
    }
});

/// Applies `speffect_id` to the character at `chr_ins`. Returns `false` if the
/// function could not be resolved.
///
/// # Safety
/// `chr_ins` must point to a live `ChrIns`, and the call must run on the game's
/// own frame task (not an arbitrary thread).
pub unsafe fn apply(chr_ins: *mut u8, speffect_id: i32, dont_sync: bool) -> bool {
    match *APPLY {
        Some(f) => {
            unsafe { f(chr_ins, speffect_id, dont_sync) };
            true
        }
        None => false,
    }
}
