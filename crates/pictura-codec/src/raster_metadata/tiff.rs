//! TIFF metadata: merge the document's EXIF IFD0 (and Exif sub-IFD), IPTC-IIM,
//! XMP and Photoshop resource block into a fresh IFD0 at the end of the file,
//! and read the resource block back. The original bytes stay in place, so
//! Qt's strip offsets and any other data remain valid.

use std::borrow::Cow;

use crate::error::PsdError;

use super::RasterMetadata;

const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_XMP: u16 = 700;
const TAG_IPTC: u16 = 33723;
const TAG_EXIF_POINTER: u16 = 34665;
const TAG_PHOTOSHOP: u16 = 34377;

/// Tags whose value is an offset into the TIFF/EXIF block they came from. The
/// destination keeps its own copies, so an offset carried over from the
/// document's EXIF blob would point into whatever happens to be at the same
/// spot in the output.
/// ponytail: GPS, Interop, thumbnail, and SubIFD blobs are dropped rather than
/// recursively copied; add a blob copier when a real file needs those fields.
fn is_offset_tag(tag: u16) -> bool {
    matches!(
        tag,
        0x0111 | 0x0117 | 0x014A | 0x0201 | 0x0202 | TAG_EXIF_IFD | 0x8825 | 0xA005
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Order {
    Little,
    Big,
}

impl Order {
    fn u16(self, bytes: &[u8], at: usize) -> u16 {
        let pair = [bytes[at], bytes[at + 1]];
        match self {
            Order::Little => u16::from_le_bytes(pair),
            Order::Big => u16::from_be_bytes(pair),
        }
    }

    fn u32(self, bytes: &[u8], at: usize) -> u32 {
        let quad = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
        match self {
            Order::Little => u32::from_le_bytes(quad),
            Order::Big => u32::from_be_bytes(quad),
        }
    }

    fn u16_bytes(self, value: u16) -> [u8; 2] {
        match self {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        }
    }

    fn u32_bytes(self, value: u32) -> [u8; 4] {
        match self {
            Order::Little => value.to_le_bytes(),
            Order::Big => value.to_be_bytes(),
        }
    }

    fn put_u32(self, dst: &mut [u8], value: u32) {
        dst.copy_from_slice(&self.u32_bytes(value));
    }
}

struct RawEntry {
    tag: u16,
    typ: u16,
    count: u32,
    value: [u8; 4],
}

struct OutEntry {
    tag: u16,
    typ: u16,
    count: u32,
    inline: Option<[u8; 4]>,
    data: Option<Vec<u8>>,
}

fn align2(value: usize) -> usize {
    (value + 1) & !1
}

fn parse_header(bytes: &[u8]) -> Result<(Order, u32), PsdError> {
    if bytes.len() < 8 {
        return Err(PsdError::Truncated);
    }
    let order = match &bytes[..2] {
        b"II" => Order::Little,
        b"MM" => Order::Big,
        _ => return Err(PsdError::Invalid("TIFF byte order".into())),
    };
    if order.u16(bytes, 2) != 42 {
        return Err(PsdError::Invalid("TIFF magic".into()));
    }
    Ok((order, order.u32(bytes, 4)))
}

fn type_size(typ: u16) -> Option<usize> {
    Some(match typ {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => return None,
    })
}

fn read_ifd(bytes: &[u8], order: Order, offset: u32) -> Result<Vec<RawEntry>, PsdError> {
    let at = offset as usize;
    if at + 2 > bytes.len() {
        return Err(PsdError::Truncated);
    }
    let count = order.u16(bytes, at) as usize;
    let mut entries = Vec::with_capacity(count);
    let mut p = at + 2;
    for _ in 0..count {
        if p + 12 > bytes.len() {
            return Err(PsdError::Truncated);
        }
        entries.push(RawEntry {
            tag: order.u16(bytes, p),
            typ: order.u16(bytes, p + 2),
            count: order.u32(bytes, p + 4),
            value: [bytes[p + 8], bytes[p + 9], bytes[p + 10], bytes[p + 11]],
        });
        p += 12;
    }
    Ok(entries)
}

fn entry_data<'a>(
    source: &'a [u8],
    order: Order,
    entry: &RawEntry,
) -> Result<Cow<'a, [u8]>, PsdError> {
    let size = type_size(entry.typ)
        .and_then(|size| size.checked_mul(entry.count as usize))
        .ok_or_else(|| PsdError::Invalid("TIFF entry size".into()))?;
    if size <= 4 {
        return Ok(Cow::Owned(entry.value[..size].to_vec()));
    }
    let at = order.u32(&entry.value, 0) as usize;
    let end = at
        .checked_add(size)
        .ok_or_else(|| PsdError::Invalid("TIFF value size".into()))?;
    source
        .get(at..end)
        .map(Cow::Borrowed)
        .ok_or(PsdError::Truncated)
}

