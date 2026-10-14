//! Client for ERNativeUI (https://github.com/Flammrock/ERNativeUI, MIT), the
//! shared native in-game settings menu host. Mirrors only the part of its C
//! ABI (`erui.h`, API 1.1) that mods here need: connect, register a provider,
//! add rows, commit, and read/write a row's value afterwards. The host
//! (`ERNativeUI.dll`) is an optional dependency: [`connect`] fails cleanly
//! when it is not loaded and the mod keeps working from its config file.
//!
//! Rules from the host's docs that shape this module:
//! - Connect and register from a worker thread after `DllMain` returned, and
//!   do it early: the host closes registration ~750 ms after the last
//!   provider activity (5 s absolute cap), later clients fail cleanly.
//! - Row values are bytes (`u8`); a slider's `minimum`/`maximum`/`step` are
//!   integers, so a fractional setting must be scaled by the caller.
//! - Value-changed callbacks run on the host's worker thread, button
//!   callbacks on the UI thread. Keep them short; never panic across the ABI.
//! - The host supports Elden Ring 2.7.0.0 and 2.7.1.0 only (matched by PE
//!   timestamp + image size); on any other exe it reports not-ready.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use crate::{config, logger};

type Result_ = u32;
const ERUI_OK: u32 = 0;
const ERUI_HOST_NOT_READY: u32 = 2;
const ERUI_HOST_FAILED: u32 = 3;

const API_VERSION_1_1: u32 = (1 << 16) | 1;
const CAP_TOGGLE: u64 = 1 << 0;
const CAP_SLIDER: u64 = 1 << 1;

pub type RowHandle = u64;
type ProviderHandle = u64;
pub type PageHandle = u64;

#[repr(C)]
struct StringView {
    data: *const u8,
    length: u32,
    reserved: u32,
}

#[repr(C)]
struct Utf16View {
    data: *const u16,
    length: u32,
    reserved: u32,
}

impl Utf16View {
    fn of(units: &[u16]) -> Self {
        Utf16View { data: units.as_ptr(), length: units.len() as u32, reserved: 0 }
    }
}

#[repr(C)]
struct ProviderDesc {
    size: u32,
    api_version: u32,
    flags: u32,
    root_priority: i32,
    owner_module: *mut c_void,
    provider_id: StringView,
    display_name: Utf16View,
}

/// Called when the player moves a slider. Runs on the host's worker thread.
pub type ValueChangedCallback = unsafe extern "C" fn(user_data: *mut c_void, value: u8);

#[repr(C)]
struct SliderDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    changed_callback: Option<ValueChangedCallback>,
    user_data: *mut c_void,
    minimum: i32,
    maximum: i32,
    step: i32,
    initial_value: u8,
    reserved8: [u8; 3],
    enabled: u32,
}

#[repr(C)]
struct ToggleDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    changed_callback: Option<ValueChangedCallback>,
    user_data: *mut c_void,
    initial_value: u8,
    reserved8: [u8; 3],
    enabled: u32,
}

#[repr(C)]
struct SubmenuDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    page_title: Utf16View,
    page_help: Utf16View,
    enabled: u32,
    reserved: u32,
}

#[repr(C)]
struct ChoiceDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    changed_callback: Option<ValueChangedCallback>,
    user_data: *mut c_void,
    options: *const Utf16View,
    option_count: u32,
    initial_index: u8,
    reserved8: [u8; 3],
}

/// What the host passes to a text-input callback (`ERUI_TextInputChangeContext`).
#[repr(C)]
pub struct TextInputChange {
    size: u32,
    flags: u32,
    provider: u64,
    row: u64,
    value: Utf16View,
    reserved: [u32; 2],
}

impl TextInputChange {
    /// The confirmed text. Only valid inside the callback.
    fn text(&self) -> String {
        if self.value.data.is_null() || self.value.length == 0 {
            return String::new();
        }
        let units = unsafe { std::slice::from_raw_parts(self.value.data, self.value.length as usize) };
        String::from_utf16_lossy(units)
    }
}

pub type TextChangedCallback = unsafe extern "C" fn(user_data: *mut c_void, change: *const TextInputChange);

#[repr(C)]
struct TextInputDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    initial_value: Utf16View,
    placeholder: Utf16View,
    changed_callback: Option<TextChangedCallback>,
    user_data: *mut c_void,
    maximum_length: u32,
    reserved: u32,
}

