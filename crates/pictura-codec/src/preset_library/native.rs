use super::{
    count, error, expect, finish, push_len, Brush, Color, Contour, ContourPoint, Diagnostic,
    Import, Kind, Library, PatternPixels, Payload, Preset, Reader, Shape, Swatch, SwatchRole,
    MAX_ENTRIES, MAX_LIBRARY_BYTES, MAX_NAME_UNITS,
};
use crate::action_descriptor::{read_action_descriptor, write_action_descriptor, ActionDescriptor};

const MAGIC: &[u8] = b"KPPRESET\0\x01";

pub(super) fn decode_native(kind: Kind, reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, MAGIC, "unknown native preset format or version")?;
    if reader.u8()? != kind as u8 {
        return Err(error(
            reader.pos - 1,
            "native set has the wrong preset type",
        ));
    }
    let entries = reader.u32()?;
    let entries = count(reader, entries, 13)?;
    let mut result = Import::new(kind);
    for _ in 0..entries {
        let length = reader.u32()? as usize;
        let mut record = Reader::new(reader.take(length)?);
        let name_length = record.u32()? as usize;
        if name_length > 4 * MAX_NAME_UNITS {
            return Err(error(
                reader.pos - length,
                "preset name exceeds the length limit",
            ));
        }
        let name = std::str::from_utf8(record.take(name_length)?)
            .map_err(|_| error(reader.pos - length, "invalid native preset name"))?
            .to_string();
        let opaque_length = record.u32()? as usize;
        let opaque = record.take(opaque_length)?.to_vec();
        let payload = read_payload(kind, &mut record)?;
        finish(&record)?;
        result.library.entries.push(Preset {
            name,
            payload,
            opaque,
        });
    }
    finish(reader)?;
    Ok(result)
}

pub fn encode_native(library: &Library) -> Result<Vec<u8>, Diagnostic> {
    if library.entries.len() > MAX_ENTRIES {
        return Err(error(0, "too many presets"));
    }
    let mut out = MAGIC.to_vec();
    out.push(library.kind as u8);
    push_len(&mut out, library.entries.len())?;
    for entry in &library.entries {
        if entry.name.len() > 4 * MAX_NAME_UNITS || entry.opaque.len() > MAX_LIBRARY_BYTES {
            return Err(error(out.len(), "preset entry exceeds the byte limit"));
        }
        let position = out.len();
        out.extend_from_slice(&[0; 4]);
        push_len(&mut out, entry.name.len())?;
        out.extend_from_slice(entry.name.as_bytes());
        push_len(&mut out, entry.opaque.len())?;
        out.extend_from_slice(&entry.opaque);
        write_payload(library.kind, &entry.payload, &mut out)?;
        if out.len() > MAX_LIBRARY_BYTES {
            return Err(error(out.len(), "preset library exceeds the byte limit"));
        }
        let length = (out.len() - position - 4) as u32;
        out[position..position + 4].copy_from_slice(&length.to_be_bytes());
    }
    Ok(out)
}

fn read_payload(kind: Kind, reader: &mut Reader<'_>) -> Result<Payload, Diagnostic> {
    Ok(match (kind, reader.u8()?) {
        (kind, 0) if kind != Kind::Swatches => Payload::BuiltIn(reader.u16()?),
        (Kind::Swatches, 1) => {
            let role = read_role(reader.u8()? as u16, reader.pos)?;
            let alpha = float(reader)?;
            let color = match reader.u8()? {
                0 => Color::Rgb([float(reader)?, float(reader)?, float(reader)?]),
                1 => Color::Cmyk([
                    float(reader)?,
                    float(reader)?,
                    float(reader)?,
                    float(reader)?,
                ]),
                2 => Color::Lab([float(reader)?, float(reader)?, float(reader)?]),
                3 => Color::Gray(float(reader)?),
                _ => return Err(error(reader.pos, "invalid native swatch color model")),
            };
            Payload::Swatch(Swatch { color, alpha, role })
        }
        (Kind::Contours, 2) => {
            let input_min = reader.i32()?;
            let input_max = reader.i32()?;
            let length = reader.u32()?;
            let length = count(reader, length, 5)?;
            let mut points = Vec::with_capacity(length);
            for _ in 0..length {
                let input = reader.u16()?;
                let output = reader.u16()?;
                let corner = match reader.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(error(reader.pos, "invalid native contour flag")),
                };
                points.push(ContourPoint {
                    input,
                    output,
                    corner,
                });
            }
            Payload::Contour(Contour {
                points,
                input_min,
                input_max,
            })
        }
        (Kind::CustomShapes, 3) => {
            let bounds = [reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?];
            let shape = Shape {
                bounds,
                path_records: blob(reader)?.to_vec(),
            };
            if shape.paths().is_none_or(|p| p.is_empty()) {
                return Err(error(reader.pos, "invalid shape path records"));
            }
            Payload::Shape(shape)
        }
        (Kind::Patterns, 4) => {
            let width = reader.u32()?;
            let height = reader.u32()?;
            let pattern_id = std::str::from_utf8(blob(reader)?)
                .map_err(|_| error(reader.pos, "invalid pattern identifier"))?
                .to_string();
            let rgba = blob(reader)?;
            check_raster(width, height, 4, rgba.len())?;
            Payload::Pattern(PatternPixels {
                pattern_id,
                width,
                height,
                rgba: rgba.to_vec(),
            })
        }
        (Kind::Gradients, 5) => Payload::Gradient(read_descriptor(reader)?),
        (Kind::Styles, 6) => Payload::Style(read_descriptor(reader)?),
        (Kind::Tools, 7) => Payload::Tool(read_descriptor(reader)?),
        (Kind::Brushes, 8) => {
            let width = reader.u32()?;
            let height = reader.u32()?;
            let pixels = blob(reader)?;
            check_raster(width, height, 1, pixels.len())?;
            Payload::Brush(Brush {
                width,
                height,
                coverage: pixels.to_vec(),
                settings: read_descriptor(reader)?,
            })
        }
        _ => {
            return Err(error(
                reader.pos,
                "native preset payload does not match its type",
            ))
        }
    })
}

