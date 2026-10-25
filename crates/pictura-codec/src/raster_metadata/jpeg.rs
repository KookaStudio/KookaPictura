//! JPEG metadata: insert APP1 Exif, APP1 XMP, and APP13 Photoshop IRB resources
//! after SOI, replacing any existing copies, and read the IRB back. The
//! entropy-coded scan after SOS is copied verbatim.

use crate::error::PsdError;

use super::RasterMetadata;

const XMP_SIGNATURE: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const PHOTOSHOP_SIGNATURE: &[u8] = b"Photoshop 3.0\0";
/// The most IRB bytes one APP13 segment holds; a longer block runs on in
/// the next segment, as Photoshop splits it.
const IRB_CHUNK: usize = u16::MAX as usize - 2 - PHOTOSHOP_SIGNATURE.len();
const ICC_SIGNATURE: &[u8] = b"ICC_PROFILE\0";
/// The ICC_PROFILE header plus its sequence/count bytes.
const ICC_HEADER: usize = 14;
/// The largest profile chunk one APP2 segment can carry.
const ICC_CHUNK: usize = u16::MAX as usize - 2 - ICC_HEADER;

pub(super) fn embed(bytes: &[u8], metadata: &RasterMetadata) -> Result<Vec<u8>, PsdError> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err(PsdError::Invalid("JPEG is missing its SOI marker".into()));
    }
    let mut out = Vec::with_capacity(bytes.len() + 4096);
    out.extend_from_slice(&bytes[..2]);
    if let Some(exif) = &metadata.exif {
        let mut payload = Vec::with_capacity(exif.len() + 6);
        if !exif.starts_with(b"Exif\0\0") {
            payload.extend_from_slice(b"Exif\0\0");
        }
        payload.extend_from_slice(exif);
        push_segment(&mut out, 0xE1, &payload)?;
    }
    if let Some(xmp) = &metadata.xmp {
        let mut payload = Vec::with_capacity(XMP_SIGNATURE.len() + xmp.len());
        payload.extend_from_slice(XMP_SIGNATURE);
        payload.extend_from_slice(xmp);
        push_segment(&mut out, 0xE1, &payload)?;
    }
    if let Some(block) = metadata.photoshop_block() {
        for chunk in block.chunks(IRB_CHUNK) {
            push_segment(&mut out, 0xED, &[PHOTOSHOP_SIGNATURE, chunk].concat())?;
        }
    }
    if let Some(icc) = &metadata.icc {
        push_icc(&mut out, icc)?;
    }

    let mut at = 2;
    while at < bytes.len() {
        if bytes[at] != 0xFF || at + 1 >= bytes.len() {
            return Err(PsdError::Invalid("JPEG marker expected".into()));
        }
        let marker = bytes[at + 1];
        if marker == 0xDA || marker == 0xD9 {
            out.extend_from_slice(&bytes[at..]);
            return Ok(out);
        }
        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            out.extend_from_slice(&bytes[at..at + 2]);
            at += 2;
            continue;
        }
        if at + 4 > bytes.len() {
            return Err(PsdError::Truncated);
        }
        let length = u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
        if length < 2 || at + 2 + length > bytes.len() {
            return Err(PsdError::Truncated);
        }
        let payload = &bytes[at + 4..at + 2 + length];
        let metadata_segment = (marker == 0xE1
            && (payload.starts_with(b"Exif\0\0") || payload.starts_with(XMP_SIGNATURE)))
            || (marker == 0xED && payload.starts_with(PHOTOSHOP_SIGNATURE))
            || (marker == 0xE2 && payload.starts_with(ICC_SIGNATURE));
        if !metadata_segment {
            out.extend_from_slice(&bytes[at..at + 2 + length]);
        }
        at += 2 + length;
    }
    Ok(out)
}

fn push_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) -> Result<(), PsdError> {
    let length = payload.len() + 2;
    if length > u16::MAX as usize {
        return Err(PsdError::Unsupported(
            "metadata segment is too large".into(),
        ));
    }
    out.push(0xFF);
    out.push(marker);
    out.extend_from_slice(&(length as u16).to_be_bytes());
    out.extend_from_slice(payload);
    Ok(())
}

