//! Rows bound to config keys: [`Setting`] for toggles, [`NumberSetting`] for numbers, and the
//! saver thread that writes what the player changed back to the config.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use super::abi::*;
use super::host::{Provider, REGISTRATION, Registration};
use crate::logger;

/// One menu row bound to a config key. Pass it as the row's `user_data` (via
/// [`Setting::user_data`]) together with [`setting_changed`] as the callback: a change is
/// applied at once with `apply` (if any) and written to the config by the saver started with
/// [`start_saver`].
pub struct Setting {
    /// The config key, e.g. `"Section.Key"`.
    pub key: &'static str,
    /// The row's byte as the text stored in the config.
    pub format: fn(u8) -> String,
    /// Runs on the host's worker thread right when the player changes the row
    /// (keep it short); `None` when the mod only needs the saved value.
    pub apply: Option<fn(u8)>,
}

impl Setting {
    /// A toggle row stored as `true` / `false`.
    pub const fn flag(key: &'static str) -> Self {
        Setting { key, format: format_bool, apply: None }
    }

    /// The `user_data` pointer for the row; `self` must be a `static`.
    pub fn user_data(&'static self) -> *mut c_void {
        self as *const Setting as *mut c_void
    }
}

/// `format` for a toggle: the byte as `true` / `false`.
pub fn format_bool(value: u8) -> String {
    (value != 0).to_string()
}

