//! Sprites of the game's own menu, read from its archives at run time: a sprite
//! sheet is a texture of the common menu pack (`/menu/hi/01_common.tpf.dcx`) and
//! its layout (`/menu/hi/01_common.sblytbnd.dcx`, `<TextureAtlas>` XML) lists the
//! rectangle of every sprite by name. Used for the world map's own icons (the
//! site of grace, the player and partner cursors). Details: `docs/map.md`.

use std::io;
use std::path::Path;

use super::archive;
use super::formats::Oodle;
use super::reader::{Reader, invalid};

const LAYOUTS: &str = "/menu/hi/01_common.sblytbnd.dcx";
const TEXTURES: &str = "/menu/hi/01_common.tpf.dcx";

/// A sprite to read: the sheet (as in the layout's `imagePath`, without `.png`) and its name.
#[derive(Clone, Copy, Debug)]
pub struct SpriteRef {
    pub sheet: &'static str,
    pub name: &'static str,
}

/// A decoded sprite, RGBA8.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The sheets of the layouts that hold the status icons, called `MENU_StatusIcon_<icon id>`.
const STATUS_ICON_SHEETS: [&str; 4] = ["SB_Status_00", "SB_Status_01", "SB_Status_02", "SB_Status_03"];

/// What the sprites are read from, opened once: the layouts and the texture pack are
/// decompressed a single time and each sheet is decoded at most once.
struct Menu {
    layouts: Vec<u8>,
    tpf: Vec<u8>,
    sheets: Vec<(String, Image)>,
}

impl Menu {
    fn open(game_dir: &Path) -> io::Result<Menu> {
        let oodle = Oodle::load(&game_dir.join("oo2core_6_win64.dll"))?;
        let layouts = oodle.decompress_dcx(&archive::open(game_dir, LAYOUTS)?.read_all()?)?;
        let tpf = oodle.decompress_dcx(&archive::open(game_dir, TEXTURES)?.read_all()?)?;
        Ok(Menu { layouts, tpf, sheets: Vec::new() })
    }

    /// The part `rect` of `sheet`, decoding the sheet the first time it is asked for.
    fn crop(&mut self, sheet: &str, rect: [u32; 4]) -> io::Result<Image> {
        let at = match self.sheets.iter().position(|(name, _)| name == sheet) {
            Some(at) => at,
            None => {
                self.sheets.push((sheet.to_owned(), decode_texture(&self.tpf, sheet)?));
                self.sheets.len() - 1
            }
        };
        crop(&self.sheets[at].1, rect)
    }

    /// `refs`, in the same order (a sprite that cannot be found is an error).
    fn sprites(&mut self, refs: &[SpriteRef]) -> io::Result<Vec<Image>> {
        let rects = refs
            .iter()
            .map(|r| sprite_rect(&self.layouts, r.sheet, r.name).ok_or_else(|| invalid(&format!("no sprite {}/{}", r.sheet, r.name))))
            .collect::<io::Result<Vec<_>>>()?;
        refs.iter().zip(rects).map(|(r, rect)| self.crop(r.sheet, rect)).collect()
    }

    /// Every status icon with its icon id (`SpEffectParam.iconId`).
    fn status_icons(&mut self) -> io::Result<Vec<(u32, Image)>> {
        let mut found: Vec<(u32, &str, [u32; 4])> = Vec::new();
        for sheet in STATUS_ICON_SHEETS {
            for (name, rect) in sheet_sprites(&self.layouts, sheet) {
                let Some(id) = name.strip_prefix("MENU_StatusIcon_").and_then(|n| n.parse::<u32>().ok()) else { continue };
                if !found.iter().any(|(known, ..)| *known == id) {
                    found.push((id, sheet, rect));
                }
            }
        }
        found.into_iter().map(|(id, sheet, rect)| Ok((id, self.crop(sheet, rect)?))).collect()
    }

