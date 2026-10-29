//! The document Patterns resource: the `Patt`/`Pat2`/`Pat3` additional-layer
//! information tagged blocks that hold pattern pixels.
//!
//! A pattern fill (`PtFl`) references a pattern by id; the pixels live in these
//! global tagged blocks in the Layer and Mask Information section, **not** in
//! image resource 1039 (that is the ICC profile). This derives a typed view
//! from the preserved [`Document::layer_section_extra`] bytes, mirroring
//! [`crate::smart_object`], and keeps whatever parsed when a block is malformed
//! rather than failing the file.

use pictura_core::Document;

use crate::common::{is_psb_big_key, Reader};
use crate::descriptor::read_unicode_string;
use crate::error::PsdError;

/// One decoded pattern: its id, size, and row-major RGBA pixels (4 bytes each).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternPixels {
    pub pattern_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const MODE_GRAYSCALE: u32 = 1;
const MODE_RGB: u32 = 3;
const MODE_INDEXED: u32 = 7;

/// Pattern planes are small, tileable images. This caps a pattern's (and each
/// channel's) rectangle so a crafted one cannot drive an unbounded allocation;
/// an over-cap pattern is skipped, never rendered as garbage. `1 << 24` is
/// 4096x4096, far above any realistic pattern (a few megapixels), and bounds a
/// single pattern's RGBA buffer to 64 MiB. Larger patterns are skipped and
/// render the placeholder.
const MAX_PATTERN_PIXELS: usize = 1 << 24;

/// Decode every pattern in the document's `Patt`/`Pat2`/`Pat3` tagged blocks.
///
/// Grayscale and RGB patterns are decoded; other colour modes, non-8-bit
/// planes, and malformed or over-sized rectangles are skipped. A malformed
/// block leaves the patterns already parsed in place. Never panics.
pub fn decode_patterns(doc: &Document) -> Vec<PatternPixels> {
    let mut out = Vec::new();
    let mut r = Reader::new(&doc.layer_section_extra);
    while r.remaining() >= 12 {
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" {
            break;
        }
        let Ok(key) = r.take(4) else { break };
        let len = if doc.is_psb && is_psb_big_key(&arr4(key)) {
            match r.u64() {
                Ok(len) => usize::try_from(len).unwrap_or(usize::MAX),
                Err(_) => break,
            }
        } else {
            match r.u32() {
                Ok(len) => len as usize,
                Err(_) => break,
            }
        };
        let Ok(data) = r.take(len) else { break };
        // A global tagged block is padded externally to a 4-byte boundary.
        if r.skip((4 - len % 4) % 4).is_err() {
            break;
        }
        if matches!(key, b"Patt" | b"Pat2" | b"Pat3") {
            parse_patterns_block(data, &mut out);
        }
    }
    out
}

/// A `Patterns` list: repeated `u32`-length-prefixed, 4-byte-padded patterns.
fn parse_patterns_block(data: &[u8], out: &mut Vec<PatternPixels>) {
    let mut r = Reader::new(data);
    while r.remaining() >= 4 {
        let Ok(len) = r.u32() else { break };
        let len = len as usize;
        let Ok(body) = r.take(len) else { break };
        if r.skip((4 - len % 4) % 4).is_err() {
            break;
        }
        if let Some(pattern) = parse_pattern(body) {
            out.push(pattern);
        }
        // A malformed pattern is skipped; the next one starts at the length
        // boundary this iteration already consumed, so the walk stays bounded.
    }
}

fn parse_pattern(body: &[u8]) -> Option<PatternPixels> {
    let mut r = Reader::new(body);
    if r.u32().ok()? != 1 {
        return None;
    }
    let mode = r.u32().ok()?;
    // Only Grayscale and RGB are decoded, and each mode fixes its colour-plane
    // count. Resolve it before reading any channel so a crafted `num_channels`
    // cannot drive a large decode (the slots actually decoded are bounded to
    // `expected_color + 2`).
    let expected_color = match mode {
        MODE_GRAYSCALE => 1,
        MODE_RGB => 3,
        _ => return None,
    };
    let _vertical = r.i16().ok()?;
    let _horizontal = r.i16().ok()?;
    let _name = read_unicode_string(&mut r).ok()?;
    let pattern_id = read_pascal_id(&mut r).ok()?;
    if mode == MODE_INDEXED {
        // The indexed colour table is 256 RGB triples plus 4 pad bytes.
        r.skip(256 * 3 + 4).ok()?;
    }
    let list = parse_channel_list(&mut r, expected_color).ok()?;
    build_pixels(mode, pattern_id, list)
}

