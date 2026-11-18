use super::{
    count, error, expect, finish, utf16, Contour, ContourPoint, Diagnostic, Import, Kind,
    PatternPixels, Payload, Preset, Reader, Shape, MAX_LIBRARY_BYTES,
};
use crate::action_descriptor::{
    read_action_descriptor_from, ActionDescriptor, ActionKey, ActionValue,
};

pub(super) fn decode_shc(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"8BFS\0\x01", "unsupported SHC header")?;
    let length = reader.u32()?;
    let length = count(reader, length, 18)?;
    let mut out = Import::new(Kind::Contours);
    for _ in 0..length {
        let offset = reader.pos;
        let version = reader.u32()?;
        if !matches!(version, 1 | 2) {
            return Err(error(offset, "unsupported SHC contour version"));
        }
        let name = wide(reader)?;
        let size = reader.u16()?;
        let size = count(reader, size.into(), if version == 2 { 5 } else { 4 })?;
        let mut points = Vec::with_capacity(size);
        for _ in 0..size {
            let output = reader.u16()?;
            let input = reader.u16()?;
            points.push(ContourPoint {
                input,
                output,
                corner: false,
            });
        }
        if version == 2 {
            for point in &mut points {
                point.corner = reader.u8()? == 0;
            }
        }
        let input_min = reader.i32()?;
        let input_max = reader.i32()?;
        if points.len() < 2 || points.iter().any(|p| p.input > 255 || p.output > 255) {
            out.diagnostics.push(error(
                offset,
                format!("unsupported contour geometry in {name}"),
            ));
            continue;
        }
        push(
            &mut out,
            name,
            Payload::Contour(Contour {
                points,
                input_min,
                input_max,
            }),
        );
    }
    finish(reader)?;
    Ok(out)
}

pub(super) fn decode_csh(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"cush\0\0\0\x02", "unsupported CSH header")?;
    let length = reader.u32()?;
    let length = count(reader, length, 29)?;
    let mut out = Import::new(Kind::CustomShapes);
    for _ in 0..length {
        let offset = reader.pos;
        let size = reader.u32()? as usize;
        let name = utf16(reader, size)?;
        reader.skip(2 * (size % 2))?;
        let version = reader.u32()?;
        let size = reader.u32()? as usize;
        let mut body = Reader::new(reader.take(size)?);
        if version != 1 {
            out.diagnostics.push(error(
                offset,
                format!("unknown CSH record version in {name}"),
            ));
            continue;
        }
        let id_len = body.u8()? as usize;
        body.skip(id_len)?;
        let bounds = [body.i32()?, body.i32()?, body.i32()?, body.i32()?];
        let whole = body.remaining() / 26 * 26;
        let path_records = body.take(whole)?.to_vec();
        let shape = Shape {
            bounds,
            path_records,
        };
        if shape.paths().is_none_or(|p| p.is_empty()) || body.remaining() > 3 {
            out.diagnostics.push(error(
                offset,
                format!("invalid CSH path geometry in {name}"),
            ));
            continue;
        }
        push(&mut out, name, Payload::Shape(shape));
    }
    finish(reader)?;
    Ok(out)
}

pub(super) fn decode_pat(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"8BPT\0\x01", "unsupported PAT header")?;
    let length = reader.u32()?;
    let length = count(reader, length, 25)?;
    let mut out = Import::new(Kind::Patterns);
    let mut decoded = 0;
    for _ in 0..length {
        let offset = reader.pos;
        if reader.u32()? != 1 {
            return Err(error(offset, "unknown PAT record version"));
        }
        let mode = reader.u32()?;
        reader.skip(4)?;
        let name = wide(reader)?;
        let id_length = reader.u8()? as usize;
        reader.skip(id_length)?;
        if mode == 2 {
            reader.skip(772)?;
        }
        let version = reader.u32()?;
        let size = reader.u32()? as usize;
        reader.skip(size)?;
        let bytes = &reader.data[offset..reader.pos];
        let Some((_, pattern)) =
            crate::patterns::parse_named_pattern(bytes).filter(|_| version == 3)
        else {
            out.diagnostics
                .push(error(offset, format!("unsupported PAT record {name}")));
            continue;
        };
        decoded += pattern.rgba.len();
        if decoded > MAX_LIBRARY_BYTES {
            return Err(error(offset, "decoded patterns exceed the byte limit"));
        }
        out.library.entries.push(Preset {
            name,
            payload: Payload::Pattern(pattern),
            opaque: bytes.to_vec(),
        });
    }
    if reader.remaining() != 0 {
        out.diagnostics
            .push(error(reader.pos, "PAT trailer was not imported"));
    }
    Ok(out)
}

