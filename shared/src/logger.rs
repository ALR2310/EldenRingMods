//! Per-mod log file, off by default ([set_enabled]). While off no file is created; the first line
//! after switching on truncates the previous run's log.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, Once};

struct LogState {
    path: Option<String>,
    file: Option<File>,
}

static LOG: LazyLock<Mutex<LogState>> =
    LazyLock::new(|| Mutex::new(LogState { path: None, file: None }));
static PANIC_HOOK_INSTALLED: Once = Once::new();
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Call it after loading the config and after every reload.
pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// A poisoned lock is still usable: logging must never panic, least of all from the panic hook.
fn state() -> MutexGuard<'static, LogState> {
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn init(log_dir: &str, file_name: &str) {
    let mut guard = state();
    if guard.path.is_none() {
        guard.path = Some(format!("{log_dir}\\{file_name}"));
    }
}

fn open(path: &str) -> Option<File> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = File::create(path);
    // Append mode, in case something else (the user clearing it in an editor) resized the file.
    OpenOptions::new().create(true).append(true).open(path).ok()
}

fn write_line(level: &str, message: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
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

pub fn log(message: &str) {
    write_line("INFO", message);
}

/// Something unexpected that the mod recovered from or keeps retrying.
pub fn warn(message: &str) {
    write_line("WARN", message);
}

/// A feature failed and stays disabled. The level already says "error".
pub fn error(message: &str) {
    write_line("ERROR", message);
}

/// Logs thread, location and message of any panic before running the previous hook; without it a
/// panic on a background thread ends the thread and the log just stops. Call once, after [init].
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
        crate::crash::install();
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
        assert!(!path.exists(), "no file while logging is off");

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
