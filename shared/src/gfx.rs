//! In-memory edit of a Scaleform movie (`.gfx`, the uncompressed SWF layout):
//! adds numbered placements (`Item_5`, `Item_6`...) to a named sprite so a HUD
//! list gets more rows. Why this approach: `shared/docs/gfx.md`.

use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub enum GfxError {
    /// Not an uncompressed `GFX`/`FWS` movie.
    NotGfx,
    /// A tag or field runs past the end of the data.
    Truncated,
    /// A name on the path has no placement, or its sprite is missing.
    PathNotFound,
    /// Fewer than two numbered placements to copy the layout from.
    NoTemplate,
    /// A placement uses fields this editor does not rewrite.
    Unsupported,
}

const TAG_END: u16 = 0;
const TAG_SHOW_FRAME: u16 = 1;
const TAG_PLACE_OBJECT2: u16 = 26;
const TAG_DEFINE_SPRITE: u16 = 39;
const TAG_PLACE_OBJECT3: u16 = 70;

/// A placement with exactly these flags: has name, matrix, character.
const PLACE_NAME_MATRIX_CHAR: u8 = 0x26;
/// Depth gap between the numbered placements the game authored.
const DEPTH_STEP: u16 = 4;

#[derive(Clone)]
struct Tag {
    code: u16,
    /// Offset of the tag header (short or long form).
    header: usize,
    body: Range<usize>,
}

