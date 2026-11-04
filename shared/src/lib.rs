//! Code shared by every mod DLL. Each mod links its own copy, so mods share no runtime state.

pub mod alloc_hook;
pub mod anim;
pub mod announce;
pub mod codepatch;
pub mod conflict;
pub mod crash;
pub mod diag;
pub mod i18n;
pub mod input;
pub mod logger;
pub mod memscan;
pub mod menu;
pub mod params;
pub mod player;
pub mod reload;
pub mod task;
pub mod toml_config;

use std::ffi::c_void;

unsafe extern "system" {
    fn GetModuleFileNameW(hmodule: *mut c_void, lp_filename: *mut u16, n_size: u32) -> u32;
}

/// Full path of a loaded module, through the wide API (the ANSI one garbles non-ASCII folders).
pub(crate) fn module_path(hmodule: *mut c_void) -> Option<String> {
    // Start at MAX_PATH and grow: the call truncates on long-path installs.
    let mut buf = vec![0u16; 260];
    let len = loop {
        // SAFETY: `buf` is writable for the length passed; a bad handle makes the call fail.
        let n = unsafe { GetModuleFileNameW(hmodule, buf.as_mut_ptr(), buf.len() as u32) };
        if (n as usize) < buf.len() || buf.len() >= 32768 {
            break n as usize;
        }
        buf.resize(buf.len() * 2, 0);
    };
    (len != 0).then(|| String::from_utf16_lossy(&buf[..len]))
}

pub(crate) fn dir_of(path: &str) -> Option<&str> {
    path.rsplit_once('\\').map(|(dir, _)| dir)
}

/// The DLL's own file name without the extension, so a mod can name its files after itself without
/// spelling the name out in the binary.
pub fn dll_stem(hmodule: u64) -> String {
    module_path(hmodule as *mut c_void)
        .and_then(|path| {
            std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "mod".to_string())
}

/// The directory the DLL was loaded from, or "." on failure. The working directory is not used:
/// mod loaders don't always set it to the game folder.
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
        assert_eq!(dir_of(r"E:\Game\mod\Example.dll"), Some(r"E:\Game\mod"));
        assert_eq!(dir_of(r"C:\x.dll"), Some("C:"));
        assert_eq!(dir_of("x.dll"), None);
    }

    #[test]
    fn dll_dir_falls_back_to_the_current_dir_for_a_bad_handle() {
        assert_eq!(dll_dir(1), ".");
    }
}
