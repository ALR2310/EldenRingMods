//! Text in the game's top-of-screen announcement banner, for example to confirm a config reload.

use eldenring::cs::{AnnounceNotification, CSMenuManImp, MenuString};
use eldenring::dltx::DLString;
use fromsoftware_shared::FromStatic;

/// Does nothing if the menu manager or the heap allocator isn't resolved yet. Call it from a
/// game task.
pub fn show_announcement(text: &str) {
    // SAFETY: called from game tasks, so nothing else holds a reference to the singleton meanwhile.
    let Ok(menu_man) = (unsafe { CSMenuManImp::instance_mut() }) else {
        return;
    };
    let Some(allocator) = crate::alloc_hook::runtime_heap_allocator() else {
        return;
    };
    let Ok(allocated_string) = DLString::from_str(text, allocator) else {
        return;
    };
    menu_man.system_announce_view_model.notifications.push_back(AnnounceNotification {
        is_active: true,
        message: MenuString { static_string: std::ptr::null(), allocated_string },
    });
}