type Unused = Option<unsafe extern "C" fn()>;

/// `ERUI_Api` (API 1.1). Field order is the ABI; entries this module does not
/// call are kept as opaque [`Unused`] pointers so the table has the exact size
/// the host expects (`ERUI_API_V1_1_SIZE`).
#[repr(C)]
#[derive(Clone, Copy)]
struct Api {
    size: u32,
    api_version: u32,
    capabilities: u64,
    register_provider: Option<
        unsafe extern "C" fn(*const ProviderDesc, *mut ProviderHandle, *mut PageHandle) -> Result_,
    >,
    add_button: Unused,
    add_toggle: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const ToggleDesc, *mut RowHandle) -> Result_,
    >,
    add_slider: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const SliderDesc, *mut RowHandle) -> Result_,
    >,
    add_inline_choice: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const ChoiceDesc, *mut RowHandle) -> Result_,
    >,
    add_popup_choice: Unused,
    add_submenu: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const SubmenuDesc,
            *mut PageHandle,
            *mut RowHandle,
        ) -> Result_,
    >,
    set_page_presentation: Unused,
    commit_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Result_>,
    abort_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Result_>,
    set_row_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, u8) -> Result_>,
    get_row_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *mut u8) -> Result_>,
    enqueue_alert: Unused,
    get_game_language: Unused,
    add_text_input: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const TextInputDesc, *mut RowHandle) -> Result_,
    >,
    set_text_input_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *const Utf16View) -> Result_>,
    get_text_input_value: Unused,
    add_color_picker: Unused,
    set_color_picker_value: Unused,
    get_color_picker_value: Unused,
    get_builtin_page: Unused,
    add_input_section: Unused,
    add_input_action: Unused,
    set_assignments_changed_handler: Unused,
    set_action_inputs: Unused,
    get_action_inputs: Unused,
    get_action_default_inputs: Unused,
    reset_action_inputs: Unused,
    open_storage: Unused,
    storage_load: Unused,
    storage_save: Unused,
    storage_get_utf8: Unused,
    storage_set_utf8: Unused,
    storage_erase: Unused,
    storage_get_action_inputs: Unused,
    storage_set_action_inputs: Unused,
    storage_apply_assignment_changes: Unused,
    storage_get_info: Unused,
}

// ERUI_API_V1_1_SIZE: 16-byte header + 38 function pointers.
const _: () = assert!(std::mem::size_of::<Api>() == 16 + 38 * 8);

type GetApiFn = unsafe extern "C" fn(u32, *mut Api) -> Result_;

unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn result_name(code: u32) -> String {
    match code {
        1 => "invalid argument".into(),
        2 => "host not ready".into(),
        3 => "host failed".into(),
        4 => "unsupported version".into(),
        5 => "duplicate provider id".into(),
        6 => "invalid handle".into(),
        7 => "already committed".into(),
        8 => "registration closed (too late, host startup already finished)".into(),
        9 => "out of memory".into(),
        13 => "not supported".into(),
        other => format!("error {other}"),
    }
}

/// A live connection to the ERNativeUI host.
#[derive(Clone, Copy)]
pub struct Connection {
    api: Api,
}

