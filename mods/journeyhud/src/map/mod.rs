//! The game's own world map under the minimap: tiles read from the game's
//! archives (`archive`, `formats`, `tiles`), where the player is on that map
//! (`world`) and the layer that draws it (`layer`). Details: `docs/map.md`.

mod archive;
pub mod beacons;
mod cache;
mod formats;
pub mod graces;
pub mod layer;
pub mod mfg;
pub mod points;
mod reader;
pub mod sprites;
pub mod tiles;
pub mod world;