/// A `VirtualMemoryArrayList`: a rectangle plus `num_channels + 2` channel
/// arrays. The first `num_channels` slots are colour, the last two are the
/// alpha region (matching psd-tools' `get_pattern_color_channels`).
struct ChannelList {
    color_slots: usize,
    width: u32,
    height: u32,
    channels: Vec<Option<PatternChannel>>,
}

/// One written channel plane with the rectangle it was decoded against.
struct PatternChannel {
    plane: Vec<u8>,
    width: usize,
    height: usize,
}

/// Parse the channel list. `expected_color` is the pattern's colour-plane count
/// (fixed by its image mode): the declared `num_channels` must equal it, and
/// only `expected_color + 2` slots are decoded, so a crafted count cannot
/// amplify the allocation.
fn parse_channel_list(r: &mut Reader, expected_color: usize) -> Result<ChannelList, PsdError> {
    let version = r.u32()?;
    if version != 3 {
        return Err(PsdError::Unsupported(format!(
            "pattern channel list version {version}"
        )));
    }
    let len = r.u32()? as usize;
    let body = r.take(len)?;
    let mut br = Reader::new(body);
    let top = br.u32()?;
    let left = br.u32()?;
    let bottom = br.u32()?;
    let right = br.u32()?;
    let num_channels = br.u32()? as usize;
    if num_channels != expected_color {
        return Err(PsdError::Invalid(format!(
            "pattern channel count {num_channels} for a {expected_color}-channel mode"
        )));
    }
    let total = expected_color + 2;
    let mut channels = Vec::with_capacity(total);
    for _ in 0..total {
        channels.push(read_channel_array(&mut br)?);
    }
    Ok(ChannelList {
        color_slots: expected_color,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
        channels,
    })
}

/// One `VirtualMemoryArray` channel. `None` when the slot is unwritten; an
/// error for a written channel this decoder cannot read (non-8-bit depth or an
/// oversized rectangle), which skips the containing pattern.
fn read_channel_array(r: &mut Reader) -> Result<Option<PatternChannel>, PsdError> {
    let is_written = r.u32()?;
    if is_written == 0 {
        return Ok(None);
    }
    let length = r.u32()? as usize;
    if length == 0 {
        return Ok(None);
    }
    let depth = r.u32()?;
    let top = r.u32()?;
    let left = r.u32()?;
    let bottom = r.u32()?;
    let right = r.u32()?;
    let pixel_depth = r.u16()?;
    let compression = r.u8()?;
    // Only 8-bit planes are decoded; 16/32-bit ones are skipped so they render
    // the placeholder instead of garbage.
    if depth != 8 || pixel_depth != 8 {
        return Err(PsdError::Unsupported(format!(
            "pattern channel depth {depth}/{pixel_depth}"
        )));
    }
    // depth (4) + rectangle (16) + pixel_depth (2) + compression (1) = 23 bytes
    // precede the compressed data inside the declared length.
    let data_len = length
        .checked_sub(23)
        .ok_or_else(|| PsdError::Invalid("pattern channel length".into()))?;
    let data = r.take(data_len)?;
    let w = right.saturating_sub(left) as usize;
    let h = bottom.saturating_sub(top) as usize;
    if w == 0 || h == 0 {
        return Ok(None);
    }
    if w.checked_mul(h).is_none_or(|n| n > MAX_PATTERN_PIXELS) {
        return Err(PsdError::Invalid("pattern channel rectangle".into()));
    }
    let plane = crate::read::decode_channel_data(compression as u16, data, w, h, false)?;
    Ok(Some(PatternChannel {
        plane,
        width: w,
        height: h,
    }))
}

