//! Little-endian classic TIFF output for the DNG writer: data blocks are
//! appended first, then each IFD after its out-of-line values, so every
//! offset an IFD names is known when it is written.

use crate::error::PsdError;

pub(super) enum Value {
    Byte(Vec<u8>),
    Ascii(String),
    Short(Vec<u16>),
    Long(Vec<u32>),
    Rational(Vec<(u32, u32)>),
    SRational(Vec<(i32, i32)>),
    Undefined(Vec<u8>),
}

impl Value {
    fn encode(&self) -> (u16, usize, Vec<u8>) {
        let mut out = Vec::new();
        let (typ, count) = match self {
            Value::Byte(v) => {
                out.extend(v);
                (1, v.len())
            }
            Value::Ascii(s) => {
                out.extend(s.bytes());
                out.push(0);
                (2, s.len() + 1)
            }
            Value::Short(v) => {
                v.iter().for_each(|x| out.extend(x.to_le_bytes()));
                (3, v.len())
            }
            Value::Long(v) => {
                v.iter().for_each(|x| out.extend(x.to_le_bytes()));
                (4, v.len())
            }
            Value::Rational(v) => {
                v.iter().for_each(|(n, d)| {
                    out.extend(n.to_le_bytes());
                    out.extend(d.to_le_bytes());
                });
                (5, v.len())
            }
            Value::SRational(v) => {
                v.iter().for_each(|(n, d)| {
                    out.extend(n.to_le_bytes());
                    out.extend(d.to_le_bytes());
                });
                (10, v.len())
            }
            Value::Undefined(v) => {
                out.extend(v);
                (7, v.len())
            }
        };
        (typ, count, out)
    }
}

/// The largest power of ten that keeps `|v| × d` within `limit`, so values
/// with few decimals are stored exactly.
fn denominator(v: f64, limit: f64) -> f64 {
    let mut d = 1_000_000_000.0;
    while d > 1.0 && (v.abs() * d).round() > limit {
        d /= 10.0;
    }
    d
}

pub(super) fn srational(v: f64) -> (i32, i32) {
    let v = if v.is_finite() { v } else { 0.0 };
    let d = denominator(v, i32::MAX as f64);
    (
        (v * d).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        d as i32,
    )
}

pub(super) fn rational(v: f64) -> (u32, u32) {
    let v = if v.is_finite() { v.max(0.0) } else { 0.0 };
    let d = denominator(v, i32::MAX as f64);
    ((v * d).round().min(u32::MAX as f64) as u32, d as u32)
}

fn offset(out: &[u8]) -> Result<u32, PsdError> {
    u32::try_from(out.len()).map_err(|_| PsdError::Invalid("DNG: the file would pass 4 GiB".into()))
}

fn align(out: &mut Vec<u8>) {
    if out.len() % 2 == 1 {
        out.push(0);
    }
}

/// A new file holding only the header; `finish` points it at IFD0.
pub(super) fn header() -> Vec<u8> {
    let mut out = b"II".to_vec();
    out.extend(42u16.to_le_bytes());
    out.extend([0; 4]);
    out
}

pub(super) fn finish(out: &mut [u8], ifd0: u32) {
    out[4..8].copy_from_slice(&ifd0.to_le_bytes());
}

/// Appends `chunks`, returning their offsets and byte counts.
pub(super) fn append_chunks(
    out: &mut Vec<u8>,
    chunks: &[Vec<u8>],
) -> Result<(Vec<u32>, Vec<u32>), PsdError> {
    let mut offsets = Vec::with_capacity(chunks.len());
    let mut counts = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        align(out);
        offsets.push(offset(out)?);
        counts.push(
            u32::try_from(chunk.len()).map_err(|_| PsdError::Invalid("DNG: chunk size".into()))?,
        );
        out.extend(chunk);
    }
    offset(out)?;
    Ok((offsets, counts))
}

/// Appends an IFD with no next IFD after its out-of-line values; returns its
/// offset.
pub(super) fn append_ifd(out: &mut Vec<u8>, mut tags: Vec<(u16, Value)>) -> Result<u32, PsdError> {
    tags.sort_by_key(|(tag, _)| *tag);
    let mut entries = Vec::with_capacity(tags.len());
    for (tag, value) in &tags {
        let (typ, count, data) = value.encode();
        let field = if data.len() > 4 {
            align(out);
            let at = offset(out)?;
            out.extend(&data);
            at.to_le_bytes()
        } else {
            let mut v = [0u8; 4];
            v[..data.len()].copy_from_slice(&data);
            v
        };
        let count =
            u32::try_from(count).map_err(|_| PsdError::Invalid("DNG: tag value size".into()))?;
        entries.push((*tag, typ, count, field));
    }
    align(out);
    let at = offset(out)?;
    out.extend((entries.len() as u16).to_le_bytes());
    for (tag, typ, count, field) in entries {
        out.extend(tag.to_le_bytes());
        out.extend(typ.to_le_bytes());
        out.extend(count.to_le_bytes());
        out.extend(field);
    }
    out.extend([0; 4]);
    offset(out)?;
    Ok(at)
}
