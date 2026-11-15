//! EXIF (TIFF/IFD) decoder.
//!
//! `parse_exif` turns a PSD EXIF image resource (1058, `Exif\0\0`-prefixed, or
//! 1059, a bare TIFF stream) into ordered `(tag, value)` entries so the File
//! Info dialog can show camera data without a dependency. The raw resource still
//! round-trips byte-for-byte; this is a read-only view.
//!
//! ponytail: IFD0 plus the Exif sub-IFD only, no GPS/Interoperability sub-IFD,
//! and a fixed tag-name table. A SHORT/LONG/RATIONAL tag with `count > 1` is
//! exposed as raw `Undefined` bytes rather than a component list. Add a sub-IFD,
//! tag, or array decoding when a real file needs it.

/// The maximum IFD entries walked, so a hostile count cannot spin the loop.
const MAX_ENTRIES: usize = 4096;

/// A decoded EXIF/TIFF value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExifValue {
    Ascii(String),
    Short(u16),
    Long(u32),
    Rational(u32, u32),
    Undefined(Vec<u8>),
}

impl ExifValue {
    /// The value as display text. Rationals render `num/den`; unknown bytes are
    /// length-described.
    pub fn display(&self) -> String {
        match self {
            ExifValue::Ascii(s) => s.clone(),
            ExifValue::Short(v) => v.to_string(),
            ExifValue::Long(v) => v.to_string(),
            ExifValue::Rational(n, d) if *d != 0 => format!("{n}/{d}"),
            ExifValue::Rational(n, d) => format!("{n}/{d}"),
            ExifValue::Undefined(b) => format!("<{} bytes>", b.len()),
        }
    }

    /// The value as `f64` when it has a numeric interpretation, for oracle
    /// comparison against an independent decoder.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ExifValue::Short(v) => Some(f64::from(*v)),
            ExifValue::Long(v) => Some(f64::from(*v)),
            ExifValue::Rational(n, d) if *d != 0 => Some(f64::from(*n) / f64::from(*d)),
            _ => None,
        }
    }
}

/// Typed GPS values decoded from the GPS sub-IFD.
#[derive(Debug, Clone, PartialEq)]
pub struct Gps {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub timestamp: Option<String>,
    pub date: Option<String>,
    pub datum: Option<String>,
}

/// Ordered decoded EXIF tags plus the optional GPS block.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Exif {
    entries: Vec<(u16, ExifValue)>,
    gps: Option<Gps>,
}

impl Exif {
    pub fn get(&self, tag: u16) -> Option<&ExifValue> {
        self.entries.iter().find(|(t, _)| *t == tag).map(|(_, v)| v)
    }

    pub fn entries(&self) -> &[(u16, ExifValue)] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.gps.is_none()
    }

    pub fn gps(&self) -> Option<&Gps> {
        self.gps.as_ref()
    }
}

/// The Human-readable name of a common EXIF/TIFF tag, or `None` when unknown.
pub fn exif_tag_name(tag: u16) -> Option<&'static str> {
    Some(match tag {
        0x010e => "Image Description",
        0x010f => "Make",
        0x0110 => "Model",
        0x0112 => "Orientation",
        0x011a => "X Resolution",
        0x011b => "Y Resolution",
        0x0128 => "Resolution Unit",
        0x0131 => "Software",
        0x0132 => "Modify Date",
        0x013b => "Artist",
        0x8298 => "Copyright",
        0x8769 => "Exif IFD Pointer",
        0x829a => "Exposure Time",
        0x829d => "F Number",
        0x8822 => "Exposure Program",
        0x8827 => "ISO Speed Ratings",
        0x9000 => "Exif Version",
        0x9003 => "Date Time Original",
        0x9004 => "Date Time Digitized",
        0x9201 => "Shutter Speed Value",
        0x9202 => "Aperture Value",
        0x9204 => "Exposure Bias Value",
        0x9209 => "Flash",
        0x920a => "Focal Length",
        0xa001 => "Color Space",
        0xa002 => "Pixel X Dimension",
        0xa003 => "Pixel Y Dimension",
        0xa405 => "Focal Length In 35mm Film",
        _ => return None,
    })
}