    /// Every world map icon with its icon id (the sprite is called `MENU_MAP_<id>`; the ids are
    /// the `iconId` of `WorldMapPointParam` rows).
    fn map_icons(&mut self) -> io::Result<Vec<(u16, Image)>> {
        let mut found: Vec<(u16, &str, [u32; 4])> = Vec::new();
        for sheet in MAP_ICON_SHEETS {
            for (name, rect) in sheet_sprites(&self.layouts, sheet) {
                let named = NAMED_MAP_ICONS.iter().find(|(sprite, _)| *sprite == name).map(|(_, id)| *id);
                let Some(id) = named.or_else(|| name.strip_prefix("MENU_MAP_").and_then(|n| n.parse::<u16>().ok())) else { continue };
                if !found.iter().any(|(known, ..)| *known == id) {
                    found.push((id, sheet, rect));
                }
            }
        }
        found.into_iter().map(|(id, sheet, rect)| Ok((id, self.crop(sheet, rect)?))).collect()
    }
}

/// What [load] read. The parts fail on their own: the minimap draws marks instead of the
/// game's icons, or leaves the points out.
pub struct Sprites {
    /// The sprites asked for, in order.
    pub icons: io::Result<Vec<Image>>,
    /// The world map's point icons by icon id.
    pub map_icons: io::Result<Vec<(u16, Image)>>,
    /// The status icons of the effects (`SpEffectParam.iconId`) by icon id.
    pub status_icons: io::Result<Vec<(u32, Image)>>,
}

/// Reads `refs` and the world map's point icons from the game in `game_dir`. Takes a couple of
/// seconds and a few hundred MB while it runs, so call it once on a background thread.
pub fn load(game_dir: &Path, refs: &[SpriteRef]) -> io::Result<Sprites> {
    let mut menu = Menu::open(game_dir)?;
    Ok(Sprites { icons: menu.sprites(refs), map_icons: menu.map_icons(), status_icons: menu.status_icons() })
}

/// The sprites of `sheet` in the layouts: name (without `.png`) and `[x, y, width, height]`.
fn sheet_sprites(layouts: &[u8], sheet: &str) -> Vec<(String, [u32; 4])> {
    let text = String::from_utf8_lossy(layouts);
    let Some(atlas) = text.find(&format!("<TextureAtlas imagePath=\"{sheet}.png\">")) else { return Vec::new() };
    let atlas = &text[atlas..];
    let atlas = &atlas[..atlas.find("</TextureAtlas>").unwrap_or(atlas.len())];
    atlas.lines().filter_map(parse_sub_texture).collect()
}

/// `<SubTexture name="N.png" x=".." y=".." width=".." height=".."/>`.
fn parse_sub_texture(line: &str) -> Option<(String, [u32; 4])> {
    let text = |key: &str| -> Option<&str> {
        let start = line.find(&format!(" {key}=\""))? + key.len() + 3;
        line[start..].split('"').next()
    };
    let number = |key: &str| -> Option<u32> { text(key)?.parse().ok() };
    let name = text("name")?.strip_suffix(".png")?.to_string();
    Some((name, [number("x")?, number("y")?, number("width")?, number("height")?]))
}

/// `[x, y, width, height]` of `name` in the layout of `sheet`.
fn sprite_rect(layouts: &[u8], sheet: &str, name: &str) -> Option<[u32; 4]> {
    sheet_sprites(layouts, sheet).into_iter().find(|(sprite, _)| sprite == name).map(|(_, rect)| rect)
}

/// The sheets of the layouts that hold the world map's icons: the base game's two and the expansion's.
const MAP_ICON_SHEETS: [&str; 3] = ["SB_MapCursor", "SB_MapCursor_02", "SB_MapCursor_03_dlc"];

/// Sprites that carry a name instead of the number of the icon they are for.
const NAMED_MAP_ICONS: [(&str, u16); 1] = [("MENU_MAP_Church", 3)];