fn write_payload(kind: Kind, payload: &Payload, out: &mut Vec<u8>) -> Result<(), Diagnostic> {
    match (kind, payload) {
        (kind, Payload::BuiltIn(index)) if kind != Kind::Swatches => {
            out.push(0);
            out.extend_from_slice(&index.to_be_bytes());
        }
        (Kind::Swatches, Payload::Swatch(swatch)) => {
            out.extend_from_slice(&[1, swatch.role as u8]);
            write_floats(out, &[swatch.alpha])?;
            let (model, values): (_, &[f32]) = match &swatch.color {
                Color::Rgb(values) => (0, values),
                Color::Cmyk(values) => (1, values),
                Color::Lab(values) => (2, values),
                Color::Gray(value) => (3, std::slice::from_ref(value)),
            };
            out.push(model);
            write_floats(out, values)?;
        }
        (Kind::Contours, Payload::Contour(contour)) => {
            if contour.points.len() > MAX_ENTRIES {
                return Err(error(out.len(), "too many contour points"));
            }
            out.push(2);
            out.extend_from_slice(&contour.input_min.to_be_bytes());
            out.extend_from_slice(&contour.input_max.to_be_bytes());
            push_len(out, contour.points.len())?;
            for point in &contour.points {
                out.extend_from_slice(&point.input.to_be_bytes());
                out.extend_from_slice(&point.output.to_be_bytes());
                out.push(u8::from(point.corner));
            }
        }
        (Kind::CustomShapes, Payload::Shape(shape)) => {
            if shape.paths().is_none_or(|p| p.is_empty()) {
                return Err(error(out.len(), "invalid shape path records"));
            }
            out.push(3);
            for bound in shape.bounds {
                out.extend_from_slice(&bound.to_be_bytes());
            }
            write_blob(out, &shape.path_records)?;
        }
        (Kind::Patterns, Payload::Pattern(pattern)) => {
            check_raster(pattern.width, pattern.height, 4, pattern.rgba.len())?;
            out.push(4);
            out.extend_from_slice(&pattern.width.to_be_bytes());
            out.extend_from_slice(&pattern.height.to_be_bytes());
            write_blob(out, pattern.pattern_id.as_bytes())?;
            write_blob(out, &pattern.rgba)?;
        }
        (Kind::Gradients, Payload::Gradient(desc)) => write_descriptor(out, 5, desc)?,
        (Kind::Styles, Payload::Style(desc)) => write_descriptor(out, 6, desc)?,
        (Kind::Tools, Payload::Tool(desc)) => write_descriptor(out, 7, desc)?,
        (Kind::Brushes, Payload::Brush(brush)) => {
            check_raster(brush.width, brush.height, 1, brush.coverage.len())?;
            out.push(8);
            out.extend_from_slice(&brush.width.to_be_bytes());
            out.extend_from_slice(&brush.height.to_be_bytes());
            write_blob(out, &brush.coverage)?;
            write_blob(out, &write_action_descriptor(&brush.settings, false))?;
        }
        _ => {
            return Err(error(
                out.len(),
                "native preset payload does not match its type",
            ))
        }
    }
    Ok(())
}

fn blob<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], Diagnostic> {
    let length = reader.u32()? as usize;
    Ok(reader.take(length)?)
}

fn write_blob(out: &mut Vec<u8>, blob: &[u8]) -> Result<(), Diagnostic> {
    if blob.len().saturating_add(out.len()).saturating_add(4) > MAX_LIBRARY_BYTES {
        return Err(error(out.len(), "preset library exceeds the byte limit"));
    }
    push_len(out, blob.len())?;
    out.extend_from_slice(blob);
    Ok(())
}

fn check_raster(width: u32, height: u32, channels: usize, length: usize) -> Result<(), Diagnostic> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(channels));
    if length == 0 || expected != Some(length) {
        return Err(error(0, "invalid native preset raster size"));
    }
    Ok(())
}

fn read_descriptor(reader: &mut Reader<'_>) -> Result<ActionDescriptor, Diagnostic> {
    let bytes = blob(reader)?;
    read_action_descriptor(bytes, false).map_err(|e| error(e.offset, e.message))
}

fn write_descriptor(out: &mut Vec<u8>, tag: u8, desc: &ActionDescriptor) -> Result<(), Diagnostic> {
    out.push(tag);
    write_blob(out, &write_action_descriptor(desc, false))
}

pub(super) fn float(reader: &mut Reader<'_>) -> Result<f32, Diagnostic> {
    let value = f32::from_bits(reader.u32()?);
    if !value.is_finite() {
        return Err(error(reader.pos - 4, "non-finite preset value"));
    }
    Ok(value)
}

pub(super) fn write_floats(out: &mut Vec<u8>, values: &[f32]) -> Result<(), Diagnostic> {
    for &value in values {
        if !value.is_finite() {
            return Err(error(out.len(), "non-finite preset value"));
        }
        out.extend_from_slice(&value.to_be_bytes());
    }
    Ok(())
}

pub(super) fn read_role(role: u16, offset: usize) -> Result<SwatchRole, Diagnostic> {
    match role {
        0 => Ok(SwatchRole::Global),
        1 => Ok(SwatchRole::Spot),
        2 => Ok(SwatchRole::Normal),
        _ => Err(error(offset, "unknown swatch role")),
    }
}