/// Decode a PSD EXIF image resource into ordered typed tags. A malformed or
/// truncated blob yields the entries decoded so far and never panics.
pub fn parse_exif(data: &[u8]) -> Exif {
    let tiff = data.strip_prefix(b"Exif\0\0").unwrap_or(data);
    let mut exif = Exif::default();
    let little = match tiff.get(0..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return exif,
    };
    if read_u16(tiff, 2, little) != Some(42) {
        return exif;
    }
    let Some(ifd0) = read_u32(tiff, 4, little).map(|o| o as usize) else {
        return exif;
    };
    let mut exif_ifd = None;
    let mut gps_ifd = None;
    // A hostile resource can point many entries at one large offset; budget the
    // total value bytes cloned so decoding stays O(resource size), not
    // O(entries * resource size).
    let mut budget = tiff.len();
    walk_ifd(
        tiff,
        ifd0,
        little,
        &mut exif,
        &mut exif_ifd,
        &mut gps_ifd,
        &mut budget,
    );
    if let Some(offset) = exif_ifd {
        walk_ifd(
            tiff,
            offset,
            little,
            &mut exif,
            &mut None,
            &mut None,
            &mut budget,
        );
    }
    if let Some(offset) = gps_ifd {
        exif.gps = parse_gps_ifd(tiff, offset, little, &mut budget);
    }
    exif
}

fn walk_ifd(
    data: &[u8],
    offset: usize,
    little: bool,
    out: &mut Exif,
    exif_ifd: &mut Option<usize>,
    gps_ifd: &mut Option<usize>,
    budget: &mut usize,
) {
    let Some(count) = read_u16(data, offset, little).map(usize::from) else {
        return;
    };
    for i in 0..count.min(MAX_ENTRIES) {
        let entry = offset + 2 + i * 12;
        let (Some(tag), Some(typ), Some(n)) = (
            read_u16(data, entry, little),
            read_u16(data, entry + 2, little),
            read_u32(data, entry + 4, little),
        ) else {
            return;
        };
        let count = n as usize;
        if tag == 0x8769 && typ == 4 {
            if let Some(ptr) = read_u32(data, entry + 8, little) {
                *exif_ifd = Some(ptr as usize);
            }
        }
        if tag == 0x8825 && typ == 4 {
            if let Some(ptr) = read_u32(data, entry + 8, little) {
                *gps_ifd = Some(ptr as usize);
            }
        }
        if let Some(value) = decode_value(data, entry, little, typ, count, budget) {
            out.entries.push((tag, value));
        }
    }
}

fn decode_value(
    data: &[u8],
    entry: usize,
    little: bool,
    typ: u16,
    count: usize,
    budget: &mut usize,
) -> Option<ExifValue> {
    let unit = match typ {
        2 | 7 => 1,
        3 => 2,
        4 => 4,
        5 => 8,
        _ => return None,
    };
    let total = count.checked_mul(unit)?;
    if total > *budget {
        return None;
    }
    let bytes = if total <= 4 {
        data.get(entry + 8..entry + 8 + total)?
    } else {
        let start = read_u32(data, entry + 8, little)? as usize;
        data.get(start..start.checked_add(total)?)?
    };
    *budget -= total;
    Some(match typ {
        2 => ExifValue::Ascii(
            String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_string(),
        ),
        3 if count == 1 => ExifValue::Short(read_u16(bytes, 0, little)?),
        4 if count == 1 => ExifValue::Long(read_u32(bytes, 0, little)?),
        5 if count == 1 => {
            ExifValue::Rational(read_u32(bytes, 0, little)?, read_u32(bytes, 4, little)?)
        }
        _ => ExifValue::Undefined(bytes.to_vec()),
    })
}