fn build_pixels(mode: u32, pattern_id: String, list: ChannelList) -> Option<PatternPixels> {
    if mode != MODE_GRAYSCALE && mode != MODE_RGB {
        return None;
    }
    let width = list.width;
    let height = list.height;
    let n = (width as usize).checked_mul(height as usize)?;
    if n == 0 || n > MAX_PATTERN_PIXELS || list.color_slots > list.channels.len() {
        return None;
    }
    // Every written channel must have been decoded against the pattern's own
    // rectangle. A crafted pattern-level rectangle that disagrees (1x1 channels
    // under a huge declared rect) is skipped here, before any allocation.
    for channel in list.channels.iter().flatten() {
        if channel.width != width as usize || channel.height != height as usize {
            return None;
        }
    }
    let color: Vec<&[u8]> = list.channels[..list.color_slots]
        .iter()
        .filter_map(|c| c.as_ref())
        .map(|c| c.plane.as_slice())
        .collect();
    // Require every colour plane: a pattern with a hole is skipped rather than
    // silently filled with black.
    let expected_color = if mode == MODE_GRAYSCALE { 1 } else { 3 };
    if color.len() != expected_color {
        return None;
    }
    // psd-tools' `draw_pattern_fill` takes the trailing written plane as alpha.
    let alpha = list.channels[list.color_slots..]
        .iter()
        .filter_map(|c| c.as_ref())
        .next_back()
        .map(|c| c.plane.as_slice());

    let mut rgba = vec![0u8; n.checked_mul(4)?];
    for i in 0..n {
        let (r, g, b) = if mode == MODE_GRAYSCALE {
            let v = *color[0].get(i)?;
            (v, v, v)
        } else {
            (*color[0].get(i)?, *color[1].get(i)?, *color[2].get(i)?)
        };
        let a = alpha.and_then(|p| p.get(i)).copied().unwrap_or(255);
        let o = i * 4;
        rgba[o] = r;
        rgba[o + 1] = g;
        rgba[o + 2] = b;
        rgba[o + 3] = a;
    }
    Some(PatternPixels {
        pattern_id,
        width,
        height,
        rgba,
    })
}

/// A Pascal pattern id: a `u8` length followed by that many ASCII bytes.
fn read_pascal_id(r: &mut Reader) -> Result<String, PsdError> {
    let len = r.u8()? as usize;
    let data = r.take(len)?;
    Ok(String::from_utf8_lossy(data).into_owned())
}

