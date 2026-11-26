//! Lays out synthetic RW2s: the header, IFD0 with Panasonic's tags, its
//! values, then the sensor data in one of the codings.

use super::rw2_encode;

/// How the samples are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Data {
    /// Shifted left by this much, under this compression.
    Words { shift: u32, compression: u32 },
    /// RawFormat 2: ten 12-bit samples to 16 bytes.
    Ten,
    /// RawFormat 7: nine 14-bit samples to 16 bytes.
    Nine,
    /// RawFormat 4, or 3 without `RawDataOffset` (no swapped blocks).
    Predictive,
    /// RawFormat 5 at this many bits.
    Blocked(u32),
    /// RawFormat 6 at this many bits.
    Paired(u32),
    /// RawFormat 8 in this many columns and rows of strips.
    Strips { columns: usize, rows: usize },
}

#[derive(Debug, Clone)]
pub struct Rw2 {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<u16>,
    pub data: Data,
    pub make: &'static str,
    pub model: &'static str,
    pub orientation: u16,
    pub bits: u16,
    pub compression: Option<u32>,
    pub raw_format: Option<u16>,
    /// Top, left, bottom, right.
    pub borders: [u16; 4],
    pub cfa: u16,
    pub black: [u16; 3],
    pub white: u16,
    pub wb: [u16; 3],
    /// Top, left, bottom, right.
    pub crop: Option<[u16; 4]>,
    /// Place the data by `RawDataOffset`, with `StripOffsets` all ones.
    pub raw_data_offset: bool,
    /// Added to `StripByteCounts`, and as many zero bytes to the data.
    pub extra: usize,
    /// RawFormat 8's first predictions: red, first green, second green, blue.
    pub initial: [u16; 4],
}

impl Rw2 {
    pub fn new(width: usize, height: usize, samples: Vec<u16>, data: Data) -> Self {
        let (bits, compression, raw_format, white) = match data {
            Data::Words { shift, compression } => (16 - shift as u16, compression, None, 4095),
            Data::Ten => (12, 34316, Some(2), 4095),
            Data::Nine => (14, 34316, Some(7), 16383),
            Data::Predictive => (12, 34316, Some(4), 4095),
            Data::Blocked(bits) | Data::Paired(bits) => {
                let format = if matches!(data, Data::Blocked(_)) {
                    5
                } else {
                    6
                };
                (bits as u16, 34316, Some(format), (1 << bits) - 1)
            }
            Data::Strips { .. } => (16, 34316, Some(8), u16::MAX),
        };
        let raw_data_offset = !matches!(data, Data::Words { .. } | Data::Ten | Data::Nine);
        Self {
            width,
            height,
            samples,
            data,
            make: "Panasonic",
            model: "DMC-FZ50",
            orientation: 1,
            bits,
            compression: Some(compression),
            raw_format,
            borders: [0, 0, height as u16, width as u16],
            cfa: 1,
            black: [0; 3],
            white,
            wb: [512, 256, 384],
            crop: None,
            raw_data_offset,
            extra: 0,
            initial: [0; 4],
        }
    }

    /// The strips' positions as `(x, y, width, height)`, row by row.
    pub fn strip_rects(&self) -> Vec<(usize, usize, usize, usize)> {
        let Data::Strips { columns, rows } = self.data else {
            return Vec::new();
        };
        let (w, h) = (self.width / columns, self.height / rows);
        let mut rects = Vec::new();
        for row in 0..rows {
            for col in 0..columns {
                let width = if col + 1 == columns {
                    self.width - col * w
                } else {
                    w
                };
                let height = if row + 1 == rows {
                    self.height - row * h
                } else {
                    h
                };
                rects.push((col * w, row * h, width, height));
            }
        }
        rects
    }

    /// Each strip's bytes and length in bits.
    fn strips(&self) -> Vec<(Vec<u8>, usize)> {
        // rawspeed starts both greens from the second green's tag.
        let [r, _, g2, b] = self.initial;
        self.strip_rects()
            .into_iter()
            .map(|rect| rw2_encode::strip(&self.samples, self.width, rect, [r, g2, g2, b]))
            .collect()
    }

    fn stored(&self) -> Vec<u8> {
        let (bits, per_group) = match self.data {
            Data::Words { shift, .. } => {
                return self
                    .samples
                    .iter()
                    .flat_map(|&v| (v << shift).to_le_bytes())
                    .collect()
            }
            Data::Ten => (12, 10),
            Data::Nine => (14, 9),
            Data::Predictive => return rw2_encode::predictive(&self.samples, self.raw_data_offset),
            Data::Blocked(bits) => return rw2_encode::blocked(&self.samples, bits),
            Data::Paired(bits) => return rw2_encode::paired(&self.samples, bits),
            Data::Strips { .. } => return self.strips().into_iter().flat_map(|s| s.0).collect(),
        };
        self.samples
            .chunks(per_group)
            .flat_map(|group| {
                // Every bit past the samples is set: the reader ignores them.
                let mut value = u128::MAX << (bits * per_group);
                for (k, &v) in group.iter().enumerate() {
                    value |= u128::from(v) << (bits * k);
                }
                value.to_le_bytes()
            })
            .collect()
    }
}