/// Convert the components of a value between byte orders. ASCII/BYTE/UNDEFINED
/// bytes are unchanged; numeric components are reversed.
fn convert(data: &[u8], typ: u16, source: Order, destination: Order) -> Vec<u8> {
    if source == destination {
        return data.to_vec();
    }
    let element = match typ {
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => 1,
    };
    if element == 1 {
        return data.to_vec();
    }
    let mut out = data.to_vec();
    for chunk in out.chunks_exact_mut(element) {
        if element == 8 && matches!(typ, 5 | 10) {
            let read = |bytes: [u8; 4]| match source {
                Order::Little => u32::from_le_bytes(bytes),
                Order::Big => u32::from_be_bytes(bytes),
            };
            let first = read([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let second = read([chunk[4], chunk[5], chunk[6], chunk[7]]);
            let first = match destination {
                Order::Little => first.to_le_bytes(),
                Order::Big => first.to_be_bytes(),
            };
            let second = match destination {
                Order::Little => second.to_le_bytes(),
                Order::Big => second.to_be_bytes(),
            };
            chunk[..4].copy_from_slice(&first);
            chunk[4..].copy_from_slice(&second);
        } else {
            chunk.reverse();
        }
    }
    out
}

fn copy_entry(
    source: &[u8],
    source_order: Order,
    destination_order: Order,
    entry: &RawEntry,
) -> Result<OutEntry, PsdError> {
    let data = entry_data(source, source_order, entry)?;
    if data.len() <= 4 {
        let converted = convert(&data, entry.typ, source_order, destination_order);
        let mut inline = [0u8; 4];
        inline[..converted.len()].copy_from_slice(&converted);
        Ok(OutEntry {
            tag: entry.tag,
            typ: entry.typ,
            count: entry.count,
            inline: Some(inline),
            data: None,
        })
    } else {
        Ok(OutEntry {
            tag: entry.tag,
            typ: entry.typ,
            count: entry.count,
            inline: None,
            data: Some(convert(&data, entry.typ, source_order, destination_order)),
        })
    }
}

fn write_ifd(out: &mut Vec<u8>, entries: Vec<OutEntry>, order: Order) -> Result<u32, PsdError> {
    if entries.len() > u16::MAX as usize {
        return Err(PsdError::Unsupported("too many TIFF entries".into()));
    }
    let base = align2(out.len());
    out.resize(base, 0);
    let data_base = base + 2 + 12 * entries.len() + 4;
    let mut table = Vec::with_capacity(2 + 12 * entries.len() + 4);
    table.extend_from_slice(&order.u16_bytes(entries.len() as u16));
    let mut data = Vec::new();
    for entry in entries {
        table.extend_from_slice(&order.u16_bytes(entry.tag));
        table.extend_from_slice(&order.u16_bytes(entry.typ));
        table.extend_from_slice(&order.u32_bytes(entry.count));
        if let Some(inline) = entry.inline {
            table.extend_from_slice(&inline);
        } else if let Some(bytes) = entry.data {
            let offset = data_base + data.len();
            table.extend_from_slice(&order.u32_bytes(offset as u32));
            data.extend_from_slice(&bytes);
            if !data.len().is_multiple_of(2) {
                data.push(0);
            }
        } else {
            table.extend_from_slice(&[0, 0, 0, 0]);
        }
    }
    table.extend_from_slice(&order.u32_bytes(0)); // next IFD
    out.extend_from_slice(&table);
    out.extend_from_slice(&data);
    Ok(base as u32)
}

pub(super) fn embed(bytes: &[u8], metadata: &RasterMetadata) -> Result<Vec<u8>, PsdError> {
    let (destination_order, destination_ifd) = parse_header(bytes)?;
    let destination_entries = read_ifd(bytes, destination_order, destination_ifd)?;
    let mut out = bytes.to_vec();

    let exif = metadata.exif.as_deref().and_then(|raw| {
        let raw = raw.strip_prefix(b"Exif\0\0".as_slice()).unwrap_or(raw);
        let (order, ifd0_offset) = parse_header(raw).ok()?;
        let ifd0 = read_ifd(raw, order, ifd0_offset).ok()?;
        let sub_ifd = ifd0
            .iter()
            .find(|entry| entry.tag == TAG_EXIF_IFD)
            .and_then(|entry| {
                let at = order.u32(&entry.value, 0);
                read_ifd(raw, order, at).ok()
            })
            .unwrap_or_default();
        Some((raw, order, ifd0, sub_ifd))
    });

    let mut exif_ifd_offset = None;
    if let Some((raw, source_order, _, sub_ifd)) = &exif {
        let entries: Vec<OutEntry> = sub_ifd
            .iter()
            .filter(|entry| !is_offset_tag(entry.tag))
            .filter_map(|entry| copy_entry(raw, *source_order, destination_order, entry).ok())
            .collect();
        exif_ifd_offset = Some(write_ifd(&mut out, entries, destination_order)?);
    }

    let mut entries: Vec<OutEntry> = destination_entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.tag,
                TAG_XMP | TAG_IPTC | TAG_EXIF_POINTER | TAG_PHOTOSHOP
            )
        })
        .map(|entry| OutEntry {
            tag: entry.tag,
            typ: entry.typ,
            count: entry.count,
            inline: Some(entry.value),
            data: None,
        })
        .collect();

    if let Some((raw, source_order, ifd0, _)) = &exif {
        for entry in ifd0 {
            if is_offset_tag(entry.tag) || entries.iter().any(|out| out.tag == entry.tag) {
                continue;
            }
            if let Ok(converted) = copy_entry(raw, *source_order, destination_order, entry) {
                entries.push(converted);
            }
        }
    }
    if let Some(iptc) = &metadata.iptc {
        entries.push(OutEntry {
            tag: TAG_IPTC,
            typ: 7,
            count: iptc.len() as u32,
            inline: None,
            data: Some(iptc.clone()),
        });
    }
    if let Some(xmp) = &metadata.xmp {
        entries.push(OutEntry {
            tag: TAG_XMP,
            typ: 1,
            count: xmp.len() as u32,
            inline: None,
            data: Some(xmp.clone()),
        });
    }
    if let Some(block) = metadata.photoshop_block() {
        entries.push(OutEntry {
            tag: TAG_PHOTOSHOP,
            typ: 7,
            count: block.len() as u32,
            inline: None,
            data: Some(block),
        });
    }
    if let Some(offset) = exif_ifd_offset {
        entries.push(OutEntry {
            tag: TAG_EXIF_POINTER,
            typ: 4,
            count: 1,
            inline: Some(destination_order.u32_bytes(offset)),
            data: None,
        });
    }
    entries.sort_by_key(|entry| entry.tag);
    let ifd0_offset = write_ifd(&mut out, entries, destination_order)?;
    destination_order.put_u32(&mut out[4..8], ifd0_offset);
    Ok(out)
}

