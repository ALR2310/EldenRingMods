//! Whether the game considers the player in combat: the flag read from the `CSSound` singleton
//! (`+0x328` -> a struct whose byte `+4` is set while the combat music plays). Read once per
//! frame by the tick, never in the pickup hook (the singleton lookup is not hook-safe). How well
//! the offsets are verified: `docs/exec_action_button.md`.

use fromsoftware_shared::{FromStatic, singleton};

#[repr(C)]
struct CombatState {
    unk0: [u8; 4],
    in_combat: u8,
}

#[repr(C)]
#[singleton("CSSound")]
struct CSSound {
    unk0: [u8; 0x328],
    combat: *const CombatState,
}

/// `None` when the singleton or the struct isn't there (title screen, loading).
pub fn in_combat() -> Option<bool> {
    // SAFETY: called from a game task; `instance` returns `Err` while the singleton does not exist.
    let sound = unsafe { CSSound::instance() }.ok()?;
    if (sound.combat as usize) < 0x10000 {
        return None;
    }
    // SAFETY: the pointer is above the null page and points to the game's own combat struct.
    Some(unsafe { (*sound.combat).in_combat } != 0)
}
