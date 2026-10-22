//! Reopening a Photoshop PDF with its layers: Pictura's own (the PSD payload
//! the writer embeds under `/PieceInfo /Pictura`, design D6) and Photoshop's
//! (its `/PieceInfo /AdobePhotoshop` editing data, rebuilt into a PSD),
//! decrypting a password-protected one first. Any other PDF goes through
//! `import` (#627).

use std::collections::HashSet;

use super::import::objects::{decode_stream, Dict, Object, PdfObjects, Unlock};
use crate::probe::ImageBudget;

/// The widest or tallest composite a rebuilt PSD header can carry (version 1).
const MAX_EDITING_DIMENSION: i64 = 30_000;

/// What a PDF's editing data opens as.
#[derive(Debug, PartialEq, Eq)]
pub enum EditingPsd {
    Psd(Vec<u8>),
    /// No editing data that can be read: import the PDF instead.
    Absent,
    /// Encrypted, and the password opens neither its user nor owner entry.
    Locked,
}

/// The PSD from a PDF's editing data (Pictura's payload, else Photoshop's),
/// decrypting an encrypted file with `password`. The empty password opens a
/// file that has only a permissions password.
pub fn editing_psd(pdf: &[u8], password: &str) -> EditingPsd {
    if let Some(psd) = embedded_psd(pdf).or_else(|| photoshop_psd(pdf)) {
        return EditingPsd::Psd(psd);
    }
    if !pdf.starts_with(b"%PDF-") || find(pdf, b"/Encrypt", 0).is_none() {
        return EditingPsd::Absent;
    }
    let Some(mut file) = PdfObjects::parse(pdf) else {
        return EditingPsd::Absent;
    };
    match file.decrypt(password) {
        Unlock::Open => {}
        Unlock::WrongPassword => return EditingPsd::Locked,
        Unlock::Unsupported => return EditingPsd::Absent,
    }
    pictura_psd_in(&file)
        .or_else(|| photoshop_psd_in(&file))
        .map_or(EditingPsd::Absent, EditingPsd::Psd)
}

/// The first leaf page's `/PieceInfo /<owner> /Private` dictionary. A page
/// tree that loops back on itself has no first leaf.
fn private_data<'a>(file: &'a PdfObjects, owner: &str) -> Option<&'a Dict> {
    let catalog = file.resolve_dict(file.trailer.get("Root"))?;
    let mut node = file.resolve_dict(catalog.get("Pages"))?;
    let mut visited = HashSet::new();
    while let Some(Object::Array(kids)) = node.get("Kids").map(|k| file.resolve(k)) {
        let kid = kids.first()?;
        if let Object::Ref(num) = kid {
            if !visited.insert(*num) {
                return None;
            }
        }
        node = file.resolve_dict(Some(kid))?;
    }
    file.resolve_dict(
        file.resolve_dict(file.resolve_dict(node.get("PieceInfo"))?.get(owner))?
            .get("Private"),
    )
}

/// Pictura's payload read through the object reader (an encrypted file,
/// once decrypted).
fn pictura_psd_in(file: &PdfObjects) -> Option<Vec<u8>> {
    match file.resolve(private_data(file, "Pictura")?.get("PSD")?) {
        Object::Stream(dict, data) => {
            decode_stream(file, dict, data).filter(|psd| psd.starts_with(b"8BPS"))
        }
        _ => None,
    }
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

/// The embedded PSD bytes of an unencrypted Pictura Photoshop PDF, or `None`
/// for any other file (no payload, encrypted, or malformed).
pub fn embedded_psd(pdf: &[u8]) -> Option<Vec<u8>> {
    if !pdf.starts_with(b"%PDF-") || find(pdf, b"/Encrypt", 0).is_some() {
        return None;
    }
    let marker = find(pdf, b"/Subtype /PicturaPSD", 0)?;
    let dict_end = find(pdf, b">>", marker)?;
    let length_at = find(&pdf[..dict_end], b"/Length ", marker)? + b"/Length ".len();
    let digits = pdf[length_at..dict_end]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let length: usize = std::str::from_utf8(&pdf[length_at..length_at + digits])
        .ok()?
        .parse()
        .ok()?;
    let start = find(pdf, b"stream\n", dict_end)? + b"stream\n".len();
    let data = pdf.get(start..start.checked_add(length)?)?;
    let psd = inflate_payload(data, ImageBudget::default().max_alloc_bytes)?;
    psd.starts_with(b"8BPS").then_some(psd)
}

/// Inflate an embedded PSD payload, or `None` past `limit` bytes or on a
/// damaged stream.
fn inflate_payload(data: &[u8], limit: u64) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let mut psd = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .take(limit.saturating_add(1))
        .read_to_end(&mut psd)
        .ok()?;
    (psd.len() as u64 <= limit).then_some(psd)
}

