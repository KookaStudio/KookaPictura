//! The layer-mask record's mask-parameters block: user and vector mask
//! density and feather.
//!
//! Layout as psd-tools 1.19 (`MaskData`, `MaskParameters`) and ag-psd 31
//! (`readLayerMaskData`) both decode it: mask flags bit 4 marks the block; a
//! selector byte then gates user density (`u8`, bit 0), user feather (`f64`,
//! bit 1), vector density (`u8`, bit 2), and vector feather (`f64`, bit 3), in
//! that order. The 18-byte real-user-mask header comes first when the layer
//! carries a `-3` channel (psd-tools keys it on the channel; the block length
//! alone is ambiguous). The record body is padded to 4 bytes.

use pictura_core::{Layer, VectorMask};

pub(crate) const FLAG_DISABLED: u8 = 0x02;
pub(crate) const FLAG_PARAMETERS: u8 = 0x10;
const REAL_HEADER: usize = 18;

/// Density and feather bits for one mask; `(255, 0)` is the default.
pub(crate) type Params = (u8, u64);
const DEFAULT: Params = (255, 0);

/// A mask record's bytes after its fixed 18-byte header, split.
pub(crate) struct Tail {
    /// The real-mask header and any unknown trailing bytes, verbatim.
    pub extra: Vec<u8>,
    pub user: Params,
    pub vector: Params,
}

pub(crate) fn read_tail(flags: u8, tail: &[u8], has_real: bool) -> Tail {
    let real = if has_real && tail.len() >= REAL_HEADER {
        REAL_HEADER
    } else {
        0
    };
    let (mut user, mut vector) = (DEFAULT, DEFAULT);
    let mut at = real;
    if flags & FLAG_PARAMETERS != 0 {
        if let Some(&selector) = tail.get(at) {
            at += 1;
            // A truncated block keeps the fields it has, as psd-tools does.
            let byte = |at: &mut usize| {
                let v = tail.get(*at).copied();
                *at += 1;
                v
            };
            if selector & 1 != 0 {
                user.0 = byte(&mut at).unwrap_or(user.0);
            }
            if selector & 2 != 0 {
                user.1 = double(tail, &mut at).unwrap_or(user.1);
            }
            if selector & 4 != 0 {
                vector.0 = byte(&mut at).unwrap_or(vector.0);
            }
            if selector & 8 != 0 {
                vector.1 = double(tail, &mut at).unwrap_or(vector.1);
            }
        }
    }
    // Without parameters the tail is kept whole, padding included, so such a
    // record re-saves byte for byte; after parameters the padding the writer
    // regenerates is dropped.
    let mut extra = tail[..real].to_vec();
    let rest = tail.get(at..).unwrap_or(&[]);
    if flags & FLAG_PARAMETERS == 0 || !is_padding(rest) {
        extra.extend_from_slice(rest);
    }
    Tail {
        extra,
        user,
        vector,
    }
}

fn is_padding(bytes: &[u8]) -> bool {
    bytes.len() < 4 && bytes.iter().all(|&b| b == 0)
}

fn double(tail: &[u8], at: &mut usize) -> Option<u64> {
    let bytes = tail.get(*at..*at + 8)?;
    *at += 8;
    Some(u64::from_be_bytes(bytes.try_into().ok()?))
}

/// The vector half of a record's parameters, as a placeholder view that
/// `resolve_vector_masks` merges into the decoded `vmsk` mask.
pub(crate) fn vector_placeholder(vector: Params) -> Option<VectorMask> {
    (vector != DEFAULT).then(|| VectorMask {
        density: vector.0,
        feather_bits: vector.1,
        ..Default::default()
    })
}

