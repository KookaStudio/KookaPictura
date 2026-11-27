use cxx_qt_lib::{QImage, QString};
use pictura_codec::action_descriptor::{ActionClass, ActionDescriptor, ActionKey, ActionValue};
use pictura_codec::preset_library::{
    decode, encode_native, encode_swatches, Color, Format, Kind, Library, Payload, Preset, Swatch,
    SwatchRole,
};
use serde_json::{json, Value};

pub(super) struct ReadPresetSet {
    library: Option<Library>,
    error: String,
    warnings: String,
}

fn kind(code: i32) -> Option<Kind> {
    Kind::ALL.get(usize::try_from(code).ok()?).copied()
}

pub(super) fn preset_read_set(code: i32, extension: &QString, bytes: &[u8]) -> Box<ReadPresetSet> {
    let result = (|| {
        let kind = kind(code).ok_or_else(|| "Unknown preset type".to_string())?;
        let format = Format::from_extension(&extension.to_string())
            .ok_or_else(|| "Unknown preset extension".to_string())?;
        decode(kind, format, bytes).map_err(|e| e.to_string())
    })();
    Box::new(match result {
        Ok(result) => ReadPresetSet {
            library: Some(result.library),
            warnings: result
                .diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
            error: String::new(),
        },
        Err(error) => ReadPresetSet {
            library: None,
            error,
            warnings: String::new(),
        },
    })
}

pub(super) fn preset_set_error(set: &ReadPresetSet) -> QString {
    QString::from(set.error.as_str())
}
pub(super) fn preset_set_warnings(set: &ReadPresetSet) -> QString {
    QString::from(set.warnings.as_str())
}
pub(super) fn preset_set_size(set: &ReadPresetSet) -> i32 {
    set.library
        .as_ref()
        .map_or(0, |set| set.entries.len() as i32)
}

fn row(set: &ReadPresetSet, index: i32) -> Option<&Preset> {
    set.library
        .as_ref()?
        .entries
        .get(usize::try_from(index).ok()?)
}

pub(super) fn preset_row_name(set: &ReadPresetSet, index: i32) -> QString {
    row(set, index).map_or_else(QString::default, |row| QString::from(row.name.as_str()))
}