fn parse_gps_ifd(data: &[u8], offset: usize, little: bool, budget: &mut usize) -> Option<Gps> {
    let count = read_u16(data, offset, little).map(usize::from)?;
    let mut lat_ref = None;
    let mut lat = None;
    let mut lon_ref = None;
    let mut lon = None;
    let mut altitude_ref = 0u8;
    let mut altitude = None;
    let mut timestamp = None;
    let mut date = None;
    let mut datum = None;
    for i in 0..count.min(MAX_ENTRIES) {
        let entry = offset + 2 + i * 12;
        let (Some(tag), Some(typ), Some(n)) = (
            read_u16(data, entry, little),
            read_u16(data, entry + 2, little),
            read_u32(data, entry + 4, little),
        ) else {
            break;
        };
        let n = n as usize;
        let Some(bytes) = entry_bytes(data, entry, little, typ, n, budget) else {
            continue;
        };
        match tag {
            0x0001 => lat_ref = Some(ascii(bytes)),
            0x0002 => lat = rationals(bytes, little, 3).map(dms),
            0x0003 => lon_ref = Some(ascii(bytes)),
            0x0004 => lon = rationals(bytes, little, 3).map(dms),
            0x0005 => altitude_ref = bytes.first().copied().unwrap_or(0),
            0x0006 => altitude = rationals(bytes, little, 1).map(|v| v[0]),
            0x0007 => {
                timestamp = rationals(bytes, little, 3).map(|v| {
                    format!(
                        "{:02}:{:02}:{:02}",
                        v[0] as u32,
                        v[1] as u32,
                        v[2].round() as u32
                    )
                })
            }
            0x0012 => datum = Some(ascii(bytes)),
            0x001d => date = Some(ascii(bytes)),
            _ => {}
        }
    }
    let mut gps = Gps {
        latitude: lat,
        longitude: lon,
        altitude,
        timestamp,
        date,
        datum,
    };
    if lat_ref.as_deref() == Some("S") {
        gps.latitude = gps.latitude.map(|v| -v);
    }
    if lon_ref.as_deref() == Some("W") {
        gps.longitude = gps.longitude.map(|v| -v);
    }
    if altitude_ref == 1 {
        gps.altitude = gps.altitude.map(|v| -v);
    }
    if gps.latitude.is_none()
        && gps.longitude.is_none()
        && gps.altitude.is_none()
        && gps.timestamp.is_none()
        && gps.date.is_none()
        && gps.datum.is_none()
    {
        return None;
    }
    Some(gps)
}

fn entry_bytes<'a>(
    data: &'a [u8],
    entry: usize,
    little: bool,
    typ: u16,
    count: usize,
    budget: &mut usize,
) -> Option<&'a [u8]> {
    let unit = match typ {
        1 | 2 | 7 => 1,
        3 => 2,
        4 => 4,
        5 => 8,
        _ => return None,
    };
    let total = count.checked_mul(unit)?;
    if total > *budget {
        return None;
    }
    let bytes = if total <= 4 {
        data.get(entry + 8..entry + 8 + total)?
    } else {
        let start = read_u32(data, entry + 8, little)? as usize;
        data.get(start..start.checked_add(total)?)?
    };
    *budget -= total;
    Some(bytes)
}

fn rationals(bytes: &[u8], little: bool, count: usize) -> Option<Vec<f64>> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = i * 8;
        let num = read_u32(bytes, at, little)?;
        let den = read_u32(bytes, at + 4, little)?;
        out.push(if den == 0 {
            0.0
        } else {
            f64::from(num) / f64::from(den)
        });
    }
    Some(out)
}

fn dms(parts: Vec<f64>) -> f64 {
    parts.first().copied().unwrap_or(0.0)
        + parts.get(1).copied().unwrap_or(0.0) / 60.0
        + parts.get(2).copied().unwrap_or(0.0) / 3600.0
}

fn ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string()
}

fn read_u16(data: &[u8], offset: usize, little: bool) -> Option<u16> {
    let b = data.get(offset..offset + 2)?;
    let b = [b[0], b[1]];
    Some(if little {
        u16::from_le_bytes(b)
    } else {
        u16::from_be_bytes(b)
    })
}