fn read_u16(data: &[u8], at: usize) -> Result<u16, GfxError> {
    let bytes = data.get(at..at + 2).ok_or(GfxError::Truncated)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], at: usize) -> Result<u32, GfxError> {
    let bytes = data.get(at..at + 4).ok_or(GfxError::Truncated)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Tags in `data[start..end]`, up to and including `End`.
fn parse_tags(data: &[u8], mut at: usize, end: usize) -> Result<Vec<Tag>, GfxError> {
    let mut tags = Vec::new();
    while at < end {
        let header = at;
        let head = read_u16(data, at)?;
        at += 2;
        let mut len = (head & 63) as usize;
        if len == 63 {
            len = read_u32(data, at)? as usize;
            at += 4;
        }
        let body = at..at.checked_add(len).filter(|&e| e <= end).ok_or(GfxError::Truncated)?;
        at = body.end;
        let code = head >> 6;
        tags.push(Tag { code, header, body });
        if code == TAG_END {
            break;
        }
    }
    Ok(tags)
}

/// Offset of the first tag, after the signature, length, frame rectangle,
/// frame rate and frame count.
fn first_tag_offset(movie: &[u8]) -> Result<usize, GfxError> {
    if movie.len() < 9 || !matches!(&movie[..3], b"GFX" | b"FWS") {
        return Err(GfxError::NotGfx);
    }
    let rect_bits = 5 + 4 * (movie[8] >> 3) as usize;
    Ok(8 + rect_bits.div_ceil(8) + 4)
}

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    fn bits(&mut self, n: u32) -> Result<u32, GfxError> {
        let mut value = 0u32;
        for _ in 0..n {
            let byte = *self.data.get(self.bit / 8).ok_or(GfxError::Truncated)?;
            value = (value << 1) | ((byte >> (7 - self.bit % 8)) & 1) as u32;
            self.bit += 1;
        }
        Ok(value)
    }

    fn signed(&mut self, n: u32) -> Result<i32, GfxError> {
        let raw = self.bits(n)?;
        Ok(if n == 0 { 0 } else { ((raw << (32 - n)) as i32) >> (32 - n) })
    }

    fn byte_end(&self) -> usize {
        self.bit.div_ceil(8)
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit: usize,
}

impl BitWriter {
    fn bits(&mut self, value: u32, n: u32) {
        for i in (0..n).rev() {
            if self.bit % 8 == 0 {
                self.bytes.push(0);
            }
            if (value >> i) & 1 == 1 {
                *self.bytes.last_mut().unwrap() |= 0x80 >> (self.bit % 8);
            }
            self.bit += 1;
        }
    }
}

/// Smallest signed field width that holds `v` (0 for zero).
fn signed_width(v: i32) -> u32 {
    if v == 0 { 0 } else { 33 - (if v < 0 { !v } else { v }).leading_zeros() }
}

/// A MATRIX record. Scale and rotate keep their original field widths.
#[derive(Clone)]
struct Matrix {
    scale: Option<(u32, i32, i32)>,
    rotate: Option<(u32, i32, i32)>,
    tx: i32,
    ty: i32,
}

impl Matrix {
    /// Parses the matrix at `data[at..]`, returns it and the byte offset after.
    fn parse(data: &[u8], at: usize) -> Result<(Matrix, usize), GfxError> {
        let mut r = BitReader { data, bit: at * 8 };
        let pair = |r: &mut BitReader| -> Result<Option<(u32, i32, i32)>, GfxError> {
            if r.bits(1)? == 0 {
                return Ok(None);
            }
            let n = r.bits(5)?;
            Ok(Some((n, r.signed(n)?, r.signed(n)?)))
        };
        let scale = pair(&mut r)?;
        let rotate = pair(&mut r)?;
        let n = r.bits(5)?;
        let (tx, ty) = (r.signed(n)?, r.signed(n)?);
        Ok((Matrix { scale, rotate, tx, ty }, r.byte_end()))
    }

    fn write(&self) -> Vec<u8> {
        let mut w = BitWriter::default();
        for pair in [&self.scale, &self.rotate] {
            match pair {
                None => w.bits(0, 1),
                Some((n, x, y)) => {
                    w.bits(1, 1);
                    w.bits(*n, 5);
                    for v in [x, y] {
                        w.bits(*v as u32 & mask(*n), *n);
                    }
                }
            }
        }
        let n = signed_width(self.tx).max(signed_width(self.ty));
        w.bits(n, 5);
        w.bits(self.tx as u32 & mask(n), n);
        w.bits(self.ty as u32 & mask(n), n);
        w.bytes
    }
}

fn mask(n: u32) -> u32 {
    if n >= 32 { u32::MAX } else { (1u32 << n) - 1 }
}

/// PlaceObject2 with flags `HasName | HasMatrix | HasCharacter`: the shape the
/// game authors for the numbered HUD rows.
struct Placement {
    depth: u16,
    char_id: u16,
    matrix: Matrix,
    name: String,
}

impl Placement {
    fn parse(body: &[u8]) -> Result<Placement, GfxError> {
        if body.first() != Some(&PLACE_NAME_MATRIX_CHAR) {
            return Err(GfxError::Unsupported);
        }
        let depth = read_u16(body, 1)?;
        let char_id = read_u16(body, 3)?;
        let (matrix, name_at) = Matrix::parse(body, 5)?;
        let name = body.get(name_at..).ok_or(GfxError::Truncated)?;
        let name = name.strip_suffix(&[0]).ok_or(GfxError::Unsupported)?;
        if name.contains(&0) {
            return Err(GfxError::Unsupported);
        }
        Ok(Placement { depth, char_id, matrix, name: String::from_utf8_lossy(name).into_owned() })
    }

    /// Complete tag (short-form header) for this placement.
    fn to_tag(&self) -> Vec<u8> {
        let mut body = vec![PLACE_NAME_MATRIX_CHAR];
        body.extend_from_slice(&self.depth.to_le_bytes());
        body.extend_from_slice(&self.char_id.to_le_bytes());
        body.extend_from_slice(&self.matrix.write());
        body.extend_from_slice(self.name.as_bytes());
        body.push(0);
        let mut tag = (TAG_PLACE_OBJECT2 << 6 | body.len() as u16).to_le_bytes().to_vec();
        tag.extend_from_slice(&body);
        tag
    }
}

/// Character id and instance name of any PlaceObject2/3, `None` for a tag
/// without both. Reads past the matrix, color transform and ratio to the name.
fn placed_name(code: u16, body: &[u8]) -> Result<Option<(u16, String)>, GfxError> {
    let flags = *body.first().ok_or(GfxError::Truncated)?;
    let (flags2, mut at) = if code == TAG_PLACE_OBJECT3 { (*body.get(1).ok_or(GfxError::Truncated)?, 2) } else { (0, 1) };
    at += 2; // depth
    let has_char = flags & 2 != 0;
    if code == TAG_PLACE_OBJECT3 && (flags2 & 8 != 0 || (flags2 & 16 != 0 && has_char)) {
        at += body.get(at..).ok_or(GfxError::Truncated)?.iter().position(|&b| b == 0).ok_or(GfxError::Truncated)? + 1;
    }
    let char_id = if has_char { Some(read_u16(body, at)?) } else { None };
    if has_char {
        at += 2;
    }
    if flags & 4 != 0 {
        at = Matrix::parse(body, at)?.1;
    }
    if flags & 8 != 0 {
        // CXFORMWITHALPHA: has-add, has-mul, 4-bit width, then 4 channels each
        let mut r = BitReader { data: body, bit: at * 8 };
        let (has_add, has_mul) = (r.bits(1)?, r.bits(1)?);
        let n = r.bits(4)?;
        r.bits(n * 4 * (has_add + has_mul))?;
        at = r.byte_end();
    }
    if flags & 16 != 0 {
        at += 2;
    }
    if flags & 32 == 0 {
        return Ok(None);
    }
    let rest = body.get(at..).ok_or(GfxError::Truncated)?;
    let len = rest.iter().position(|&b| b == 0).ok_or(GfxError::Truncated)?;
    Ok(char_id.map(|id| (id, String::from_utf8_lossy(&rest[..len]).into_owned())))
}

/// Character id of the placement named `name` among `tags`.
fn find_named(movie: &[u8], tags: &[Tag], name: &str) -> Option<u16> {
    tags.iter()
        .filter(|t| matches!(t.code, TAG_PLACE_OBJECT2 | TAG_PLACE_OBJECT3))
        .filter_map(|t| placed_name(t.code, &movie[t.body.clone()]).ok().flatten())
        .find_map(|(id, n)| (n == name).then_some(id))
}

/// The top-level `DefineSprite` with character id `id`.
fn find_sprite<'a>(movie: &[u8], top: &'a [Tag], id: u16) -> Option<&'a Tag> {
    top.iter().find(|t| t.code == TAG_DEFINE_SPRITE && read_u16(movie, t.body.start).ok() == Some(id))
}