pub(super) fn decode_grd(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"8BGR\0\x05", "unsupported GRD header")?;
    let root = descriptor(reader)?;
    finish(reader)?;
    let mut out = Import::new(Kind::Gradients);
    for value in list(&root, b"GrdL").ok_or_else(|| error(6, "missing GRD gradient list"))? {
        let Some(gradient) = object(value)
            .and_then(|o| item(o, b"Grad"))
            .and_then(object)
        else {
            out.diagnostics
                .push(error(6, "unsupported GRD gradient record"));
            continue;
        };
        let name = named(gradient);
        if list(gradient, b"Clrs").is_none() || list(gradient, b"Trns").is_none() {
            out.diagnostics
                .push(error(6, format!("unsupported gradient type in {name}")));
            continue;
        }
        let mut usable = 0;
        if list(gradient, b"Clrs").unwrap().is_empty() {
            out.diagnostics
                .push(error(6, format!("no color stops in {name}")));
        }
        for stop in list(gradient, b"Clrs").unwrap() {
            let supported = object(stop).is_some_and(|stop| {
                number(stop, b"Lctn").is_some()
                    && match item(stop, b"Type") {
                        Some(ActionValue::Enumerated { value, .. })
                            if matches!(key_bytes(value), b"FrgC" | b"BckC") =>
                        {
                            true
                        }
                        Some(_) => item(stop, b"Clr ").and_then(object).is_some_and(|color| {
                            [b"Rd  ", b"Grn ", b"Bl  "]
                                .iter()
                                .all(|key| number(color, *key).is_some())
                        }),
                        None => false,
                    }
            });
            if supported {
                usable += 1;
            } else {
                out.diagnostics
                    .push(error(6, format!("unsupported color stop in {name}")));
            }
        }
        for stop in list(gradient, b"Trns").unwrap() {
            if !object(stop).is_some_and(|stop| {
                number(stop, b"Lctn").is_some() && number(stop, b"Opct").is_some()
            }) {
                out.diagnostics
                    .push(error(6, format!("unsupported opacity stop in {name}")));
            }
        }
        if usable > 0 {
            push(&mut out, name, Payload::Gradient(gradient.clone()));
        }
    }
    Ok(out)
}

pub(super) fn decode_asl(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"\0\x028BSL\0\x03", "unsupported ASL header")?;
    let pattern_len = reader.u32()? as usize;
    let pattern_bytes = reader.take(pattern_len)?;
    let length = reader.u32()?;
    let length = count(reader, length, 12)?;
    let mut out = Import::new(Kind::Styles);
    for _ in 0..length {
        let offset = reader.pos;
        let size = reader.u32()? as usize;
        let mut record = Reader::new(reader.take(size)?);
        let result = descriptor(&mut record).and_then(|identity| {
            let style = descriptor(&mut record)?;
            let effect = item(&style, b"Lefx")
                .and_then(object)
                .ok_or_else(|| error(offset, "the style has no supported effects"))?;
            Ok((named(&identity), effect.clone()))
        });
        match result {
            Ok((name, effect)) => {
                let mut opaque = Vec::new();
                if item(&effect, b"patternFill").is_some()
                    || item(&effect, b"ebbl").is_some()
                    || item(&effect, b"FrFX").is_some()
                {
                    let total = (out.library.entries.len() + 1).saturating_mul(pattern_bytes.len());
                    if total > MAX_LIBRARY_BYTES {
                        return Err(error(
                            offset,
                            "embedded style patterns exceed the byte limit",
                        ));
                    }
                    opaque.extend_from_slice(pattern_bytes);
                }
                out.library.entries.push(Preset {
                    name,
                    payload: Payload::Style(effect),
                    opaque,
                });
            }
            Err(e) => out.diagnostics.push(error(offset, e.message)),
        }
    }
    finish(reader)?;
    Ok(out)
}

pub(super) fn style_patterns(bytes: &[u8]) -> Result<Vec<(String, PatternPixels)>, Diagnostic> {
    let mut reader = Reader::new(bytes);
    let mut patterns = Vec::new();
    let mut decoded = 0;
    while reader.remaining() != 0 {
        let length = reader.u32()? as usize;
        let record = reader.take(length)?;
        reader.skip((4 - length % 4) % 4)?;
        if let Some((name, pattern)) = crate::patterns::parse_named_pattern(record) {
            decoded += pattern.rgba.len();
            if decoded > MAX_LIBRARY_BYTES {
                return Err(error(
                    reader.pos,
                    "decoded style patterns exceed the byte limit",
                ));
            }
            patterns.push((name, pattern));
        }
    }
    Ok(patterns)
}

