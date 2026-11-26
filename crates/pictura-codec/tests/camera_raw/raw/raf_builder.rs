//! Lays out synthetic Fujifilm RAFs: the big-endian header, a camera JPEG
//! holding only an Exif segment, the directory records, and the sensor
//! section (a TIFF whose one entry points at the raw directory), with
//! encoders for the uncompressed codings.

/// How the samples are stored.
#[derive(Debug, Clone)]
pub enum Coding {
    /// 16-bit words in the section's byte order.
    Words,
    /// 12-bit samples, least significant bit first.
    Lsb12,
    /// 14-bit samples, most significant bit first over little-endian 32-bit
    /// words.
    Msb14,
    /// The data bytes as given, with this row length in 32-bit words.
    Bytes(Vec<u8>, Option<u32>),
}

/// The camera JPEG's Exif tags.
#[derive(Debug, Clone, Copy)]
pub struct Exif {
    pub make: &'static str,
    pub model: &'static str,
    pub orientation: u16,
}

#[derive(Debug, Clone)]
pub struct Raf {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<u16>,
    pub coding: Coding,
    pub bits: u32,
    /// The sensor section's byte order.
    pub big_endian: bool,
    /// 32-bit words of padding after each packed row.
    pub row_pad: usize,
    pub layout: [u8; 4],
    /// `0x0131` as written: the CFA reversed.
    pub xtrans_record: Option<Vec<u8>>,
    pub black: Vec<u32>,
    /// Top, left, height, width.
    pub crop: Option<[u16; 4]>,
    /// Green, red, blue.
    pub wb: Option<[u32; 3]>,
    pub exif: Option<Exif>,
    pub header_name: &'static str,
    pub second_frame: bool,
    /// `0x9650`: numerator and denominator of the raw exposure bias.
    pub exposure_bias: Option<(i16, i16)>,
    /// A raw directory in the sensor section, else a SuperCCD-style one.
    pub sensor_tiff: bool,
}

pub const BAYER: [u8; 4] = [0x0A, 0x0B, 0x09, 0x08];
pub const XTRANS: [u8; 4] = [0x0C; 4];
pub const EXR: [u8; 4] = [0x08, 0x09, 0x0B, 0x0A];

/// The X100S's CFA at the plane's origin.
pub const X100S_CFA: [u8; 36] = [
    0, 2, 1, 2, 0, 1, //
    1, 1, 0, 1, 1, 2, //
    1, 1, 2, 1, 1, 0, //
    2, 0, 1, 0, 2, 1, //
    1, 1, 2, 1, 1, 0, //
    1, 1, 0, 1, 1, 2, //
];

impl Raf {
    /// An X-Trans RAF of 14-bit words with a 6 × 6 black of 1024.
    pub fn new(width: usize, height: usize, samples: Vec<u16>, coding: Coding) -> Self {
        let bits = match coding {
            Coding::Lsb12 => 12,
            _ => 14,
        };
        Self {
            width,
            height,
            samples,
            coding,
            bits,
            big_endian: false,
            row_pad: 0,
            layout: XTRANS,
            xtrans_record: Some(X100S_CFA.iter().rev().copied().collect()),
            black: vec![1024; 36],
            crop: Some([2, 4, height as u16 - 4, width as u16 - 8]),
            wb: Some([302, 577, 454]),
            exif: Some(Exif {
                make: "FUJIFILM",
                model: "X100S",
                orientation: 1,
            }),
            header_name: "X100S",
            second_frame: false,
            exposure_bias: Some((-72, 100)),
            sensor_tiff: true,
        }
    }

    pub fn bayer(mut self) -> Self {
        self.layout = BAYER;
        self.xtrans_record = None;
        self.black = vec![256, 255, 254, 253];
        self
    }

    /// Bytes in one packed row.
    pub fn stride(&self) -> usize {
        (self.width * self.bits as usize).div_ceil(32) * 4 + self.row_pad * 4
    }
}

