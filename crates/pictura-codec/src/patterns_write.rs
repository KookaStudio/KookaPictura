//! Write patterns into a document's `Patt` global block, so a layer effect or
//! fill that names one renders it and the PSD carries it. The record layout is
//! the one [`crate::decode_patterns`] reads: an RGB pattern with three raw
//! 8-bit planes and no alpha.

use pictura_core::Document;

use crate::common::is_psb_big_key;

fn be32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

/// One RGB pattern record (an entry of the `Patt` list) from row-major RGBA;
/// the alpha is dropped. `None` when the size is empty, does not fit the
/// record, or disagrees with `rgba`.
pub fn encode_rgb_pattern(
    name: &str,
    id: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Option<Vec<u8>> {
    let n = (width as usize).checked_mul(height as usize)?;
    if n == 0 || rgba.len() != n * 4 || width > i16::MAX as u32 || height > i16::MAX as u32 {
        return None;
    }
    let id = id.as_bytes();
    let id_len = u8::try_from(id.len()).ok()?;
    let mut out = Vec::new();
    be32(&mut out, 1);
    be32(&mut out, 3); // RGB
    out.extend_from_slice(&(height as i16).to_be_bytes());
    out.extend_from_slice(&(width as i16).to_be_bytes());
    let units: Vec<u16> = name.encode_utf16().collect();
    be32(&mut out, units.len() as u32);
    for u in units {
        out.extend_from_slice(&u.to_be_bytes());
    }
    out.push(id_len);
    out.extend_from_slice(id);

    let mut list = Vec::new();
    for v in [0, 0, height, width] {
        be32(&mut list, v);
    }
    be32(&mut list, 3);
    for plane in 0..3 {
        be32(&mut list, 1); // written
        be32(&mut list, (23 + n) as u32);
        be32(&mut list, 8);
        for v in [0, 0, height, width] {
            be32(&mut list, v);
        }
        list.extend_from_slice(&8u16.to_be_bytes());
        list.push(0); // raw
        list.extend(rgba.as_chunks::<4>().0.iter().map(|p| p[plane]));
    }
    // The two alpha-region slots stay unwritten: the pattern is opaque.
    be32(&mut list, 0);
    be32(&mut list, 0);
    be32(&mut out, 3);
    be32(&mut out, list.len() as u32);
    out.extend_from_slice(&list);
    Some(out)
}

/// Add `record` (from [`encode_rgb_pattern`]) to the document's `Patt` block,
/// creating the block when there is none. False, changing nothing, when a
/// pattern with `id` is already there or the layer-section extra data cannot
/// be walked.
pub fn add_document_pattern(doc: &mut Document, id: &str, record: &[u8]) -> bool {
    if crate::decode_patterns(doc)
        .iter()
        .any(|p| p.pattern_id == id)
    {
        return false;
    }
    let mut entry = Vec::new();
    be32(&mut entry, record.len() as u32);
    entry.extend_from_slice(record);
    entry.resize(entry.len().next_multiple_of(4), 0);

    let extra = &doc.layer_section_extra;
    let mut out = Vec::with_capacity(extra.len() + entry.len() + 12);
    let mut at = 0;
    let mut added = false;
    while at < extra.len() {
        let Some(head) = extra.get(at..at + 8) else {
            return false;
        };
        if &head[..4] != b"8BIM" {
            return false;
        }
        let key: [u8; 4] = head[4..8].try_into().unwrap_or_default();
        let big = doc.is_psb && is_psb_big_key(&key);
        let width = if big { 8 } else { 4 };
        let Some(len_bytes) = extra.get(at + 8..at + 8 + width) else {
            return false;
        };
        let len = len_bytes
            .iter()
            .fold(0usize, |acc, &b| (acc << 8) | b as usize);
        let start = at + 8 + width;
        let Some(data) = extra.get(start..start + len) else {
            return false;
        };
        let next = (start + len).next_multiple_of(4).min(extra.len());
        if &key == b"Patt" && !added {
            let mut data = data.to_vec();
            data.extend_from_slice(&entry);
            push_block(&mut out, &key, &data);
            added = true;
        } else {
            out.extend_from_slice(&extra[at..next]);
        }
        at = next;
    }
    if !added {
        push_block(&mut out, b"Patt", &entry);
    }
    doc.layer_section_extra = out.into();
    true
}

fn push_block(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    be32(out, data.len() as u32);
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(4), 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_patterns;
    use pictura_core::{BitDepth, ColorMode};

    fn tile(seed: u8) -> Vec<u8> {
        (0..4u8 * 3 * 4).map(|i| i.wrapping_mul(seed) | 3).collect()
    }

    #[test]
    fn written_patterns_decode_back() {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let (a, b) = (tile(7), tile(11));
        let rec_a = encode_rgb_pattern("Grid", "kooka-a", 4, 3, &a).unwrap();
        let rec_b = encode_rgb_pattern("Dots", "kooka-b", 4, 3, &b).unwrap();
        assert!(add_document_pattern(&mut doc, "kooka-a", &rec_a));
        assert!(add_document_pattern(&mut doc, "kooka-b", &rec_b));
        assert!(
            !add_document_pattern(&mut doc, "kooka-a", &rec_a),
            "already there"
        );
        let decoded = decode_patterns(&doc);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].pattern_id, "kooka-a");
        assert_eq!((decoded[1].width, decoded[1].height), (4, 3));
        let opaque =
            |t: &[u8]| -> Vec<u8> { t.chunks(4).flat_map(|p| [p[0], p[1], p[2], 255]).collect() };
        assert_eq!(decoded[0].rgba, opaque(&a));
        assert_eq!(decoded[1].rgba, opaque(&b));
        assert_eq!(doc.layer_section_extra.len() % 4, 0);
    }

    #[test]
    fn other_blocks_are_kept_and_a_save_keeps_the_patterns() {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layer_section_extra = [b"8BIM".as_slice(), b"Xyzw", &[0, 0, 0, 2], &[9, 9, 0, 0]]
            .concat()
            .into();
        let rec = encode_rgb_pattern("P", "kooka-p", 4, 3, &tile(5)).unwrap();
        assert!(add_document_pattern(&mut doc, "kooka-p", &rec));
        assert!(doc
            .layer_section_extra
            .starts_with(b"8BIMXyzw\0\0\0\x02\x09\x09\0\0"));
        let reopened = crate::read_psd(&crate::write_psd(&doc).unwrap()).unwrap();
        assert_eq!(decode_patterns(&reopened)[0].pattern_id, "kooka-p");
    }

    #[test]
    fn a_bad_size_is_refused() {
        assert!(encode_rgb_pattern("P", "x", 0, 3, &[]).is_none());
        assert!(encode_rgb_pattern("P", "x", 2, 2, &[0; 15]).is_none());
    }
}
