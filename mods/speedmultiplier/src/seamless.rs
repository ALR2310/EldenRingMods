//! Seamless Co-op retargets the thunk the game reads `animation_speed` through, so our writes are
//! never read. When the thunk stops jumping into the game's code, it is pointed at a stub that
//! returns the field for the player and Torrent. Details: `docs/animation_speed.md`.

use std::sync::atomic::{AtomicU64, Ordering};

use shared::{codepatch, logger, memscan};

/// `lea rcx,[rdi+17B0h]; call <thunk>; ...` in the behavior update; the call is at +7.
const GETTER_CALL_AOB: &str =
    "48 8D 8F B0 17 00 00 E8 ?? ?? ?? ?? 0F 28 F0 F3 0F 59 B7 C0 15 00 00";
const CALL_OFFSET: usize = 7;

/// `ChrIns` of the player and Torrent, set every frame by `speed.rs`; the stub compares against them.
pub static OWN_CHRS: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];

pub fn set_own(player: usize, torrent: usize) {
    OWN_CHRS[0].store(player as u64, Ordering::Relaxed);
    OWN_CHRS[1].store(torrent as u64, Ordering::Relaxed);
}

/// rcx = behavior + 0x17B0, result in xmm0; `fallback` is Seamless's code:
/// ```text
/// mov  rax, [rcx-0x17A8]      ; behavior->owner (behavior + 0x8)
/// mov  r10, &OWN_CHRS
/// cmp  rax, [r10]   ; je own  ; the player
/// cmp  rax, [r10+8] ; je own  ; Torrent
/// mov  r10, fallback          ; anyone else: the hook we replaced
/// jmp  r10
/// own: movss xmm0, [rcx+0x18] ; animation_speed
/// ret
/// ```
fn stub(fallback: u64) -> Vec<u8> {
    let mut s = Vec::new();
    s.extend_from_slice(&[0x48, 0x8B, 0x81, 0x58, 0xE8, 0xFF, 0xFF]);
    s.extend_from_slice(&[0x49, 0xBA]);
    s.extend_from_slice(&(OWN_CHRS.as_ptr() as u64).to_le_bytes());
    s.extend_from_slice(&[0x49, 0x3B, 0x02, 0x74, 0x00]);
    let je1 = s.len() - 1;
    s.extend_from_slice(&[0x49, 0x3B, 0x42, 0x08, 0x74, 0x00]);
    let je2 = s.len() - 1;
    s.extend_from_slice(&[0x49, 0xBA]);
    s.extend_from_slice(&fallback.to_le_bytes());
    s.extend_from_slice(&[0x41, 0xFF, 0xE2]);
    let own = s.len();
    s.extend_from_slice(&[0xF3, 0x0F, 0x10, 0x41, 0x18, 0xC3]);
    s[je1] = (own - (je1 + 1)) as u8;
    s[je2] = (own - (je2 + 1)) as u8;
    s
}

pub struct GetterFix {
    thunk: Option<*mut u8>,
    last_target: u64,
    stub: u64,
    logged_scan_failure: bool,
}

// SAFETY: the thunk pointer is only used from the task that owns this value.
unsafe impl Send for GetterFix {}

impl GetterFix {
    pub const fn new() -> Self {
        Self { thunk: None, last_target: 0, stub: 0, logged_scan_failure: false }
    }

    pub fn check(&mut self) {
        let thunk = match self.thunk {
            Some(t) => t,
            None => {
                let Some(site) = memscan::find_pattern_in_module(GETTER_CALL_AOB) else {
                    if !std::mem::replace(&mut self.logged_scan_failure, true) {
                        logger::warn("Seamless: behavior update not found");
                    }
                    self.thunk = Some(std::ptr::null_mut());
                    return;
                };
                // SAFETY: the call is inside the matched pattern, in the game image.
                let call = unsafe { site.add(CALL_OFFSET) };
                // SAFETY: `call` is in the game image.
                let Some(thunk) = unsafe { codepatch::rel32_target(call) }.map(|a| a as *mut u8)
                else {
                    return;
                };
                // SAFETY: the thunk was just read by `rel32_target`.
                if unsafe { *thunk } != 0xE9 {
                    logger::warn("Seamless: thunk is not a jmp, left alone");
                    self.thunk = Some(std::ptr::null_mut());
                    return;
                }
                self.thunk = Some(thunk);
                thunk
            }
        };
        if thunk.is_null() {
            return;
        }
        // SAFETY: `thunk` is a jmp in the game image.
        let Some(target) = (unsafe { codepatch::rel32_target(thunk) }) else {
            return;
        };
        if target == self.last_target || target == self.stub {
            return;
        }
        self.last_target = target;
        // Inside the game's image it is the original getter.
        let owner = shared::diag::module_containing(target as usize);
        if owner.as_ref().is_some_and(|m| m.name.eq_ignore_ascii_case("eldenring.exe")) {
            return;
        }
        // The hook's code usually sits in memory its DLL allocated, which belongs to no module.
        let seamless_loaded = shared::diag::is_module_loaded("ersc.dll");
        let who = match owner {
            Some(m) => m.name,
            None if seamless_loaded => "Seamless Co-op".to_string(),
            None => "Another mod".to_string(),
        };
        // SAFETY: `thunk` is a jmp in the game image; the stub ends with a jump to the old target.
        match unsafe { codepatch::redirect_rel32(thunk, &stub(target)) } {
            Some(stub) => {
                self.stub = stub as u64;
                logger::log(&format!("Seamless: {who} hooks the animation speed, bypassing it"));
            }
            None => logger::error(&format!(
                "Seamless: {who} hooks the animation speed, could not bypass it"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stub;

    #[test]
    fn stub_jumps_land_on_the_own_branch() {
        let s = stub(0x1122334455667788);
        let own = s.len() - 6;
        assert_eq!(&s[own..], &[0xF3, 0x0F, 0x10, 0x41, 0x18, 0xC3]);
        // The OWN_CHRS address can itself contain a 0x74 byte, so find the first `je` after its `cmp`.
        let first = s.windows(4).position(|w| w == [0x49, 0x3B, 0x02, 0x74]).unwrap() + 3;
        assert_eq!(first + 2 + s[first + 1] as usize, own);
        let second = s.windows(4).position(|w| w == [0x49, 0x3B, 0x42, 0x08]).unwrap() + 4;
        assert_eq!(s[second], 0x74);
        assert_eq!(second + 2 + s[second + 1] as usize, own);
        assert!(s.windows(8).any(|w| w == 0x1122334455667788u64.to_le_bytes()));
    }
}