fn put16(out: &mut Vec<u8>, v: u16, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

fn put32(out: &mut Vec<u8>, v: u32, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

fn patch32(out: &mut [u8], at: usize, v: u32, be: bool) {
    out[at..at + 4].copy_from_slice(&if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    bytes: Vec<u8>,
}

fn longs(tag: u16, values: &[u32], be: bool) -> Entry {
    let mut bytes = Vec::new();
    for &v in values {
        put32(&mut bytes, v, be);
    }
    Entry {
        tag,
        typ: 4,
        count: values.len() as u32,
        bytes,
    }
}

fn ascii(tag: u16, s: &str) -> Entry {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    Entry {
        tag,
        typ: 2,
        count: bytes.len() as u32,
        bytes,
    }
}

/// Writes an IFD at the end of `out`, its long values after the table;
/// returns where each entry's value field lies.
fn write_ifd(out: &mut Vec<u8>, entries: &[Entry], be: bool) -> Vec<usize> {
    let at = out.len();
    let values_at = at + 2 + 12 * entries.len() + 4;
    let mut values = Vec::new();
    let mut fields = Vec::new();
    put16(out, entries.len() as u16, be);
    for e in entries {
        put16(out, e.tag, be);
        put16(out, e.typ, be);
        put32(out, e.count, be);
        fields.push(out.len());
        if e.bytes.len() <= 4 {
            let mut inline = e.bytes.clone();
            inline.resize(4, 0);
            out.extend(inline);
        } else {
            put32(out, (values_at + values.len()) as u32, be);
            values.extend(&e.bytes);
            if values.len() % 2 == 1 {
                values.push(0);
            }
        }
    }
    put32(out, 0, be);
    out.extend(values);
    fields
}

fn lsb12_row(samples: &[u16], stride: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(stride);
    let (mut acc, mut have) = (0u32, 0);
    for &v in samples {
        acc |= u32::from(v & 0xFFF) << have;
        have += 12;
        while have >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            have -= 8;
        }
    }
    if have > 0 {
        out.push(acc as u8);
    }
    out.resize(stride, 0);
    out
}

fn msb14_row(samples: &[u16], stride: usize) -> Vec<u8> {
    let mut stream = Vec::with_capacity(stride);
    let (mut acc, mut have) = (0u32, 0);
    for &v in samples {
        acc = acc << 14 | u32::from(v & 0x3FFF);
        have += 14;
        while have >= 8 {
            have -= 8;
            stream.push((acc >> have) as u8);
        }
    }
    if have > 0 {
        stream.push((acc << (8 - have)) as u8);
    }
    stream.resize(stride, 0);
    for word in stream.chunks_mut(4) {
        word.reverse();
    }
    stream
}

/// The sensor data and its row length in 32-bit words.
fn data(r: &Raf) -> (Vec<u8>, Option<u32>) {
    let rows = r.samples.chunks(r.width);
    match &r.coding {
        Coding::Words => {
            let mut out = Vec::new();
            for &v in &r.samples {
                put16(&mut out, v, r.big_endian);
            }
            (out, None)
        }
        Coding::Lsb12 => (
            rows.flat_map(|row| lsb12_row(row, r.stride())).collect(),
            Some((r.stride() / 4) as u32),
        ),
        Coding::Msb14 => (
            rows.flat_map(|row| msb14_row(row, r.stride())).collect(),
            Some((r.stride() / 4) as u32),
        ),
        Coding::Bytes(bytes, words) => (bytes.clone(), *words),
    }
}

fn sensor_section(r: &Raf) -> Vec<u8> {
    let be = r.big_endian;
    let mut out = if be {
        b"MM\0*".to_vec()
    } else {
        b"II*\0".to_vec()
    };
    if !r.sensor_tiff {
        out = vec![0, 0x20, 0, 0x20];
    }
    put32(&mut out, 8, be);
    let top = write_ifd(&mut out, &[longs(0xF000, &[0], be)], be);
    let raw_at = out.len() as u32;
    patch32(&mut out, top[0], raw_at, be);
    let (bytes, row_words) = data(r);
    let mut entries = vec![
        longs(0xF001, &[r.width as u32], be),
        longs(0xF002, &[r.height as u32], be),
        longs(0xF003, &[r.bits], be),
    ];
    if let Some(words) = row_words {
        entries.push(longs(0xF005, &[words], be));
    }
    entries.push(longs(0xF007, &[0], be));
    entries.push(longs(0xF008, &[bytes.len() as u32], be));
    if !r.black.is_empty() {
        entries.push(longs(0xF00A, &r.black, be));
    }
    if let Some(wb) = r.wb {
        entries.push(longs(0xF00E, &wb, be));
    }
    let offset_entry = entries.iter().position(|e| e.tag == 0xF007).unwrap();
    let fields = write_ifd(&mut out, &entries, be);
    out.resize(out.len().next_multiple_of(16), 0);
    let data_at = out.len() as u32;
    patch32(&mut out, fields[offset_entry], data_at, be);
    out.extend(bytes);
    out
}

fn jpeg(exif: Exif) -> Vec<u8> {
    let mut tiff = b"II*\0".to_vec();
    put32(&mut tiff, 8, false);
    let mut orientation = Vec::new();
    put16(&mut orientation, exif.orientation, false);
    write_ifd(
        &mut tiff,
        &[
            ascii(271, exif.make),
            ascii(272, exif.model),
            Entry {
                tag: 274,
                typ: 3,
                count: 1,
                bytes: orientation,
            },
        ],
        false,
    );
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend(((tiff.len() + 8) as u16).to_be_bytes());
    out.extend(b"Exif\0\0");
    out.extend(tiff);
    out.extend([0xFF, 0xD9]);
    out
}

fn records(r: &Raf) -> Vec<u8> {
    let pair = |a: u16, b: u16| [a.to_be_bytes(), b.to_be_bytes()].concat();
    let mut list: Vec<(u16, Vec<u8>)> = vec![(0x0100, pair(r.height as u16, r.width as u16))];
    if let Some([top, left, h, w]) = r.crop {
        list.push((0x0110, pair(top, left)));
        list.push((0x0111, pair(h, w)));
    }
    list.push((0x0130, r.layout.to_vec()));
    if let Some(x) = &r.xtrans_record {
        list.push((0x0131, x.clone()));
    }
    if let Some((num, den)) = r.exposure_bias {
        list.push((0x9650, [num.to_be_bytes(), den.to_be_bytes()].concat()));
    }
    let mut out = (list.len() as u32).to_be_bytes().to_vec();
    for (tag, value) in list {
        out.extend(tag.to_be_bytes());
        out.extend((value.len() as u16).to_be_bytes());
        out.extend(value);
    }
    out
}

/// The whole file.
pub fn raf(r: &Raf) -> Vec<u8> {
    let mut out = b"FUJIFILMCCD-RAW 0201FF000000".to_vec();
    let mut name = r.header_name.as_bytes().to_vec();
    name.resize(32, 0);
    out.extend(name);
    out.extend(b"0100");
    out.resize(0x94, 0);
    let mut sections = Vec::new();
    for body in [
        r.exif.map(jpeg).unwrap_or_default(),
        records(r),
        sensor_section(r),
    ] {
        sections.push((out.len() as u32, body.len() as u32));
        out.extend(body);
    }
    for (i, (at, len)) in sections.into_iter().enumerate() {
        patch32(&mut out, 0x54 + 8 * i, at, true);
        patch32(&mut out, 0x58 + 8 * i, len, true);
    }
    if r.second_frame {
        patch32(&mut out, 0x78, 0x100, true);
    }
    out
}
