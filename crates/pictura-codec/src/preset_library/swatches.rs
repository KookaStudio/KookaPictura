use super::native::{float, read_role, write_floats};
use super::{
    count, error, expect, finish, push_len, utf16, Color, Diagnostic, Format, Import, Kind,
    Library, Payload, Preset, Reader, Swatch, SwatchRole, MAX_LIBRARY_BYTES, MAX_NAME_UNITS,
};

pub(super) fn decode_aco(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    let version = reader.u16()?;
    let mut result = read_aco_table(reader, version)?;
    if version == 1 && reader.remaining() != 0 {
        expect(reader, &[0, 2], "expected the named ACO table")?;
        result = read_aco_table(reader, 2)?;
    }
    finish(reader)?;
    Ok(result)
}

fn read_aco_table(reader: &mut Reader<'_>, version: u16) -> Result<Import, Diagnostic> {
    if !matches!(version, 1 | 2) {
        return Err(error(reader.pos - 2, "unknown ACO version"));
    }
    let length = reader.u16()?;
    let length = count(reader, length.into(), if version == 1 { 10 } else { 14 })?;
    let mut result = Import::new(Kind::Swatches);
    for index in 0..length {
        let offset = reader.pos;
        let model = reader.u16()?;
        let components = [reader.u16()?, reader.u16()?, reader.u16()?, reader.u16()?];
        let name = if version == 1 {
            format!("Swatch {}", index + 1)
        } else {
            let units = reader.u32()? as usize;
            utf16(reader, units)?
        };
        let full = |channel: u16| f32::from(channel) / 65535.0;
        let color = match model {
            0 => Some(Color::Rgb([
                full(components[0]),
                full(components[1]),
                full(components[2]),
            ])),
            2 => Some(Color::Cmyk(
                components.map(|channel| full(u16::MAX - channel)),
            )),
            7 if components[0] <= 10_000 => Some(Color::Lab([
                f32::from(components[0]) / 100.0,
                f32::from(components[1] as i16) / 100.0,
                f32::from(components[2] as i16) / 100.0,
            ])),
            8 if components[0] <= 10_000 => Some(Color::Gray(f32::from(components[0]) / 10_000.0)),
            _ => None,
        };
        if let Some(color) = color {
            result
                .library
                .entries
                .push(swatch(name, color, SwatchRole::Normal));
        } else {
            result.diagnostics.push(error(
                offset,
                format!("unsupported ACO color {model} in {name}"),
            ));
        }
    }
    Ok(result)
}

pub(super) fn decode_ase(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(
        reader,
        b"ASEF\0\x01\0\0",
        "unknown ASE signature or version",
    )?;
    let length = reader.u32()?;
    let length = count(reader, length, 6)?;
    let mut result = Import::new(Kind::Swatches);
    for _ in 0..length {
        let offset = reader.pos;
        let kind = reader.u16()?;
        let size = reader.u32()? as usize;
        let mut block = Reader::new(reader.take(size)?);
        match kind {
            1 => match read_ase_swatch(&mut block) {
                Ok(swatch) => result.library.entries.push(swatch),
                Err(mut diagnostic) => {
                    diagnostic.offset += offset + 6;
                    result.diagnostics.push(diagnostic);
                }
            },
            0xc001 | 0xc002 => {}
            _ => result
                .diagnostics
                .push(error(offset, format!("unsupported ASE block {kind:#06x}"))),
        }
    }
    finish(reader)?;
    Ok(result)
}

fn read_ase_swatch(block: &mut Reader<'_>) -> Result<Preset, Diagnostic> {
    let units = block.u16()? as usize;
    let name = utf16(block, units)?;
    let color = match block.take(4)? {
        b"RGB " => Color::Rgb([float(block)?, float(block)?, float(block)?]),
        b"CMYK" => Color::Cmyk([float(block)?, float(block)?, float(block)?, float(block)?]),
        b"LAB " => Color::Lab([float(block)? * 100.0, float(block)?, float(block)?]),
        b"Gray" | b"GRAY" => Color::Gray(float(block)?),
        _ => {
            return Err(error(
                block.pos - 4,
                format!("unsupported ASE color model in {name}"),
            ))
        }
    };
    if !valid_color(color) {
        return Err(error(
            block.pos,
            format!("out-of-range ASE color in {name}"),
        ));
    }
    let role = read_role(block.u16()?, block.pos - 2)?;
    finish(block)?;
    Ok(swatch(name, color, role))
}

fn swatch(name: String, color: Color, role: SwatchRole) -> Preset {
    Preset {
        name,
        payload: Payload::Swatch(Swatch {
            color,
            alpha: 1.0,
            role,
        }),
        opaque: Vec::new(),
    }
}