/// The IRB of every APP13 Photoshop segment before the scan, joined in file
/// order; `None` when there is none or the markers are malformed.
pub(super) fn photoshop_block(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut block: Option<Vec<u8>> = None;
    let mut at = 2;
    while at + 4 <= bytes.len() && bytes[at] == 0xFF {
        let marker = bytes[at + 1];
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 || marker == 0xFF {
            at += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let length = u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
        let payload = bytes.get(at + 4..(at + 2 + length).max(at + 4))?;
        if marker == 0xED {
            if let Some(irb) = payload.strip_prefix(PHOTOSHOP_SIGNATURE) {
                block.get_or_insert_with(Vec::new).extend_from_slice(irb);
            }
        }
        at += 2 + length.max(2);
    }
    block
}

fn push_icc(out: &mut Vec<u8>, icc: &[u8]) -> Result<(), PsdError> {
    let chunks = icc.len().div_ceil(ICC_CHUNK);
    if chunks > u8::MAX as usize {
        return Err(PsdError::Unsupported("ICC profile is too large".into()));
    }
    for (index, chunk) in icc.chunks(ICC_CHUNK).enumerate() {
        let mut payload = Vec::with_capacity(ICC_HEADER + chunk.len());
        payload.extend_from_slice(ICC_SIGNATURE);
        payload.push(index as u8 + 1);
        payload.push(chunks as u8);
        payload.extend_from_slice(chunk);
        push_segment(out, 0xE2, &payload)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn count(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }

    #[test]
    fn jpeg_metadata_injection() {
        let metadata = RasterMetadata {
            exif: Some(b"II*\0exif".to_vec()),
            iptc: Some(b"iptc".to_vec()),
            xmp: Some(b"<x:xmpmeta/>".to_vec()),
            vanishing_point: None,
            icc: Some(b"fake-icc".to_vec()),
        };
        let mut existing = vec![0xFF, 0xD8];
        push_segment(&mut existing, 0xE1, b"Exif\0\0old").expect("existing exif");
        push_segment(&mut existing, 0xED, b"Photoshop 3.0\0old").expect("existing iptc");
        existing.extend_from_slice(&[0xFF, 0xD9]);

        let out = embed(&existing, &metadata).expect("embed");
        assert!(out.starts_with(&[0xFF, 0xD8, 0xFF, 0xE1]));
        assert!(out.ends_with(&[0xFF, 0xD9]));
        assert_eq!(count(&out, b"Exif\0\0"), 1, "the old Exif is replaced");
        assert_eq!(
            count(&out, b"http://ns.adobe.com/xap/1.0/\0"),
            1,
            "the XMP packet is inserted"
        );
        assert_eq!(
            count(&out, b"Photoshop 3.0\0"),
            1,
            "the old IRB is replaced"
        );
        assert!(find(&out, &[0x04, 0x04]), "the IPTC resource id is present");
        assert!(find(&out, b"<x:xmpmeta/>"));
        assert!(find(&out, b"fake-icc"), "the ICC profile is inserted");
    }

    #[test]
    fn the_photoshop_block_round_trips_and_splits_across_segments() {
        let planes = vec![7u8; 100_001];
        let metadata = RasterMetadata {
            iptc: Some(b"iptc".to_vec()),
            vanishing_point: Some(planes.clone()),
            ..RasterMetadata::default()
        };
        let mut plain = vec![0xFF, 0xD8];
        push_segment(&mut plain, 0xE0, b"JFIF\0").expect("app0");
        plain.extend_from_slice(&[0xFF, 0xDA, 0, 2, 0xFF, 0xD9]);
        assert_eq!(photoshop_block(&plain), None);

        let out = embed(&plain, &metadata).expect("embed");
        assert_eq!(
            count(&out, PHOTOSHOP_SIGNATURE),
            2,
            "split over two APP13 segments"
        );
        let block = photoshop_block(&out).expect("the block reads back");
        assert_eq!(Some(block), metadata.photoshop_block());
    }
}
