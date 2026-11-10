//! Tells the user when another mod already hooked a site this mod patches (`shared::conflict`,
//! `shared/docs/conflict.md`). A site that fails is left unpatched and named in the log and in one
//! message box.

use std::sync::OnceLock;

use shared::conflict;

/// Seamless Co-op patches nearby code on purpose and is handled separately
/// (`docs/seamless_conflict.md`), so its hooks are never reported.
const COMPATIBLE: &str = "ersc";

/// The DLL's own name, for the report.
static NAME: OnceLock<String> = OnceLock::new();

pub fn init(name: &str) {
    let _ = NAME.set(name.to_string());
}

/// Whether `site` starts with a jump planted by another mod; if so it is reported as turning
/// `feature` off and the caller must not patch.
///
/// # Safety
/// `site..site + 13` must be readable memory of this process.
pub unsafe fn check(site: *const u8, feature: &str) -> bool {
    // SAFETY: readable, by this function's contract.
    let Some(hook) = (unsafe { conflict::foreign_hook_at(site) }) else {
        return false;
    };
    if hook.owner.as_deref().is_some_and(|name| name.to_ascii_lowercase().contains(COMPATIBLE)) {
        return false;
    }
    let name = NAME.get().map_or("SpiritMultiplier", String::as_str);
    conflict::report(name, feature, &format!("{name}.log"), &hook);
    true
}