type Pending = (&'static str, String);

static SAVER: OnceLock<Sender<Pending>> = OnceLock::new();

/// How long the player must stop changing rows before the config is written.
const SAVE_DELAY: Duration = Duration::from_millis(1000);

/// Starts the thread that saves menu changes once the player has stopped changing rows for a
/// second, then calls `on_saved` (for example to make the mod re-read its config). The thread
/// sleeps until there is something to save. Call once, before registering rows.
///
/// `write` gets the changed `(key, text)` pairs (the latest per key) and returns whether they
/// were saved. `Setting::key` is whatever `write` expects, `"Section.Key"` for
/// [`crate::toml_config::TomlConfig::save_values`].
pub fn start_saver(
    mut write: impl FnMut(&[(&'static str, String)]) -> bool + Send + 'static,
    on_saved: Option<fn()>,
) {
    let (tx, rx) = mpsc::channel::<Pending>();
    if SAVER.set(tx).is_err() {
        return;
    }
    std::thread::spawn(move || {
        let mut changes: Vec<Pending> = Vec::new();
        while let Ok(first) = rx.recv() {
            let mut next = Some(first);
            loop {
                if let Some((key, value)) = next.take() {
                    match changes.iter_mut().find(|(k, _)| *k == key) {
                        Some(entry) => entry.1 = value,
                        None => changes.push((key, value)),
                    }
                }
                match rx.recv_timeout(SAVE_DELAY) {
                    Ok(change) => next = Some(change),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            if write(&changes) {
                logger::log(&format!("Menu: saved {changes:?}"));
            } else {
                logger::log("Menu: config save failed");
            }
            changes.clear();
            if let Some(on_saved) = on_saved {
                on_saved();
            }
        }
    });
}

/// Value-changed callback for rows whose `user_data` is a [`Setting`]. Runs on
/// the host's worker thread; never panics across the C ABI.
///
/// # Safety
/// `user_data` must be null or [`Setting::user_data`] of a `static` [`Setting`]
/// (the host hands back the pointer given when the row was added).
pub unsafe extern "C" fn setting_changed(user_data: *mut c_void, value: u8) {
    let _ = std::panic::catch_unwind(|| {
        if user_data.is_null() {
            return;
        }
        // SAFETY: the row was registered with a `static` setting as `user_data`, see the function's contract.
        let setting = unsafe { &*(user_data as *const Setting) };
        if let Some(apply) = setting.apply {
            apply(value);
        }
        if let Some(tx) = SAVER.get() {
            let _ = tx.send((setting.key, (setting.format)(value)));
        }
    });
}

/// Longest number text the row accepts (UTF-16 units; the host allows 35).
const NUMBER_MAX_LENGTH: u32 = 12;

/// One text row holding a number bound to a config key (`"Section.Key"`, see
/// [`start_saver`]). Declare it as a `static` and add the row with [`Provider::add_number`].
pub struct NumberSetting {
    pub key: &'static str,
    pub min: f64,
    pub max: f64,
    /// Runs on the host's worker thread with the accepted value (keep it short).
    pub apply: Option<fn(f64)>,
    /// The row, set by `add_number`, so rejected text can be put back.
    pub row: OnceLock<RowHandle>,
    /// Last accepted value (`f64` bits).
    pub last: AtomicU64,
    /// Step of the native slider (`0` = text row only, see [`NumberSetting::stepped`]).
    pub step: f64,
    /// Range of the native slider when it is narrower than `min..max`
    /// ([`NumberSetting::slider`]); `None` = the slider covers `min..max`.
    pub slider_range: Option<(f64, f64)>,
    /// Set by `add_number`: `0` for a text row, else the slider's decimal
    /// places plus one.
    slider: AtomicU32,
}

/// A numeric slider's range as exact base-10 integers (`value * 10^places`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct SliderPlan {
    places: u32,
    min: i64,
    max: i64,
    step: i64,
}

impl SliderPlan {
    /// `value` as the scaled integer of the nearest slider position.
    fn snap(&self, value: f64) -> i64 {
        let scale = 10f64.powi(self.places as i32);
        let last = (self.max - self.min) / self.step;
        let index = (((value * scale) - self.min as f64) / self.step as f64)
            .round()
            .clamp(0.0, last as f64) as i64;
        self.min + index * self.step
    }
}

impl NumberSetting {
    /// A number stored in a TOML float field: a whole number is shown and saved with a decimal
    /// point (`2` becomes `2.0`), so the file keeps a float.
    pub const fn new(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        NumberSetting {
            key,
            min,
            max,
            apply,
            row: OnceLock::new(),
            last: AtomicU64::new(0),
            step: 0.0,
            slider_range: None,
            slider: AtomicU32::new(0),
        }
    }

    /// Makes the row a native slider from `min` to `max` in steps of `step`
    /// when the host has API 1.2 (a text row otherwise). The range must hold
    /// 2 to 256 positions and `step` at most six decimals; if not, the row
    /// stays a text row.
    pub const fn stepped(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Like [`NumberSetting::stepped`] for a setting whose accepted range
    /// (`min..max`, used by the typed text row and for clamping) is much wider
    /// than what a slider can show: the slider covers `from..=to` in steps of
    /// `step`. A value outside it is shown at the nearest end; the config is
    /// only changed when the player moves the slider.
    pub const fn slider(mut self, from: f64, to: f64, step: f64) -> Self {
        self.step = step;
        self.slider_range = Some((from, to));
        self
    }

    /// The slider's exact range, or `None` when this setting has no step or the
    /// range cannot be a native slider.
    fn slider_plan(&self) -> Option<SliderPlan> {
        if self.step <= 0.0 {
            return None;
        }
        let places = (0..=6u32).find(|p| {
            let scaled = self.step * 10f64.powi(*p as i32);
            (scaled - scaled.round()).abs() < 1e-9
        })?;
        let scale = 10f64.powi(places as i32);
        let (low, high) = self.slider_range.unwrap_or((self.min, self.max));
        let plan = SliderPlan {
            places,
            min: (low * scale).round() as i64,
            max: (high * scale).round() as i64,
            step: (self.step * scale).round() as i64,
        };
        let span = plan.max - plan.min;
        let positions = span / plan.step + 1;
        (plan.step > 0 && span % plan.step == 0 && (2..=256).contains(&positions)).then_some(plan)
    }

    /// `value` as the row accepts it: clamped to the range.
    fn normalize(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }

    /// The text shown in the row and written to the config for `value`.
    fn text(&self, value: f64) -> String {
        let text = number_text(value);
        if text.contains('.') { text } else { format!("{text}.0") }
    }

    /// Shows `value` (normalized) in the row, e.g. after the config was reloaded.
    pub fn show(&'static self, registration: &Registration, value: f64) -> Result<(), String> {
        let value = self.normalize(value);
        self.last.store(value.to_bits(), Ordering::Relaxed);
        let Some(row) = self.row.get() else { return Ok(()) };
        match self.slider_plan().filter(|_| self.slider.load(Ordering::Relaxed) != 0) {
            Some(plan) => registration.set_numeric(*row, plan.snap(value), plan.places),
            None => registration.set_text(*row, &self.text(value)),
        }
    }
}

/// Shortest text that reads back as `value` (`1`, `0.5`, never `1e-7`),
/// rounded to six decimals so a value that went through `f32` (`1.2` read back
/// as `1.2000000476837158`) shows as the number the player typed.
fn number_text(value: f64) -> String {
    let rounded = if value.abs() < 1e9 { (value * 1e6).round() / 1e6 } else { value };
    format!("{rounded}")
}

/// Parses what the player typed: `1.5` or `1,5`, surrounding spaces ignored.
fn parse_number(text: &str) -> Option<f64> {
    let value: f64 = text.trim().replace(',', ".").parse().ok()?;
    value.is_finite().then_some(value)
}

unsafe extern "C" fn number_changed(user_data: *mut c_void, change: *const TextInputChange) {
    let _ = std::panic::catch_unwind(|| {
        if user_data.is_null() || change.is_null() {
            return;
        }
        // SAFETY: the row was registered with a `static` setting as `user_data`.
        let setting = unsafe { &*(user_data as *const NumberSetting) };
        // SAFETY: `change` was checked non-null and the host keeps it valid for the call.
        let typed = unsafe { &*change }.text();
        let accepted = parse_number(&typed).map(|v| setting.normalize(v));
        let value = match accepted {
            Some(v) => v,
            None => f64::from_bits(setting.last.load(Ordering::Relaxed)),
        };
        // Show the number actually in use (clamped, normalised, or the old one
        // when the text was not a number).
        let shown = setting.text(value);
        if shown != typed.trim()
            && let (Some(registration), Some(row)) = (REGISTRATION.get(), setting.row.get())
        {
            let _ = registration.set_text(*row, &shown);
        }
        if accepted.is_none() {
            return;
        }
        accept_number(setting, value);
    });
}

/// A number the row now holds: remember it, apply it and queue it for saving.
fn accept_number(setting: &NumberSetting, value: f64) {
    setting.last.store(value.to_bits(), Ordering::Relaxed);
    if let Some(apply) = setting.apply {
        apply(value);
    }
    if let Some(tx) = SAVER.get() {
        let _ = tx.send((setting.key, setting.text(value)));
    }
}

unsafe extern "C" fn number_slider_changed(
    user_data: *mut c_void,
    change: *const NumericSliderChange,
) {
    let _ = std::panic::catch_unwind(|| {
        if user_data.is_null() || change.is_null() {
            return;
        }
        // SAFETY: the row was registered with a `static` setting as `user_data`.
        let setting = unsafe { &*(user_data as *const NumberSetting) };
        // SAFETY: `change` was checked non-null and the host keeps it valid for the call.
        let value = unsafe { &*change }.value();
        accept_number(setting, setting.normalize(value));
    });
}

impl Provider {
    /// Adds a number row, kept in range and written to the config like any
    /// other [`Setting`]. A setting with a step ([`NumberSetting::stepped`]) is a
    /// native slider when the host has API 1.2; otherwise it is a text row: the
    /// player types `1.5` (a comma works too), out-of-range values are clamped
    /// and bad text is put back to the last good value. `initial` is the number
    /// the row starts with.
    pub fn add_number(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        setting: &'static NumberSetting,
        initial: f64,
    ) -> Result<RowHandle, String> {
        let initial = setting.normalize(initial);
        setting.last.store(initial.to_bits(), Ordering::Relaxed);
        if self.numeric_slider
            && let Some(plan) = setting.slider_plan()
        {
            let row =
                self.add_numeric_slider(page, label, help, &plan, plan.snap(initial), setting)?;
            setting.slider.store(plan.places + 1, Ordering::Relaxed);
            let _ = setting.row.set(row);
            return Ok(row);
        }
        let row = self.add_text_input(
            page,
            label,
            help,
            &setting.text(initial),
            "",
            NUMBER_MAX_LENGTH,
            number_changed,
            setting as *const NumberSetting as *mut c_void,
        )?;
        let _ = setting.row.set(row);
        Ok(row)
    }

    fn add_numeric_slider(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        plan: &SliderPlan,
        initial_scaled: i64,
        setting: &'static NumberSetting,
    ) -> Result<RowHandle, String> {
        let label = wide(label);
        let help = wide(help);
        let none: [u16; 0] = [];
        let desc = NumericSliderDesc {
            size: std::mem::size_of::<NumericSliderDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            changed_callback: Some(number_slider_changed),
            user_data: setting as *const NumberSetting as *mut c_void,
            minimum_scaled_value: plan.min,
            maximum_scaled_value: plan.max,
            step_scaled_value: plan.step,
            initial_scaled_value: initial_scaled,
            decimal_places: plan.places,
            enabled: 1,
            prefix: Utf16View::of(&none),
            suffix: Utf16View::of(&none),
            reserved: [0; 2],
        };
        let mut row: RowHandle = 0;
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status =
            unsafe { (self.api.add_numeric_slider.unwrap())(self.provider, page, &desc, &mut row) };
        check(status, "add_numeric_slider")?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_settings_describe_an_exact_slider() {
        let speed = NumberSetting::new("A", 0.1, 10.0, None).stepped(0.1);
        let plan = speed.slider_plan().unwrap();
        assert_eq!(plan, SliderPlan { places: 1, min: 1, max: 100, step: 1 });
        assert_eq!(plan.snap(1.2), 12);
        assert_eq!(plan.snap(1.23), 12);
        assert_eq!(plan.snap(99.0), 100);
        assert_eq!(plan.snap(-5.0), 1);

        let fine = NumberSetting::new("B", -1.0, 1.0, None).stepped(0.25);
        assert_eq!(
            fine.slider_plan().unwrap(),
            SliderPlan { places: 2, min: -100, max: 100, step: 25 }
        );

        // A slider narrower than the accepted range: values outside snap to the ends.
        let wide = NumberSetting::new("W", 0.0, 1000.0, None).slider(0.0, 10.0, 0.5);
        let plan = wide.slider_plan().unwrap();
        assert_eq!(plan, SliderPlan { places: 1, min: 0, max: 100, step: 5 });
        assert_eq!(plan.snap(2.0), 20);
        assert_eq!(plan.snap(500.0), 100);
        assert_eq!(wide.normalize(500.0), 500.0); // the typed row still accepts it

        // Not a slider: no step, too many positions, a step that misses the max.
        assert!(NumberSetting::new("C", 0.0, 10.0, None).slider_plan().is_none());
        assert!(NumberSetting::new("D", 0.0, 1000.0, None).stepped(1.0).slider_plan().is_none());
        assert!(NumberSetting::new("E", 0.0, 1.0, None).stepped(0.3).slider_plan().is_none());
    }

    #[test]
    fn numbers_are_parsed_leniently() {
        assert_eq!(parse_number("1.5"), Some(1.5));
        assert_eq!(parse_number(" 2,25 "), Some(2.25));
        assert_eq!(parse_number("-3"), Some(-3.0));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("inf"), None);
        assert_eq!(parse_number("NaN"), None);
    }

    #[test]
    fn number_settings_normalize_and_format() {
        let setting = NumberSetting::new("A", 0.0, 10.0, None);
        assert_eq!(setting.normalize(1.5), 1.5);
        assert_eq!(setting.normalize(99.0), 10.0);
        assert_eq!(setting.normalize(-1.0), 0.0);
        assert_eq!(setting.text(2.0), "2.0"); // keeps a TOML float field a float
        assert_eq!(setting.text(0.5), "0.5");
    }

    #[test]
    fn number_text_is_short_and_round_trips() {
        for v in [0.0, 1.0, 0.5, 0.25, 1000.0, 0.1, 12345.678] {
            assert_eq!(parse_number(&number_text(v)), Some(v));
        }
        assert_eq!(number_text(1.0), "1");
        assert_eq!(number_text(0.5), "0.5");
        assert_eq!(number_text(f64::from(1.2f32)), "1.2");
        assert_eq!(number_text(f64::from(0.07f32)), "0.07");
    }
}
