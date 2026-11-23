//! Lays out synthetic RW2s: the header, IFD0 with Panasonic's tags, its
//! values, then the sensor data in one of the plain codings.

/// How the samples are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Data {
    /// Shifted left by this much, under this compression.
    Words { shift: u32, compression: u32 },
    /// RawFormat 2: ten 12-bit samples to 16 bytes.
    Ten,
    /// RawFormat 7: nine 14-bit samples to 16 bytes.
    Nine,
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
}

impl Rw2 {
    pub fn new(width: usize, height: usize, samples: Vec<u16>, data: Data) -> Self {
        let (bits, compression, raw_format, white) = match data {
            Data::Words { shift, compression } => (16 - shift as u16, compression, None, 4095),
            Data::Ten => (12, 34316, Some(2), 4095),
            Data::Nine => (14, 34316, Some(7), 16383),
        };
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
            raw_data_offset: false,
            extra: 0,
        }
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