#[derive(Clone)]
enum Value {
    Short(u16),
    Long(u32),
    Ascii(&'static str),
    Bytes(Vec<u8>),
}

/// A Panasonic list tag: a 16-bit count, then the values.
fn counted<const N: usize>(values: impl ExactSizeIterator<Item = u64>) -> Value {
    let mut bytes = (values.len() as u16).to_le_bytes().to_vec();
    for v in values {
        bytes.extend(&v.to_le_bytes()[..N]);
    }
    Value::Bytes(bytes)
}

fn strip_tags(r: &Rw2, data_at: u32) -> Vec<(u16, Value)> {
    use Value::Short;
    let rects = r.strip_rects();
    let strips = r.strips();
    let mut offsets = Vec::new();
    let mut at = u64::from(data_at);
    for (bytes, _) in &strips {
        offsets.push(at);
        at += bytes.len() as u64;
    }
    let Data::Strips { columns, rows } = r.data else {
        unreachable!()
    };
    let codes = rw2_encode::codes();
    let mut table = vec![17u16];
    for (len, code) in codes {
        table.extend([len, code]);
    }
    let table = Value::Bytes(table.iter().flat_map(|v| v.to_le_bytes()).collect());
    vec![
        (0x0039, counted::<4>([65536u64; 6].into_iter())),
        (0x003A, counted::<4>([0u64; 6].into_iter())),
        (0x003B, Short(u16::MAX)),
        (0x003C, Short(r.initial[0])),
        (0x003D, Short(r.initial[1])),
        (0x003E, Short(r.initial[2])),
        (0x003F, Short(r.initial[3])),
        (0x0040, table),
        (0x0041, counted::<2>([0u64; 17].into_iter())),
        (0x0042, Short(columns as u16)),
        (0x0043, Short(rows as u16)),
        (0x0044, counted::<4>(offsets.into_iter())),
        (
            0x0045,
            counted::<4>(rects.iter().map(|&(x, y, ..)| (y as u64) << 16 | x as u64)),
        ),
        (
            0x0046,
            counted::<4>(strips.iter().map(|(_, bits)| *bits as u64)),
        ),
        (0x0047, counted::<2>(rects.iter().map(|r| r.2 as u64))),
        (0x0048, counted::<2>(rects.iter().map(|r| r.3 as u64))),
    ]
}

fn directory(r: &Rw2, data_at: u32, data_len: u32) -> Vec<(u16, Value)> {
    use Value::{Ascii, Long, Short};
    let mut tags = vec![
        (0x0002, Short(r.width as u16)),
        (0x0003, Short(r.height as u16)),
        (0x0009, Short(r.cfa)),
        (0x000A, Short(r.bits)),
        (0x000F, Short(r.white)),
        (0x010F, Ascii(r.make)),
        (0x0110, Ascii(r.model)),
        (0x0112, Short(r.orientation)),
        (0x0117, Long(data_len)),
    ];
    let mut shorts = |first: u16, values: &[u16]| {
        for (i, &v) in values.iter().enumerate() {
            tags.push((first + i as u16, Short(v)));
        }
    };
    shorts(0x0004, &r.borders);
    shorts(0x001C, &r.black);
    shorts(0x0024, &r.wb);
    if let Some(format) = r.raw_format {
        shorts(0x002D, &[format]);
    }
    if let Some(crop) = r.crop {
        shorts(0x002F, &crop);
    }
    if let Some(compression) = r.compression {
        tags.push((0x000B, Long(compression)));
    }
    if matches!(r.data, Data::Strips { .. }) {
        tags.extend(strip_tags(r, data_at));
    }
    if r.raw_data_offset {
        tags.push((0x0111, Long(u32::MAX)));
        tags.push((0x0118, Long(data_at)));
    } else {
        tags.push((0x0111, Long(data_at)));
    }
    tags.sort_by_key(|&(tag, _)| tag);
    tags
}

/// Where the sensor data starts in `rw2(r)`.
pub fn data_start(r: &Rw2) -> usize {
    let tags = directory(r, 0, 0);
    let values: usize = tags
        .iter()
        .map(|(_, value)| match value {
            Value::Ascii(text) if text.len() >= 4 => text.len() + 1,
            Value::Bytes(bytes) if bytes.len() > 4 => bytes.len(),
            _ => 0,
        })
        .sum();
    8 + 2 + 12 * tags.len() + 4 + values
}

pub fn rw2(r: &Rw2) -> Vec<u8> {
    let mut data = r.stored();
    data.resize(data.len() + r.extra, 0);
    let start = data_start(r);
    let tags = directory(r, start as u32, data.len() as u32);

    let mut out = b"IIU\0\x08\0\0\0".to_vec();
    out.extend((tags.len() as u16).to_le_bytes());
    let mut values_at = 8 + 2 + 12 * tags.len() + 4;
    let mut values: Vec<u8> = Vec::new();
    for (tag, value) in &tags {
        out.extend(tag.to_le_bytes());
        let (typ, count, field) = match *value {
            Value::Short(v) => (3u16, 1u32, u32::from(v).to_le_bytes()),
            Value::Long(v) => (4, 1, v.to_le_bytes()),
            Value::Bytes(ref bytes) => {
                let mut field = [0; 4];
                if bytes.len() <= 4 {
                    field[..bytes.len()].copy_from_slice(bytes);
                } else {
                    field = (values_at as u32).to_le_bytes();
                    values_at += bytes.len();
                    values.extend(bytes);
                }
                (7, bytes.len() as u32, field)
            }
            Value::Ascii(text) => {
                let bytes = [text.as_bytes(), &[0]].concat();
                let mut field = [0; 4];
                if bytes.len() <= 4 {
                    field[..bytes.len()].copy_from_slice(&bytes);
                } else {
                    field = (values_at as u32).to_le_bytes();
                    values_at += bytes.len();
                    values.extend(&bytes);
                }
                (2, bytes.len() as u32, field)
            }
        };
        out.extend(typ.to_le_bytes());
        out.extend(count.to_le_bytes());
        out.extend(field);
    }
    out.extend([0; 4]);
    out.extend(values);
    assert_eq!(out.len(), start, "directory layout");
    out.extend(data);
    out
}
