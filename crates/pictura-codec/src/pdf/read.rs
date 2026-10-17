//! Reopening a Photoshop PDF with its layers: Pictura's own (the PSD payload
//! the writer embeds under `/PieceInfo /Pictura`, design D6) and Photoshop's
//! (its `/PieceInfo /AdobePhotoshop` editing data, rebuilt into a PSD). Any
//! other PDF goes through `import` (#627).

use std::io::Read as _;

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
    let mut psd = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut psd)
        .ok()?;
    psd.starts_with(b"8BPS").then_some(psd)
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
/// Bitmap, Duotone, or Multichannel source, or an encrypted file, is `None`
/// and imports flattened instead.
pub fn photoshop_psd(pdf: &[u8]) -> Option<Vec<u8>> {
    use super::import::objects::{decode_stream, Object, PdfObjects};
    if !pdf.starts_with(b"%PDF-") || find(pdf, b"/AdobePhotoshop", 0).is_none() {
        return None;
    }
    let file = PdfObjects::parse(pdf)?;
    if file.encrypted() {
        return None;
    }
    let catalog = file.resolve_dict(file.trailer.get("Root"))?;
    let mut node = file.resolve_dict(catalog.get("Pages"))?;
    // The first leaf page.
    while let Some(Object::Array(kids)) = node.get("Kids").map(|k| file.resolve(k)) {
        node = file.resolve_dict(kids.first())?;
    }
    let private = file.resolve_dict(
        file.resolve_dict(
            file.resolve_dict(node.get("PieceInfo"))?
                .get("AdobePhotoshop"),
        )?
        .get("Private"),
    )?;
    let stream = |key: &str| -> Option<(super::import::objects::Dict, Vec<u8>)> {
        match file.resolve(private.get(key)?) {
            Object::Stream(dict, data) => Some((dict.clone(), decode_stream(&file, dict, data)?)),
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
    if !(depth == 8 || depth == 16) || width <= 0 || height <= 0 {
        return None;
    }
    let (channels, mode) = composite_mode(&file, composite_dict.get("ColorSpace")?)?;
    let bytes = depth as usize / 8;
    let plane = width as usize * height as usize * bytes;
    if composite.len() < plane * channels {
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
    let section = 4 + layer_info.len() + 4 + global_mask.len() + tagged.len();
    psd.extend_from_slice(&u32b(section));
    psd.extend_from_slice(&u32b(layer_info.len()));
    psd.extend_from_slice(&layer_info);
    psd.extend_from_slice(&u32b(global_mask.len()));
    psd.extend_from_slice(&global_mask);
    psd.extend_from_slice(&tagged);
    // Raw planar image data from the interleaved composite.
    psd.extend_from_slice(&u16b(0));
    let pixels = width as usize * height as usize;
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
fn composite_mode(
    file: &super::import::objects::PdfObjects,
    space: &super::import::objects::Object,
) -> Option<(usize, usize)> {
    use super::import::objects::Object;
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