impl Image {
    /// The part of this image that is not transparent (alpha above `threshold`); the same image
    /// when it has none. The game's sprites keep a margin around their art, and a size set for the
    /// art alone needs it gone.
    pub fn trimmed(&self, threshold: u8) -> Image {
        let (mut min, mut max) = ([self.width, self.height], [0u32, 0u32]);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.rgba[((y * self.width + x) * 4 + 3) as usize] > threshold {
                    min = [min[0].min(x), min[1].min(y)];
                    max = [max[0].max(x), max[1].max(y)];
                }
            }
        }
        if min[0] > max[0] {
            return Image { width: self.width, height: self.height, rgba: self.rgba.clone() };
        }
        let rect = [min[0], min[1], max[0] - min[0] + 1, max[1] - min[1] + 1];
        crop(self, rect).unwrap_or_else(|_| Image { width: self.width, height: self.height, rgba: self.rgba.clone() })
    }

    /// This image reduced by a whole factor (box filter, alpha-weighted) until neither side is
    /// longer than `max_side`; the same image if it already fits.
    pub fn shrunk(&self, max_side: u32) -> Image {
        let factor = self.width.max(self.height).div_ceil(max_side).max(1);
        if factor == 1 {
            return Image { width: self.width, height: self.height, rgba: self.rgba.clone() };
        }
        let (width, height) = (self.width.div_ceil(factor), self.height.div_ceil(factor));
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let (mut sum, mut alpha, mut count) = ([0u32; 3], 0u32, 0u32);
                for sy in y * factor..((y + 1) * factor).min(self.height) {
                    for sx in x * factor..((x + 1) * factor).min(self.width) {
                        let at = ((sy * self.width + sx) * 4) as usize;
                        let a = u32::from(self.rgba[at + 3]);
                        for c in 0..3 {
                            sum[c] += u32::from(self.rgba[at + c]) * a;
                        }
                        alpha += a;
                        count += 1;
                    }
                }
                if alpha == 0 {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                } else {
                    rgba.extend_from_slice(&[(sum[0] / alpha) as u8, (sum[1] / alpha) as u8, (sum[2] / alpha) as u8, (alpha / count) as u8]);
                }
            }
        }
        Image { width, height, rgba }
    }
}

/// The texture called `name` in a TPF, decoded.
fn decode_texture(tpf: &[u8], name: &str) -> io::Result<Image> {
    let r = Reader(tpf);
    if r.bytes(0, 4)? != b"TPF\0" {
        return Err(invalid("not a TPF"));
    }
    for i in 0..r.u32(0x8)? as usize {
        let entry = 0x10 + i * 0x14;
        if r.utf16(r.u32(entry + 12)? as usize)? == name {
            let dds = Reader(r.bytes(r.u32(entry)? as usize, r.u32(entry + 4)? as usize)?);
            return decode_dds(&dds);
        }
    }
    Err(invalid(&format!("no texture {name}")))
}

/// The top mip of a DDS (BC1, BC3 or BC7), as RGBA8.
fn decode_dds(dds: &Reader<'_>) -> io::Result<Image> {
    if dds.bytes(0, 4)? != b"DDS " {
        return Err(invalid("not a DDS"));
    }
    let (height, width) = (dds.u32(0xC)? as usize, dds.u32(0x10)? as usize);
    let mut pixels = vec![0u32; width * height];
    let result = match (dds.bytes(0x54, 4)?, dds.u32(0x80)?) {
        (b"DX10", 98 | 99) => texture2ddecoder::decode_bc7(&dds.0[0x94..], width, height, &mut pixels),
        (b"DX10", 71 | 72) => texture2ddecoder::decode_bc1(&dds.0[0x94..], width, height, &mut pixels),
        (b"DX10", 77 | 78) => texture2ddecoder::decode_bc3(&dds.0[0x94..], width, height, &mut pixels),
        (b"DXT1", _) => texture2ddecoder::decode_bc1(&dds.0[0x80..], width, height, &mut pixels),
        (b"DXT5", _) => texture2ddecoder::decode_bc3(&dds.0[0x80..], width, height, &mut pixels),
        _ => return Err(invalid("unsupported texture format")),
    };
    result.map_err(invalid)?;
    // The decoder packs BGRA; images are RGBA.
    let rgba = pixels
        .iter()
        .flat_map(|p| {
            let [b, g, r, a] = p.to_le_bytes();
            [r, g, b, a]
        })
        .collect();
    Ok(Image { width: width as u32, height: height as u32, rgba })
}

