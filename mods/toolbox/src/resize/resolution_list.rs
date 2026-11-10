//! Puts `WindowResize.Width` x `Height` into the game's windowed resolution list, so the game
//! renders at that size instead of rounding it down to the biggest entry that fits (a 2560x1080
//! request became 1920x1080). `docs/resize_approach.md`.

use std::time::Duration;

use shared::{codepatch, logger, memscan};

use crate::config;

const TABLE_PATTERN: &str =
    "48 8D 3D ?? ?? ?? ?? 44 2B 74 24 ?? BD 01 00 00 00 44 8B 7C 24 ?? 44 2B 7C 24 ??";
const LEA_LEN: isize = 7;
const ENTRIES: usize = 11;

type Size = (i32, i32);

pub fn run() {
    let resize = config::get().window_resize.clone();
    if resize.width <= 0 || resize.height <= 0 {
        return;
    }
    let Some(site) = memscan::wait_for_pattern_in_module(
        TABLE_PATTERN,
        Duration::from_millis(100),
        Duration::from_secs(30),
    ) else {
        logger::error("Resolutions: the list was not found.");
        return;
    };
    // SAFETY: the pattern matched, so the 4 bytes after the `lea` opcode are its displacement.
    let displacement = unsafe { (site.add(3) as *const i32).read_unaligned() };
    let table = site.wrapping_offset(LEA_LEN + displacement as isize);

    // SAFETY: the game's table is 11 pairs of i32 in its data section, mapped for the process.
    let current: [Size; ENTRIES] = std::array::from_fn(|i| unsafe {
        let entry = (table as *const i32).add(i * 2);
        (entry.read_unaligned(), entry.add(1).read_unaligned())
    });
    // A game update that changed the table must not get a patch written over something else.
    if !current.iter().all(|&(w, h)| w > 0 && w * 9 == h * 16) {
        logger::error(&format!("Resolutions: the list is not the expected one: {current:?}."));
        return;
    }
    let size = (resize.width, resize.height);
    let Some(patched) = with_size(current, size) else {
        logger::log(&format!("Resolutions: {}x{} is in the game's list already.", size.0, size.1));
        return;
    };
    let bytes: Vec<u8> =
        patched.iter().flat_map(|&(w, h)| [w, h]).flat_map(i32::to_le_bytes).collect();
    // SAFETY: the table is the game's own data, `bytes` has the same length.
    if unsafe { codepatch::overwrite_bytes(table, &bytes) } {
        logger::log(&format!("Resolutions: {}x{} added to the game's list.", size.0, size.1));
    } else {
        logger::error("Resolutions: the list could not be written.");
    }
}

/// `entries` with the entry of the closest area replaced by `size`, kept sorted by width then
/// height as the game expects. `None` if `size` is in the list already.
fn with_size(mut entries: [Size; ENTRIES], size: Size) -> Option<[Size; ENTRIES]> {
    if entries.contains(&size) {
        return None;
    }
    let area = |(w, h): Size| i64::from(w) * i64::from(h);
    let closest = (0..ENTRIES).min_by_key(|&i| (area(entries[i]) - area(size)).abs())?;
    entries[closest] = size;
    entries.sort_unstable();
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: [Size; ENTRIES] = [
        (1024, 576),
        (1152, 648),
        (1280, 720),
        (1440, 810),
        (1600, 900),
        (1680, 945),
        (1920, 1080),
        (2048, 1152),
        (2560, 1440),
        (3200, 1800),
        (3840, 2160),
    ];

    #[test]
    fn an_ultrawide_size_replaces_the_closest_area_and_stays_sorted() {
        let list = with_size(GAME, (2560, 1080)).unwrap();
        assert!(!list.contains(&(2048, 1152)));
        assert_eq!(list[7], (2560, 1080));
        assert_eq!(list[8], (2560, 1440));
        assert!(list.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_size_in_the_list_changes_nothing() {
        assert_eq!(with_size(GAME, (1920, 1080)), None);
    }
}
