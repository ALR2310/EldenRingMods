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
use std::time::{Duration, Instant};

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
    add_inline_choice: Unused,
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
    add_text_input: Unused,
    set_text_input_value: Unused,
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
        Ok(Registration { api: self.api, provider: self.provider })
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

impl Registration {
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
