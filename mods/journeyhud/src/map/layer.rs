// The window logic follows kubilaiswf-minimap (MIT, Copyright (c) 2026 kubilaiswf, see
// THIRD-PARTY-NOTICES.txt), `minimap.rs`. See docs/map.md.
//! The map layer of the minimap: asks the loader for a 3x3 cell window of tiles
//! around the player, uploads a finished one as a texture and hands it out to be
//! drawn. hudhook leaks the staging buffer of every upload, so a new window is
//! only requested when the player nears the edge of the one shown, and it is
//! only uploaded while the game is in normal play (on loading screens the upload
//! can wait on a GPU queue that waits on the game).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use hudhook::RenderContext;
use hudhook::imgui::TextureId;

use common::logger;

use super::mfg;
use super::sprites::{self, Image, SpriteRef};
use super::tiles::{Loader, MapId, TILE, WINDOW, map_pixel};

/// The game's own map icons the minimap draws marks with.
#[derive(Clone, Copy)]
pub enum Icon {
    /// A site of grace.
    Grace,
    /// The player's own cursor: a needle that points the way, turning on its ring.
    Player,
    /// Another player's cursor, same shape.
    Partner,
    /// The figure inside the ring of the player's cursor.
    PlayerEmblem,
    /// The figure inside the ring of another player's cursor.
    PartnerEmblem,
    /// A summoned spirit.
    Spirit,
    /// The locked-on enemy.
    Enemy,
    /// A beacon: a thin beam whose lower tip is the spot.
    Beacon,
    /// The status buildup icons, in the order of `target::BUILDUP`.
    Poison,
    ScarletRot,
    Bleed,
    DeathBlight,
    Frostbite,
    Sleep,
    Madness,
}

impl Icon {
    /// The icon of buildup `kind` (an index into `target::BUILDUP`).
    pub fn buildup(kind: usize) -> Option<Icon> {
        [Icon::Poison, Icon::ScarletRot, Icon::Bleed, Icon::DeathBlight, Icon::Frostbite, Icon::Sleep, Icon::Madness]
            .get(kind)
            .copied()
    }
}

const ICON_COUNT: usize = 15;
/// The icons from this one on (the buildup ones) are trimmed to their art.
const BUILDUP_ICONS_FROM: usize = Icon::Poison as usize;

/// Where each icon is in the game's menu sprite sheets, in the order of [Icon].
const ICON_SPRITES: [SpriteRef; ICON_COUNT] = [
    SpriteRef { sheet: "SB_MapCursor_02", name: "MENU_MAP_01_Bonfire" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Player_01" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Coop_01" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Player_02" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Coop_02" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Friend_00" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Enemy_00" },
    SpriteRef { sheet: "SB_MapCursor", name: "MENU_MAP_Marker" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Poison" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Plague" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_bleed" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Death" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Cold" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Sleep" },
    SpriteRef { sheet: "SB_Condition", name: "MENU_RE_Mad" },
];

/// An uploaded icon and its size in pixels.
#[derive(Clone, Copy)]
pub struct IconTexture {
    pub texture: TextureId,
    pub size: [f32; 2],
}

/// One world map icon in the atlas the icons are packed into.
#[derive(Clone, Copy)]
pub struct MapIcon {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Size of the game's sprite in pixels; the atlas holds it shrunk, same proportions.
    pub size: [f32; 2],
}

struct MapIcons {
    texture: TextureId,
    cells: HashMap<u16, MapIcon>,
}

/// The atlas of world map icons is this many pixels on a side.
const ATLAS: u32 = 1024;
/// Longest side the icons are shrunk to, from the sharpest that fits the atlas to the smallest.
const ICON_SIDES: [u32; 4] = [96, 80, 64, 48];
const ICON_PADDING: u32 = 2;

/// The window a request asks for: the map, its south-west cell and the fragments shown.
type Wanted = (MapId, [i32; 2], u32);

