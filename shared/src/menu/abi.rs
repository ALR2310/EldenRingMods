//! The part of ERNativeUI's C ABI (`erui.h`) that the menu client uses: descriptor structs, the
//! function table, and the checks the host's docs ask of a strict-C client.

use std::ffi::c_void;

use crate::logger;

/// `ERUI_Result`.
pub(super) type Status = u32;
pub(super) const ERUI_OK: u32 = 0;
pub(super) const ERUI_HOST_NOT_READY: u32 = 2;
pub(super) const ERUI_HOST_FAILED: u32 = 3;
pub(super) const ERUI_UNSUPPORTED_VERSION: u32 = 4;

pub(super) const API_VERSION_1_1: u32 = (1 << 16) | 1;
pub(super) const API_VERSION_1_2: u32 = (1 << 16) | 2;
pub(super) const CAP_TOGGLE: u64 = 1 << 0;
pub(super) const CAP_SLIDER: u64 = 1 << 1;
pub(super) const CAP_NUMERIC_SLIDER: u64 = 1 << 16;

pub type RowHandle = u64;
pub(super) type ProviderHandle = u64;
pub type PageHandle = u64;

#[repr(C)]
pub(super) struct StringView {
    pub(super) data: *const u8,
    pub(super) length: u32,
    pub(super) reserved: u32,
}

#[repr(C)]
pub(super) struct Utf16View {
    pub(super) data: *const u16,
    pub(super) length: u32,
    pub(super) reserved: u32,
}

impl Utf16View {
    pub(super) fn of(units: &[u16]) -> Self {
        Utf16View { data: units.as_ptr(), length: units.len() as u32, reserved: 0 }
    }
}

#[repr(C)]
pub(super) struct ProviderDesc {
    pub(super) size: u32,
    pub(super) api_version: u32,
    pub(super) flags: u32,
    pub(super) root_priority: i32,
    pub(super) owner_module: *mut c_void,
    pub(super) provider_id: StringView,
    pub(super) display_name: Utf16View,
}

/// Called when the player moves a slider. Runs on the host's worker thread.
pub type ValueChangedCallback = unsafe extern "C" fn(user_data: *mut c_void, value: u8);

#[repr(C)]
pub(super) struct ToggleDesc {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) label: Utf16View,
    pub(super) help: Utf16View,
    pub(super) changed_callback: Option<ValueChangedCallback>,
    pub(super) user_data: *mut c_void,
    pub(super) initial_value: u8,
    pub(super) reserved8: [u8; 3],
    pub(super) enabled: u32,
}

#[repr(C)]
pub(super) struct SubmenuDesc {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) label: Utf16View,
    pub(super) help: Utf16View,
    pub(super) page_title: Utf16View,
    pub(super) page_help: Utf16View,
    pub(super) enabled: u32,
    pub(super) reserved: u32,
}

/// `ERUI_DecimalValue`: `scaled_value / 10^decimal_places`.
#[repr(C)]
pub(super) struct DecimalValue {
    pub(super) scaled_value: i64,
    pub(super) decimal_places: u32,
    pub(super) reserved: u32,
}

/// What the host passes to a numeric-slider callback
/// (`ERUI_NumericSliderChangeContext`).
#[repr(C)]
pub struct NumericSliderChange {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) provider: u64,
    pub(super) row: u64,
    pub(super) value: DecimalValue,
    pub(super) index: u32,
    pub(super) value_count: u32,
    pub(super) reserved: [u32; 2],
}

/// Called when the player moves a numeric slider (host worker thread).
pub type NumericChangedCallback =
    unsafe extern "C" fn(user_data: *mut c_void, change: *const NumericSliderChange);

#[repr(C)]
pub(super) struct NumericSliderDesc {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) label: Utf16View,
    pub(super) help: Utf16View,
    pub(super) changed_callback: Option<NumericChangedCallback>,
    pub(super) user_data: *mut c_void,
    pub(super) minimum_scaled_value: i64,
    pub(super) maximum_scaled_value: i64,
    pub(super) step_scaled_value: i64,
    pub(super) initial_scaled_value: i64,
    pub(super) decimal_places: u32,
    pub(super) enabled: u32,
    pub(super) prefix: Utf16View,
    pub(super) suffix: Utf16View,
    pub(super) reserved: [u32; 2],
}

