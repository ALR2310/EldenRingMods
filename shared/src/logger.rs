//! Per-mod log file, written only while `LogFile` is on: off means no file is
//! created or written at all, on creates it (truncating the previous run's
//! log) at the first line. Checked on every write, so flipping the key and
//! reloading takes effect at once. Details and history: `shared/docs/logger.md`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, Once};

use crate::config;

struct LogState {
    /// Set by [init]; `None` means `init` hasn't run yet.
    path: Option<String>,
    /// Opened lazily by the first write with `LogFile` on.
    file: Option<File>,
}

static LOG: LazyLock<Mutex<LogState>> = LazyLock::new(|| Mutex::new(LogState { path: None, file: None }));
static PANIC_HOOK_INSTALLED: Once = Once::new();

/// What [ENABLED] holds: read the ini's `LogFile`, or forced off / on by [set_enabled].
const FROM_INI: u8 = 0;
const FORCED_OFF: u8 = 1;
const FORCED_ON: u8 = 2;
static ENABLED: AtomicU8 = AtomicU8::new(FROM_INI);

/// Turns the log file on/off directly, for mods whose `LogFile` isn't in the
/// ini map (TOML config). Call it after loading and after every reload.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(if enabled { FORCED_ON } else { FORCED_OFF }, Ordering::Relaxed);
}

fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        FROM_INI => config::get_bool("LogFile", false),
        state => state == FORCED_ON,
    }
}

/// Locks the log state. A poisoned lock is still usable: logging must never
/// panic, least of all from the panic hook.
fn state() -> MutexGuard<'static, LogState> {
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Records where the log goes (`file_name`, e.g. "AutoRegen.log", in
/// `log_dir`) without touching the disk. Call after the ini is loaded. No-op
/// if already initialized.
pub fn init(log_dir: &str, file_name: &str) {
    let mut guard = state();
    if guard.path.is_none() {
        guard.path = Some(format!("{log_dir}\\{file_name}"));
    }
}

/// Truncates and opens the log file at `path`.
fn open(path: &str) -> Option<File> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = File::create(path); // truncate
    // Append mode, so a line lands at the real end of the file even if
    // something else (the user clearing it in an editor) resized it meanwhile.
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Writes one line as `[timestamp] [LEVEL] message`.
fn write_line(level: &str, message: &str) {
    // Read before taking the log lock: never hold both at once.
    if !enabled() {
        return;
    }
    let mut guard = state();
    if guard.file.is_none() {
        let Some(path) = guard.path.clone() else {
            return;
        };
        guard.file = open(&path);
    }
    if let Some(file) = guard.file.as_mut() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{now}] [{level}] {message}");
    }
}

/// Plain informational line (INFO). Named `log` rather than `info` so the
/// call sites across all mods keep working.
pub fn log(message: &str) {
    write_line("INFO", message);
}

/// Something unexpected that the mod recovered from or is still retrying
/// past: the feature is not (yet) lost.
pub fn warn(message: &str) {
    write_line("WARN", message);
}

/// A feature failed and stays disabled for the rest of the session. Don't
/// repeat the word "ERROR" in `message`; the level already says it.
pub fn error(message: &str) {
    write_line("ERROR", message);
}

/// Verbose detail only useful when diagnosing one feature (byte dumps,
/// per-tick values); call sites should sit behind their own ini flag.
pub fn debug(message: &str) {
    write_line("DEBUG", message);
}

/// Installs a process-wide panic hook that logs an ERROR line (thread,
/// location, message) before running the previous hook. Without it a panic on
/// a background thread kills that thread silently and the log just stops. Call
/// once, early in `DllMain`, after `init`; later calls do nothing.
pub fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("<unnamed>");
            let location = info
                .location()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "<unknown location>".to_string());
            let payload = info.payload();
            let message = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string panic payload>".to_string()
            };
            error(&format!("PANIC on thread '{thread_name}' at {location}: {message}"));
            default_hook(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_only_while_enabled_and_truncates_on_first_write() {
        let dir = std::env::temp_dir().join("common_logger_test");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("test.log");
        init(dir.to_str().unwrap(), "test.log");

        set_enabled(false);
        log("never written");
        assert!(!path.exists(), "no file while LogFile is off");

        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "previous run\n").unwrap();

        set_enabled(true);
        warn("hello");
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[WARN] hello"), "{text}");
        assert!(!text.contains("never written"));
        assert!(!text.contains("previous run"), "the first write truncates");

        let _ = fs::remove_dir_all(&dir);
    }
}