pub struct MapLayer {
    loader: Loader,
    texture: Option<TextureId>,
    /// The window in the texture: its map and south-west cell.
    shown: Option<(MapId, [i32; 2])>,
    requested: Option<Wanted>,
    /// The sprites read on a background thread, until they are uploaded (or it is clear they
    /// never will be).
    /// In a `Mutex` only because the render loop must be `Sync`; `&mut self` reaches it unlocked.
    sprites_pending: Mutex<Option<Receiver<sprites::Sprites>>>,
    icons: Option<[IconTexture; ICON_COUNT]>,
    map_icons: Option<MapIcons>,
    status_icons: Option<StatusIcons>,
    goblins: Goblins,
}

impl MapLayer {
    /// Starts reading the map archives of the game in `game_dir` in the background. If they
    /// cannot be read no window is ever produced and the minimap stays without a map.
    pub fn new(game_dir: &Path) -> Self {
        logger::log("Map: loader started");
        let (sender, sprites_pending) = channel();
        let dir = game_dir.to_owned();
        std::thread::spawn(move || match sprites::load(&dir, &ICON_SPRITES) {
            Ok(sprites) => {
                let _ = sender.send(sprites);
            }
            Err(e) => logger::warn(&format!("Map: sprites not read ({e}), drawn marks instead")),
        });
        MapLayer {
            loader: Loader::spawn(game_dir),
            texture: None,
            shown: None,
            requested: None,
            sprites_pending: Mutex::new(Some(sprites_pending)),
            icons: None,
            map_icons: None,
            status_icons: None,
            goblins: Goblins::Waiting,
        }
    }

    /// Asks for a window around `position` on `map` when there is none, when it is for
    /// another map, when the fragments to show changed, or when the player is within `margin`
    /// meters of its edge.
    pub fn follow(&mut self, map: MapId, position: [f32; 2], margin: f32, fragments: u32) {
        let pixel = map_pixel(map, position);
        let reload = match self.requested {
            // A newly found fragment redraws the window in place.
            Some((requested_map, origin, shown)) if requested_map == map => {
                shown != fragments
                    || (0..2).any(|axis| {
                        let offset = pixel[axis] - (origin[axis] * TILE as i32) as f32;
                        !(margin..WINDOW as f32 - margin).contains(&offset)
                    })
            }
            _ => true,
        };
        if reload {
            let cell = pixel.map(|p| (p / TILE as f32).floor() as i32);
            let wanted = (map, [cell[0] - 1, cell[1] - 1], fragments);
            self.loader.request(wanted);
            self.requested = Some(wanted);
        }
    }