fn crop(sheet: &Image, [x, y, w, h]: [u32; 4]) -> io::Result<Image> {
    if x + w > sheet.width || y + h > sheet.height {
        return Err(invalid("sprite outside its sheet"));
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let start = ((row * sheet.width + x) * 4) as usize;
        rgba.extend_from_slice(&sheet.rgba[start..start + (w * 4) as usize]);
    }
    Ok(Image { width: w, height: h, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sprite_rect_is_read_from_its_sheets_layout() {
        let layouts = br#"<TextureAtlas imagePath="A.png">
<SubTexture name="ONE.png" x="1" y="2" width="3" height="4" half="0"/>
</TextureAtlas>
<TextureAtlas imagePath="B.png">
<SubTexture name="ONE.png" x="10" y="20" width="30" height="40" half="0"/>
</TextureAtlas>"#;
        assert_eq!(sprite_rect(layouts, "A", "ONE"), Some([1, 2, 3, 4]));
        // The same name on another sheet is another sprite.
        assert_eq!(sprite_rect(layouts, "B", "ONE"), Some([10, 20, 30, 40]));
        assert_eq!(sprite_rect(layouts, "B", "TWO"), None);
        assert_eq!(sprite_rect(layouts, "C", "ONE"), None);
    }

    #[test]
    fn a_sheet_lists_all_its_sprites() {
        let layouts = br#"<TextureAtlas imagePath="A.png">
<SubTexture name="ONE.png" x="1" y="2" width="3" height="4" half="0"/>
<SubTexture name="TWO.png" x="5" y="6" width="7" height="8" half="0"/>
</TextureAtlas>"#;
        let names: Vec<_> = sheet_sprites(layouts, "A").into_iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["ONE", "TWO"]);
        assert!(sheet_sprites(layouts, "B").is_empty());
    }

    #[test]
    fn shrinking_averages_blocks_and_keeps_a_small_image() {
        // 4x2: left half opaque red, right half transparent.
        let mut rgba = Vec::new();
        for _ in 0..2 {
            rgba.extend_from_slice(&[200, 0, 0, 255, 200, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0]);
        }
        let image = Image { width: 4, height: 2, rgba };
        let small = image.shrunk(2);
        assert_eq!((small.width, small.height), (2, 1));
        // The opaque block keeps its color, the empty one stays empty.
        assert_eq!(small.rgba, [200, 0, 0, 255, 0, 0, 0, 0]);
        let same = image.shrunk(8);
        assert_eq!((same.width, same.height), (4, 2));
    }

    #[test]
    fn trimming_keeps_the_part_that_is_not_transparent() {
        // 4x3 with opaque pixels only at (1, 1) and (2, 2).
        let mut rgba = vec![0u8; 4 * 3 * 4];
        for (x, y) in [(1, 1), (2, 2)] {
            rgba[(y * 4 + x) * 4 + 3] = 255;
            rgba[(y * 4 + x) * 4] = 7;
        }
        let image = Image { width: 4, height: 3, rgba }.trimmed(8);
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.rgba[0], 7);
        let empty = Image { width: 2, height: 2, rgba: vec![0; 16] }.trimmed(8);
        assert_eq!((empty.width, empty.height), (2, 2));
    }

    #[test]
    fn crop_takes_the_rectangle_row_by_row() {
        // 3x2 sheet, each pixel's red channel is its index.
        let rgba: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
        let sheet = Image { width: 3, height: 2, rgba };
        let part = crop(&sheet, [1, 0, 2, 2]).unwrap();
        assert_eq!(part.rgba.iter().step_by(4).copied().collect::<Vec<_>>(), vec![1, 2, 4, 5]);
        assert!(crop(&sheet, [2, 0, 2, 1]).is_err());
    }
}