/// Write `layer`'s length-prefixed mask record: its raster mask's, or a
/// zero-rect parameter-only record when only the vector mask is refined.
pub(crate) fn write_record(out: &mut Vec<u8>, layer: &Layer) {
    let user = layer
        .mask
        .as_ref()
        .map_or(DEFAULT, |m| (m.density, m.feather_bits));
    let vector = layer
        .vector_mask
        .as_ref()
        .map_or(DEFAULT, |v| (v.density, v.feather_bits));
    let params = encode(user, vector);
    // ponytail: CS6's default colour for a parameter-only record is not
    // sourced; 255 hides nothing in a reader that applies the record.
    let ([top, left, bottom, right], default_color, flags, extra) = match &layer.mask {
        Some(m) => {
            let rect = [m.rect.top, m.rect.left, m.rect.bottom, m.rect.right];
            let disabled = if m.disabled { FLAG_DISABLED } else { 0 };
            let flags = (m.flags & !(FLAG_DISABLED | FLAG_PARAMETERS)) | disabled;
            (rect, m.default_color, flags, m.extra.as_slice())
        }
        None if !params.is_empty() => ([0; 4], 255, 0, &[][..]),
        None => {
            out.extend_from_slice(&0u32.to_be_bytes());
            return;
        }
    };
    let has_real = layer.raw_channels.iter().any(|c| c.id == -3);
    let real = if has_real && extra.len() >= REAL_HEADER {
        REAL_HEADER
    } else {
        0
    };
    let mut body = Vec::with_capacity(20 + extra.len() + params.len());
    for v in [top, left, bottom, right] {
        body.extend_from_slice(&v.to_be_bytes());
    }
    body.push(default_color);
    body.push(if params.is_empty() {
        flags
    } else {
        flags | FLAG_PARAMETERS
    });
    body.extend_from_slice(&extra[..real]);
    if params.is_empty() {
        body.extend_from_slice(&extra[real..]);
    } else {
        body.extend_from_slice(&params);
        // An old record's padding is regenerated below.
        if !is_padding(&extra[real..]) {
            body.extend_from_slice(&extra[real..]);
        }
        body.resize(body.len().next_multiple_of(4), 0);
    }
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
}

/// The selector byte and the non-default fields; empty when all are default.
fn encode(user: Params, vector: Params) -> Vec<u8> {
    let mut selector = 0u8;
    let mut fields = Vec::new();
    if user.0 != DEFAULT.0 {
        selector |= 1;
        fields.push(user.0);
    }
    if user.1 != DEFAULT.1 {
        selector |= 2;
        fields.extend_from_slice(&user.1.to_be_bytes());
    }
    if vector.0 != DEFAULT.0 {
        selector |= 4;
        fields.push(vector.0);
    }
    if vector.1 != DEFAULT.1 {
        selector |= 8;
        fields.extend_from_slice(&vector.1.to_be_bytes());
    }
    if selector == 0 {
        return Vec::new();
    }
    let mut out = vec![selector];
    out.extend_from_slice(&fields);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_without_parameters_keeps_its_tail_whole() {
        let tail = read_tail(0, &[0, 0], false);
        assert_eq!(tail.extra, vec![0, 0], "padding re-saves byte for byte");
        assert_eq!((tail.user, tail.vector), (DEFAULT, DEFAULT));
        let padded = read_tail(FLAG_PARAMETERS, &[0x01, 9, 0, 0], false);
        assert_eq!(padded.user.0, 9);
        assert!(
            padded.extra.is_empty(),
            "padding after parameters is dropped"
        );
    }

    #[test]
    fn parameters_read_in_selector_order() {
        let mut tail = vec![0x0f, 128];
        tail.extend_from_slice(&12.5f64.to_be_bytes());
        tail.push(64);
        tail.extend_from_slice(&3.0f64.to_be_bytes());
        let t = read_tail(FLAG_PARAMETERS, &tail, false);
        assert_eq!(t.user, (128, 12.5f64.to_bits()));
        assert_eq!(t.vector, (64, 3.0f64.to_bits()));
        assert!(t.extra.is_empty());
    }

    #[test]
    fn the_real_mask_header_precedes_the_parameters() {
        let mut tail: Vec<u8> = (1..=18).collect();
        tail.extend_from_slice(&[0x04, 200]);
        let t = read_tail(FLAG_PARAMETERS, &tail, true);
        assert_eq!(t.extra, (1..=18).collect::<Vec<u8>>());
        assert_eq!(t.vector.0, 200);
        assert_eq!(t.user, DEFAULT);
    }

    #[test]
    fn a_truncated_block_keeps_what_it_has() {
        let t = read_tail(FLAG_PARAMETERS, &[0x03, 90, 1, 2], false);
        assert_eq!(t.user.0, 90);
        assert_eq!(t.user.1, DEFAULT.1, "a short double is dropped");
    }

    #[test]
    fn encode_writes_only_non_default_fields() {
        assert!(encode(DEFAULT, DEFAULT).is_empty());
        let mut want = vec![0x05, 128, 64];
        assert_eq!(encode((128, 0), (64, 0)), want);
        want = vec![0x08];
        want.extend_from_slice(&2.0f64.to_be_bytes());
        assert_eq!(encode(DEFAULT, (255, 2.0f64.to_bits())), want);
    }
}
