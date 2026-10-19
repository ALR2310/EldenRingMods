#![allow(non_snake_case)] // crate name is "AnimCancel" to control the output DLL's filename

mod cancel;
mod config;
mod menu;
mod probe;
mod weapon;

use common::{dll_dir, logger};

/// # Safety
/// This is exposed this way so the library loader can call it. Do not call it
/// yourself.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: u64, reason: u32) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason != DLL_PROCESS_ATTACH {
        return true;
    }

    std::thread::spawn(move || {
        let dir = dll_dir(hmodule);
        let report = config::init(&format!("{dir}\\AnimCancelV1.toml"));

        // [Logging] LogFile gates the log file entirely - see common::logger.
        logger::set_enabled(config::get().logging.log_file);
        logger::init(&dir, "AnimCancel.log");
        logger::install_panic_hook();

        // Registers the in-game menu right away: the host only accepts
        // registrations in its first seconds. No-op without ERNativeUI.
        menu::start(hmodule);

        logger::log("Activating AnimCancel...");
        if report.created {
            logger::log("AnimCancelV1.toml created with the default settings.");
        }
        if report.added_keys > 0 {
            logger::log(&format!(
                "AnimCancelV1.toml updated: added {} new key(s) from a newer default template.",
                report.added_keys
            ));
        }
        if let Some((from, to)) = report.migrated {
            logger::log(&format!("AnimCancelV1.toml updated from format version {from} to {to}."));
        }
        if let Some((file, current)) = report.newer {
            logger::warn(&format!(
                "AnimCancelV1.toml is from a newer version of the mod (format {file}, this one knows {current}) - left as it is."
            ));
        }
        if !report.unknown.is_empty() {
            logger::warn(&format!(
                "AnimCancelV1.toml: {} key(s) this version doesn't use were moved to comments at the end of the file:\n  {}",
                report.unknown.len(),
                report.unknown.join("\n  ")
            ));
        }
        if let Some(err) = report.error {
            logger::error(&format!(
                "AnimCancelV1.toml has an error, running on the default settings:\n{err}"
            ));
        }

        std::thread::spawn(|| {
            common::reload::run_with(
                || config::get().general.reload_key.clone(),
                || config::get().general.reload_banner,
                || {
                    config::reload()?;
                    logger::set_enabled(config::get().logging.log_file);
                    menu::sync_from_config();
                    Ok(())
                },
            )
        });

        common::diag::log_environment_when_game_ready();
        probe::start();
        cancel::run();
    });

    true
}
