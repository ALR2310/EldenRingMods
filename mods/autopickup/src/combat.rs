//! The game's "player is in combat" flag from the `CSSound` singleton. Read once per frame in the
//! tick, not in the hook: the singleton lookup is not hook-safe. See `docs/exec_action_button.md`.

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

pub fn in_combat() -> Option<bool> {
    // SAFETY: called from a game task; `instance` returns `Err` while the singleton does not exist.
    let sound = unsafe { CSSound::instance() }.ok()?;
    if (sound.combat as usize) < 0x10000 {
        return None;
    }
    // SAFETY: the pointer is above the null page and points to the game's own combat struct.
    Some(unsafe { (*sound.combat).in_combat } != 0)
}
