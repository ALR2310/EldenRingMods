// The API reading and the icon decoding follow kubilaiswf-minimap (MIT, Copyright (c) 2026
// kubilaiswf, see THIRD-PARTY-NOTICES.txt), `mfg.rs`. See docs/map.md.
//! Map for Goblins DLL's map API (`sdk/mfg_map_api.h` of that mod, since its 2.1.6): the markers
//! it shows, where they are and their icons, read from the mod instead of guessed from its param
//! rows. Nothing of its art is shipped: the icons come out of the loaded DLL.

use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use flate2::read::ZlibDecoder;
use hudhook::windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use hudhook::windows::core::{s, w};

use eldenring::cs::BlockId;

use super::cache::Refreshed;
use super::sprites::Image;
use super::tiles::MapId;
use super::world;

const ABI: u32 = 1;
const OK: i32 = 0;
/// Bit 62 of a point's id marks a focus ring (a highlight, not a place).
const FOCUS_RING: u64 = 1 << 62;
/// Looking for the API again after a miss: the DLL may load after this one.
const RETRY: Duration = Duration::from_secs(2);
/// The markers change rarely (a toggle, a piece collected), so the list is rebuilt a few times a
/// second at most.
const RECHECK: Duration = Duration::from_millis(300);

#[repr(C)]
struct RawPoint {
    id: u64,
    /// Stable across Map for Goblins versions, unlike the world map's icon frames.
    icon_id: u32,
    /// Area, grid X and grid Z in the top three bytes, like a block id.
    raw_map_id: u32,
    /// Position in that block, the same as a `WorldMapPointParam` row's.
    x: f32,
    z: f32,
    draw_order: u32,
    reserved: u32,
}

#[repr(C)]
struct RawIcon {
    lossless2: *const u8,
    lossless2_size: u32,
    source_icon_id: u32,
    matrix: *const u8,
    matrix_size: u32,
    reserved: u32,
}

#[repr(C)]
struct Api {
    size: u32,
    abi_version: u32,
    point_size: u32,
    icon_size: u32,
    icon_count: u32,
    reserved: u32,
    view_points: unsafe extern "C" fn(u32, *mut *const RawPoint, *mut u32) -> i32,
    get_icon: unsafe extern "C" fn(u32, *mut RawIcon) -> i32,
}

/// Where the API was looked for last, and the table once it was found.
struct Lookup {
    api: Option<&'static Api>,
    tried: Option<Instant>,
}

static LOOKUP: Mutex<Lookup> = Mutex::new(Lookup { api: None, tried: None });

/// `None` while Map for Goblins DLL is not loaded or predates the API. A miss is asked again
/// every few seconds; the table it hands out stays valid while the module stays loaded.
fn api() -> Option<&'static Api> {
    let mut lookup = LOOKUP.lock().unwrap_or_else(|e| e.into_inner());
    if lookup.api.is_none() && lookup.tried.is_none_or(|at| at.elapsed() >= RETRY) {
        lookup.tried = Some(Instant::now());
        lookup.api = find();
    }
    lookup.api
}

fn find() -> Option<&'static Api> {
    // SAFETY: the export is the header's `MfgGetMapAPI`, and the table it returns is static.
    unsafe {
        let module = GetModuleHandleW(w!("MapForGoblins.dll")).ok()?;
        let get = GetProcAddress(module, s!("MfgGetMapAPI"))?;
        let get: unsafe extern "C" fn(u32) -> *const Api = std::mem::transmute(get);
        let api = get(ABI).as_ref()?;
        (api.abi_version == ABI
            && api.point_size as usize == size_of::<RawPoint>()
            && api.icon_size as usize == size_of::<RawIcon>())
        .then_some(api)
    }
}

pub fn available() -> bool {
    api().is_some()
}

/// A marker of Map for Goblins as the minimap draws it.
#[derive(Clone, Copy)]
pub struct Marker {
    /// Meters from the player along the world's X and Z axes.
    pub offset: [f32; 2],
    /// The source id of its icon, see [icons].
    pub icon: u32,
}

/// A marker placed on the overworld grid, whatever the player's position.
struct Located {
    position: [f32; 2],
    icon: u32,
}

static VISIBLE: Mutex<Refreshed<(MapId, i32), Located>> = Mutex::new(Refreshed::new());

/// The markers on `map` and in `space` within `reach` meters of `position`.
pub fn nearby(map: MapId, position: [f32; 2], space: i32, reach: f32) -> Vec<Marker> {
    let mut cache = VISIBLE.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get((map, space), RECHECK, || located(map, space))
        .iter()
        .map(|point| (point, [point.position[0] - position[0], point.position[1] - position[1]]))
        .filter(|(_, offset)| offset[0].hypot(offset[1]) <= reach)
        .map(|(point, offset)| Marker { offset, icon: point.icon })
        .collect()
}