fn valid_color(color: Color) -> bool {
    let unit = |value| (0.0..=1.0).contains(&value);
    match color {
        Color::Rgb(values) => values.into_iter().all(unit),
        Color::Cmyk(values) => values.into_iter().all(unit),
        Color::Gray(value) => unit(value),
        Color::Lab([l, a, b]) => {
            (0.0..=100.0).contains(&l)
                && (-128.0..=127.0).contains(&a)
                && (-128.0..=127.0).contains(&b)
        }
    }
}

pub fn encode_swatches(format: Format, library: &Library) -> Result<Vec<u8>, Diagnostic> {
    if library.kind != Kind::Swatches || !matches!(format, Format::Aco | Format::Ase) {
        return Err(error(0, "expected an ACO or ASE swatch export"));
    }
    let length = u16::try_from(library.entries.len()).map_err(|_| error(0, "too many swatches"))?;
    if format == Format::Aco {
        let mut buffer = Vec::new();
        for version in [1u16, 2] {
            buffer.extend_from_slice(&version.to_be_bytes());
            buffer.extend_from_slice(&length.to_be_bytes());
            for entry in &library.entries {
                let data = representable(entry, buffer.len())?;
                if data.role != SwatchRole::Normal {
                    return Err(error(
                        buffer.len(),
                        format!("ACO cannot store the role of {}", entry.name),
                    ));
                }
                write_aco_color(&mut buffer, data.color);
                if version == 2 {
                    write_name(&mut buffer, &entry.name, false)?;
                }
                bound_export(&buffer)?;
            }
        }
        return Ok(buffer);
    }

    let mut buffer = b"ASEF\0\x01\0\0".to_vec();
    push_len(&mut buffer, library.entries.len())?;
    for entry in &library.entries {
        let data = representable(entry, buffer.len())?;
        let mut block = Vec::new();
        write_name(&mut block, &entry.name, true)?;
        match data.color {
            Color::Rgb(values) => write_ase_color(&mut block, b"RGB ", &values)?,
            Color::Cmyk(values) => write_ase_color(&mut block, b"CMYK", &values)?,
            Color::Lab([l, a, b]) => write_ase_color(&mut block, b"LAB ", &[l / 100.0, a, b])?,
            Color::Gray(value) => write_ase_color(&mut block, b"Gray", &[value])?,
        }
        block.extend_from_slice(&(data.role as u16).to_be_bytes());
        buffer.extend_from_slice(&1u16.to_be_bytes());
        push_len(&mut buffer, block.len())?;
        buffer.extend(block);
        bound_export(&buffer)?;
    }
    Ok(buffer)
}

fn bound_export(buffer: &[u8]) -> Result<(), Diagnostic> {
    if buffer.len() > MAX_LIBRARY_BYTES {
        return Err(error(buffer.len(), "swatch set exceeds the byte limit"));
    }
    Ok(())
}

fn representable(entry: &Preset, offset: usize) -> Result<Swatch, Diagnostic> {
    if let Payload::Swatch(data) = entry.payload {
        if data.alpha == 1.0 && valid_color(data.color) && entry.opaque.is_empty() {
            return Ok(data);
        }
    }
    Err(error(
        offset,
        format!("swatch {} cannot be represented", entry.name),
    ))
}

fn write_name(buffer: &mut Vec<u8>, name: &str, short: bool) -> Result<(), Diagnostic> {
    let size = name.encode_utf16().count() + 1;
    if size > MAX_NAME_UNITS {
        return Err(error(buffer.len(), "swatch name exceeds the length limit"));
    }
    if short {
        buffer.extend_from_slice(&(size as u16).to_be_bytes());
    } else {
        push_len(buffer, size)?;
    }
    for unit in name.encode_utf16().chain([0]) {
        buffer.extend_from_slice(&unit.to_be_bytes());
    }
    Ok(())
}

fn write_aco_color(buffer: &mut Vec<u8>, color: Color) {
    let full = |value: f32| (value * 65535.0).round() as u16;
    let (model, components): (u16, [u16; 4]) = match color {
        Color::Rgb([r, g, b]) => (0, [full(r), full(g), full(b), 0]),
        Color::Cmyk(values) => (2, values.map(|value| u16::MAX - full(value))),
        Color::Lab([l, a, b]) => (
            7,
            [
                (l * 100.0).round() as u16,
                (a * 100.0).round() as i16 as u16,
                (b * 100.0).round() as i16 as u16,
                0,
            ],
        ),
        Color::Gray(value) => (8, [(value * 10_000.0).round() as u16, 0, 0, 0]),
    };
    buffer.extend_from_slice(&model.to_be_bytes());
    for value in components {
        buffer.extend_from_slice(&value.to_be_bytes());
    }
}

fn write_ase_color(
    buffer: &mut Vec<u8>,
    model: &[u8; 4],
    values: &[f32],
) -> Result<(), Diagnostic> {
    buffer.extend_from_slice(model);
    write_floats(buffer, values)
}