pub(super) fn decode_tpl(reader: &mut Reader<'_>) -> Result<Import, Diagnostic> {
    expect(reader, b"8BTP\0\0\0\x03", "unsupported TPL header")?;
    let sections = reader.u32()?;
    let sections = count(reader, sections, 12)?;
    let mut out = Import::new(Kind::Tools);
    for _ in 0..sections {
        let offset = reader.pos;
        let (key, bytes) = section(reader)?;
        if key != b"tptp" {
            if !bytes.is_empty() {
                out.diagnostics
                    .push(error(offset, "embedded TPL resources were not mapped"));
            }
            continue;
        }
        let mut tools = Reader::new(bytes);
        let length = tools.u32()?;
        let length = count(&tools, length, 16)?;
        for _ in 0..length {
            let name = wide(&mut tools)?;
            let name = name
                .strip_prefix("$$$/")
                .and_then(|n| n.split_once('='))
                .map_or(name.as_str(), |(_, n)| n)
                .to_string();
            let value = match descriptor(&mut tools) {
                Ok(value) => value,
                Err(error) => {
                    out.diagnostics
                        .push(super::error(offset + 12 + error.offset, error.message));
                    break;
                }
            };
            if !matches!(
                key_bytes(&value.class.id),
                b"paintbrushTool" | b"pencilTool"
            ) {
                out.diagnostics.push(error(
                    offset + 12 + tools.pos,
                    format!("unsupported tool in {name}"),
                ));
                continue;
            }
            let unmapped: Vec<_> = value
                .items
                .iter()
                .filter_map(|(key, _)| {
                    let key = key_bytes(key);
                    (!matches!(key, b"Opct" | b"Flw " | b"flow" | b"Brsh"))
                        .then(|| String::from_utf8_lossy(key))
                })
                .collect();
            if !unmapped.is_empty() {
                out.diagnostics.push(error(
                    offset + 12 + tools.pos,
                    format!("{name}: unsupported tool settings {}", unmapped.join(", ")),
                ));
            }
            push(&mut out, name, Payload::Tool(value));
        }
    }
    finish(reader)?;
    Ok(out)
}

pub(super) fn section<'a>(reader: &mut Reader<'a>) -> Result<(&'a [u8], &'a [u8]), Diagnostic> {
    expect(reader, b"8BIM", "invalid Photoshop preset section")?;
    let key = reader.take(4)?;
    let length = reader.u32()? as usize;
    let bytes = reader.take(length)?;
    if reader.remaining() > 0 && !reader.data[reader.pos..].starts_with(b"8BIM") {
        reader.skip((4 - length % 4) % 4)?;
    }
    Ok((key, bytes))
}

pub(super) fn descriptor(reader: &mut Reader<'_>) -> Result<ActionDescriptor, Diagnostic> {
    if reader.u32()? != 16 {
        return Err(error(
            reader.pos - 4,
            "unsupported preset descriptor version",
        ));
    }
    read_action_descriptor_from(reader, false).map_err(|e| error(e.offset, e.message))
}

pub fn key_bytes(key: &ActionKey) -> &[u8] {
    match key {
        ActionKey::Code(code) => code,
        ActionKey::String(text) => text,
    }
}

pub fn item<'a>(desc: &'a ActionDescriptor, key: &[u8]) -> Option<&'a ActionValue> {
    desc.items
        .iter()
        .find_map(|(id, value)| (key_bytes(id) == key).then_some(value))
}

pub fn object(value: &ActionValue) -> Option<&ActionDescriptor> {
    match value {
        ActionValue::Descriptor(desc) | ActionValue::GlobalObject(desc) => Some(desc),
        _ => None,
    }
}

pub fn list<'a>(desc: &'a ActionDescriptor, key: &[u8]) -> Option<&'a [ActionValue]> {
    match item(desc, key)? {
        ActionValue::List(values) => Some(values),
        _ => None,
    }
}

pub fn number(desc: &ActionDescriptor, key: &[u8]) -> Option<f64> {
    match item(desc, key)? {
        ActionValue::Integer(i) => Some(f64::from(*i)),
        ActionValue::Double(value) | ActionValue::UnitFloat { value, .. } if value.is_finite() => {
            Some(*value)
        }
        _ => None,
    }
}

pub(super) fn wide(reader: &mut Reader<'_>) -> Result<String, Diagnostic> {
    let size = reader.u32()? as usize;
    utf16(reader, size)
}

pub(super) fn named(desc: &ActionDescriptor) -> String {
    match item(desc, b"Nm  ") {
        Some(ActionValue::Text(name)) => name.clone(),
        _ => desc.name.clone(),
    }
}

pub(super) fn push(report: &mut Import, name: String, payload: Payload) {
    report.library.entries.push(Preset {
        name,
        payload,
        opaque: Vec::new(),
    });
}
