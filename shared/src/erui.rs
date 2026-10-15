//! Client for ERNativeUI (https://github.com/Flammrock/ERNativeUI, MIT), the
//! shared native in-game settings menu host. Mirrors the part of its C ABI
//! (`erui.h`, API 1.1, plus the API 1.2 numeric slider) that mods here need. The host is an optional
//! dependency: [`connect`] fails cleanly when it is not loaded and the mod
//! keeps working from its config file.
//!
//! Host rules that shape this module (registration window, byte-only values,
//! callback threads, supported game versions): `docs/ingame_menu.md`. In
//! short: register early from a worker thread, keep callbacks short, never
//! panic across the ABI. A number row ([`NumberSetting`]) is a native slider
//! when the setting has a step and the host has API 1.2, else a text row where
//! the number is typed (decimals and negatives work in both).
//!
//! A mod normally needs only [`spawn_menu`] + [`MenuBuilder`], with
//! [`Setting`] / [`NumberSetting`] rows and [`start_saver`] to persist them.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use crate::{config, logger};

/// `ERUI_Result`.
type Status = u32;
const ERUI_OK: u32 = 0;
const ERUI_HOST_NOT_READY: u32 = 2;
const ERUI_HOST_FAILED: u32 = 3;
const ERUI_UNSUPPORTED_VERSION: u32 = 4;

const API_VERSION_1_1: u32 = (1 << 16) | 1;
const API_VERSION_1_2: u32 = (1 << 16) | 2;
const CAP_TOGGLE: u64 = 1 << 0;
const CAP_SLIDER: u64 = 1 << 1;
const CAP_NUMERIC_SLIDER: u64 = 1 << 16;

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

/// `ERUI_DecimalValue`: `scaled_value / 10^decimal_places`.
#[repr(C)]
struct DecimalValue {
    scaled_value: i64,
    decimal_places: u32,
    reserved: u32,
}

/// What the host passes to a numeric-slider callback
/// (`ERUI_NumericSliderChangeContext`).
#[repr(C)]
pub struct NumericSliderChange {
    size: u32,
    flags: u32,
    provider: u64,
    row: u64,
    value: DecimalValue,
    index: u32,
    value_count: u32,
    reserved: [u32; 2],
}

/// Called when the player moves a numeric slider (host worker thread).
pub type NumericChangedCallback = unsafe extern "C" fn(user_data: *mut c_void, change: *const NumericSliderChange);

#[repr(C)]
struct NumericSliderDesc {
    size: u32,
    flags: u32,
    label: Utf16View,
    help: Utf16View,
    changed_callback: Option<NumericChangedCallback>,
    user_data: *mut c_void,
    minimum_scaled_value: i64,
    maximum_scaled_value: i64,
    step_scaled_value: i64,
    initial_scaled_value: i64,
    decimal_places: u32,
    enabled: u32,
    prefix: Utf16View,
    suffix: Utf16View,
    reserved: [u32; 2],
}

/// `ERUI_GameLanguageInfo`: the host's effective language.
#[repr(C)]
struct GameLanguageInfo {
    size: u32,
    known_language: u32,
    identifier: StringView,
    reserved: [u32; 2],
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

impl NumericSliderChange {
    /// The slider's new value.
    fn value(&self) -> f64 {
        self.value.scaled_value as f64 / 10f64.powi(self.value.decimal_places as i32)
    }
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

/// `ERUI_Api` (API 1.2). Field order is the ABI; entries this module does not
/// call are kept as opaque [`Unused`] pointers so the table has the exact size
/// the host expects (`ERUI_API_V1_2_SIZE`; an API 1.1 host only fills the first
/// `ERUI_API_V1_1_SIZE` bytes).
#[repr(C)]
#[derive(Clone, Copy)]
struct Api {
    size: u32,
    api_version: u32,
    capabilities: u64,
    register_provider: Option<
        unsafe extern "C" fn(*const ProviderDesc, *mut ProviderHandle, *mut PageHandle) -> Status,
    >,
    add_button: Unused,
    add_toggle: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const ToggleDesc, *mut RowHandle) -> Status,
    >,
    add_slider: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const SliderDesc, *mut RowHandle) -> Status,
    >,
    add_inline_choice: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const ChoiceDesc, *mut RowHandle) -> Status,
    >,
    add_popup_choice: Unused,
    add_submenu: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const SubmenuDesc,
            *mut PageHandle,
            *mut RowHandle,
        ) -> Status,
    >,
    set_page_presentation: Unused,
    commit_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Status>,
    abort_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Status>,
    set_row_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, u8) -> Status>,
    get_row_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *mut u8) -> Status>,
    enqueue_alert: Unused,
    get_game_language: Option<unsafe extern "C" fn(*mut GameLanguageInfo) -> Status>,
    add_text_input: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const TextInputDesc, *mut RowHandle) -> Status,
    >,
    set_text_input_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *const Utf16View) -> Status>,
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
    add_numeric_slider: Option<
        unsafe extern "C" fn(ProviderHandle, PageHandle, *const NumericSliderDesc, *mut RowHandle) -> Status,
    >,
    set_numeric_slider_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *const DecimalValue) -> Status>,
    get_numeric_slider_value: Unused,
    add_custom_slider: Unused,
    set_custom_slider_index: Unused,
    get_custom_slider_index: Unused,
}