/// Polls for the already-loaded `ERNativeUI.dll` and negotiates API 1.1.
/// Returns `Err` with a human-readable reason when the host is missing, failed
/// or does not become ready within `timeout`; callers should log it and carry
/// on without a menu.
pub fn connect(timeout: Duration) -> Result<Connection, String> {
    let deadline = Instant::now() + timeout;
    let host_name = wide("ERNativeUI.dll\0");
    let mut saw_host = false;
    loop {
        let host = unsafe { GetModuleHandleW(host_name.as_ptr()) };
        if !host.is_null() {
            saw_host = true;
            let symbol = unsafe { GetProcAddress(host, c"ERUI_GetApi".as_ptr() as *const u8) };
            if symbol.is_null() {
                return Err("ERNativeUI.dll does not export ERUI_GetApi".into());
            }
            let get_api: GetApiFn = unsafe { std::mem::transmute(symbol) };
            let mut api: Api = unsafe { std::mem::zeroed() };
            api.size = std::mem::size_of::<Api>() as u32;
            let status = unsafe { get_api(API_VERSION_1_1, &mut api) };
            match status {
                ERUI_OK => {
                    // The docs ask a strict-C client to verify the exact version,
                    // table size, capability bits and function pointers.
                    if api.api_version != API_VERSION_1_1
                        || api.size != std::mem::size_of::<Api>() as u32
                        || api.capabilities & (CAP_TOGGLE | CAP_SLIDER) != (CAP_TOGGLE | CAP_SLIDER)
                    {
                        return Err("ERNativeUI returned an incompatible API 1.1 table".into());
                    }
                    if api.register_provider.is_none()
                        || api.add_slider.is_none()
                        || api.add_toggle.is_none()
                        || api.add_submenu.is_none()
                        || api.add_inline_choice.is_none()
                        || api.add_text_input.is_none()
                        || api.set_text_input_value.is_none()
                        || api.commit_provider.is_none()
                        || api.abort_provider.is_none()
                        || api.set_row_value.is_none()
                    {
                        return Err("ERNativeUI returned an incomplete API 1.1 table".into());
                    }
                    return Ok(Connection { api });
                }
                ERUI_HOST_FAILED => {
                    return Err("ERNativeUI host initialization failed (see ERNativeUI.log)".into());
                }
                ERUI_HOST_NOT_READY => {}
                other => return Err(format!("ERNativeUI API 1.1 negotiation failed: {}", result_name(other))),
            }
        }
        if Instant::now() >= deadline {
            return Err(if saw_host {
                "ERNativeUI.dll did not become ready in time (unsupported game version?)".into()
            } else {
                "ERNativeUI.dll is not loaded".into()
            });
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// A provider being built (rows are added to its root page, which is the
/// shared System > Game Options page). Dropping it without [`Provider::commit`]
/// aborts the registration.
pub struct Provider {
    api: Api,
    provider: ProviderHandle,
    root: PageHandle,
    committed: bool,
}

impl Connection {
    /// `provider_id`: 1-255 ASCII letters, digits, `.`, `_`, `-`; unique
    /// process-wide. `owner_module`: the mod's own `HMODULE` (the `hmodule`
    /// passed to `DllMain`), which the host pins so row callbacks stay valid.
    pub fn register_provider(&self, provider_id: &str, display_name: &str, owner_module: u64) -> Result<Provider, String> {
        let name = wide(display_name);
        let desc = ProviderDesc {
            size: std::mem::size_of::<ProviderDesc>() as u32,
            api_version: self.api.api_version,
            flags: 0,
            root_priority: 100,
            owner_module: owner_module as *mut c_void,
            provider_id: StringView { data: provider_id.as_ptr(), length: provider_id.len() as u32, reserved: 0 },
            display_name: Utf16View::of(&name),
        };
        let mut provider: ProviderHandle = 0;
        let mut root: PageHandle = 0;
        let status = unsafe { (self.api.register_provider.unwrap())(&desc, &mut provider, &mut root) };
        if status != ERUI_OK {
            return Err(format!("register_provider: {}", result_name(status)));
        }
        Ok(Provider { api: self.api, provider, root, committed: false })
    }
}

impl Provider {
    /// The shared System > Game Options page.
    pub fn root_page(&self) -> PageHandle {
        self.root
    }

    /// Adds a row on `parent` that opens a new child page owned by this
    /// provider; returns the child page (to add rows to) and the row handle.
    /// `page_title`/`page_help` fall back to `label`/`help` when empty.
    pub fn add_submenu(
        &self,
        parent: PageHandle,
        label: &str,
        help: &str,
        page_title: &str,
        page_help: &str,
    ) -> Result<(PageHandle, RowHandle), String> {
        let label = wide(label);
        let help = wide(help);
        let page_title = wide(page_title);
        let page_help = wide(page_help);
        let desc = SubmenuDesc {
            size: std::mem::size_of::<SubmenuDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            page_title: Utf16View::of(&page_title),
            page_help: Utf16View::of(&page_help),
            enabled: 1,
            reserved: 0,
        };
        let mut child: PageHandle = 0;
        let mut row: RowHandle = 0;
        let status =
            unsafe { (self.api.add_submenu.unwrap())(self.provider, parent, &desc, &mut child, &mut row) };
        if status != ERUI_OK {
            return Err(format!("add_submenu: {}", result_name(status)));
        }
        Ok((child, row))
    }

    /// Adds a slider row to `page`. `step` and the range are integers; the host owns the
    /// byte value and reports player changes through `callback`.
    pub fn add_slider(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        minimum: i32,
        maximum: i32,
        step: i32,
        initial: u8,
        callback: ValueChangedCallback,
        user_data: *mut c_void,
    ) -> Result<RowHandle, String> {
        let label = wide(label);
        let help = wide(help);
        let desc = SliderDesc {
            size: std::mem::size_of::<SliderDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            changed_callback: Some(callback),
            user_data,
            minimum,
            maximum,
            step,
            initial_value: initial,
            reserved8: [0; 3],
            enabled: 1,
        };
        let mut row: RowHandle = 0;
        let status = unsafe { (self.api.add_slider.unwrap())(self.provider, page, &desc, &mut row) };
        if status != ERUI_OK {
            return Err(format!("add_slider: {}", result_name(status)));
        }
        Ok(row)
    }

    /// Adds a row cycled with left/right through `options` (1-32 labels); the
    /// value is the zero-based index of the selected label.
    pub fn add_inline_choice(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        options: &[&str],
        initial_index: u8,
        callback: ValueChangedCallback,
        user_data: *mut c_void,
    ) -> Result<RowHandle, String> {
        let label = wide(label);
        let help = wide(help);
        let option_text: Vec<Vec<u16>> = options.iter().map(|o| wide(o)).collect();
        let views: Vec<Utf16View> = option_text.iter().map(|o| Utf16View::of(o)).collect();
        let desc = ChoiceDesc {
            size: std::mem::size_of::<ChoiceDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            changed_callback: Some(callback),
            user_data,
            options: views.as_ptr(),
            option_count: views.len() as u32,
            initial_index,
            reserved8: [0; 3],
        };
        let mut row: RowHandle = 0;
        let status = unsafe { (self.api.add_inline_choice.unwrap())(self.provider, page, &desc, &mut row) };
        if status != ERUI_OK {
            return Err(format!("add_inline_choice: {}", result_name(status)));
        }
        Ok(row)
    }

    /// Adds a row that opens the game's text editor; the callback gets the
    /// confirmed text when it changed. `maximum_length` is 1..=35 UTF-16 units.
    pub fn add_text_input(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        initial: &str,
        placeholder: &str,
        maximum_length: u32,
        callback: TextChangedCallback,
        user_data: *mut c_void,
    ) -> Result<RowHandle, String> {
        let label = wide(label);
        let help = wide(help);
        let initial = wide(initial);
        let placeholder = wide(placeholder);
        let desc = TextInputDesc {
            size: std::mem::size_of::<TextInputDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            initial_value: Utf16View::of(&initial),
            placeholder: Utf16View::of(&placeholder),
            changed_callback: Some(callback),
            user_data,
            maximum_length,
            reserved: 0,
        };
        let mut row: RowHandle = 0;
        let status = unsafe { (self.api.add_text_input.unwrap())(self.provider, page, &desc, &mut row) };
        if status != ERUI_OK {
            return Err(format!("add_text_input: {}", result_name(status)));
        }
        Ok(row)
    }

    /// Adds a text row for a number, kept in range and written to the config
    /// like any other [`Setting`]: the player types `1.5` (a comma works too),
    /// out-of-range values are clamped and bad text is put back to the last
    /// good value. `initial` is the number the row starts with.
    pub fn add_number(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        setting: &'static NumberSetting,
        initial: f64,
    ) -> Result<RowHandle, String> {
        let initial = if setting.integer { initial.round() } else { initial };
        let initial = initial.clamp(setting.min, setting.max);
        setting.last.store(initial.to_bits(), Ordering::Relaxed);
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

    /// Adds an on/off row to `page`.
    pub fn add_toggle(
        &self,
        page: PageHandle,
        label: &str,
        help: &str,
        initial: bool,
        callback: ValueChangedCallback,
        user_data: *mut c_void,
    ) -> Result<RowHandle, String> {
        let label = wide(label);
        let help = wide(help);
        let desc = ToggleDesc {
            size: std::mem::size_of::<ToggleDesc>() as u32,
            flags: 0,
            label: Utf16View::of(&label),
            help: Utf16View::of(&help),
            changed_callback: Some(callback),
            user_data,
            initial_value: initial as u8,
            reserved8: [0; 3],
            enabled: 1,
        };
        let mut row: RowHandle = 0;
        let status = unsafe { (self.api.add_toggle.unwrap())(self.provider, page, &desc, &mut row) };
        if status != ERUI_OK {
            return Err(format!("add_toggle: {}", result_name(status)));
        }
        Ok(row)
    }

    /// Commits the provider; blocks until the host finished its native
    /// installation (or failed). Keep the returned [`Registration`] to push
    /// values to rows later.
    pub fn commit(mut self) -> Result<Registration, String> {
        let status = unsafe { (self.api.commit_provider.unwrap())(self.provider) };
        if status != ERUI_OK {
            // The host already dropped the draft; nothing to abort.
            self.committed = true;
            return Err(format!("commit_provider: {}", result_name(status)));
        }
        self.committed = true;
        let registration = Registration { api: self.api, provider: self.provider };
        // Callbacks (see `NumberSetting`) need it to put text back.
        let _ = REGISTRATION.set(registration);
        Ok(registration)
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        if !self.committed {
            unsafe { (self.api.abort_provider.unwrap())(self.provider) };
        }
    }
}

/// A committed provider. Values written here do not fire the row's change
/// callback and reach the screen asynchronously.
#[derive(Clone, Copy)]
pub struct Registration {
    api: Api,
    provider: ProviderHandle,
}

static REGISTRATION: OnceLock<Registration> = OnceLock::new();

impl Registration {
    /// Replaces a text row's value (no callback); the screen catches up shortly after.
    pub fn set_text(&self, row: RowHandle, text: &str) -> Result<(), String> {
        let units = wide(text);
        let view = Utf16View::of(&units);
        let status = unsafe { (self.api.set_text_input_value.unwrap())(self.provider, row, &view) };
        if status == ERUI_OK { Ok(()) } else { Err(format!("set_text_input_value: {}", result_name(status))) }
    }

    pub fn set_row_value(&self, row: RowHandle, value: u8) -> Result<(), String> {
        let status = unsafe { (self.api.set_row_value.unwrap())(self.provider, row, value) };
        if status == ERUI_OK { Ok(()) } else { Err(format!("set_row_value: {}", result_name(status))) }
    }

    pub fn get_row_value(&self, row: RowHandle) -> Result<u8, String> {
        let mut value = 0u8;
        let status = unsafe { (self.api.get_row_value.unwrap())(self.provider, row, &mut value) };
        if status == ERUI_OK { Ok(value) } else { Err(format!("get_row_value: {}", result_name(status))) }
    }
}

// ---------------------------------------------------------------------------
// Settings saved back to the ini
// ---------------------------------------------------------------------------

/// One menu row bound to an ini key. Pass it as the row's `user_data` (via
/// [`Setting::user_data`]) together with [`setting_changed`] as the callback:
/// a change is applied at once with `apply` (if any) and written to the ini
/// by the saver started with [`start_saver`].
pub struct Setting {
    /// The ini key, e.g. `"Multiplier"`.
    pub key: &'static str,
    /// The row's byte as the text stored in the ini.
    pub format: fn(u8) -> String,
    /// Runs on the host's worker thread right when the player changes the row
    /// (keep it short); `None` when the mod only needs the saved value.
    pub apply: Option<fn(u8)>,
}

impl Setting {
    /// The `user_data` pointer for the row; `self` must be a `static`.
    pub fn user_data(&'static self) -> *mut c_void {
        self as *const Setting as *mut c_void
    }
}

type Pending = (&'static str, String);

static SAVER: OnceLock<Sender<Pending>> = OnceLock::new();

/// How long the player must stop changing rows before the ini is written.
const SAVE_DELAY: Duration = Duration::from_millis(1000);

/// Starts the thread that writes menu changes into `ini_path` (and the shared
/// config map) once the player has stopped for a second, then calls
/// `on_saved` (e.g. to tell the mod to re-read its config). The thread sleeps
/// until there is something to save. Call once, before registering rows.
pub fn start_saver(ini_path: String, on_saved: Option<fn()>) {
    start_saver_with(
        move |changes| {
            // Also updates the shared config map, which a mod (and the log
            // switch, see `logger`) reads from.
            config::set_values(&ini_path, changes)
        },
        on_saved,
    );
}

/// [`start_saver`] for a mod with its own way of saving, e.g. a TOML config:
/// `write` gets the changed `(key, text)` pairs (the latest per key) and
/// returns whether they were saved. `Setting::key` is then whatever `write`
/// expects (`"Section.Key"` for [`crate::toml_config::TomlConfig::save_values`]).
pub fn start_saver_with(mut write: impl FnMut(&[(&'static str, String)]) -> bool + Send + 'static, on_saved: Option<fn()>) {
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
                logger::log(&format!("In-game menu: saved {changes:?}."));
            } else {
                logger::log("In-game menu: couldn't save the config; the change applies until the game closes.");
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
pub unsafe extern "C" fn setting_changed(user_data: *mut c_void, value: u8) {
    let _ = std::panic::catch_unwind(|| {
        if user_data.is_null() {
            return;
        }
        let setting = unsafe { &*(user_data as *const Setting) };
        if let Some(apply) = setting.apply {
            apply(value);
        }
        if let Some(tx) = SAVER.get() {
            let _ = tx.send((setting.key, (setting.format)(value)));
        }
    });
}

// ---------------------------------------------------------------------------
// Numbers typed in a text row
// ---------------------------------------------------------------------------

/// Longest number text the row accepts (UTF-16 units; the host allows 35).
const NUMBER_MAX_LENGTH: u32 = 12;

/// One text row holding a number bound to an ini key (or `"Section.Key"` for a
/// TOML mod, see [`start_saver_with`]). Declare it as a `static` and add the
/// row with [`Provider::add_number`].
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
    /// Write whole numbers with a decimal point (`2.0`), for a TOML float
    /// field; an ini just takes `2`.
    pub decimal_point: bool,
    /// The mod reads the key as a whole number (`config::get_int`): typed
    /// decimals are rounded, since `1.5` would not parse and the mod would
    /// fall back to its default.
    pub integer: bool,
}

impl NumberSetting {
    pub const fn new(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        NumberSetting { key, min, max, apply, row: OnceLock::new(), last: AtomicU64::new(0), decimal_point: false, integer: false }
    }

    /// Like [`NumberSetting::new`] for a key the mod reads as a whole number.
    pub const fn whole(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        NumberSetting { key, min, max, apply, row: OnceLock::new(), last: AtomicU64::new(0), decimal_point: false, integer: true }
    }

    /// Like [`NumberSetting::new`] for a TOML float field: `2` is shown and
    /// saved as `2.0`, so the file keeps its style.
    pub const fn float(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        NumberSetting { key, min, max, apply, row: OnceLock::new(), last: AtomicU64::new(0), decimal_point: true, integer: false }
    }

    /// The text shown in the row and written to the config for `value`.
    fn text(&self, value: f64) -> String {
        let text = number_text(value);
        if self.decimal_point && !text.contains('.') { format!("{text}.0") } else { text }
    }
}

impl NumberSetting {
    /// Shows `value` (clamped) in the row, e.g. after the config was reloaded.
    pub fn show(&'static self, registration: &Registration, value: f64) -> Result<(), String> {
        let value = if self.integer { value.round() } else { value };
        let value = value.clamp(self.min, self.max);
        self.last.store(value.to_bits(), Ordering::Relaxed);
        match self.row.get() {
            Some(row) => registration.set_text(*row, &self.text(value)),
            None => Ok(()),
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
        let setting = unsafe { &*(user_data as *const NumberSetting) };
        let typed = unsafe { &*change }.text();
        let accepted = parse_number(&typed).map(|v| {
            let v = if setting.integer { v.round() } else { v };
            v.clamp(setting.min, setting.max)
        });
        let value = match accepted {
            Some(v) => v,
            None => f64::from_bits(setting.last.load(Ordering::Relaxed)),
        };
        // Show the number actually in use (clamped, normalised, or the old one
        // when the text was not a number).
        let shown = setting.text(value);
        if shown != typed.trim() {
            if let (Some(registration), Some(row)) = (REGISTRATION.get(), setting.row.get()) {
                let _ = registration.set_text(*row, &shown);
            }
        }
        if accepted.is_none() {
            return;
        }
        setting.last.store(value.to_bits(), Ordering::Relaxed);
        if let Some(apply) = setting.apply {
            apply(value);
        }
        if let Some(tx) = SAVER.get() {
            let _ = tx.send((setting.key, shown));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