/// Every marker Map for Goblins shows on `map`, placed on the overworld grid, that lies in
/// `space`; `None` while its API is not there.
fn located(map: MapId, space: i32) -> Option<Vec<Located>> {
    let api = api()?;
    let layer = match map {
        MapId::Overworld => 0,
        MapId::Underground => 1,
    };
    let mut points = std::ptr::null();
    let mut count = 0;
    // SAFETY: the view stays valid until this thread asks for the same layer again.
    let view = unsafe {
        if (api.view_points)(layer, &mut points, &mut count) != OK {
            return None;
        }
        if points.is_null() { &[] } else { std::slice::from_raw_parts(points, count as usize) }
    };
    Some(
        view.iter()
            .filter(|point| point.id & FOCUS_RING == 0)
            .filter_map(|point| {
                let [area, x, z, _] = point.raw_map_id.to_be_bytes();
                let (position, at_space, _) =
                    world::locate(BlockId::from_parts(area, x, z, 0), [point.x, point.z]);
                (at_space == space).then_some(Located { position, icon: point.icon_id })
            })
            .collect(),
    )
}

/// An icon of Map for Goblins by its source id, straight RGBA.
pub struct Icon {
    pub id: u32,
    pub image: Image,
}

/// Every icon the API offers, decoded; the ones that cannot be decoded are left out.
pub fn icons() -> Vec<Icon> {
    let Some(api) = api() else { return Vec::new() };
    (0..api.icon_count)
        .filter_map(|index| {
            let mut raw = RawIcon {
                lossless2: std::ptr::null(),
                lossless2_size: 0,
                source_icon_id: 0,
                matrix: std::ptr::null(),
                matrix_size: 0,
                reserved: 0,
            };
            // SAFETY: the returned tag is immutable for as long as the module stays loaded.
            let body = unsafe {
                if (api.get_icon)(index, &mut raw) != OK || raw.lossless2.is_null() {
                    return None;
                }
                std::slice::from_raw_parts(raw.lossless2, raw.lossless2_size as usize)
            };
            decode_lossless2(body).map(|image| Icon { id: raw.source_icon_id, image })
        })
        .collect()
}

/// A SWF DefineBitsLossless2 body: character id, format, width, height, then zlib data, either a
/// palette of premultiplied RGBA with rows of indices padded to 4 bytes (format 3), or rows of
/// premultiplied ARGB (format 5).
fn decode_lossless2(body: &[u8]) -> Option<Image> {
    let format = *body.get(2)?;
    let width = u32::from(u16::from_le_bytes([*body.get(3)?, *body.get(4)?]));
    let height = u32::from(u16::from_le_bytes([*body.get(5)?, *body.get(6)?]));
    let (colours, data) = match format {
        3 => (usize::from(*body.get(7)?) + 1, body.get(8..)?),
        5 => (0, body.get(7..)?),
        _ => return None,
    };
    let mut raw = Vec::new();
    ZlibDecoder::new(data).read_to_end(&mut raw).ok()?;
    let (w, h) = (width as usize, height as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    if format == 3 {
        let (table, indices) = raw.split_at_checked(colours * 4)?;
        let stride = (w + 3) & !3;
        for y in 0..h {
            for x in 0..w {
                let i = usize::from(*indices.get(y * stride + x)?);
                rgba.extend_from_slice(table.get(i * 4..i * 4 + 4)?);
            }
        }
    } else {
        for argb in raw.get(..w * h * 4)?.chunks_exact(4) {
            rgba.extend_from_slice(&[argb[1], argb[2], argb[3], argb[0]]);
        }
    }
    // The art is premultiplied; the texture is straight.
    for pixel in rgba.chunks_exact_mut(4) {
        let a = u16::from(pixel[3]);
        for c in &mut pixel[..3] {
            if let Some(straight) = (u16::from(*c) * 255 + a / 2).checked_div(a) {
                *c = straight.min(255) as u8;
            }
        }
    }
    Some(Image { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn zlib(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn an_argb_icon_becomes_straight_rgba() {
        // 2x1: an opaque red pixel and a half transparent one (premultiplied 64, 0, 0, alpha 128).
        let mut body = vec![0, 0, 5, 2, 0, 1, 0];
        body.extend(zlib(&[255, 255, 0, 0, 128, 64, 0, 0]));
        let image = decode_lossless2(&body).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]);
        assert_eq!(&image.rgba[4..8], &[128, 0, 0, 128]);
    }

    #[test]
    fn a_palette_icon_reads_its_indices_with_padded_rows() {
        // 2x2, two colours; rows of indices are padded to 4 bytes.
        let mut body = vec![0, 0, 3, 2, 0, 2, 0, 1];
        let mut data = vec![255, 0, 0, 255, 0, 255, 0, 255]; // red, green
        data.extend([0, 1, 0, 0, 1, 0, 0, 0]);
        body.extend(zlib(&data));
        let image = decode_lossless2(&body).unwrap();
        assert_eq!(image.rgba, [255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn a_broken_body_is_refused() {
        assert!(decode_lossless2(&[0, 0, 5, 1, 0]).is_none());
        assert!(decode_lossless2(&[0, 0, 9, 1, 0, 1, 0, 0]).is_none());
    }
}