/// A PSD rebuilt from the editing data Photoshop stores in a PDF saved with
/// Preserve Photoshop Editing Capabilities, or `None` for any other file.
///
/// The first page's `/PieceInfo /AdobePhotoshop /Private` dictionary holds
/// `/ImageResources` (a PSD image-resources section), `/StandardImageFileData`
/// (the layer data as the tagged blocks a layered TIFF carries: `Layr`, then
/// `LMsk`, then additional layer information, each padded to 4 bytes), and
/// `/CompositeImage` (the flattened image, whose size, colour space, and depth
/// give the PSD header). Layout read from Photoshop's own output; Adobe does
/// not publish it.
///
/// ponytail: Gray, RGB, CMYK, and Lab composites at 8 or 16 bits; an Indexed,
/// Bitmap, Duotone, or Multichannel source is `None` and imports flattened
/// instead. An encrypted file is `None` here; `editing_psd` decrypts it.
pub fn photoshop_psd(pdf: &[u8]) -> Option<Vec<u8>> {
    if !pdf.starts_with(b"%PDF-") || find(pdf, b"/AdobePhotoshop", 0).is_none() {
        return None;
    }
    let file = PdfObjects::parse(pdf)?;
    if file.encrypted() {
        return None;
    }
    photoshop_psd_in(&file)
}

fn photoshop_psd_in(file: &PdfObjects) -> Option<Vec<u8>> {
    let private = private_data(file, "AdobePhotoshop")?;
    let stream = |key: &str| -> Option<(Dict, Vec<u8>)> {
        match file.resolve(private.get(key)?) {
            Object::Stream(dict, data) => Some((dict.clone(), decode_stream(file, dict, data)?)),
            _ => None,
        }
    };
    let (_, resources) = stream("ImageResources")?;
    let (_, blocks) = stream("StandardImageFileData")?;
    let (composite_dict, composite) = stream("CompositeImage")?;

    let int = |key: &str| {
        composite_dict
            .get(key)
            .map(|v| file.resolve(v))
            .and_then(Object::as_int)
    };
    let (width, height, depth) = (int("Width")?, int("Height")?, int("BitsPerComponent")?);
    if !(depth == 8 || depth == 16)
        || !(1..=MAX_EDITING_DIMENSION).contains(&width)
        || !(1..=MAX_EDITING_DIMENSION).contains(&height)
    {
        return None;
    }
    let (channels, mode) = composite_mode(file, composite_dict.get("ColorSpace")?)?;
    let bytes = depth as usize / 8;
    let pixels = width as usize * height as usize;
    let plane = pixels.checked_mul(bytes)?;
    if composite.len() < plane.checked_mul(channels)? {
        return None;
    }

    let mut layer_info = None;
    let mut global_mask = Vec::new();
    let mut tagged = Vec::new();
    let mut at = 0;
    while at + 12 <= blocks.len() {
        let signature = &blocks[at..at + 4];
        if signature != b"8BIM" && signature != b"8B64" {
            return None;
        }
        let key = &blocks[at + 4..at + 8];
        let len = u32::from_be_bytes(blocks[at + 8..at + 12].try_into().ok()?) as usize;
        let data = blocks.get(at + 12..at + 12 + len)?;
        match key {
            b"Layr" => layer_info = Some(data.to_vec()),
            b"LMsk" => global_mask = data.to_vec(),
            _ => {
                tagged.extend_from_slice(&blocks[at..at + 12]);
                tagged.extend_from_slice(data);
                tagged.resize(tagged.len() + (4 - len % 4) % 4, 0);
            }
        }
        at += 12 + len;
        at += (4 - at % 4) % 4;
    }
    let mut layer_info = layer_info.unwrap_or_default();
    if layer_info.len() % 2 == 1 {
        layer_info.push(0);
    }

    // Every length below must fit its PSD field; a larger one is refused,
    // never truncated into a different file.
    let section = 4 + layer_info.len() + 4 + global_mask.len() + tagged.len();
    u32::try_from(section).ok()?;
    u32::try_from(resources.len()).ok()?;
    let u16b = |v: usize| (v as u16).to_be_bytes();
    let u32b = |v: usize| (v as u32).to_be_bytes();
    let mut psd = b"8BPS".to_vec();
    psd.extend_from_slice(&u16b(1));
    psd.extend_from_slice(&[0; 6]);
    psd.extend_from_slice(&u16b(channels));
    psd.extend_from_slice(&u32b(height as usize));
    psd.extend_from_slice(&u32b(width as usize));
    psd.extend_from_slice(&u16b(depth as usize));
    psd.extend_from_slice(&u16b(mode));
    psd.extend_from_slice(&u32b(0));
    psd.extend_from_slice(&u32b(resources.len()));
    psd.extend_from_slice(&resources);
    psd.extend_from_slice(&u32b(section));
    psd.extend_from_slice(&u32b(layer_info.len()));
    psd.extend_from_slice(&layer_info);
    psd.extend_from_slice(&u32b(global_mask.len()));
    psd.extend_from_slice(&global_mask);
    psd.extend_from_slice(&tagged);
    // Raw planar image data from the interleaved composite.
    psd.extend_from_slice(&u16b(0));
    // PSD stores CMYK as ink-inverted values; PDF DeviceCMYK is ink amount.
    let invert = mode == 4;
    for c in 0..channels {
        for p in 0..pixels {
            let at = (p * channels + c) * bytes;
            for b in 0..bytes {
                let v = composite[at + b];
                psd.push(if invert { 255 - v } else { v });
            }
        }
    }
    Some(psd)
}

