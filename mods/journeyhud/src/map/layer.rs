// The window logic follows kubilaiswf-minimap (MIT, Copyright (c) 2026 kubilaiswf, see
// THIRD-PARTY-NOTICES.txt), `minimap.rs`. See docs/map.md.
//! The map layer of the minimap: asks the loader for a 3x3 cell window of tiles
//! around the player, uploads a finished one as a texture and hands it out to be
//! drawn. hudhook leaks the staging buffer of every upload, so a new window is
//! only requested when the player nears the edge of the one shown, and it is
//! only uploaded while the game is in normal play (on loading screens the upload
//! can wait on a GPU queue that waits on the game).

use std::path::Path;

use hudhook::RenderContext;
use hudhook::imgui::TextureId;

use common::logger;

use super::tiles::{Loader, MapId, TILE, WINDOW, map_pixel};

/// The window a request asks for: the map, its south-west cell and the fragments shown.
type Wanted = (MapId, [i32; 2], u32);

pub struct MapLayer {
    loader: Loader,
    texture: Option<TextureId>,
    /// The window in the texture: its map and south-west cell.
    shown: Option<(MapId, [i32; 2])>,
    requested: Option<Wanted>,
}

impl MapLayer {
    /// Starts reading the map archives of the game in `game_dir` in the background. If they
    /// cannot be read no window is ever produced and the minimap stays without a map.
    pub fn new(game_dir: &Path) -> Self {
        logger::log("Map: loader started");
        MapLayer { loader: Loader::spawn(game_dir), texture: None, shown: None, requested: None }
    }

    /// Asks for a window around `position` on `map` when there is none, when it is for
    /// another map, or when the player is within `margin` meters of its edge.
    pub fn follow(&mut self, map: MapId, position: [f32; 2], margin: f32) {
        let pixel = map_pixel(position);
        let reload = match self.requested {
            Some((requested_map, origin, _)) if requested_map == map => (0..2).any(|axis| {
                let offset = pixel[axis] - (origin[axis] * TILE as i32) as f32;
                !(margin..WINDOW as f32 - margin).contains(&offset)
            }),
            _ => true,
        };
        if reload {
            let cell = pixel.map(|p| (p / TILE as f32).floor() as i32);
            let wanted = (map, [cell[0] - 1, cell[1] - 1], u32::MAX);
            self.loader.request(wanted);
            self.requested = Some(wanted);
        }
    }

    /// Uploads the newest finished window, if any. Only call it in normal play.
    pub fn upload(&mut self, ctx: &mut dyn RenderContext) {
        let Some(window) = self.loader.poll() else { return };
        let size = WINDOW as u32;
        let uploaded = match self.texture {
            Some(texture) => ctx.replace_texture(texture, &window.rgba, size, size).is_ok(),
            None => match ctx.load_texture(&window.rgba, size, size) {
                Ok(texture) => {
                    self.texture = Some(texture);
                    true
                }
                Err(_) => false,
            },
        };
        if uploaded {
            if self.shown.is_none() {
                logger::log("Map: first window uploaded");
            }
            self.shown = Some((window.map, window.origin));
        } else {
            // Ask again next frame instead of waiting until the player nears the edge.
            self.requested = None;
        }
    }

    /// The texture and the south-west cell of its window, if it shows `map`.
    pub fn window(&self, map: MapId) -> Option<(TextureId, [i32; 2])> {
        match (self.texture, self.shown) {
            (Some(texture), Some((shown, origin))) if shown == map => Some((texture, origin)),
            _ => None,
        }
    }
}