/// IFD0's Photoshop resource block; `None` for a file that is not a TIFF or
/// has none.
pub(super) fn photoshop_block(bytes: &[u8]) -> Option<Vec<u8>> {
    let (order, offset) = parse_header(bytes).ok()?;
    let entry = read_ifd(bytes, order, offset)
        .ok()?
        .into_iter()
        .find(|entry| entry.tag == TAG_PHOTOSHOP && matches!(entry.typ, 1 | 7))?;
    entry_data(bytes, order, &entry)
        .ok()
        .map(|data| data.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_tiff() -> Vec<u8> {
        let mut out = vec![b'I', b'I', 42, 0, 8, 0, 0, 0];
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&256u16.to_le_bytes()); // ImageWidth
        out.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    fn exif_blob() -> Vec<u8> {
        let mut out = vec![b'I', b'I', 42, 0, 8, 0, 0, 0];
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&271u16.to_le_bytes()); // Make
        out.extend_from_slice(&2u16.to_le_bytes()); // ASCII
        out.extend_from_slice(&6u32.to_le_bytes());
        out.extend_from_slice(&26u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"Canon\0");
        out
    }

    fn find(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn tiff_metadata_injection() {
        let metadata = RasterMetadata {
            exif: Some(exif_blob()),
            iptc: Some(b"iptc".to_vec()),
            xmp: Some(b"<x:xmpmeta/>".to_vec()),
            vanishing_point: None,
        };
        let out = embed(&minimal_tiff(), &metadata).expect("embed");
        let (order, offset) = parse_header(&out).expect("header");
        let entries = read_ifd(&out, order, offset).expect("IFD0");
        for tag in [256u16, 271, 700, 33723] {
            assert!(entries.iter().any(|entry| entry.tag == tag), "tag {tag}");
        }
        assert!(find(&out, b"Canon\0"), "Make survives");
        assert!(find(&out, b"<x:xmpmeta/>"), "XMP survives");
        assert!(find(&out, b"iptc"), "IPTC survives");
    }

    #[test]
    fn the_photoshop_block_round_trips() {
        assert_eq!(photoshop_block(&minimal_tiff()), None);
        let metadata = RasterMetadata {
            vanishing_point: Some(b"planes".to_vec()),
            ..RasterMetadata::default()
        };
        let once = embed(&minimal_tiff(), &metadata).expect("embed");
        assert_eq!(photoshop_block(&once), metadata.photoshop_block());
        let twice = embed(&once, &metadata).expect("re-embed");
        let (order, offset) = parse_header(&twice).expect("header");
        let entries = read_ifd(&twice, order, offset).expect("IFD0");
        assert_eq!(
            entries.iter().filter(|e| e.tag == TAG_PHOTOSHOP).count(),
            1,
            "a re-save replaces the block"
        );
        assert_eq!(photoshop_block(&twice), metadata.photoshop_block());
    }
}
