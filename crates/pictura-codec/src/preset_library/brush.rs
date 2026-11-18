use std::collections::BTreeMap;

use crate::action_descriptor::ActionValue;
use crate::read::decode_packbits;

use super::adobe::{descriptor, item, list, named, object, push, section};
use super::{
    error, expect, finish, Brush, Diagnostic, Import, Kind, Payload, Reader, MAX_LIBRARY_BYTES,
};

const MAX_SAMPLE_PIXELS: usize = 25_000_000;

pub(super) fn decode_abr(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, &[0, 10, 0, 1], "unsupported ABR version")?;
    let mut report = Import::new(Kind::Brushes);
    let mut samples = BTreeMap::new();
    let mut description = None;
    while reader.remaining() > 0 {
        let offset = reader.pos;
        let (key, bytes) = section(reader)?;
        match key {
            b"samp" => read_samples(bytes, offset + 12, &mut samples, &mut report)?,
            b"desc" if description.is_none() => description = Some((offset + 12, bytes)),
            _ => report
                .diagnostics
                .push(error(offset, "ABR section is not applied")),
        }
    }
    let (offset, bytes) = description.ok_or_else(|| error(4, "missing ABR settings descriptor"))?;
    let mut reader = Reader::new(bytes);
    let root = descriptor(&mut reader)?;
    finish(&reader)?;
    let brushes = list(&root, b"Brsh").ok_or_else(|| error(offset, "missing ABR brush list"))?;
    let mut decoded = 0;
    for entry in brushes {
        let Some(brush) = object(entry) else {
            report
                .diagnostics
                .push(error(offset, "unsupported ABR brush descriptor"));
            continue;
        };
        let name = named(brush);
        let id = item(brush, b"Brsh")
            .and_then(object)
            .and_then(|tip| item(tip, b"sampledData"));
        let Some(ActionValue::Text(id)) = id else {
            report
                .diagnostics
                .push(error(offset, format!("no supported sampled tip in {name}")));
            continue;
        };
        let Some((width, height, mask)) = samples.get(id) else {
            report.diagnostics.push(error(
                offset,
                format!("cannot resolve sampled tip for {name}"),
            ));
            continue;
        };
        decoded += mask.len();
        if decoded > MAX_LIBRARY_BYTES {
            return Err(error(offset, "decoded ABR presets exceed the byte limit"));
        }
        if brush.items.iter().any(|(key, value)| {
            matches!(value, ActionValue::Boolean(true))
                && super::adobe::key_bytes(key).starts_with(b"use")
        }) {
            report.diagnostics.push(error(
                offset,
                format!("some advanced brush options in {name} are retained only"),
            ));
        }
        push(
            &mut report,
            name,
            Payload::Brush(Brush {
                width: *width,
                height: *height,
                coverage: mask.clone(),
                settings: brush.clone(),
            }),
        );
    }
    Ok(report)
}

type SampleMap = BTreeMap<String, (u32, u32, Vec<u8>)>;

fn read_samples(
    bytes: &[u8],
    base: usize,
    samples: &mut SampleMap,
    report: &mut Import,
) -> Result<(), Diagnostic> {
    let mut reader = Reader::new(bytes);
    let mut decoded: usize = samples.values().map(|(_, _, pixels)| pixels.len()).sum();
    while reader.remaining() != 0 {
        let offset = base + reader.pos;
        let length = reader.u32()? as usize;
        let mut entry = Reader::new(reader.take(length)?);
        reader.skip((4 - length % 4) % 4)?;
        let result = read_sample(&mut entry);
        match result {
            Ok((id, width, height, coverage)) => {
                decoded += coverage.len();
                if decoded > MAX_LIBRARY_BYTES || samples.len() >= super::MAX_ENTRIES {
                    return Err(error(offset, "ABR sample collection exceeds its limits"));
                }
                samples.entry(id).or_insert((width, height, coverage));
            }
            Err(e) => report
                .diagnostics
                .push(error(offset + 4 + e.offset, e.message)),
        }
    }
    Ok(())
}

fn read_sample(reader: &mut Reader<'_>) -> Result<(String, u32, u32, Vec<u8>), Diagnostic> {
    let id_len = reader.u8()? as usize;
    let id = std::str::from_utf8(reader.take(id_len)?)
        .map_err(|_| error(reader.pos, "invalid ABR sample identifier"))?
        .to_owned();
    reader.skip(10)?;
    let (top, left, bottom, right) = (reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?);
    let extent = |end: i32, start: i32| {
        end.checked_sub(start)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0)
    };
    let width = extent(right, left).ok_or_else(|| error(reader.pos, "invalid ABR width"))?;
    let height = extent(bottom, top).ok_or_else(|| error(reader.pos, "invalid ABR height"))?;
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .filter(|n| *n <= MAX_SAMPLE_PIXELS)
        .ok_or_else(|| error(reader.pos, "ABR sample exceeds the pixel limit"))?;
    if reader.u16()? != 8 {
        return Err(error(reader.pos - 2, "unsupported ABR sample depth"));
    }
    let coverage = match reader.u8()? {
        0 => reader.take(pixels)?.to_vec(),
        1 => {
            let lengths = reader.take(height as usize * 2)?;
            let mut out = vec![0; pixels];
            for (&size, row) in lengths
                .as_chunks::<2>()
                .0
                .iter()
                .zip(out.chunks_exact_mut(width as usize))
            {
                let n = u16::from_be_bytes(size) as usize;
                decode_packbits(reader.take(n)?, row)?;
            }
            out
        }
        _ => return Err(error(reader.pos - 1, "unsupported ABR sample compression")),
    };
    Ok((id, width, height, coverage))
}
