//! The game's own HUD bar textures, embedded in the DLL (`assets/`) and
//! uploaded to the overlay once. They are grayscale: the bar is tinted to the
//! wanted color when drawn. Details: `docs/design.md`.

use hudhook::RenderContext;
use hudhook::imgui::TextureId;

const BAR: &[u8] = include_bytes!("../assets/Bar.png");
const BACKGROUND: &[u8] = include_bytes!("../assets/BarBG.png");
const EDGE: &[u8] = include_bytes!("../assets/BarEdge.png");
const GLOW: &[u8] = include_bytes!("../assets/BarEdge2.png");

/// Rows of `Bar.png` / `BarBG.png` that are visible (of 26, the rest is
/// transparent padding), used to size the other textures against the bar.
pub const BAR_ROWS: f32 = 24.0;
/// `BarEdge.png`: the thin divider at the end of the health.
pub const EDGE_SIZE: [f32; 2] = [6.0, 36.0];
/// `BarEdge2.png`: the glow at the end of the health; its bright edge sits
/// `GLOW_TAIL` pixels before the right side.
pub const GLOW_SIZE: [f32; 2] = [98.0, 40.0];
pub const GLOW_TAIL: f32 = 6.0;

pub struct Textures {
    pub bar: TextureId,
    pub background: TextureId,
    pub edge: TextureId,
    pub glow: TextureId,
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
        for (png, size) in [(BAR, (820, 26)), (BACKGROUND, (820, 26)), (EDGE, (6, 36)), (GLOW, (98, 40))] {
            let (pixels, w, h) = decode(png).unwrap();
            assert_eq!((w, h), size);
            assert_eq!(pixels.len(), (w * h * 4) as usize);
        }
    }
}
