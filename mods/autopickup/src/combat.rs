//! Whether the game considers the player in combat: the flag Elden_Ring_game_tools
//! and ErdTools read from the `CSSound` singleton (`+0x328` -> a struct whose
//! byte `+4` is set while the combat music plays). Read once per frame by the
//! tick, never in the pickup hook (the singleton lookup is not hook-safe).
//! Offsets taken from those mods; how they were checked: `docs/exec_action_button.md`.

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
    let sound = unsafe { CSSound::instance() }.ok()?;
    if (sound.combat as usize) < 0x10000 {
        return None;
    }
    Some(unsafe { (*sound.combat).in_combat } != 0)
}