fn arr4(s: &[u8]) -> [u8; 4] {
    let mut a = [0u8; 4];
    a.copy_from_slice(&s[..4]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    fn be32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }

    fn unicode(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut v = be32(units.len() as u32).to_vec();
        for u in units {
            v.extend_from_slice(&u.to_be_bytes());
        }
        v
    }

    fn pascal(s: &str) -> Vec<u8> {
        let mut v = vec![s.len() as u8];
        v.extend_from_slice(s.as_bytes());
        v
    }

    fn channel(data: &[u8], w: u32, h: u32) -> Vec<u8> {
        channel_ex(data, w, h, 8, 8)
    }

    fn channel_ex(data: &[u8], w: u32, h: u32, depth: u16, pixel_depth: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&be32(1)); // is_written
        v.extend_from_slice(&be32((23 + data.len()) as u32)); // length
        v.extend_from_slice(&be32(depth as u32)); // depth
        v.extend_from_slice(&be32(0));
        v.extend_from_slice(&be32(0));
        v.extend_from_slice(&be32(h));
        v.extend_from_slice(&be32(w));
        v.extend_from_slice(&pixel_depth.to_be_bytes()); // pixel_depth
        v.push(0); // compression raw
        v.extend_from_slice(data);
        v
    }

    fn unwritten() -> Vec<u8> {
        be32(0).to_vec()
    }

    /// A written channel compressed with ZIP (compression 2).
    fn channel_zip(payload: &[u8], w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&be32(1)); // is_written
        v.extend_from_slice(&be32((23 + payload.len()) as u32)); // length
        v.extend_from_slice(&be32(8)); // depth
        v.extend_from_slice(&be32(0));
        v.extend_from_slice(&be32(0));
        v.extend_from_slice(&be32(h));
        v.extend_from_slice(&be32(w));
        v.extend_from_slice(&8u16.to_be_bytes()); // pixel_depth
        v.push(2); // compression ZIP
        v.extend_from_slice(payload);
        v
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    /// A pattern body with explicit channel slots and pattern-level rectangle,
    /// for malformed/edge cases `pattern` cannot express.
    fn pattern_raw(
        id: &str,
        mode: u32,
        num_color: u32,
        channels: &[Vec<u8>],
        pattern_rect: (u32, u32, u32, u32),
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&be32(1)); // version
        body.extend_from_slice(&be32(mode));
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&unicode("P"));
        body.extend_from_slice(&pascal(id));
        body.extend_from_slice(&be32(3)); // VirtualMemoryArrayList version
        let (top, left, bottom, right) = pattern_rect;
        let mut list = Vec::new();
        list.extend_from_slice(&be32(top));
        list.extend_from_slice(&be32(left));
        list.extend_from_slice(&be32(bottom));
        list.extend_from_slice(&be32(right));
        list.extend_from_slice(&be32(num_color));
        for c in channels {
            list.extend_from_slice(c);
        }
        body.extend_from_slice(&be32(list.len() as u32));
        body.extend_from_slice(&list);
        body
    }

    fn pattern(
        id: &str,
        mode: u32,
        planes: &[&[u8]],
        w: u32,
        h: u32,
        alpha: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&be32(1)); // version
        body.extend_from_slice(&be32(mode));
        body.extend_from_slice(&2u16.to_be_bytes()); // vertical point
        body.extend_from_slice(&2u16.to_be_bytes()); // horizontal point
        body.extend_from_slice(&unicode("P"));
        body.extend_from_slice(&pascal(id));

        let mut chans = Vec::new();
        for p in planes {
            chans.extend_from_slice(&channel(p, w, h));
        }
        chans.extend_from_slice(&unwritten()); // first alpha-region slot
        match alpha {
            Some(a) => chans.extend_from_slice(&channel(a, w, h)),
            None => chans.extend_from_slice(&unwritten()),
        }

        body.extend_from_slice(&be32(3)); // VirtualMemoryArrayList version
        let mut list = Vec::new();
        list.extend_from_slice(&be32(0)); // top
        list.extend_from_slice(&be32(0)); // left
        list.extend_from_slice(&be32(h));
        list.extend_from_slice(&be32(w));
        list.extend_from_slice(&be32(planes.len() as u32)); // num colour channels
        list.extend_from_slice(&chans);
        body.extend_from_slice(&be32(list.len() as u32));
        body.extend_from_slice(&list);
        body
    }

    fn tagged_patt(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"8BIM");
        out.extend_from_slice(b"Patt");
        out.extend_from_slice(&be32(data.len() as u32));
        out.extend_from_slice(data);
        out.extend_from_slice(&vec![0u8; (4 - data.len() % 4) % 4]);
        out
    }

    fn list_block(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut data = Vec::new();
        for e in entries {
            data.extend_from_slice(&be32(e.len() as u32));
            data.extend_from_slice(e);
            data.extend_from_slice(&vec![0u8; (4 - e.len() % 4) % 4]);
        }
        data
    }

    fn doc_with(extra: Vec<u8>) -> Document {
        let mut d = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        d.layer_section_extra = extra.into();
        d
    }

    #[test]
    fn rgb_pattern_decodes_id_size_and_rgba() {
        let r = [255u8, 0, 0, 255];
        let g = [0u8, 255, 0, 255];
        let b = [0u8, 0, 255, 255];
        let a = [10u8, 20, 30, 40];
        let entry = pattern("pictura-pattern", MODE_RGB, &[&r, &g, &b], 2, 2, Some(&a));
        let doc = doc_with(tagged_patt(&list_block(&[entry])));

        let got = decode_patterns(&doc);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].pattern_id, "pictura-pattern");
        assert_eq!((got[0].width, got[0].height), (2, 2));
        assert_eq!(
            got[0].rgba,
            vec![255, 0, 0, 10, 0, 255, 0, 20, 0, 0, 255, 30, 255, 255, 255, 40,]
        );
    }

    #[test]
    fn grayscale_pattern_replicates_its_channel() {
        let gray = [7u8, 8, 9, 10];
        let entry = pattern("g", MODE_GRAYSCALE, &[&gray], 2, 2, None);
        let doc = doc_with(tagged_patt(&list_block(&[entry])));

        let got = decode_patterns(&doc);
        assert_eq!(got.len(), 1);
        for (i, v) in [7u8, 8, 9, 10].into_iter().enumerate() {
            let o = i * 4;
            assert_eq!(&got[0].rgba[o..o + 4], &[v, v, v, 255]);
        }
    }

    #[test]
    fn unsupported_mode_is_skipped_and_truncated_block_keeps_earlier() {
        let cmyk = pattern("cmyk", 4, &[&[1, 2, 3, 4]], 2, 2, None);
        let good = pattern("good", MODE_GRAYSCALE, &[&[5, 6, 7, 8]], 2, 2, None);
        // A well-formed entry, then a length that overruns the block.
        let mut data = list_block(&[cmyk, good]);
        data.extend_from_slice(&be32(64));
        data.extend_from_slice(&[0, 1, 2, 3]);
        let doc = doc_with(tagged_patt(&data));

        let got = decode_patterns(&doc);
        assert_eq!(got.len(), 1, "only the supported pattern survives");
        assert_eq!(got[0].pattern_id, "good");
    }

    #[test]
    fn no_patterns_block_is_empty() {
        assert!(decode_patterns(&doc_with(Vec::new())).is_empty());
        assert!(decode_patterns(&doc_with(b"8BIMlnk2\0\0\0\0".to_vec())).is_empty());
    }

    #[test]
    fn crafted_oversized_pattern_rect_is_skipped_without_allocating() {
        // Three 1x1 colour planes under a declared 70000x70000 pattern
        // rectangle: before the guard this drove `vec![0u8; n * 4]` into an
        // abort instead of a skip. The 1x1 channel rects do not match the
        // declared rect, so `pattern_pixel_cap_boundary` below pins the cap
        // itself with matching rects.
        let mut bad_channels = Vec::new();
        for _ in 0..3 {
            bad_channels.push(channel(&[1], 1, 1));
        }
        bad_channels.push(unwritten());
        bad_channels.push(channel(&[255], 1, 1));
        let bad = pattern_raw("bad", MODE_RGB, 3, &bad_channels, (0, 0, 70_000, 70_000));

        let good_channels = [
            channel(&[1, 2, 3, 4], 2, 2),
            channel(&[5, 6, 7, 8], 2, 2),
            channel(&[9, 10, 11, 12], 2, 2),
            unwritten(),
            unwritten(),
        ];
        let good = pattern_raw("good", MODE_RGB, 3, &good_channels, (0, 0, 2, 2));

        let got = decode_patterns(&doc_with(tagged_patt(&list_block(&[bad, good]))));
        assert_eq!(
            got.len(),
            1,
            "the oversized pattern is skipped, not panicking"
        );
        assert_eq!(got[0].pattern_id, "good");
    }

    #[test]
    fn pattern_rect_must_match_the_channel_rects() {
        // The channel planes are 2x2 but the pattern declares 1x4 (same area).
        let entry = pattern_raw(
            "mismatch",
            MODE_GRAYSCALE,
            1,
            &[channel(&[1, 2, 3, 4], 2, 2), unwritten(), unwritten()],
            (0, 0, 1, 4),
        );
        assert!(decode_patterns(&doc_with(tagged_patt(&list_block(&[entry])))).is_empty());
    }

    #[test]
    fn non_8_bit_pattern_is_skipped() {
        let bad = pattern_raw(
            "p16",
            MODE_RGB,
            3,
            &[
                channel_ex(&[0; 8], 2, 2, 16, 16),
                channel(&[0; 4], 2, 2),
                channel(&[0; 4], 2, 2),
                unwritten(),
                unwritten(),
            ],
            (0, 0, 2, 2),
        );
        let good = pattern("good", MODE_GRAYSCALE, &[&[1, 2, 3, 4]], 2, 2, None);

        let got = decode_patterns(&doc_with(tagged_patt(&list_block(&[bad, good]))));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].pattern_id, "good");
    }

    #[test]
    fn both_alpha_slots_picks_the_last_written() {
        let entry = pattern_raw(
            "two-alpha",
            MODE_RGB,
            3,
            &[
                channel(&[1, 2, 3, 4], 2, 2),
                channel(&[5, 6, 7, 8], 2, 2),
                channel(&[9, 10, 11, 12], 2, 2),
                channel(&[1, 1, 1, 1], 2, 2), // earlier alpha slot
                channel(&[9, 9, 9, 9], 2, 2), // trailing alpha slot wins
            ],
            (0, 0, 2, 2),
        );
        let got = decode_patterns(&doc_with(tagged_patt(&list_block(&[entry]))));
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0]
                .rgba
                .iter()
                .skip(3)
                .step_by(4)
                .copied()
                .collect::<Vec<_>>(),
            vec![9, 9, 9, 9]
        );
    }

    #[test]
    fn pattern_pixel_cap_boundary() {
        // The channel rects match the pattern rect, so this exercises the pixel
        // cap rather than the rectangle-consistency guard.
        let cap = MAX_PATTERN_PIXELS;

        // One pixel under the cap decodes (a 1-pixel-tall strip keeps the plane
        // and RGBA allocation well-defined).
        let under_w = cap - 1;
        let under = pattern_raw(
            "under",
            MODE_GRAYSCALE,
            1,
            &[
                channel(&vec![7u8; under_w], under_w as u32, 1),
                unwritten(),
                unwritten(),
            ],
            (0, 0, 1, under_w as u32),
        );
        let got = decode_patterns(&doc_with(tagged_patt(&list_block(&[under]))));
        assert_eq!(got.len(), 1, "a pattern just under the cap decodes");
        assert_eq!(got[0].rgba.len(), under_w * 4);
        assert_eq!(got[0].rgba[0], 7);

        // One pixel over the cap is skipped; its channel is rejected before the
        // (deliberately empty) payload is read, so nothing is allocated.
        let over_w = cap + 1;
        let over = pattern_raw(
            "over",
            MODE_GRAYSCALE,
            1,
            &[channel(&[], over_w as u32, 1), unwritten(), unwritten()],
            (0, 0, 1, over_w as u32),
        );
        assert!(decode_patterns(&doc_with(tagged_patt(&list_block(&[over])))).is_empty());
    }

    #[test]
    fn oversized_channel_count_is_rejected_before_decoding() {
        // num_channels = 64 for an RGB pattern (3 expected) over a full-cap
        // rect, with a ZIP channel that would inflate to the cap if read. Before
        // the mode gate this decoded `num_channels + 2` cap-sized planes
        // (~1-2 GiB); now the count is rejected before any plane is decoded.
        let cap = MAX_PATTERN_PIXELS;
        let flood = pattern_raw(
            "flood",
            MODE_RGB,
            64,
            &[channel_zip(&zlib(&vec![0u8; cap]), cap as u32, 1)],
            (0, 0, 1, cap as u32),
        );
        let good = pattern("good", MODE_GRAYSCALE, &[&[1, 2, 3, 4]], 2, 2, None);

        let got = decode_patterns(&doc_with(tagged_patt(&list_block(&[flood, good]))));
        assert_eq!(
            got.len(),
            1,
            "the over-count pattern is rejected before decoding"
        );
        assert_eq!(got[0].pattern_id, "good");
    }

    #[test]
    fn rgb_pattern_with_a_missing_colour_plane_is_skipped() {
        let entry = pattern_raw(
            "hole",
            MODE_RGB,
            3,
            &[
                channel(&[1, 2, 3, 4], 2, 2),
                unwritten(),
                channel(&[9, 10, 11, 12], 2, 2),
                unwritten(),
                unwritten(),
            ],
            (0, 0, 2, 2),
        );
        assert!(decode_patterns(&doc_with(tagged_patt(&list_block(&[entry])))).is_empty());
    }

    #[test]
    fn grayscale_pattern_with_two_colour_planes_is_skipped() {
        let entry = pattern_raw(
            "g2",
            MODE_GRAYSCALE,
            2,
            &[
                channel(&[1, 2, 3, 4], 2, 2),
                channel(&[5, 6, 7, 8], 2, 2),
                unwritten(),
                unwritten(),
            ],
            (0, 0, 2, 2),
        );
        assert!(decode_patterns(&doc_with(tagged_patt(&list_block(&[entry])))).is_empty());
    }
}