/// `ERUI_GameLanguageInfo`: the host's effective language.
#[repr(C)]
pub(super) struct GameLanguageInfo {
    pub(super) size: u32,
    pub(super) known_language: u32,
    pub(super) identifier: StringView,
    pub(super) reserved: [u32; 2],
}

/// What the host passes to a text-input callback (`ERUI_TextInputChangeContext`).
#[repr(C)]
pub struct TextInputChange {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) provider: u64,
    pub(super) row: u64,
    pub(super) value: Utf16View,
    pub(super) reserved: [u32; 2],
}

impl NumericSliderChange {
    /// The slider's new value.
    pub(super) fn value(&self) -> f64 {
        self.value.scaled_value as f64 / 10f64.powi(self.value.decimal_places as i32)
    }
}

impl TextInputChange {
    /// The confirmed text. Only valid inside the callback.
    pub(super) fn text(&self) -> String {
        if self.value.data.is_null() || self.value.length == 0 {
            return String::new();
        }
        // SAFETY: the host passes a valid UTF-16 buffer of `length` units that lives for the callback.
        let units =
            unsafe { std::slice::from_raw_parts(self.value.data, self.value.length as usize) };
        String::from_utf16_lossy(units)
    }
}

pub type TextChangedCallback =
    unsafe extern "C" fn(user_data: *mut c_void, change: *const TextInputChange);

#[repr(C)]
pub(super) struct TextInputDesc {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) label: Utf16View,
    pub(super) help: Utf16View,
    pub(super) initial_value: Utf16View,
    pub(super) placeholder: Utf16View,
    pub(super) changed_callback: Option<TextChangedCallback>,
    pub(super) user_data: *mut c_void,
    pub(super) maximum_length: u32,
    pub(super) reserved: u32,
}

pub(super) type Unused = Option<unsafe extern "C" fn()>;

/// `ERUI_Api` (API 1.2). Field order is the ABI; entries this module does not
/// call are kept as opaque [`Unused`] pointers so the table has the exact size
/// the host expects (`ERUI_API_V1_2_SIZE`; an API 1.1 host only fills the first
/// `ERUI_API_V1_1_SIZE` bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Api {
    pub(super) size: u32,
    pub(super) api_version: u32,
    pub(super) capabilities: u64,
    pub(super) register_provider: Option<
        unsafe extern "C" fn(*const ProviderDesc, *mut ProviderHandle, *mut PageHandle) -> Status,
    >,
    pub(super) add_button: Unused,
    pub(super) add_toggle: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const ToggleDesc,
            *mut RowHandle,
        ) -> Status,
    >,
    pub(super) add_slider: Unused,
    pub(super) add_inline_choice: Unused,
    pub(super) add_popup_choice: Unused,
    pub(super) add_submenu: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const SubmenuDesc,
            *mut PageHandle,
            *mut RowHandle,
        ) -> Status,
    >,
    pub(super) set_page_presentation: Unused,
    pub(super) commit_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Status>,
    pub(super) abort_provider: Option<unsafe extern "C" fn(ProviderHandle) -> Status>,
    pub(super) set_row_value: Option<unsafe extern "C" fn(ProviderHandle, RowHandle, u8) -> Status>,
    pub(super) get_row_value:
        Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *mut u8) -> Status>,
    pub(super) enqueue_alert: Unused,
    pub(super) get_game_language: Option<unsafe extern "C" fn(*mut GameLanguageInfo) -> Status>,
    pub(super) add_text_input: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const TextInputDesc,
            *mut RowHandle,
        ) -> Status,
    >,
    pub(super) set_text_input_value:
        Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *const Utf16View) -> Status>,
    pub(super) get_text_input_value: Unused,
    pub(super) add_color_picker: Unused,
    pub(super) set_color_picker_value: Unused,
    pub(super) get_color_picker_value: Unused,
    pub(super) get_builtin_page: Unused,
    pub(super) add_input_section: Unused,
    pub(super) add_input_action: Unused,
    pub(super) set_assignments_changed_handler: Unused,
    pub(super) set_action_inputs: Unused,
    pub(super) get_action_inputs: Unused,
    pub(super) get_action_default_inputs: Unused,
    pub(super) reset_action_inputs: Unused,
    pub(super) open_storage: Unused,
    pub(super) storage_load: Unused,
    pub(super) storage_save: Unused,
    pub(super) storage_get_utf8: Unused,
    pub(super) storage_set_utf8: Unused,
    pub(super) storage_erase: Unused,
    pub(super) storage_get_action_inputs: Unused,
    pub(super) storage_set_action_inputs: Unused,
    pub(super) storage_apply_assignment_changes: Unused,
    pub(super) storage_get_info: Unused,
    pub(super) add_numeric_slider: Option<
        unsafe extern "C" fn(
            ProviderHandle,
            PageHandle,
            *const NumericSliderDesc,
            *mut RowHandle,
        ) -> Status,
    >,
    pub(super) set_numeric_slider_value:
        Option<unsafe extern "C" fn(ProviderHandle, RowHandle, *const DecimalValue) -> Status>,
    pub(super) get_numeric_slider_value: Unused,
    pub(super) add_custom_slider: Unused,
    pub(super) set_custom_slider_index: Unused,
    pub(super) get_custom_slider_index: Unused,
}