// ERUI_API_V1_1_SIZE: 16-byte header + 38 function pointers; API 1.2 adds 6.
const API_V1_1_SIZE: u32 = 16 + 38 * 8;
const API_V1_2_SIZE: u32 = 16 + 44 * 8;
const _: () = assert!(std::mem::size_of::<Api>() == API_V1_2_SIZE as usize);

type GetApiFn = unsafe extern "C" fn(u32, *mut Api) -> Status;

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

/// `Ok` for `ERUI_OK`, else `"<what>: <reason>"`.
fn check(status: Status, what: &str) -> Result<(), String> {
    if status == ERUI_OK { Ok(()) } else { Err(format!("{what}: {}", result_name(status))) }
}

/// The checks the host's docs ask a strict-C client to make on the table it
/// returned: exact version and size, the capability bits, and every function
/// pointer this module calls (they are `unwrap`ped later). `version` is the one
/// requested.
fn validate_api(api: &Api, version: u32) -> Result<(), String> {
    let size = if version == API_VERSION_1_2 { API_V1_2_SIZE } else { API_V1_1_SIZE };
    if api.api_version != version
        || api.size != size
        || api.capabilities & (CAP_TOGGLE | CAP_SLIDER) != (CAP_TOGGLE | CAP_SLIDER)
    {
        return Err(format!("ERNativeUI returned an incompatible API {} table", version_name(version)));
    }
    let complete = api.register_provider.is_some()
        && api.add_slider.is_some()
        && api.add_toggle.is_some()
        && api.add_submenu.is_some()
        && api.add_inline_choice.is_some()
        && api.add_text_input.is_some()
        && api.set_text_input_value.is_some()
        && api.commit_provider.is_some()
        && api.abort_provider.is_some()
        && api.set_row_value.is_some()
        && api.get_row_value.is_some();
    if complete {
        Ok(())
    } else {
        Err(format!("ERNativeUI returned an incomplete API {} table", version_name(version)))
    }
}

fn version_name(version: u32) -> String {
    format!("{}.{}", version >> 16, version & 0xFFFF)
}

/// Tells [`crate::i18n`] the host's language (exact identifier, e.g.
/// `"vietnamese"`), so menu text can be translated. Anything unavailable leaves
/// the English default.
fn select_language(api: &Api) {
    let Some(get_language) = api.get_game_language else { return };
    let mut info = GameLanguageInfo {
        size: std::mem::size_of::<GameLanguageInfo>() as u32,
        known_language: 0,
        identifier: StringView { data: std::ptr::null(), length: 0, reserved: 0 },
        reserved: [0; 2],
    };
    if unsafe { get_language(&mut info) } != ERUI_OK || info.identifier.data.is_null() {
        return;
    }
    let bytes = unsafe { std::slice::from_raw_parts(info.identifier.data, info.identifier.length as usize) };
    if let Ok(identifier) = std::str::from_utf8(bytes) {
        crate::i18n::set_language(identifier);
        logger::log(&format!("In-game menu language: {identifier}."));
    }
}

/// Whether the negotiated table can build numeric sliders (API 1.2).
fn has_numeric_slider(api: &Api) -> bool {
    api.api_version >= API_VERSION_1_2
        && api.capabilities & CAP_NUMERIC_SLIDER != 0
        && api.add_numeric_slider.is_some()
        && api.set_numeric_slider_value.is_some()
}

/// A live connection to the ERNativeUI host.
#[derive(Clone, Copy)]
pub struct Connection {
    api: Api,
    numeric_slider: bool,
}

