//! Client for ERNativeUI (https://github.com/Flammrock/ERNativeUI, MIT), the shared in-game
//! settings menu host, an optional dependency: [`connect`] fails cleanly when it is not loaded and
//! the mod keeps working from its config file. Register early from a worker thread, keep callbacks
//! short and never panic across the ABI (see `docs/menu.md`). A mod normally needs only
//! [`spawn_menu`] and [`MenuBuilder`], with [`Setting`] and [`NumberSetting`] rows and
//! [`start_saver`] to persist them.

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