fn read_u32(data: &[u8], offset: usize, little: bool) -> Option<u32> {
    let b = data.get(offset..offset + 4)?;
    let b = [b[0], b[1], b[2], b[3]];
    Some(if little {
        u32::from_le_bytes(b)
    } else {
        u32::from_be_bytes(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le16(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }
    fn le32(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }

    /// Little-endian TIFF: IFD0 (Make ASCII + Orientation SHORT + Exif IFD
    /// pointer) and an Exif sub-IFD (FNumber RATIONAL + ISO SHORT).
    fn sample_tiff() -> Vec<u8> {
        let make = b"ACME\x00";
        let ifd0_size = 2 + 3 * 12 + 4;
        let exif_off = 8 + ifd0_size;
        let exif_size = 2 + 2 * 12 + 4;
        let make_off = exif_off + exif_size;
        let fnum_off = make_off + make.len();

        let mut ifd0 = le16(3).to_vec();
        ifd0.extend_from_slice(&le16(0x010f)); // Make
        ifd0.extend_from_slice(&le16(2));
        ifd0.extend_from_slice(&le32(make.len() as u32));
        ifd0.extend_from_slice(&le32(make_off as u32));
        ifd0.extend_from_slice(&le16(0x0112)); // Orientation
        ifd0.extend_from_slice(&le16(3));
        ifd0.extend_from_slice(&le32(1));
        ifd0.extend_from_slice(&le16(6));
        ifd0.extend_from_slice(&[0, 0]);
        ifd0.extend_from_slice(&le16(0x8769)); // Exif sub-IFD pointer
        ifd0.extend_from_slice(&le16(4));
        ifd0.extend_from_slice(&le32(1));
        ifd0.extend_from_slice(&le32(exif_off as u32));
        ifd0.extend_from_slice(&le32(0)); // next IFD

        let mut exif = le16(2).to_vec();
        exif.extend_from_slice(&le16(0x829d)); // FNumber
        exif.extend_from_slice(&le16(5));
        exif.extend_from_slice(&le32(1));
        exif.extend_from_slice(&le32(fnum_off as u32));
        exif.extend_from_slice(&le16(0x8827)); // ISO
        exif.extend_from_slice(&le16(3));
        exif.extend_from_slice(&le32(1));
        exif.extend_from_slice(&le16(200));
        exif.extend_from_slice(&[0, 0]);
        exif.extend_from_slice(&le32(0)); // next IFD

        let mut out = Vec::new();
        out.extend_from_slice(b"II");
        out.extend_from_slice(&le16(42));
        out.extend_from_slice(&le32(8)); // IFD0 at 8
        out.extend_from_slice(&ifd0);
        out.extend_from_slice(&exif);
        out.extend_from_slice(make);
        out.extend_from_slice(&le32(28));
        out.extend_from_slice(&le32(10));
        out
    }

    #[test]
    fn decodes_ifd0_and_exif_sub_ifd() {
        let exif = parse_exif(&sample_tiff());
        assert_eq!(exif.get(0x010f), Some(&ExifValue::Ascii("ACME".into())));
        assert_eq!(exif.get(0x0112), Some(&ExifValue::Short(6)));
        assert_eq!(exif.get(0x829d), Some(&ExifValue::Rational(28, 10)));
        assert_eq!(exif.get(0x8827), Some(&ExifValue::Short(200)));
    }

    #[test]
    fn exif_gps_decodes_the_fixture_values() {
        let bytes = include_bytes!("../tests/fixtures/metadata.psd");
        let doc = crate::read_psd(bytes).expect("fixture parses");
        let metadata = crate::metadata::read_metadata(&doc);
        let gps = metadata.exif.gps().expect("fixture carries GPS");
        assert!((gps.latitude.unwrap() - 48.858_333).abs() < 1e-6);
        assert!((gps.longitude.unwrap() + 2.291_667).abs() < 1e-6);
        assert!((gps.altitude.unwrap() - 35.0).abs() < 1e-9);
        assert_eq!(gps.timestamp.as_deref(), Some("12:00:00"));
        assert_eq!(gps.date.as_deref(), Some("2026:09:22"));
        assert_eq!(gps.datum.as_deref(), Some("WGS-84"));
    }

    #[test]
    fn exif_gps_survives_a_truncated_ifd() {
        let bytes = include_bytes!("../tests/fixtures/metadata.psd");
        let doc = crate::read_psd(bytes).expect("fixture parses");
        let resource = crate::decode_image_resources(&doc)
            .into_iter()
            .find(|r| r.id == crate::EXIF_DATA_1)
            .expect("fixture carries EXIF");
        for cut in 0..resource.data.len() {
            let _ = parse_exif(&resource.data[..cut]);
        }
    }

    #[test]
    fn exif_gps_absent_without_the_pointer() {
        assert!(parse_exif(&sample_tiff()).gps().is_none());
    }

    #[test]
    fn accepts_the_exif_prefix() {
        let mut prefixed = b"Exif\0\0".to_vec();
        prefixed.extend_from_slice(&sample_tiff());
        assert_eq!(parse_exif(&prefixed), parse_exif(&sample_tiff()));
    }

    #[test]
    fn decodes_big_endian() {
        let be = |v: u16| v.to_be_bytes();
        let mut data = Vec::new();
        data.extend_from_slice(b"MM");
        data.extend_from_slice(&be(42));
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(&be(1));
        data.extend_from_slice(&be(0x0112));
        data.extend_from_slice(&be(3));
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(&be(8));
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&0u32.to_be_bytes());
        let exif = parse_exif(&data);
        assert_eq!(exif.get(0x0112), Some(&ExifValue::Short(8)));
    }

    #[test]
    fn truncation_is_panic_free() {
        let full = sample_tiff();
        for cut in 0..full.len() {
            let _ = parse_exif(&full[..cut]);
        }
        assert!(parse_exif(&[]).is_empty());
        assert!(parse_exif(b"not a tiff").is_empty());
    }

    /// A LE TIFF whose IFD0 holds one SHORT and one RATIONAL, each `count = 2`.
    fn array_tiff() -> Vec<u8> {
        let mut ifd0 = le16(2).to_vec();
        ifd0.extend_from_slice(&le16(0x8827)); // ISO, SHORT x2 inline
        ifd0.extend_from_slice(&le16(3));
        ifd0.extend_from_slice(&le32(2));
        ifd0.extend_from_slice(&le16(100));
        ifd0.extend_from_slice(&le16(200));
        let data_off = 8 + 2 + 2 * 12 + 4;
        ifd0.extend_from_slice(&le16(0x013f)); // PrimaryChromaticities, RATIONAL x2
        ifd0.extend_from_slice(&le16(5));
        ifd0.extend_from_slice(&le32(2));
        ifd0.extend_from_slice(&le32(data_off as u32));
        ifd0.extend_from_slice(&le32(0));
        let mut out = Vec::new();
        out.extend_from_slice(b"II");
        out.extend_from_slice(&le16(42));
        out.extend_from_slice(&le32(8));
        out.extend_from_slice(&ifd0);
        out.extend_from_slice(&le32(1)); // first rational 1/2
        out.extend_from_slice(&le32(2));
        out.extend_from_slice(&le32(3)); // second rational 3/4
        out.extend_from_slice(&le32(4));
        out
    }

    #[test]
    fn array_values_are_exposed_as_raw_bytes() {
        let exif = parse_exif(&array_tiff());
        assert_eq!(
            exif.get(0x8827),
            Some(&ExifValue::Undefined(vec![100, 0, 200, 0])),
            "a SHORT array keeps its raw bytes"
        );
        assert_eq!(
            exif.get(0x013f),
            Some(&ExifValue::Undefined(vec![
                1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0
            ])),
            "a RATIONAL array keeps its raw bytes"
        );
    }

    #[test]
    fn a_shared_offset_cannot_amplify_beyond_the_input() {
        // 4096 IFD entries all pointing at the same big value must not clone
        // more than the input size in total.
        let n = 4096usize;
        let ifd0_size = 2 + n * 12 + 4;
        let data_off = 8 + ifd0_size;
        let blob = vec![0xabu8; 4096];
        let mut ifd0 = le16(n as u16).to_vec();
        for i in 0..n {
            ifd0.extend_from_slice(&le16(i as u16));
            ifd0.extend_from_slice(&le16(7)); // UNDEFINED
            ifd0.extend_from_slice(&le32(blob.len() as u32));
            ifd0.extend_from_slice(&le32(data_off as u32));
        }
        ifd0.extend_from_slice(&le32(0));
        let mut out = Vec::new();
        out.extend_from_slice(b"II");
        out.extend_from_slice(&le16(42));
        out.extend_from_slice(&le32(8));
        out.extend_from_slice(&ifd0);
        out.extend_from_slice(&blob);
        let exif = parse_exif(&out);
        let cloned: usize = exif
            .entries()
            .iter()
            .map(|(_, v)| match v {
                ExifValue::Undefined(b) => b.len(),
                _ => 0,
            })
            .sum();
        assert!(cloned <= out.len(), "cloned {cloned} > input {}", out.len());
    }
}