#[cfg(test)]
mod extract {
    use super::*;

    /// Research: writes the HUD movie to `ER_GFX_OUT` for a look at its clip tree. Needs the game
    /// (`ER_GAME_DIR`, default the Steam install). `cargo test -p journeyhud --release -- --ignored hud_movie --nocapture`.
    #[test]
    #[ignore]
    fn hud_movie_from_the_installed_game() {
        let dir = std::path::PathBuf::from(
            std::env::var("ER_GAME_DIR").unwrap_or_else(|_| r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game".to_string()),
        );
        let out = std::env::var("ER_GFX_OUT").expect("set ER_GFX_OUT");
        let oodle = Oodle::load(&dir.join("oo2core_6_win64.dll")).unwrap();
        for path in ["/menu/01_000_fe.gfx.dcx", "/menu/hi/01_000_fe.gfx.dcx", "/menu/01_000_fe.gfx", "/menu/hi/01_000_fe.gfx"] {
            match archive::open(&dir, path) {
                Ok(mut file) => {
                    let raw = file.read_all().unwrap();
                    let data = if path.ends_with(".dcx") { oodle.decompress_dcx(&raw).unwrap() } else { raw };
                    println!("{path}: {} bytes", data.len());
                    std::fs::write(&out, data).unwrap();
                    return;
                }
                Err(e) => println!("{path}: {e}"),
            }
        }
        panic!("no HUD movie found");
    }

    /// Research: writes the HUD bar sprites (`MENU_HP_Base*`, `MENU_FL_*Bar*`...) as PNG files into
    /// `ER_SPRITE_OUT`, one per sprite named `<sheet>__<name>.png`, with their `[x, y, w, h]` printed.
    /// `cargo test -p journeyhud --release -- --ignored hud_bar_sprites --nocapture`.
    #[test]
    #[ignore]
    fn hud_bar_sprites_from_the_installed_game() {
        let dir = std::path::PathBuf::from(
            std::env::var("ER_GAME_DIR").unwrap_or_else(|_| r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game".to_string()),
        );
        let out = std::path::PathBuf::from(std::env::var("ER_SPRITE_OUT").expect("set ER_SPRITE_OUT"));
        std::fs::create_dir_all(&out).unwrap();
        let mut menu = Menu::open(&dir).unwrap();
        let text = String::from_utf8_lossy(&menu.layouts).into_owned();
        let sheets: Vec<String> = text
            .split("<TextureAtlas imagePath=\"")
            .skip(1)
            .filter_map(|part| part.split(".png\"").next().map(str::to_owned))
            .collect();
        let wanted = ["HP_Base", "FL_BarDeco", "FL_HP", "FL_BarEdge", "HP_Bar", "Bar_Loss", "Bar_Regain", "Condition", "CO_", "Gauge", "Waku", "Ruler", "Frame", "_FL_"];
        for sheet in sheets {
            for (name, rect) in sheet_sprites(&menu.layouts, &sheet) {
                if !wanted.iter().any(|w| name.contains(w)) {
                    continue;
                }
                println!("{sheet}/{name}: {rect:?}");
                if let Ok(image) = menu.crop(&sheet, rect) {
                    let file = std::fs::File::create(out.join(format!("{sheet}__{name}.png"))).unwrap();
                    let mut encoder = png::Encoder::new(file, image.width, image.height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
                }
            }
        }
    }
}
