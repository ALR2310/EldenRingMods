//! The connection to the ERNativeUI host: finding it, negotiating the API version, registering
//! a provider, adding rows and committing.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::abi::*;

/// A live connection to the ERNativeUI host.
#[derive(Clone, Copy)]
pub struct Connection {
    pub(super) api: Api,
    pub(super) numeric_slider: bool,
}

/// Polls for the already-loaded `ERNativeUI.dll` and negotiates API 1.2,
/// falling back to 1.1 (no numeric slider: [`super::NumberSetting`] rows are typed).
/// Returns `Err` with a human-readable reason when the host is missing, failed
/// or does not become ready within `timeout`; callers should log it and carry
/// on without a menu.
pub fn connect(timeout: Duration) -> Result<Connection, String> {
    let deadline = Instant::now() + timeout;
    let host_name = wide("ERNativeUI.dll\0");
    let mut saw_host = false;
    loop {
        // SAFETY: `host_name` is a NUL-terminated wide string.
        let host = unsafe { GetModuleHandleW(host_name.as_ptr()) };
        if !host.is_null() {
            saw_host = true;
            // SAFETY: `host` is a loaded module and the name is a C string literal.
            let symbol = unsafe { GetProcAddress(host, c"ERUI_GetApi".as_ptr() as *const u8) };
            if symbol.is_null() {
                return Err("ERNativeUI.dll does not export ERUI_GetApi".into());
            }
            // SAFETY: `ERUI_GetApi` has the `GetApiFn` signature (erui.h).
            let get_api: GetApiFn = unsafe { std::mem::transmute(symbol) };
            let mut version = API_VERSION_1_2;
            // SAFETY: `Api` holds integers and nullable function pointers, for which all-zero is valid.
            let mut api: Api = unsafe { std::mem::zeroed() };
            api.size = API_V1_2_SIZE;
            // SAFETY: `api` is writable and `api.size` tells the host how much it may fill.
            let mut status = unsafe { get_api(version, &mut api) };
            if status == ERUI_UNSUPPORTED_VERSION {
                // An API 1.1 host (ERNativeUI before 1.2).
                version = API_VERSION_1_1;
                // SAFETY: `Api` holds integers and nullable function pointers, for which all-zero is valid.
                api = unsafe { std::mem::zeroed() };
                api.size = API_V1_1_SIZE;
                // SAFETY: `api` is writable and `api.size` tells the host how much it may fill.
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
                other => {
                    return Err(format!(
                        "ERNativeUI API negotiation failed: {}",
                        result_name(other)
                    ));
                }
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
    pub(super) api: Api,
    pub(super) provider: ProviderHandle,
    pub(super) root: PageHandle,
    pub(super) numeric_slider: bool,
    pub(super) committed: bool,
}

impl Connection {
    /// `provider_id`: 1-255 ASCII letters, digits, `.`, `_`, `-`; unique
    /// process-wide. `owner_module`: the mod's own `HMODULE` (the `hmodule`
    /// passed to `DllMain`), which the host pins so row callbacks stay valid.
    pub fn register_provider(
        &self,
        provider_id: &str,
        display_name: &str,
        owner_module: u64,
    ) -> Result<Provider, String> {
        let name = wide(display_name);
        let desc = ProviderDesc {
            size: std::mem::size_of::<ProviderDesc>() as u32,
            api_version: self.api.api_version,
            flags: 0,
            root_priority: 100,
            owner_module: owner_module as *mut c_void,
            provider_id: StringView {
                data: provider_id.as_ptr(),
                length: provider_id.len() as u32,
                reserved: 0,
            },
            display_name: Utf16View::of(&name),
        };
        let mut provider: ProviderHandle = 0;
        let mut root: PageHandle = 0;
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status =
            unsafe { (self.api.register_provider.unwrap())(&desc, &mut provider, &mut root) };
        check(status, "register_provider")?;
        Ok(Provider {
            api: self.api,
            provider,
            root,
            numeric_slider: self.numeric_slider,
            committed: false,
        })
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
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status = unsafe {
            (self.api.add_submenu.unwrap())(self.provider, parent, &desc, &mut child, &mut row)
        };
        check(status, "add_submenu")?;
        Ok((child, row))
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
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status =
            unsafe { (self.api.add_text_input.unwrap())(self.provider, page, &desc, &mut row) };
        check(status, "add_text_input")?;
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
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status =
            unsafe { (self.api.add_toggle.unwrap())(self.provider, page, &desc, &mut row) };
        check(status, "add_toggle")?;
        Ok(row)
    }

    /// Commits the provider; blocks until the host finished its native
    /// installation (or failed). Keep the returned [`Registration`] to push
    /// values to rows later.
    pub fn commit(mut self) -> Result<Registration, String> {
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
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
            // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
            unsafe { (self.api.abort_provider.unwrap())(self.provider) };
        }
    }
}

/// A committed provider. Values written here do not fire the row's change
/// callback and reach the screen asynchronously.
#[derive(Clone, Copy)]
pub struct Registration {
    pub(super) api: Api,
    pub(super) provider: ProviderHandle,
}

pub(super) static REGISTRATION: OnceLock<Registration> = OnceLock::new();

impl Registration {
    /// Replaces a text row's value (no callback); the screen catches up shortly after.
    pub fn set_text(&self, row: RowHandle, text: &str) -> Result<(), String> {
        let units = wide(text);
        let view = Utf16View::of(&units);
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status = unsafe { (self.api.set_text_input_value.unwrap())(self.provider, row, &view) };
        check(status, "set_text_input_value")
    }

    /// Moves a numeric slider to `scaled / 10^places` (no callback).
    pub(super) fn set_numeric(
        &self,
        row: RowHandle,
        scaled: i64,
        places: u32,
    ) -> Result<(), String> {
        let value = DecimalValue { scaled_value: scaled, decimal_places: places, reserved: 0 };
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status =
            unsafe { (self.api.set_numeric_slider_value.unwrap())(self.provider, row, &value) };
        check(status, "set_numeric_slider_value")
    }

    pub fn set_row_value(&self, row: RowHandle, value: u8) -> Result<(), String> {
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status = unsafe { (self.api.set_row_value.unwrap())(self.provider, row, value) };
        check(status, "set_row_value")
    }

    pub fn get_row_value(&self, row: RowHandle) -> Result<u8, String> {
        let mut value = 0u8;
        // SAFETY: the function pointers were checked by `validate_api`; the descriptor and the strings it views outlive the call.
        let status = unsafe { (self.api.get_row_value.unwrap())(self.provider, row, &mut value) };
        check(status, "get_row_value").map(|()| value)
    }
}