fn tag_header(code: u16, len: usize) -> Vec<u8> {
    if len < 63 {
        return (code << 6 | len as u16).to_le_bytes().to_vec();
    }
    let mut head = (code << 6 | 63).to_le_bytes().to_vec();
    head.extend_from_slice(&(len as u32).to_le_bytes());
    head
}

/// Suffix of `name` after `prefix` when it is all digits.
fn index_of(name: &str, prefix: &str) -> Option<u32> {
    name.strip_prefix(prefix).filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))?.parse().ok()
}

/// Walks `path` (instance names from the stage down: `["Buddy", "Anim",
/// "GaugeList"]`) to a sprite and makes it hold `prefix0 ..= prefix{count-1}`
/// by copying its last numbered placement: same character and scale, each new
/// one a further step in the position difference of the last two. `Ok(None)`
/// when the sprite already has `count` rows. Returns the whole new movie.
pub fn extend_numbered_placements(movie: &[u8], path: &[&str], prefix: &str, count: u32) -> Result<Option<Vec<u8>>, GfxError> {
    let top = parse_tags(movie, first_tag_offset(movie)?, movie.len())?;
    let mut scope = top.clone();
    let mut sprite_tag = None;
    for name in path {
        let id = find_named(movie, &scope, name).ok_or(GfxError::PathNotFound)?;
        let sprite = find_sprite(movie, &top, id).ok_or(GfxError::PathNotFound)?;
        scope = parse_tags(movie, sprite.body.start + 4, sprite.body.end)?;
        sprite_tag = Some(sprite);
    }
    let sprite = sprite_tag.ok_or(GfxError::PathNotFound)?;

    let mut rows: Vec<(u32, Placement)> = Vec::new();
    let mut max_depth = 0;
    for tag in scope.iter().filter(|t| t.code == TAG_PLACE_OBJECT2) {
        let body = &movie[tag.body.clone()];
        max_depth = max_depth.max(read_u16(body, 1)?);
        let named = placed_name(tag.code, body)?.and_then(|(_, n)| index_of(&n, prefix));
        if let Some(index) = named {
            rows.push((index, Placement::parse(body)?));
        }
    }
    rows.sort_by_key(|(index, _)| *index);
    if rows.last().is_some_and(|(last, _)| *last + 1 >= count) {
        return Ok(None);
    }
    let [.., (prev_index, prev), (last_index, last)] = rows.as_slice() else {
        return Err(GfxError::NoTemplate);
    };
    let step = (last.matrix.tx - prev.matrix.tx, last.matrix.ty - prev.matrix.ty);
    let span = (*last_index - *prev_index) as i32;
    let (step_x, step_y) = (step.0 / span, step.1 / span);

    let mut added = Vec::new();
    for index in last_index + 1..count {
        let n = (index - last_index) as i32;
        let mut matrix = last.matrix.clone();
        matrix.tx += step_x * n;
        matrix.ty += step_y * n;
        let depth = max_depth + DEPTH_STEP * (n as u16);
        added.extend(Placement { depth, char_id: last.char_id, matrix, name: format!("{prefix}{index}") }.to_tag());
    }

    // The new rows go in before the first ShowFrame, so they exist in frame 1.
    let show = scope.iter().find(|t| t.code == TAG_SHOW_FRAME).ok_or(GfxError::Truncated)?;
    let mut body = movie[sprite.body.start..show.header].to_vec();
    body.extend(added);
    body.extend_from_slice(&movie[show.header..sprite.body.end]);

    let mut out = movie[..sprite.header].to_vec();
    out.extend(tag_header(TAG_DEFINE_SPRITE, body.len()));
    out.extend(&body);
    out.extend_from_slice(&movie[sprite.body.end..]);
    // Declared length counts every byte after the 8-byte head; keep the same
    // relation to the physical length as the original.
    let delta = out.len() as i64 - movie.len() as i64;
    let declared = read_u32(&out, 4)? as i64 + delta;
    out[4..8].copy_from_slice(&(declared as u32).to_le_bytes());
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GaugeList` of the game's HUD movie: Item_4..Item_0 at depths 1..17,
    /// character 474, scale 74632/58982, y 0 down to -1200 twips.
    const GAUGE_LIST_BODY: &str = "db010100bf0612000000260100da01c923883999804974656d5f3400bf0615000000260500da01c9238839998a002d404974656d5f3300bf0615000000260900da01c9238839998b0016a04974656d5f3200bf0615000000260d00da01c9238839998b0011f04974656d5f3100bf0615000000261100da01c9238839998c000b504974656d5f300040000000";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn tag(code: u16, body: &[u8]) -> Vec<u8> {
        let mut t = tag_header(code, body.len());
        t.extend_from_slice(body);
        t
    }

    /// Placement of `char_id` named `name` with an identity-ish matrix.
    fn place(depth: u16, char_id: u16, name: &str) -> Vec<u8> {
        Placement { depth, char_id, matrix: Matrix { scale: None, rotate: None, tx: 0, ty: 0 }, name: name.into() }.to_tag()
    }

    fn sprite(id: u16, children: &[Vec<u8>]) -> Vec<u8> {
        let mut body = id.to_le_bytes().to_vec();
        body.extend_from_slice(&1u16.to_le_bytes());
        children.iter().for_each(|c| body.extend_from_slice(c));
        body.extend(tag(TAG_SHOW_FRAME, &[]));
        body.extend(tag(TAG_END, &[]));
        tag(TAG_DEFINE_SPRITE, &body)
    }

    /// Stage -> Buddy(1) -> Anim(2) -> GaugeList(3, the real rows).
    fn movie() -> Vec<u8> {
        let mut m = b"GFX\x0b".to_vec();
        m.extend_from_slice(&[0; 4]); // length, filled below
        m.extend_from_slice(&[0x78, 0x00, 0x05, 0x5F, 0x00, 0x00, 0x0F, 0xA0, 0x00]); // rect
        m.extend_from_slice(&[0x00, 0x18, 0x01, 0x00]); // frame rate, frame count
        // sprite 3 = the real rows, re-labelled from the game's id 475
        let mut rows = hex(GAUGE_LIST_BODY);
        let mut body = 3u16.to_le_bytes().to_vec();
        body.extend_from_slice(&rows.split_off(2));
        m.extend(tag(TAG_DEFINE_SPRITE, &body));
        m.extend(sprite(2, &[place(1, 3, "GaugeList")]));
        m.extend(sprite(1, &[place(1, 2, "Anim")]));
        m.extend(place(447, 1, "Buddy"));
        m.extend(tag(TAG_SHOW_FRAME, &[]));
        m.extend(tag(TAG_END, &[]));
        let declared = (m.len() - 8) as u32;
        m[4..8].copy_from_slice(&declared.to_le_bytes());
        m
    }

    const PATH: [&str; 3] = ["Buddy", "Anim", "GaugeList"];

    fn rows_of(movie: &[u8]) -> Vec<(String, u16, i32)> {
        let top = parse_tags(movie, first_tag_offset(movie).unwrap(), movie.len()).unwrap();
        let mut scope = top.clone();
        for name in PATH {
            let id = find_named(movie, &scope, name).unwrap();
            let s = find_sprite(movie, &top, id).unwrap();
            scope = parse_tags(movie, s.body.start + 4, s.body.end).unwrap();
        }
        scope
            .iter()
            .filter(|t| t.code == TAG_PLACE_OBJECT2)
            .map(|t| Placement::parse(&movie[t.body.clone()]).unwrap())
            .map(|p| (p.name, p.depth, p.matrix.ty))
            .collect()
    }

    #[test]
    fn placement_roundtrips_the_games_own_bytes() {
        let body = hex(GAUGE_LIST_BODY);
        let tags = parse_tags(&body, 4, body.len()).unwrap();
        for t in tags.iter().filter(|t| t.code == TAG_PLACE_OBJECT2) {
            // the game writes long-form headers, this writes short ones: compare bodies
            let rewritten = Placement::parse(&body[t.body.clone()]).unwrap().to_tag();
            assert_eq!(rewritten[2..], body[t.body.clone()]);
        }
    }

    #[test]
    fn extends_the_list_with_the_same_spacing() {
        let before = movie();
        let after = extend_numbered_placements(&before, &PATH, "Item_", 8).unwrap().unwrap();
        let rows = rows_of(&after);
        let by_name = |n: &str| rows.iter().find(|r| r.0 == n).unwrap().clone();
        assert_eq!(rows.len(), 8);
        // Item_4 is at y 0 and each lower index 300 twips higher up
        assert_eq!(by_name("Item_5").2, 300);
        assert_eq!(by_name("Item_7").2, 900);
        assert_eq!(by_name("Item_0").2, -1200);
        assert_eq!(by_name("Item_5").1, 21);
        assert_eq!(by_name("Item_7").1, 29);
        let declared = read_u32(&after, 4).unwrap() as usize;
        assert_eq!(declared, after.len() - 8);
    }

    #[test]
    fn already_long_enough_is_left_alone() {
        let m = movie();
        assert_eq!(extend_numbered_placements(&m, &PATH, "Item_", 5), Ok(None));
        let bigger = extend_numbered_placements(&m, &PATH, "Item_", 6).unwrap().unwrap();
        assert_eq!(extend_numbered_placements(&bigger, &PATH, "Item_", 6), Ok(None));
    }

    #[test]
    fn rejects_other_files_and_missing_paths() {
        assert_eq!(extend_numbered_placements(b"CWS\x0b........", &PATH, "Item_", 8), Err(GfxError::NotGfx));
        let m = movie();
        assert_eq!(extend_numbered_placements(&m, &["Nope"], "Item_", 8), Err(GfxError::PathNotFound));
        assert_eq!(extend_numbered_placements(&m[..40], &PATH, "Item_", 8), Err(GfxError::Truncated));
    }

    /// Needs the game's `menu/01_000_fe.gfx`: `ER_HUD_GFX=<path> ER_HUD_GFX_OUT=<path>
    /// cargo test -p common gfx -- --ignored` writes the extended movie there.
    #[test]
    #[ignore]
    fn real_hud_movie() {
        let movie = std::fs::read(std::env::var("ER_HUD_GFX").unwrap()).unwrap();
        let after = extend_numbered_placements(&movie, &PATH, "Item_", 10).unwrap().unwrap();
        assert_eq!(rows_of(&after).len(), 10);
        std::fs::write(std::env::var("ER_HUD_GFX_OUT").unwrap(), after).unwrap();
    }
}