/// Polls for the already-loaded `ERNativeUI.dll` and negotiates API 1.2,
/// falling back to 1.1 (no numeric slider: [`NumberSetting`] rows are typed).
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
            let mut version = API_VERSION_1_2;
            let mut api: Api = unsafe { std::mem::zeroed() };
            api.size = API_V1_2_SIZE;
            let mut status = unsafe { get_api(version, &mut api) };
            if status == ERUI_UNSUPPORTED_VERSION {
                // An API 1.1 host (ERNativeUI before 1.2).
                version = API_VERSION_1_1;
                api = unsafe { std::mem::zeroed() };
                api.size = API_V1_1_SIZE;
                status = unsafe { get_api(version, &mut api) };
            }
            match status {
                ERUI_OK => {
                    validate_api(&api, version)?;
                    select_language(&api);
                    return Ok(Connection { api, numeric_slider: has_numeric_slider(&api) });
                }
                ERUI_HOST_FAILED => {
                    return Err("ERNativeUI host initialization failed (see ERNativeUI.log)".into());
                }
                ERUI_HOST_NOT_READY => {}
                other => return Err(format!("ERNativeUI API negotiation failed: {}", result_name(other))),
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
    numeric_slider: bool,
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
        check(status, "register_provider")?;
        Ok(Provider { api: self.api, provider, root, numeric_slider: self.numeric_slider, committed: false })
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
        check(status, "add_submenu")?;
        Ok((child, row))
    }

    /// Adds a slider row to `page`. `step` and the range are integers; the host owns the
    /// byte value and reports player changes through `callback`.
    #[allow(clippy::too_many_arguments)]
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
        check(status, "add_slider")?;
        Ok(row)
    }

    /// Adds a row cycled with left/right through `options` (1-32 labels); the
    /// value is the zero-based index of the selected label.
    #[allow(clippy::too_many_arguments)]
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
        check(status, "add_inline_choice")?;
        Ok(row)
    }

    /// Adds a row that opens the game's text editor; the callback gets the
    /// confirmed text when it changed. `maximum_length` is 1..=35 UTF-16 units.
    #[allow(clippy::too_many_arguments)]
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
        check(status, "add_text_input")?;
        Ok(row)
    }

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
            let row = self.add_numeric_slider(page, label, help, &plan, plan.snap(initial), setting)?;
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
        let status = unsafe { (self.api.add_numeric_slider.unwrap())(self.provider, page, &desc, &mut row) };
        check(status, "add_numeric_slider")?;
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
        check(status, "add_toggle")?;
        Ok(row)
    }

    /// Commits the provider; blocks until the host finished its native
    /// installation (or failed). Keep the returned [`Registration`] to push
    /// values to rows later.
    pub fn commit(mut self) -> Result<Registration, String> {
        let status = unsafe { (self.api.commit_provider.unwrap())(self.provider) };
        // A rejected commit (e.g. a callback outside this module) leaves the
        // draft in the host; Drop aborts it so it cannot stall the host's startup.
        check(status, "commit_provider")?;
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
        check(status, "set_text_input_value")
    }

    /// Moves a numeric slider to `scaled / 10^places` (no callback).
    fn set_numeric(&self, row: RowHandle, scaled: i64, places: u32) -> Result<(), String> {
        let value = DecimalValue { scaled_value: scaled, decimal_places: places, reserved: 0 };
        let status = unsafe { (self.api.set_numeric_slider_value.unwrap())(self.provider, row, &value) };
        check(status, "set_numeric_slider_value")
    }

    pub fn set_row_value(&self, row: RowHandle, value: u8) -> Result<(), String> {
        let status = unsafe { (self.api.set_row_value.unwrap())(self.provider, row, value) };
        check(status, "set_row_value")
    }

    pub fn get_row_value(&self, row: RowHandle) -> Result<u8, String> {
        let mut value = 0u8;
        let status = unsafe { (self.api.get_row_value.unwrap())(self.provider, row, &mut value) };
        check(status, "get_row_value").map(|()| value)
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
    /// A toggle row stored as `true` / `false`.
    pub const fn flag(key: &'static str) -> Self {
        Setting { key, format: format_bool, apply: None }
    }

    /// A choice or slider row stored as its byte (`0`, `1`, ...).
    pub const fn int(key: &'static str) -> Self {
        Setting { key, format: format_int, apply: None }
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

/// `format` for a choice or slider: the byte as a number.
pub fn format_int(value: u8) -> String {
    value.to_string()
}

/// Set by [`reload_from_menu`] and consumed by the next [`Menu::sync`], so a
/// re-apply that the menu itself asked for does not put every row back to the
/// config's values while the player is already editing another one.
static MENU_RELOAD: AtomicBool = AtomicBool::new(false);

/// For `on_saved` of a mod that re-applies on `RELOAD_GENERATION`: asks the mod
/// to re-apply like the reload key does, without the menu re-reading its rows.
pub fn reload_from_menu() {
    MENU_RELOAD.store(true, Ordering::Release);
    crate::reload::RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
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
            // switch, see `logger`) reads from, so a failed write still applies
            // until the game closes.
            let saved = config::set_values(&ini_path, changes);
            if !saved {
                logger::log("In-game menu: couldn't write the ini; the change applies until the game closes.");
            }
            saved
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
                logger::log("In-game menu: couldn't save the config.");
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
// A mod's whole menu: build once, follow config reloads
// ---------------------------------------------------------------------------

type ByteReader = fn() -> u8;
type NumberReader = fn() -> f64;

/// A committed menu: every row with the function that reads its value from the
/// mod's config, so [`Menu::sync`] can show a reloaded config.
pub struct Menu {
    registration: Registration,
    rows: Vec<(RowHandle, ByteReader)>,
    numbers: Vec<(&'static NumberSetting, NumberReader)>,
}

impl Menu {
    /// Shows the current config values in every row (call after a config reload).
    /// Skipped once when the reload was asked for by the menu itself
    /// ([`reload_from_menu`]): the rows already hold what the player typed.
    pub fn sync(&self) {
        if MENU_RELOAD.swap(false, Ordering::AcqRel) {
            return;
        }
        for (row, read) in &self.rows {
            if let Err(reason) = self.registration.set_row_value(*row, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
        for (setting, read) in &self.numbers {
            if let Err(reason) = setting.show(&self.registration, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}

/// Collects the rows of one mod's menu; every row added here is remembered with
/// its reader for [`Menu::sync`].
pub struct MenuBuilder {
    provider: Provider,
    rows: Vec<(RowHandle, ByteReader)>,
    numbers: Vec<(&'static NumberSetting, NumberReader)>,
}

impl MenuBuilder {
    /// The mod's row on System > Game Options is the parent of its top page.
    pub fn root(&self) -> PageHandle {
        self.provider.root_page()
    }

    /// The underlying provider, for a row the helpers below do not cover.
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// A row on `parent` that opens a page titled `name`; returns that page.
    pub fn submenu(&self, parent: PageHandle, name: &str, help: &str) -> Result<PageHandle, String> {
        Ok(self.provider.add_submenu(parent, name, help, name, help)?.0)
    }

    /// An on/off row; `read` gives its current value (`0` or `1`) from the config.
    pub fn toggle(&mut self, page: PageHandle, label: &str, help: &str, setting: &'static Setting, read: ByteReader) -> Result<(), String> {
        let row = self.provider.add_toggle(page, label, help, read() != 0, setting_changed, setting.user_data())?;
        self.rows.push((row, read));
        Ok(())
    }

    /// A left/right row over `options` (1-32 labels); the value is the index.
    pub fn choice(
        &mut self,
        page: PageHandle,
        label: &str,
        help: &str,
        options: &[&str],
        setting: &'static Setting,
        read: ByteReader,
    ) -> Result<(), String> {
        let row = self.provider.add_inline_choice(page, label, help, options, read(), setting_changed, setting.user_data())?;
        self.rows.push((row, read));
        Ok(())
    }

    /// A slider over the whole numbers `min..=max` (step 1, at most 255).
    #[allow(clippy::too_many_arguments)]
    pub fn slider(
        &mut self,
        page: PageHandle,
        label: &str,
        help: &str,
        min: i32,
        max: i32,
        setting: &'static Setting,
        read: ByteReader,
    ) -> Result<(), String> {
        let row = self.provider.add_slider(page, label, help, min, max, 1, read(), setting_changed, setting.user_data())?;
        self.rows.push((row, read));
        Ok(())
    }

    /// A text row for a number (see [`NumberSetting`]); `read` gives the config's value.
    pub fn number(&mut self, page: PageHandle, label: &str, help: &str, setting: &'static NumberSetting, read: NumberReader) -> Result<(), String> {
        self.provider.add_number(page, label, help, setting, read())?;
        self.numbers.push((setting, read));
        Ok(())
    }
}

/// Registers a mod's menu on a worker thread and stores it in `slot` once the
/// host has installed it. Call it very early in the mod's startup thread: the
/// host only accepts registrations in its first seconds. Without the host (or on
/// an unsupported game build) it logs one line and the mod runs from its config
/// file exactly as before. `build` adds the rows (typically a submenu on
/// [`MenuBuilder::root`] with everything inside).
pub fn spawn_menu(
    slot: &'static OnceLock<Menu>,
    hmodule: u64,
    provider_id: &'static str,
    display_name: &'static str,
    build: fn(&mut MenuBuilder) -> Result<(), String>,
) {
    std::thread::spawn(move || {
        let connection = match connect(Duration::from_secs(10)) {
            Ok(connection) => connection,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the config file only."));
                return;
            }
        };
        let result = (|| {
            let provider = connection.register_provider(provider_id, display_name, hmodule)?;
            let mut builder = MenuBuilder { provider, rows: Vec::new(), numbers: Vec::new() };
            build(&mut builder)?;
            let MenuBuilder { provider, rows, numbers } = builder;
            let registration = provider.commit()?;
            Ok::<_, String>(Menu { registration, rows, numbers })
        })();
        match result {
            Ok(menu) => {
                let _ = slot.set(menu);
                logger::log("In-game menu: registered.");
            }
            Err(reason) => logger::log(&format!("In-game menu registration failed: {reason}")),
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
    /// Step of the native slider (`0` = text row only, see [`NumberSetting::stepped`]).
    pub step: f64,
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
        let index = (((value * scale) - self.min as f64) / self.step as f64).round().clamp(0.0, last as f64) as i64;
        self.min + index * self.step
    }
}

impl NumberSetting {
    const fn make(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>, decimal_point: bool, integer: bool) -> Self {
        NumberSetting {
            key,
            min,
            max,
            apply,
            row: OnceLock::new(),
            last: AtomicU64::new(0),
            decimal_point,
            integer,
            step: 0.0,
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
        let plan = SliderPlan {
            places,
            min: (self.min * scale).round() as i64,
            max: (self.max * scale).round() as i64,
            step: (self.step * scale).round() as i64,
        };
        let span = plan.max - plan.min;
        let positions = span / plan.step + 1;
        (plan.step > 0 && span % plan.step == 0 && (2..=256).contains(&positions)).then_some(plan)
    }

    pub const fn new(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        Self::make(key, min, max, apply, false, false)
    }

    /// Like [`NumberSetting::new`] for a key the mod reads as a whole number.
    pub const fn whole(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        Self::make(key, min, max, apply, false, true)
    }

    /// Like [`NumberSetting::new`] for a TOML float field: `2` is shown and
    /// saved as `2.0`, so the file keeps its style.
    pub const fn float(key: &'static str, min: f64, max: f64, apply: Option<fn(f64)>) -> Self {
        Self::make(key, min, max, apply, true, false)
    }

    /// `value` as the row accepts it: rounded for a whole-number key, then
    /// clamped to the range.
    fn normalize(&self, value: f64) -> f64 {
        let value = if self.integer { value.round() } else { value };
        value.clamp(self.min, self.max)
    }

    /// The text shown in the row and written to the config for `value`.
    fn text(&self, value: f64) -> String {
        let text = number_text(value);
        if self.decimal_point && !text.contains('.') { format!("{text}.0") } else { text }
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
        let setting = unsafe { &*(user_data as *const NumberSetting) };
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

unsafe extern "C" fn number_slider_changed(user_data: *mut c_void, change: *const NumericSliderChange) {
    let _ = std::panic::catch_unwind(|| {
        if user_data.is_null() || change.is_null() {
            return;
        }
        let setting = unsafe { &*(user_data as *const NumberSetting) };
        let value = unsafe { &*change }.value();
        accept_number(setting, setting.normalize(value));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_settings_describe_an_exact_slider() {
        let speed = NumberSetting::float("A", 0.1, 10.0, None).stepped(0.1);
        let plan = speed.slider_plan().unwrap();
        assert_eq!(plan, SliderPlan { places: 1, min: 1, max: 100, step: 1 });
        assert_eq!(plan.snap(1.2), 12);
        assert_eq!(plan.snap(1.23), 12);
        assert_eq!(plan.snap(99.0), 100);
        assert_eq!(plan.snap(-5.0), 1);

        let fine = NumberSetting::new("B", -1.0, 1.0, None).stepped(0.25);
        assert_eq!(fine.slider_plan().unwrap(), SliderPlan { places: 2, min: -100, max: 100, step: 25 });

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
        let plain = NumberSetting::new("A", 0.0, 10.0, None);
        assert_eq!(plain.normalize(1.5), 1.5);
        assert_eq!(plain.normalize(99.0), 10.0);
        assert_eq!(plain.normalize(-1.0), 0.0);
        assert_eq!(plain.text(2.0), "2");

        let whole = NumberSetting::whole("B", -5.0, 5.0, None);
        assert_eq!(whole.normalize(1.5), 2.0); // rounded: `1.5` would not parse as an int
        assert_eq!(whole.normalize(-9.0), -5.0);

        let float = NumberSetting::float("C", 0.0, 10.0, None);
        assert_eq!(float.text(2.0), "2.0"); // keeps a TOML float field a float
        assert_eq!(float.text(0.5), "0.5");
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
