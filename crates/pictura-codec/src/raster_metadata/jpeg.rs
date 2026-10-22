//! JPEG metadata: insert APP1 Exif, APP1 XMP, and APP13 Photoshop IRB resources
//! after SOI, replacing any existing copies. The entropy-coded scan after SOS is
//! copied verbatim.

use crate::error::PsdError;

use super::RasterMetadata;

const XMP_SIGNATURE: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const PHOTOSHOP_SIGNATURE: &[u8] = b"Photoshop 3.0\0";
const IPTC_RESOURCE: u16 = 0x0404;

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
    if let Some(iptc) = &metadata.iptc {
        push_segment(&mut out, 0xED, &photoshop_irb(iptc))?;
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
            || (marker == 0xED && payload.starts_with(PHOTOSHOP_SIGNATURE));
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

fn photoshop_irb(iptc: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(PHOTOSHOP_SIGNATURE.len() + 16 + iptc.len());
    out.extend_from_slice(PHOTOSHOP_SIGNATURE);
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(&IPTC_RESOURCE.to_be_bytes());
    out.extend_from_slice(&[0, 0]); // empty Pascal name, padded
    out.extend_from_slice(&(iptc.len() as u32).to_be_bytes());
    out.extend_from_slice(iptc);
    if !iptc.len().is_multiple_of(2) {
        out.push(0);
    }
    out
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
    }
}
