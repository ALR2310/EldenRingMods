//! IDA-style AOB (array-of-bytes) scanner over the executable sections of the
//! game's main module, for hooking call sites that `fromsoftware-rs` doesn't
//! reflect. A pattern is hex bytes with `?` or `??` as a wildcard:
//! `"48 8B 05 ?? ?? ?? ?? 48 85 C0 74 07"`. Resolve cache and retry
//! rationale: `shared/docs/memscan.md`.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

unsafe extern "system" {
    fn GetModuleHandleA(lp_module_name: *const u8) -> *mut c_void;
}

#[repr(C)]
struct ImageDosHeader {
    e_magic: u16,
    _reserved: [u16; 29],
    e_lfanew: i32,
}

#[repr(C)]
struct ImageFileHeader {
    _machine: u16,
    number_of_sections: u16,
    _reserved: [u8; 12],
    size_of_optional_header: u16,
    _characteristics: u16,
}

#[repr(C)]
struct ImageSectionHeader {
    _name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    _reserved: [u8; 24],
    characteristics: u32,
}

const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;

/// `Some(byte)` must match exactly, `None` is a wildcard.
type Pattern = Vec<Option<u8>>;

/// A token that isn't `?`, `??` or hex counts as byte `00`, so a typo in a
/// pattern shows up as "not found" rather than a panic inside the game.
fn parse_pattern(pattern: &str) -> Pattern {
    pattern
        .split_whitespace()
        .map(|token| match token {
            "?" | "??" => None,
            hex => Some(u8::from_str_radix(hex, 16).unwrap_or(0)),
        })
        .collect()
}

fn find_pattern(haystack: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    haystack.windows(pattern.len()).position(|window| {
        window.iter().zip(pattern).all(|(byte, expected)| expected.is_none_or(|e| e == *byte))
    })
}

/// A process-lifetime cache for one resolved address. Code addresses in the
/// game exe never move once found, so scanning the same pattern again is
/// wasted work. Only a successful resolve is cached; a miss is retried by the
/// next caller.
pub struct CachedAddr(Mutex<Option<usize>>);

impl CachedAddr {
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }

    /// Returns the cached address, or runs `resolve` (holding the lock, so
    /// concurrent callers wait and reuse its result) and caches it if `Some`.
    /// `resolve` runs only while nothing is cached, so a "found" log inside it
    /// appears once per process.
    pub fn get_or_resolve(&self, resolve: impl FnOnce() -> Option<usize>) -> Option<usize> {
        let mut cached = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if cached.is_none() {
            *cached = resolve();
        }
        *cached
    }
}

impl Default for CachedAddr {
    fn default() -> Self {
        Self::new()
    }
}

/// Retries [find_pattern_in_module] every `retry_interval` until it matches
/// or `timeout` elapses (then `None`). For a one-shot startup scan that may
/// race the game's anti-tamper unpacking.
pub fn wait_for_pattern_in_module(pattern: &str, retry_interval: Duration, timeout: Duration) -> Option<*mut u8> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(addr) = find_pattern_in_module(pattern) {
            return Some(addr);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(retry_interval);
    }
}

/// Scans the executable sections of the main module (the game .exe) for
/// `pattern`, returning the absolute address of the first match.
pub fn find_pattern_in_module(pattern: &str) -> Option<*mut u8> {
    let bytes = parse_pattern(pattern);
    let base = unsafe { GetModuleHandleA(std::ptr::null()) } as *const u8;
    if base.is_null() {
        return None;
    }

    unsafe {
        let dos = &*(base as *const ImageDosHeader);
        let nt_base = base.add(dos.e_lfanew as usize);
        // IMAGE_NT_HEADERS64: Signature(u32), FileHeader, OptionalHeader; the
        // section table follows the optional header.
        let file_header = &*(nt_base.add(4) as *const ImageFileHeader);
        let section_table = nt_base
            .add(4 + size_of::<ImageFileHeader>() + file_header.size_of_optional_header as usize)
            as *const ImageSectionHeader;

        for i in 0..file_header.number_of_sections as usize {
            let section = &*section_table.add(i);
            if section.characteristics & IMAGE_SCN_MEM_EXECUTE == 0 {
                continue;
            }
            let section_start = base.add(section.virtual_address as usize);
            let section_slice = std::slice::from_raw_parts(section_start, section.virtual_size as usize);
            if let Some(offset) = find_pattern(section_slice, &bytes) {
                return Some(section_start.add(offset) as *mut u8);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_wildcards() {
        assert_eq!(parse_pattern("48 8B ?? ? 0f"), vec![Some(0x48), Some(0x8B), None, None, Some(0x0F)]);
        assert_eq!(parse_pattern("zz"), vec![Some(0)]); // typo: matches 00, never panics
        assert!(parse_pattern("").is_empty());
    }

    #[test]
    fn finds_first_match_with_wildcards() {
        let hay = [0u8, 1, 2, 3, 4, 1, 9, 3];
        assert_eq!(find_pattern(&hay, &parse_pattern("01 ?? 03")), Some(1));
        assert_eq!(find_pattern(&hay, &parse_pattern("03 04")), Some(3));
        assert_eq!(find_pattern(&hay, &parse_pattern("01 09")), Some(5));
    }

    #[test]
    fn matches_at_the_very_end_and_rejects_oversized_or_empty_patterns() {
        let hay = [1u8, 2, 3];
        assert_eq!(find_pattern(&hay, &parse_pattern("02 03")), Some(1));
        assert_eq!(find_pattern(&hay, &parse_pattern("01 02 03 04")), None);
        assert_eq!(find_pattern(&hay, &[]), None);
        assert_eq!(find_pattern(&hay, &parse_pattern("05")), None);
    }

    #[test]
    fn cached_addr_caches_only_a_hit() {
        let cache = CachedAddr::new();
        assert_eq!(cache.get_or_resolve(|| None), None);
        assert_eq!(cache.get_or_resolve(|| Some(7)), Some(7));
        assert_eq!(cache.get_or_resolve(|| panic!("must not resolve again")), Some(7));
    }

    #[test]
    fn scans_the_test_binary_itself() {
        // The PE header's own first bytes are "MZ", but only executable
        // sections are scanned: an all-wildcard pattern matches at .text's start.
        assert!(find_pattern_in_module("?? ?? ??").is_some());
        assert!(find_pattern_in_module("DE AD BE EF DE AD BE EF DE AD BE EF 01 02 03 04 05 06 07 08 09").is_none());
    }
}
