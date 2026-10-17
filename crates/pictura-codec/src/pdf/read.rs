//! Reopening a Pictura Photoshop PDF: find the PSD payload the writer embeds
//! under `/PieceInfo /Pictura` (design D6). This reads Pictura's own output
//! only; importing arbitrary PDFs is #627.

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