    /// Uploads the newest finished window, if any. Only call it in normal play.
    pub fn upload(&mut self, ctx: &mut dyn RenderContext) {
        self.upload_sprites(ctx);
        self.upload_goblins(ctx);
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

impl MapLayer {
    /// Uploads the sprites once they are read, and stops asking once they are or cannot be.
    fn upload_sprites(&mut self, ctx: &mut dyn RenderContext) {
        let pending = self.sprites_pending.get_mut().unwrap_or_else(|e| e.into_inner());
        let Some(receiver) = pending.as_ref() else { return };
        let sprites = match receiver.try_recv() {
            Ok(sprites) => sprites,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                *pending = None;
                return;
            }
        };
        *pending = None;
        match sprites.icons {
            Ok(images) => self.upload_icons(ctx, &images),
            Err(e) => logger::warn(&format!("Map: icons not read ({e}), drawn marks instead")),
        }
        match sprites.map_icons {
            Ok(icons) => self.upload_map_icons(ctx, &icons),
            Err(e) => logger::warn(&format!("Map: point icons not read ({e})")),
        }
        match sprites.status_icons {
            Ok(icons) => self.upload_status_icons(ctx, &icons),
            Err(e) => logger::warn(&format!("Effects: status icons not read ({e})")),
        }
    }

    /// Uploads the icons (all of them or none).
    fn upload_icons(&mut self, ctx: &mut dyn RenderContext, images: &[Image]) {
        let uploaded: Vec<IconTexture> = images
            .iter()
            .enumerate()
            .filter_map(|(index, image)| {
                // The buildup icons are sized for their art, without the margin of the sprite.
                let image = if index >= BUILDUP_ICONS_FROM {
                    image.trimmed(24)
                } else {
                    Image { width: image.width, height: image.height, rgba: image.rgba.clone() }
                };
                let texture = ctx.load_texture(&image.rgba, image.width, image.height).ok()?;
                Some(IconTexture { texture, size: [image.width as f32, image.height as f32] })
            })
            .collect();
        match <[IconTexture; ICON_COUNT]>::try_from(uploaded) {
            Ok(icons) => {
                logger::log("Map: icons uploaded");
                self.icons = Some(icons);
            }
            Err(_) => logger::warn("Map: icons could not be uploaded, drawn marks instead"),
        }
    }

    /// The texture of a game icon, once it is read and uploaded.
    pub fn icon(&self, icon: Icon) -> Option<IconTexture> {
        self.icons.map(|icons| icons[icon as usize])
    }
}

impl MapLayer {
    /// Packs the world map's point icons into one texture and uploads it.
    fn upload_map_icons(&mut self, ctx: &mut dyn RenderContext, icons: &[(u16, Image)]) {
        let Some((rgba, cells)) = ICON_SIDES.iter().find_map(|&side| pack(icons, side)) else {
            logger::warn("Map: point icons do not fit the atlas");
            return;
        };
        match ctx.load_texture(&rgba, ATLAS, ATLAS) {
            Ok(texture) => {
                logger::log(&format!("Map: {} point icon(s) uploaded", cells.len()));
                self.map_icons = Some(MapIcons { texture, cells });
            }
            Err(e) => logger::warn(&format!("Map: point icons could not be uploaded ({e:?})")),
        }
    }

    /// The atlas texture and the cell of the world map icon `id` (a `WorldMapPointParam` icon id).
    pub fn map_icon(&self, id: u16) -> Option<(TextureId, MapIcon)> {
        let icons = self.map_icons.as_ref()?;
        Some((icons.texture, *icons.cells.get(&id)?))
    }
}

/// The status icons of the effects in one texture, a grid of `STATUS_CELL` pixel cells.
struct StatusIcons {
    texture: TextureId,
    cells: HashMap<u32, MapIcon>,
}

/// The game's status icons are 40 px.
const STATUS_CELL: u32 = 40;
const STATUS_COLUMNS: u32 = 16;

impl MapLayer {
    fn upload_status_icons(&mut self, ctx: &mut dyn RenderContext, icons: &[(u32, Image)]) {
        let items: Vec<(u32, &Image)> = icons.iter().map(|(id, image)| (*id, image)).collect();
        let Some(grid) = pack_grid(&items, STATUS_CELL, STATUS_COLUMNS) else {
            logger::warn("Effects: no status icons to upload");
            return;
        };
        match ctx.load_texture(&grid.rgba, grid.width, grid.height) {
            Ok(texture) => {
                logger::log(&format!("Effects: {} status icon(s) uploaded", grid.cells.len()));
                self.status_icons = Some(StatusIcons { texture, cells: grid.cells });
            }
            Err(e) => logger::warn(&format!("Effects: status icons could not be uploaded ({e:?})")),
        }
    }