pub(super) fn preset_row_rgba(set: &ReadPresetSet, index: i32) -> u32 {
    let Some(Payload::Swatch(swatch)) = row(set, index).map(|row| &row.payload) else {
        return 0;
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let rgb = match swatch.color {
        Color::Rgb(rgb) => rgb.map(byte),
        Color::Gray(gray) => [byte(gray); 3],
        Color::Cmyk([c, m, y, k]) => [c, m, y].map(|v| byte((1.0 - v) * (1.0 - k))),
        Color::Lab([l, a, b]) => {
            let rgb = pictura_codec::lab_to_rgb(&[
                byte(l / 100.0),
                byte((a + 128.0) / 255.0),
                byte((b + 128.0) / 255.0),
            ]);
            [rgb[0], rgb[1], rgb[2]]
        }
    };
    u32::from_be_bytes([byte(swatch.alpha), rgb[0], rgb[1], rgb[2]])
}

pub(super) fn preset_row_values(set: &ReadPresetSet, index: i32) -> QString {
    use pictura_codec::preset_library::{item, key_bytes, list, number, object};
    let Some(row) = row(set, index) else {
        return QString::default();
    };
    let values = match &row.payload {
        Payload::BuiltIn(index) => json!({ "index": index }),
        Payload::Contour(values) => {
            let pairs = values
                .points
                .iter()
                .map(|p| format!("{},{}", p.input, p.output))
                .collect::<Vec<_>>()
                .join(" ");
            json!({ "points": pairs })
        }
        Payload::Tool(values) => {
            let class = String::from_utf8_lossy(key_bytes(&values.class.id)).to_string();
            let mut result = json!({
                "tool": class,
                "toolId": pictura_tool_id(&class),
                "opacity": number(values, b"Opct"),
                "flow": number(values, b"Flw ").or_else(|| number(values, b"flow")),
                "size": item(values, b"Brsh").and_then(object).and_then(|tip| number(tip, b"Dmtr")),
            });
            if let Ok(Value::Object(options)) = serde_json::from_slice::<Value>(&row.opaque) {
                result["options"] = Value::Object(options);
            }
            result
        }
        Payload::Pattern(pattern) => {
            json!({"patternId": pictura_codec::preset_library::document_pattern_id(pattern)})
        }
        Payload::Brush(brush) => {
            let tip = item(&brush.settings, b"Brsh")
                .and_then(object)
                .unwrap_or(&brush.settings);
            json!({
                "size": number(tip, b"Dmtr").unwrap_or(f64::from(brush.width.max(brush.height))),
                "spacing": number(tip, b"Spcn"), "angle": number(tip, b"Angl"),
                "roundness": number(tip, b"Rndn"),
            })
        }
        Payload::Gradient(values) => {
            let colors = list(values, b"Clrs").into_iter().flatten().filter_map(|stop| {
                let stop = object(stop)?;
                let color = match item(stop, b"Type")? {
                    ActionValue::Enumerated { value, .. } if key_bytes(value) == b"FrgC" => json!("foreground"),
                    ActionValue::Enumerated { value, .. } if key_bytes(value) == b"BckC" => json!("background"),
                    _ => {
                        let color = item(stop, b"Clr ").and_then(object)?;
                        json!([number(color, b"Rd  ")?, number(color, b"Grn ")?, number(color, b"Bl  ")?])
                    }
                };
                Some(json!({"at": number(stop, b"Lctn")?, "mid": number(stop, b"Mdpn").unwrap_or(50.0), "color": color}))
            }).collect::<Vec<_>>();
            let opacity = list(values, b"Trns").into_iter().flatten().filter_map(|stop| {
                let stop = object(stop)?;
                Some(json!({"at": number(stop, b"Lctn")?, "mid": number(stop, b"Mdpn").unwrap_or(50.0), "value": number(stop, b"Opct")?}))
            }).collect::<Vec<_>>();
            json!({"stops": colors, "opacity": opacity, "smoothness": number(values, b"Intr").map(|v| v * 100.0 / 4096.0)})
        }
        _ => Value::Null,
    };
    QString::from(values.to_string().as_str())
}

pub(super) fn preset_row_preview(set: &ReadPresetSet, index: i32) -> QImage {
    const EDGE: u32 = 24;
    let Some(row) = row(set, index) else {
        return QImage::default();
    };
    if let Payload::Shape(shape) = &row.payload {
        let Some(mut paths) = shape.paths() else {
            return QImage::default();
        };
        let [top, left, bottom, right] = shape.bounds;
        let scale = f64::from(EDGE - 4) / f64::from((bottom - top).max(right - left));
        let map = |(x, y)| (2.0 + x * scale, 2.0 + y * scale);
        for path in &mut paths {
            for point in &mut path.points {
                point.anchor = map(point.anchor);
                point.in_handle = point.in_handle.map(map);
                point.out_handle = point.out_handle.map(map);
            }
        }
        let rgba = pictura_core::shape::paths_coverage(&paths, EDGE, EDGE)
            .into_iter()
            .flat_map(|alpha| [110, 110, 110, alpha])
            .collect();
        return super::super::helpers_composite::rgba_image(rgba, EDGE as i32, EDGE as i32);
    }
    let (width, height) = match &row.payload {
        Payload::Brush(brush) => (brush.width, brush.height),
        Payload::Pattern(pattern) => (pattern.width, pattern.height),
        _ => return QImage::default(),
    };
    let max = width.max(height);
    let w = (u64::from(width) * u64::from(EDGE) / u64::from(max)).max(1) as u32;
    let h = (u64::from(height) * u64::from(EDGE) / u64::from(max)).max(1) as u32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let sx = u64::from(x) * u64::from(width) / u64::from(w);
            let sy = u64::from(y) * u64::from(height) / u64::from(h);
            let at = (sy * u64::from(width) + sx) as usize;
            match &row.payload {
                Payload::Brush(brush) => {
                    rgba.extend_from_slice(&[110, 110, 110, brush.coverage[at]])
                }
                Payload::Pattern(pattern) => {
                    rgba.extend_from_slice(&pattern.rgba[4 * at..4 * at + 4])
                }
                _ => unreachable!(),
            }
        }
    }
    super::super::helpers_composite::rgba_image(rgba, w as i32, h as i32)
}

