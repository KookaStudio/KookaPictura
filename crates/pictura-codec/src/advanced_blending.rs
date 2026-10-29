//! Parse/serialize a layer's advanced-blending data: `knko`/`clbl`/`infx`
//! tagged blocks and the blending-ranges Blend If view.
//!
//! The raw `blending_ranges` field stays the write source of truth for an
//! unmodified open→save; callers that edit [`BlendIf`] assign
//! `layer.blending_ranges = encode_blend_if(&view)` before writing.

use pictura_core::{BlendIf, Knockout, Layer};

use crate::write::write_tag;

/// Parse a blending-ranges body into a typed view: 8 bytes of composite
/// source/dest ranges, then 8-byte channel groups. Empty or a length that is
/// not a multiple of 8 yields `None` (raw bytes still round-trip).
pub(crate) fn parse_blend_if(bytes: &[u8]) -> Option<BlendIf> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(8) {
        return None;
    }
    let pair = |b: &[u8]| -> (u16, u16) {
        (
            u16::from_be_bytes([b[0], b[1]]),
            u16::from_be_bytes([b[2], b[3]]),
        )
    };
    Some(BlendIf {
        composite_source: pair(&bytes[0..4]),
        composite_dest: pair(&bytes[4..8]),
        channel_ranges: bytes[8..]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| (pair(&c[0..4]), pair(&c[4..8])))
            .collect(),
    })
}

/// Rebuild a blending-ranges body from a view as big-endian `u16` pairs.
pub fn encode_blend_if(view: &BlendIf) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + view.channel_ranges.len() * 8);
    for pair in [view.composite_source, view.composite_dest] {
        out.extend_from_slice(&pair.0.to_be_bytes());
        out.extend_from_slice(&pair.1.to_be_bytes());
    }
    for (source, dest) in &view.channel_ranges {
        for pair in [*source, *dest] {
            out.extend_from_slice(&pair.0.to_be_bytes());
            out.extend_from_slice(&pair.1.to_be_bytes());
        }
    }
    out
}

/// Write the layer blending-ranges field: raw when it still matches the typed
/// view, re-encoded when `blend_if` was edited.
pub(crate) fn write_ranges(out: &mut Vec<u8>, layer: &Layer) {
    let ranges = match &layer.blend_if {
        Some(view) if parse_blend_if(&layer.blending_ranges).as_ref() != Some(view) => {
            encode_blend_if(view)
        }
        _ => layer.blending_ranges.to_vec(),
    };
    out.extend_from_slice(&(ranges.len() as u32).to_be_bytes());
    out.extend_from_slice(&ranges);
}

/// Emit non-default advanced-blending tagged blocks after the other layer
/// attribute tags.
pub(crate) fn write_advanced_tags(out: &mut Vec<u8>, layer: &Layer, psb: bool) {
    if layer.knockout != Knockout::None {
        write_tag(out, b"knko", &[layer.knockout.to_byte(), 0, 0, 0], psb);
    }
    if !layer.blend_clipping {
        write_tag(out, b"clbl", &[0, 0, 0, 0], psb);
    }
    if !layer.blend_interior {
        write_tag(out, b"infx", &[0, 0, 0, 0], psb);
    }
}
