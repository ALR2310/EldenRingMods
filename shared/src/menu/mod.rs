//! Client for ERNativeUI (https://github.com/Flammrock/ERNativeUI, MIT), the shared native
//! in-game settings menu host. It mirrors the part of the host's C ABI (`erui.h`, API 1.1 plus
//! the API 1.2 numeric slider) that the mods need. The host is an optional dependency:
//! [`connect`] fails cleanly when it is not loaded and the mod keeps working from its config
//! file.
//!
//! The host's rules shape this module: register early from a worker thread, keep callbacks
//! short and never panic across the ABI (see `docs/menu.md`). A number row ([`NumberSetting`])
//! is a native slider when the setting has a step and the host has API 1.2, otherwise a text
//! row where the number is typed. Decimals and negatives work in both.
//!
//! A mod normally needs only [`spawn_menu`] and [`MenuBuilder`], with [`Setting`] and
//! [`NumberSetting`] rows and [`start_saver`] to persist them.

mod abi;
mod builder;
mod host;
mod settings;

pub use abi::{
    NumericChangedCallback, NumericSliderChange, PageHandle, RowHandle, TextChangedCallback,
    TextInputChange, ValueChangedCallback,
};
pub use builder::{Menu, MenuBuilder, spawn_menu, sync};
pub use host::{Connection, Provider, Registration, connect};
pub use settings::{NumberSetting, Setting, format_bool, setting_changed, start_saver};