/// The PSD channel count and colour-mode number for a composite's colour
/// space, or `None` for a space the rebuild does not handle.
fn composite_mode(file: &PdfObjects, space: &Object) -> Option<(usize, usize)> {
    match file.resolve(space) {
        Object::Name(n) if n == b"DeviceGray" => Some((1, 1)),
        Object::Name(n) if n == b"DeviceRGB" => Some((3, 3)),
        Object::Name(n) if n == b"DeviceCMYK" => Some((4, 4)),
        Object::Array(items) => match items.first() {
            Some(name) if name.is_name("Lab") => Some((3, 9)),
            Some(name) if name.is_name("ICCBased") => {
                let n = file
                    .resolve_dict(items.get(1))?
                    .get("N")
                    .and_then(Object::as_int)?;
                match n {
                    1 => Some((1, 1)),
                    3 => Some((3, 3)),
                    4 => Some((4, 4)),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zlib(data: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn an_embedded_payload_past_the_limit_is_refused() {
        let payload = zlib(&[0u8; 4096]);
        assert_eq!(inflate_payload(&payload, 4096).map(|p| p.len()), Some(4096));
        assert_eq!(inflate_payload(&payload, 4095), None);
    }

    /// A one-page PDF whose page `PieceInfo /AdobePhotoshop /Private` holds
    /// the three editing streams, the composite with `composite` as its dict.
    fn photoshop_pdf(composite: &str, pages: &str) -> Vec<u8> {
        let mut pdf = b"%PDF-1.7\n".to_vec();
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            pages.to_string(),
            "<< /Type /Page /Parent 2 0 R /PieceInfo << /AdobePhotoshop << /Private 4 0 R >> >> >>"
                .to_string(),
            "<< /ImageResources 5 0 R /StandardImageFileData 6 0 R /CompositeImage 7 0 R >>"
                .to_string(),
        ];
        for (i, body) in objects.iter().enumerate() {
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        for (num, dict, data) in [
            (5, "<< /Length 0 >>", &b""[..]),
            (6, "<< /Length 0 >>", &b""[..]),
            (7, composite, &[0u8; 6][..]),
        ] {
            pdf.extend_from_slice(format!("{num} 0 obj\n{dict}\nstream\n").as_bytes());
            pdf.extend_from_slice(data);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }
        pdf.extend_from_slice(b"trailer\n<< /Root 1 0 R >>\n%%EOF\n");
        pdf
    }

    #[test]
    fn an_overflowing_composite_size_is_refused() {
        // Width 2^62 + 1 wrapped `width * height * bytes` past the 6-byte
        // composite before the dimension cap (#844 H4).
        let pdf = photoshop_pdf(
            "<< /Length 6 /Width 4611686018427387905 /Height 2 /BitsPerComponent 8 /ColorSpace /DeviceGray >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        assert_eq!(photoshop_psd(&pdf), None);
        let fits = photoshop_pdf(
            "<< /Length 6 /Width 3 /Height 2 /BitsPerComponent 8 /ColorSpace /DeviceGray >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        assert!(photoshop_psd(&fits).is_some_and(|psd| psd.starts_with(b"8BPS")));
    }

    #[test]
    fn a_self_referential_page_tree_ends() {
        // `/Kids [2 0 R]` on the Pages node itself looped forever (#844 H6).
        let pdf = photoshop_pdf(
            "<< /Length 6 /Width 3 /Height 2 /BitsPerComponent 8 /ColorSpace /DeviceGray >>",
            "<< /Type /Pages /Kids [2 0 R] /Count 1 >>",
        );
        assert_eq!(photoshop_psd(&pdf), None);
    }
}
