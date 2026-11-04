//! IDA-style AOB scanner over the executable sections of the game's main module. A pattern is hex
//! bytes with `?` or `??` as a wildcard: `"48 8B 05 ?? ?? ?? ?? 48 85 C0 74 07"`. It must match
//! exactly once on every supported build; see `docs/research/README.md`.

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

/// A token that isn't `?`, `??` or hex counts as byte `00`, so a typo reads as "not found", not a
/// panic inside the game.
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

/// Cache for one resolved address: code addresses in the game exe never move, so a hit is kept
/// and a miss is retried by the next caller.
pub struct CachedAddr(Mutex<Option<usize>>);

impl CachedAddr {
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }

    /// The cached address, or the result of `resolve` (run under the lock, so concurrent callers
    /// wait and reuse it).
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

/// Retries [find_pattern_in_module] until it matches or `timeout` elapses, for a startup scan that
/// may race the game's anti-tamper unpacking.
pub fn wait_for_pattern_in_module(
    pattern: &str,
    retry_interval: Duration,
    timeout: Duration,
) -> Option<*mut u8> {
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

struct Section {
    start: *const u8,
    size: usize,
    characteristics: u32,
}

fn module_sections() -> Vec<Section> {
    // SAFETY: a null name asks for the main module.
    let base = unsafe { GetModuleHandleA(std::ptr::null()) } as *const u8;
    if base.is_null() {
        return Vec::new();
    }
    // SAFETY: the main module's headers stay mapped for the life of the process, and every offset below comes from them.
    unsafe {
        let dos = &*(base as *const ImageDosHeader);
        let nt_base = base.add(dos.e_lfanew as usize);
        // IMAGE_NT_HEADERS64: Signature(u32), FileHeader, OptionalHeader; the
        // section table follows the optional header.
        let file_header = &*(nt_base.add(4) as *const ImageFileHeader);
        let section_table = nt_base
            .add(4 + size_of::<ImageFileHeader>() + file_header.size_of_optional_header as usize)
            as *const ImageSectionHeader;
        (0..file_header.number_of_sections as usize)
            .map(|i| {
                let section = &*section_table.add(i);
                Section {
                    start: base.add(section.virtual_address as usize),
                    size: section.virtual_size as usize,
                    characteristics: section.characteristics,
                }
            })
            .collect()
    }
}

/// The absolute address of the first match of `pattern` in the executable sections.
pub fn find_pattern_in_module(pattern: &str) -> Option<*mut u8> {
    let bytes = parse_pattern(pattern);
    module_sections()
        .into_iter()
        .filter(|s| s.characteristics & IMAGE_SCN_MEM_EXECUTE != 0)
        .find_map(|section| {
            // SAFETY: a section of the loaded image, mapped for the process lifetime.
            let slice = unsafe { std::slice::from_raw_parts(section.start, section.size) };
            find_pattern(slice, &bytes)
                // SAFETY: `offset` is an index into the section's own slice.
                .map(|offset| unsafe { section.start.add(offset) as *mut u8 })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_wildcards() {
        assert_eq!(
            parse_pattern("48 8B ?? ? 0f"),
            vec![Some(0x48), Some(0x8B), None, None, Some(0x0F)]
        );
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
        assert!(
            find_pattern_in_module(
                "DE AD BE EF DE AD BE EF DE AD BE EF 01 02 03 04 05 06 07 08 09"
            )
            .is_none()
        );
    }
}
