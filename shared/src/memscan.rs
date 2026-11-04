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

/// Offsets in `code` of every `jmp rel32` (`E9`) that lands on `target`, when `code` starts at `base`.
fn find_jumps(code: &[u8], base: usize, target: usize) -> Vec<usize> {
    let mut offsets = Vec::new();
    for i in 0..code.len().saturating_sub(4) {
        if code[i] != 0xE9 {
            continue;
        }
        let rel = i32::from_le_bytes([code[i + 1], code[i + 2], code[i + 3], code[i + 4]]);
        if (base + i + 5).wrapping_add_signed(rel as isize) == target {
            offsets.push(i);
        }
    }
    offsets
}

/// Every `jmp rel32` in the main module's executable sections that lands on `target`: the tail jump of
/// a small wrapper into a function.
pub fn find_jumps_to(target: usize) -> Vec<*mut u8> {
    module_sections()
        .into_iter()
        .filter(|s| s.characteristics & IMAGE_SCN_MEM_EXECUTE != 0)
        .flat_map(|section| {
            // SAFETY: a section of the loaded image, mapped for the process lifetime.
            let code = unsafe { std::slice::from_raw_parts(section.start, section.size) };
            find_jumps(code, section.start as usize, target).into_iter().map(move |i| {
                // SAFETY: `i` is an offset inside the section just scanned.
                unsafe { section.start.add(i) as *mut u8 }
            })
        })
        .collect()
}

const PAGE_SIZE: usize = 0x1000;

/// Every 8-byte-aligned slot in the main module's non-executable sections that holds `value`: where
/// a function address sits in a vtable or a call table. Only readable pages are read, since a section
/// can hold guard or no-access pages that crash the process when touched.
pub fn find_data_pointers(value: usize) -> Vec<*mut usize> {
    let needle = value.to_le_bytes();
    let mut slots = Vec::new();
    for section in
        module_sections().into_iter().filter(|s| s.characteristics & IMAGE_SCN_MEM_EXECUTE == 0)
    {
        let end = section.start as usize + section.size;
        let mut at = (section.start as usize).next_multiple_of(8);
        while at < end {
            let readable = crate::conflict::readable_len(at);
            if readable == 0 {
                at = (at | (PAGE_SIZE - 1)) + 1;
                continue;
            }
            let chunk_end = (at + readable).min(end);
            if chunk_end - at >= 8 {
                // SAFETY: `at..chunk_end` is committed, readable memory, just queried.
                let chunk = unsafe { std::slice::from_raw_parts(at as *const u8, chunk_end - at) };
                slots.extend(
                    chunk
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .enumerate()
                        .filter(|(_, word)| **word == needle)
                        .map(|(i, _)| (at + i * 8) as *mut usize),
                );
            }
            at = chunk_end.next_multiple_of(8);
        }
    }
    slots
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
    fn finds_a_pointer_stored_in_the_modules_own_data() {
        // The needle lives in this test binary's data; a missing value is not found, and no page that
        // cannot be read is touched on the way.
        static NEEDLE: usize = 0x1BAD_B002_CAFE_F00D;
        let slots = find_data_pointers(NEEDLE);
        assert!(slots.iter().any(|&slot| std::ptr::eq(slot, &NEEDLE)));
        assert!(find_data_pointers(0x0123_4567_89AB_CDEF ^ 0x5A5A_5A5A_5A5A_5A5A).is_empty());
    }

    #[test]
    fn finds_jumps_that_land_on_the_target() {
        // E9 at offset 2 (next instruction at 7), displacement 0x10: lands on base + 0x17.
        let code = [0x90, 0x90, 0xE9, 0x10, 0, 0, 0, 0x90, 0xE9, 0x00];
        assert_eq!(find_jumps(&code, 0x1000, 0x1017), vec![2]);
        assert!(find_jumps(&code, 0x1000, 0x1018).is_empty());
        // A trailing E9 with too few bytes after it is ignored, not read past the end.
        assert!(find_jumps(&[0xE9, 1, 2], 0, 0).is_empty());
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