pub(super) fn preset_row_bytes(set: &ReadPresetSet, index: i32) -> Vec<u8> {
    let Some(set) = &set.library else {
        return Vec::new();
    };
    let Some(row) = set.entries.get(index as usize) else {
        return Vec::new();
    };
    encode_native(&Library {
        kind: set.kind,
        entries: vec![row.clone()],
    })
    .unwrap_or_default()
}

pub(super) fn preset_encode_set(
    code: i32,
    extension: &QString,
    mut rows: &[u8],
    output: &mut Vec<u8>,
) -> QString {
    let result = (|| {
        let kind = kind(code).ok_or_else(|| "Unknown preset type".to_string())?;
        let format = Format::from_extension(&extension.to_string())
            .ok_or_else(|| "Unknown preset extension".to_string())?;
        let mut entries = Vec::new();
        if rows.len() > 128 * 1024 * 1024 {
            return Err("Preset set exceeds the byte limit".to_string());
        }
        while !rows.is_empty() {
            let name = std::str::from_utf8(take(&mut rows)?).map_err(|e| e.to_string())?;
            let bytes = take(&mut rows)?;
            let result = decode(kind, Format::Native, bytes).map_err(|e| e.to_string())?;
            let mut decoded = result.library.entries.into_iter();
            let Some(mut entry) = decoded.next() else {
                return Err("Missing preset row".to_string());
            };
            if decoded.next().is_some() {
                return Err("Multiple records in preset row".to_string());
            }
            entry.name = name.to_owned();
            entries.push(entry);
        }
        let library = Library { kind, entries };
        let encoded = if format == Format::Native {
            encode_native(&library)
        } else {
            encode_swatches(format, &library)
        };
        encoded.map_err(|e| e.to_string())
    })();
    match result {
        Ok(bytes) => {
            *output = bytes;
            QString::default()
        }
        Err(error) => QString::from(error.as_str()),
    }
}

fn take<'a>(rows: &mut &'a [u8]) -> Result<&'a [u8], String> {
    let (length, rest) = rows
        .split_at_checked(4)
        .ok_or_else(|| "Truncated preset row".to_string())?;
    let length = u32::from_be_bytes(length.try_into().unwrap()) as usize;
    let (record, rest) = rest
        .split_at_checked(length)
        .ok_or_else(|| "Truncated preset row".to_string())?;
    *rows = rest;
    Ok(record)
}

fn encode_one(kind: Kind, name: &QString, payload: Payload) -> Vec<u8> {
    let entries = vec![Preset {
        name: name.to_string(),
        payload,
        opaque: Vec::new(),
    }];
    encode_native(&Library { kind, entries }).unwrap_or_default()
}

fn pictura_tool_id(class: &str) -> &str {
    match class {
        "paintbrushTool" => "brush",
        "pencilTool" => "pencil",
        other => other,
    }
}

pub(super) fn preset_make_tool(name: &QString, tool: &QString, options: &QString) -> Vec<u8> {
    let tool = tool.to_string();
    let descriptor = ActionDescriptor {
        name: String::new(),
        class: ActionClass {
            name: tool.clone(),
            id: ActionKey::String(tool.into_bytes()),
        },
        items: Vec::new(),
    };
    let entries = vec![Preset {
        name: name.to_string(),
        payload: Payload::Tool(descriptor),
        opaque: options.to_string().into_bytes(),
    }];
    encode_native(&Library {
        kind: Kind::Tools,
        entries,
    })
    .unwrap_or_default()
}

pub(super) fn preset_make_swatch(name: &QString, argb: u32) -> Vec<u8> {
    let [a, r, g, b] = argb.to_be_bytes().map(|x| f32::from(x) / 255.0);
    encode_one(
        Kind::Swatches,
        name,
        Payload::Swatch(Swatch {
            color: Color::Rgb([r, g, b]),
            alpha: a,
            role: SwatchRole::Normal,
        }),
    )
}

pub(super) fn preset_make_builtin(code: i32, name: &QString, index: i32) -> Vec<u8> {
    let (Some(kind), Ok(index)) = (kind(code), u16::try_from(index)) else {
        return Vec::new();
    };
    encode_one(kind, name, Payload::BuiltIn(index))
}