    /// The atlas texture and the cell of the status icon `id` (`SpEffectParam.iconId`).
    pub fn status_icon(&self, id: i32) -> Option<(TextureId, MapIcon)> {
        let icons = self.status_icons.as_ref()?;
        Some((icons.texture, *icons.cells.get(&u32::try_from(id).ok()?)?))
    }
}

/// Where the icons of Map for Goblins are: not asked for yet, being decoded, or uploaded (`None`
/// when there are none to show).
enum Goblins {
    Waiting,
    Decoding(Mutex<Receiver<Vec<mfg::Icon>>>),
    Ready(Option<GoblinIcons>),
}

/// The icons of Map for Goblins in one texture, a grid of `GOBLIN_CELL` pixel cells.
struct GoblinIcons {
    texture: TextureId,
    cells: HashMap<u32, MapIcon>,
}

/// Map for Goblins draws its icons from art up to 96 px.
const GOBLIN_CELL: u32 = 96;
const GOBLIN_COLUMNS: u32 = 16;

impl MapLayer {
    /// Decodes the icons of Map for Goblins once its DLL answers (it may load after this one),
    /// on a thread of their own, and uploads them. Nothing happens without the DLL or with the
    /// layer switched off.
    fn upload_goblins(&mut self, ctx: &mut dyn RenderContext) {
        match &self.goblins {
            Goblins::Waiting => {
                if crate::settings::minimap_goblins() && mfg::available() {
                    let (sender, receiver) = channel();
                    std::thread::spawn(move || {
                        let _ = sender.send(mfg::icons());
                    });
                    self.goblins = Goblins::Decoding(Mutex::new(receiver));
                }
            }
            Goblins::Decoding(receiver) => {
                let icons = match receiver.lock().unwrap_or_else(|e| e.into_inner()).try_recv() {
                    Ok(icons) => icons,
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => Vec::new(),
                };
                let uploaded = pack_goblins(&icons).and_then(|grid| {
                    let texture = ctx.load_texture(&grid.rgba, grid.width, grid.height).ok()?;
                    logger::log(&format!("Map: {} Map for Goblins icon(s) uploaded", grid.cells.len()));
                    Some(GoblinIcons { texture, cells: grid.cells })
                });
                if uploaded.is_none() {
                    logger::warn("Map: Map for Goblins icons not available");
                }
                self.goblins = Goblins::Ready(uploaded);
            }
            Goblins::Ready(_) => {}
        }
    }

    /// The atlas texture and the cell of the Map for Goblins icon `id`.
    pub fn goblin_icon(&self, id: u32) -> Option<(TextureId, MapIcon)> {
        match &self.goblins {
            Goblins::Ready(Some(icons)) => Some((icons.texture, *icons.cells.get(&id)?)),
            _ => None,
        }
    }
}

/// Lays the icons that fit a cell out in a grid.
/// The RGBA texture of the grid, its size and each icon's cell.
struct Grid {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    cells: HashMap<u32, MapIcon>,
}

fn pack_goblins(icons: &[mfg::Icon]) -> Option<Grid> {
    let items: Vec<(u32, &Image)> = icons.iter().map(|icon| (icon.id, &icon.image)).collect();
    pack_grid(&items, GOBLIN_CELL, GOBLIN_COLUMNS)
}

/// Lays the images that fit a `cell` pixel square out in a grid of `columns` columns, each in its
/// own cell, by id.
fn pack_grid(items: &[(u32, &Image)], cell: u32, columns: u32) -> Option<Grid> {
    let fitting: Vec<&(u32, &Image)> = items.iter().filter(|(_, image)| image.width <= cell && image.height <= cell).collect();
    if fitting.is_empty() {
        return None;
    }
    let width = cell * columns;
    let height = cell * (fitting.len() as u32).div_ceil(columns);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let mut cells = HashMap::new();
    for (i, (id, image)) in fitting.iter().map(|item| (item.0, item.1)).enumerate() {
        let (x, y) = (i as u32 % columns * cell, i as u32 / columns * cell);
        for row in 0..image.height {
            let from = (row * image.width * 4) as usize;
            let to = (((y + row) * width + x) * 4) as usize;
            rgba[to..to + (image.width * 4) as usize].copy_from_slice(&image.rgba[from..from + (image.width * 4) as usize]);
        }
        let uv = |px: u32, py: u32| [px as f32 / width as f32, py as f32 / height as f32];
        cells.insert(
            id,
            MapIcon {
                uv_min: uv(x, y),
                uv_max: uv(x + image.width, y + image.height),
                size: [image.width as f32, image.height as f32],
            },
        );
    }
    Some(Grid { rgba, width, height, cells })
}

/// Shrinks every icon to at most `side` pixels and packs them in rows into an `ATLAS` square
/// (RGBA), padded so filtering does not bleed one into the next. `None` if they do not fit.
fn pack(icons: &[(u16, Image)], side: u32) -> Option<(Vec<u8>, HashMap<u16, MapIcon>)> {
    let mut shrunk: Vec<(u16, [f32; 2], Image)> =
        icons.iter().map(|(id, image)| (*id, [image.width as f32, image.height as f32], image.shrunk(side))).collect();
    shrunk.sort_by_key(|(_, _, image)| std::cmp::Reverse(image.height));
    let mut rgba = vec![0u8; (ATLAS * ATLAS * 4) as usize];
    let mut cells = HashMap::new();
    let (mut x, mut y, mut row) = (ICON_PADDING, ICON_PADDING, 0);
    for (id, size, image) in &shrunk {
        if x + image.width + ICON_PADDING > ATLAS {
            (x, y, row) = (ICON_PADDING, y + row + ICON_PADDING, 0);
        }
        if y + image.height + ICON_PADDING > ATLAS {
            return None;
        }
        for line in 0..image.height {
            let from = (line * image.width * 4) as usize;
            let to = (((y + line) * ATLAS + x) * 4) as usize;
            rgba[to..to + (image.width * 4) as usize].copy_from_slice(&image.rgba[from..from + (image.width * 4) as usize]);
        }
        let uv = |px: u32, py: u32| [px as f32 / ATLAS as f32, py as f32 / ATLAS as f32];
        cells.insert(*id, MapIcon { uv_min: uv(x, y), uv_max: uv(x + image.width, y + image.height), size: *size });
        x += image.width + ICON_PADDING;
        row = row.max(image.height);
    }
    Some((rgba, cells))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side: u32, shade: u8) -> Image {
        Image { width: side, height: side, rgba: [shade, 0, 0, 255].repeat((side * side) as usize) }
    }

