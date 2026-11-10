//! Resizable game window (was the `WindowResize` mod): `window.rs` is the Win32 side,
//! `geometry.rs` the size math. `docs/resize_approach.md`.

mod geometry;
pub mod resolution_list;
mod window;

pub use window::{find_game_window, run};
