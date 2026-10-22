//! Code shared by every mod DLL in this workspace: config and logging,
//! memory scanning and patching, and helpers for the game's engine types.
//! Each mod links its own copy (shared source, no shared runtime state).
//! Layout and per-module notes: `shared/docs/`.

pub mod alloc_hook;
pub mod anim;
pub mod announce;
pub mod codepatch;
pub mod config;
pub mod conflict;
pub mod crash;
pub mod diag;
pub mod erui;
pub mod i18n;
pub mod input;
pub mod logger;
pub mod memscan;
pub mod params;
pub mod player;
pub mod reload;
pub mod task;
pub mod task_hook;
pub mod toml_config;

use std::ffi::c_void;

unsafe extern "system" {
    fn GetModuleFileNameW(hmodule: *mut c_void, lp_filename: *mut u16, n_size: u32) -> u32;
}

/// Full path of a module loaded in this process (`hmodule` = its base
/// address), or `None` if the WinAPI call fails. Uses the wide API: the ANSI
/// one garbled non-ASCII folder names and the ini next to the DLL was never
/// found.
pub(crate) fn module_path(hmodule: *mut c_void) -> Option<String> {
    // Start at MAX_PATH and grow: the call truncates when the buffer is too
    // small, which long-path installs can hit.
    let mut buf = vec![0u16; 260];
    let len = loop {
        let n = unsafe { GetModuleFileNameW(hmodule, buf.as_mut_ptr(), buf.len() as u32) };
        if (n as usize) < buf.len() || buf.len() >= 32768 {
            break n as usize;
        }
        buf.resize(buf.len() * 2, 0);
    };
    (len != 0).then(|| String::from_utf16_lossy(&buf[..len]))
}

/// The directory part of a Windows path, `None` if it has no `\`.
pub(crate) fn dir_of(path: &str) -> Option<&str> {
    path.rsplit_once('\\').map(|(dir, _)| dir)
}

/// Directory the calling DLL itself was loaded from - not the process's
/// current working directory, which mod loaders don't always set to the game
/// folder. Falls back to "." if the WinAPI call fails for any reason.
pub fn dll_dir(hmodule: u64) -> String {
    module_path(hmodule as *mut c_void)
        .and_then(|path| dir_of(&path).map(str::to_string))
        .unwrap_or_else(|| ".".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_of_cuts_the_file_name() {
        assert_eq!(dir_of(r"E:\Game\mod\AutoRegen.dll"), Some(r"E:\Game\mod"));
        assert_eq!(dir_of(r"C:\x.dll"), Some("C:"));
        assert_eq!(dir_of("x.dll"), None);
    }

    #[test]
    fn dll_dir_falls_back_to_the_current_dir_for_a_bad_handle() {
        assert_eq!(dll_dir(1), ".");
    }
}