    #[test]
    fn icons_are_packed_without_overlapping_and_keep_their_original_size() {
        let icons = vec![(4, square(40, 10)), (5, square(300, 20)), (6, square(40, 30))];
        let (rgba, cells) = pack(&icons, 100).unwrap();
        assert_eq!(cells.len(), 3);
        // The 300 px icon is shrunk to 100 px in the atlas but reports its original size.
        let big = cells[&5];
        assert_eq!(big.size, [300.0, 300.0]);
        assert!(((big.uv_max[0] - big.uv_min[0]) * ATLAS as f32 - 100.0).abs() < 0.5);
        // Every cell holds its own shade at its top-left pixel, so none was overwritten.
        for (id, shade) in [(4, 10), (5, 20), (6, 30)] {
            let cell = cells[&id];
            let at = (((cell.uv_min[1] * ATLAS as f32) as u32 * ATLAS + (cell.uv_min[0] * ATLAS as f32) as u32) * 4) as usize;
            assert_eq!(rgba[at], shade, "icon {id}");
        }
    }

    #[test]
    fn goblin_icons_sit_in_their_own_cells_and_oversized_ones_are_left_out() {
        let icon = |id, side| mfg::Icon { id, image: square(side, id as u8) };
        let Grid { rgba, width, cells, .. } = pack_goblins(&[icon(1, 40), icon(2, 96), icon(3, 200)]).unwrap();
        assert_eq!(cells.len(), 2);
        // The second icon starts in the second cell of the first row.
        assert!((cells[&2].uv_min[0] * width as f32 - GOBLIN_CELL as f32).abs() < 0.5);
        assert_eq!(rgba[0], 1);
        assert_eq!(rgba[(GOBLIN_CELL * 4) as usize], 2);
        assert!(pack_goblins(&[icon(3, 200)]).is_none());
    }

    #[test]
    fn icons_that_do_not_fit_are_refused() {
        let icons: Vec<_> = (0..400u16).map(|id| (id, square(90, 1))).collect();
        assert!(pack(&icons, 96).is_none());
    }
}
