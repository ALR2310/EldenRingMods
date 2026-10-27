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

#[cfg(test)]
mod tests {
    use super::tiles::{Loader, MapId, WINDOW};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    /// Reads the installed game's archives and writes the 3x3 cell window around
    /// Limgrave to the PNG named by `ER_MAP_PNG`, for a look. Needs the game:
    /// `ER_GAME_DIR` (default: the Steam install) and `ER_MAP_PNG` set, then
    /// `cargo test -p journeyhud --release -- --ignored map_window --nocapture`.
    #[test]
    #[ignore]
    fn map_window_from_the_installed_game() {
        let dir = PathBuf::from(std::env::var("ER_GAME_DIR").unwrap_or_else(|_| {
            r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game".to_string()
        }));
        let out = std::env::var("ER_MAP_PNG").expect("set ER_MAP_PNG to the PNG to write");
        let started = Instant::now();
        let loader = Loader::spawn(&dir);
        // Map cell (i, j) shows overworld tile m60_{i+28}_{j+24}: Limgrave is around (14, 12).
        loader.request((MapId::Overworld, [13, 11], u32::MAX));
        let window = loop {
            if let Some(window) = loader.poll() {
                break window;
            }
            assert!(started.elapsed() < Duration::from_secs(180), "no window after 180 s");
            std::thread::sleep(Duration::from_millis(200));
        };
        println!("window ready after {:?}", started.elapsed());
        let file = std::fs::File::create(out).unwrap();
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), WINDOW as u32, WINDOW as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&window.rgba).unwrap();
    }
}