// ERUI_API_V1_1_SIZE: 16-byte header + 38 function pointers; API 1.2 adds 6.
pub(super) const API_V1_1_SIZE: u32 = 16 + 38 * 8;
pub(super) const API_V1_2_SIZE: u32 = 16 + 44 * 8;
const _: () = assert!(std::mem::size_of::<Api>() == API_V1_2_SIZE as usize);

pub(super) type GetApiFn = unsafe extern "C" fn(u32, *mut Api) -> Status;

unsafe extern "system" {
    pub(super) fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    pub(super) fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

pub(super) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

pub(super) fn result_name(code: u32) -> String {
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
pub(super) fn check(status: Status, what: &str) -> Result<(), String> {
    if status == ERUI_OK { Ok(()) } else { Err(format!("{what}: {}", result_name(status))) }
}

/// The checks the host's docs ask a strict-C client to make on the table it
/// returned: exact version and size, the capability bits, and every function
/// pointer this module calls (they are `unwrap`ped later). `version` is the one
/// requested.
pub(super) fn validate_api(api: &Api, version: u32) -> Result<(), String> {
    let size = if version == API_VERSION_1_2 { API_V1_2_SIZE } else { API_V1_1_SIZE };
    if api.api_version != version
        || api.size != size
        || api.capabilities & (CAP_TOGGLE | CAP_SLIDER) != (CAP_TOGGLE | CAP_SLIDER)
    {
        return Err(format!(
            "ERNativeUI returned an incompatible API {} table",
            version_name(version)
        ));
    }
    let complete = api.register_provider.is_some()
        && api.add_toggle.is_some()
        && api.add_submenu.is_some()
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

pub(super) fn version_name(version: u32) -> String {
    format!("{}.{}", version >> 16, version & 0xFFFF)
}

/// Tells [`crate::i18n`] the host's language (exact identifier, e.g.
/// `"vietnamese"`), so menu text can be translated. Anything unavailable leaves
/// the English default.
pub(super) fn select_language(api: &Api) {
    let Some(get_language) = api.get_game_language else { return };
    let mut info = GameLanguageInfo {
        size: std::mem::size_of::<GameLanguageInfo>() as u32,
        known_language: 0,
        identifier: StringView { data: std::ptr::null(), length: 0, reserved: 0 },
        reserved: [0; 2],
    };
    // SAFETY: `info` is a correctly sized out-struct and `get_language` is a non-null host function.
    if unsafe { get_language(&mut info) } != ERUI_OK || info.identifier.data.is_null() {
        return;
    }
    // SAFETY: the host filled `identifier` with a UTF-8 buffer of `length` bytes, checked non-null above.
    let bytes = unsafe {
        std::slice::from_raw_parts(info.identifier.data, info.identifier.length as usize)
    };
    if let Ok(identifier) = std::str::from_utf8(bytes) {
        crate::i18n::set_language(identifier);
        logger::log(&format!("Menu: language {identifier}"));
    }
}

/// Whether the negotiated table can build numeric sliders (API 1.2).
pub(super) fn has_numeric_slider(api: &Api) -> bool {
    api.api_version >= API_VERSION_1_2
        && api.capabilities & CAP_NUMERIC_SLIDER != 0
        && api.add_numeric_slider.is_some()
        && api.set_numeric_slider_value.is_some()
}
