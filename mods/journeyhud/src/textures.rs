//! The game's own HUD bar textures, embedded in the DLL (`assets/`) and
//! uploaded to the overlay once. They are grayscale: the bar is tinted to the
//! wanted color when drawn. Details: `docs/design.md`.

use hudhook::RenderContext;
use hudhook::imgui::TextureId;

const BAR: &[u8] = include_bytes!("../assets/Bar.png");
const BACKGROUND: &[u8] = include_bytes!("../assets/BarBG.png");
const EDGE: &[u8] = include_bytes!("../assets/BarEdge.png");
const GLOW: &[u8] = include_bytes!("../assets/BarEdge2.png");
const WAKU: &[u8] = include_bytes!("../assets/BarWaku.png");
const WAKU_LEFT: &[u8] = include_bytes!("../assets/BarWakuL.png");

/// Rows of `Bar.png` / `BarBG.png` that are visible (of 26, the rest is
/// transparent padding), used to size the other textures against the bar.
pub const BAR_ROWS: f32 = 24.0;
/// `BarEdge.png`: the thin divider at the end of the health.
pub const EDGE_SIZE: [f32; 2] = [6.0, 36.0];
/// `BarEdge2.png`: the glow at the end of the health; its bright edge sits
/// `GLOW_TAIL` pixels before the right side.
pub const GLOW_SIZE: [f32; 2] = [98.0, 40.0];
pub const GLOW_TAIL: f32 = 6.0;
/// `BarWaku.png` (840x48): the box the game draws around its buildup gauge (`MENU_FL_ConditionWaku`).
/// A line runs along rows 6 to 12 (centered on 9.2) and another along rows 35 to 41 (centered on
/// 37.7); the cap at the right end is columns 820 to 840, its art ending at column 832.
pub const WAKU_SIZE: [f32; 2] = [840.0, 48.0];
pub const WAKU_TOP_LINE: [f32; 3] = [6.0, 12.0, 9.2];
pub const WAKU_BOTTOM_LINE: [f32; 3] = [35.0, 41.0, 37.7];
pub const WAKU_CAP_FROM: f32 = 820.0;
pub const WAKU_CAP_ART_END: f32 = 832.0;
/// `BarWakuL.png` (12x48): the cap at the left end; its art is columns 5 to 12.
pub const WAKU_LEFT_SIZE: [f32; 2] = [12.0, 48.0];

pub struct Textures {
    pub bar: TextureId,
    pub background: TextureId,
    pub edge: TextureId,
    pub glow: TextureId,
    pub waku: TextureId,
    pub waku_left: TextureId,
}

pub fn load(ctx: &mut dyn RenderContext) -> Result<Textures, String> {
    let mut upload = |name: &str, png: &[u8]| -> Result<TextureId, String> {
        let (rgba, width, height) = decode(png).map_err(|e| format!("{name}: {e}"))?;
        ctx.load_texture(&rgba, width, height).map_err(|e| format!("{name}: {e:?}"))
    };
    Ok(Textures {
        bar: upload("Bar", BAR)?,
        background: upload("BarBG", BACKGROUND)?,
        edge: upload("BarEdge", EDGE)?,
        glow: upload("BarEdge2", GLOW)?,
        waku: upload("BarWaku", WAKU)?,
        waku_left: upload("BarWakuL", WAKU_LEFT)?,
    })
}

/// 8-bit RGBA pixels of a PNG, with its width and height.
fn decode(png_bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png_bytes)).read_info().map_err(|e| e.to_string())?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("expected 8-bit RGBA, got {:?} {:?}", info.color_type, info.bit_depth));
    }
    pixels.truncate(info.buffer_size());
    Ok((pixels, info.width, info.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_pngs_decode_to_the_sizes_the_layout_assumes() {
        for (png, size) in [(BAR, (820, 26)), (BACKGROUND, (820, 26)), (EDGE, (6, 36)), (GLOW, (98, 40)), (WAKU, (840, 48)), (WAKU_LEFT, (12, 48))] {
            let (pixels, w, h) = decode(png).unwrap();
            assert_eq!((w, h), size);
            assert_eq!(pixels.len(), (w * h * 4) as usize);
        }
    }
}
