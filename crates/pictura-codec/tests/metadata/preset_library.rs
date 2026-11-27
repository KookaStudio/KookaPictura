use pictura_codec::action_descriptor::{
    write_action_descriptor, ActionClass, ActionDescriptor, ActionKey, ActionValue,
};
use pictura_codec::preset_library::{
    decode, encode_native, encode_swatches, Color, Format, Kind, Library, Payload, Preset, Swatch,
    SwatchRole,
};

fn swatch(name: &str, color: Color) -> Preset {
    Preset {
        name: name.into(),
        payload: Payload::Swatch(Swatch {
            color,
            alpha: 1.0,
            role: SwatchRole::Normal,
        }),
        opaque: Vec::new(),
    }
}

fn word(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_be_bytes());
}

fn number(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_be_bytes());
}

fn wide(buf: &mut Vec<u8>, name: &str, short_count: bool) {
    let units = name.encode_utf16().chain([0]).collect::<Vec<_>>();
    if short_count {
        word(buf, units.len() as u16);
    } else {
        number(buf, units.len() as u32);
    }
    for unit in units {
        word(buf, unit);
    }
}

fn aco_colors(block: &mut Vec<u8>, version: u16, records: &[(u16, [u16; 4], &str)]) {
    word(block, version);
    word(block, records.len() as u16);
    for (model, channels, name) in records {
        word(block, *model);
        for value in channels {
            word(block, *value);
        }
        if version == 2 {
            wide(block, name, false);
        }
    }
}

#[test]
fn preset_swatches_decode_independently_generated_formats() {
    let records = [
        (0, [65535, 0, 0, 0], "Ω palette"),
        (2, [0, 65535, 0, 65535], "Inks"),
        (7, [5100, (-625i16) as u16, 1100, 0], "Lab"),
        (8, [2500, 0, 0, 0], "Gray"),
    ];
    let mut aco = Vec::new();
    aco_colors(&mut aco, 1, &records);
    aco_colors(&mut aco, 2, &records);
    let expected = vec![
        swatch("Ω palette", Color::Rgb([1.0, 0.0, 0.0])),
        swatch("Inks", Color::Cmyk([1.0, 0.0, 1.0, 0.0])),
        swatch("Lab", Color::Lab([51.0, -6.25, 11.0])),
        swatch("Gray", Color::Gray(0.25)),
    ];
    let report = decode(Kind::Swatches, Format::Aco, &aco).unwrap();
    assert_eq!(report.library.entries, expected);
    assert!(report.diagnostics.is_empty());

    let mut ase = b"ASEF\x00\x01\x00\x00\x00\x00\x00\x01".to_vec();
    let mut block = Vec::new();
    wide(&mut block, "A🟣", true);
    block.extend_from_slice(b"LAB ");
    for sample in [0.5f32, -1.0, 10.0] {
        number(&mut block, sample.to_bits());
    }
    word(&mut block, 2);
    word(&mut ase, 1);
    number(&mut ase, block.len() as u32);
    ase.extend(block);
    let report = decode(Kind::Swatches, Format::Ase, &ase).unwrap();
    assert_eq!(
        report.library.entries,
        [swatch("A🟣", Color::Lab([50.0, -1.0, 10.0]))]
    );
}

fn object(code: [u8; 4], items: Vec<([u8; 4], ActionValue)>) -> ActionDescriptor {
    ActionDescriptor {
        name: String::new(),
        class: ActionClass {
            name: String::new(),
            id: ActionKey::Code(code),
        },
        items: items
            .into_iter()
            .map(|(key, value)| (ActionKey::Code(key), value))
            .collect(),
    }
}

fn descriptor(out: &mut Vec<u8>, value: &ActionDescriptor) {
    number(out, 16);
    out.extend(write_action_descriptor(value, false));
}

fn assert_native_fidelity(library: &Library) {
    let entry = &library.entries[0];
    let mut first = entry.clone();
    first.name = "正".into();
    let mut second = first.clone();
    second.opaque.extend([1, 0, 255]);
    let expected = Library {
        kind: library.kind,
        entries: vec![second, first],
    };
    let file = encode_native(&expected).unwrap();
    assert_eq!(
        decode(library.kind, Format::Native, &file).unwrap().library,
        expected
    );
}

#[test]
fn preset_libraries_decode_supported_payloads() {
    preset_swatches_decode_independently_generated_formats();

    let mut shc = b"8BFS\0\x01\0\0\0\x01".to_vec();
    number(&mut shc, 2);
    wide(&mut shc, "Curve", false);
    word(&mut shc, 3);
    for v in [0, 0, 170, 125, 255, 255] {
        word(&mut shc, v);
    }
    shc.extend([1, 0, 1]);
    number(&mut shc, 0);
    number(&mut shc, 255);
    let decoded = decode(Kind::Contours, Format::Shc, &shc).unwrap();
    assert_native_fidelity(&decoded.library);
    assert_eq!(decoded.library.entries[0].name, "Curve");
    let Payload::Contour(contour) = &decoded.library.entries[0].payload else {
        panic!("missing contour")
    };
    assert_eq!(
        (
            contour.points[1].input,
            contour.points[1].output,
            contour.points[1].corner
        ),
        (125, 170, true)
    );

    let mut shape = b"cush\0\0\0\x02\0\0\0\x01".to_vec();
    wide(&mut shape, "Knot", false);
    word(&mut shape, 0);
    number(&mut shape, 1);
    let mut shape_body = vec![4, b't', b'e', b's', b't'];
    for v in [1, 2, 21, 42] {
        number(&mut shape_body, v);
    }
    word(&mut shape_body, 0);
    word(&mut shape_body, 1);
    shape_body.extend([0; 22]);
    word(&mut shape_body, 1);
    for v in [0, 0x800000, 0x800000, 0x800000, 0x1000000, 0x800000] {
        number(&mut shape_body, v);
    }
    shape_body.resize(shape_body.len().next_multiple_of(4), 0);
    number(&mut shape, shape_body.len() as u32);
    shape.extend(shape_body);
    let decoded = decode(Kind::CustomShapes, Format::Csh, &shape).unwrap();
    assert_native_fidelity(&decoded.library);
    let Payload::Shape(shape) = &decoded.library.entries[0].payload else {
        panic!("missing shape")
    };
    assert_eq!(shape.bounds, [1, 2, 21, 42]);
    assert_eq!(shape.paths().unwrap()[0].points[0].anchor, (20.0, 10.0));

    let tile = [15, 23, 37, 255, 48, 53, 75, 255];
    let pattern = pictura_codec::encode_rgb_pattern("Tile", "tile-id", 2, 1, &tile).unwrap();
    let mut pat = b"8BPT\0\x01\0\0\0\x01".to_vec();
    pat.extend(pattern);
    let decoded = decode(Kind::Patterns, Format::Pat, &pat).unwrap();
    assert_native_fidelity(&decoded.library);
    assert_eq!(decoded.library.entries[0].name, "Tile");
    let Payload::Pattern(pattern) = &decoded.library.entries[0].payload else {
        panic!("missing pattern")
    };
    assert_eq!(pattern.rgba, tile);

    let mut style = b"\0\x028BSL\0\x03\0\0\0\0\0\0\0\x01".to_vec();
    let mut record = Vec::new();
    descriptor(
        &mut record,
        &object(
            *b"Styl",
            vec![(*b"Nm  ", ActionValue::Text("Shadow".into()))],
        ),
    );
    let effect = object(*b"Lefx", vec![(*b"Scl ", ActionValue::Double(47.0))]);
    descriptor(
        &mut record,
        &object(
            *b"Styl",
            vec![(*b"Lefx", ActionValue::Descriptor(Box::new(effect.clone())))],
        ),
    );
    number(&mut style, record.len() as u32);
    style.extend(record);
    let decoded = decode(Kind::Styles, Format::Asl, &style).unwrap();
    assert_native_fidelity(&decoded.library);
    assert_eq!(decoded.library.entries[0].name, "Shadow");
    assert_eq!(decoded.library.entries[0].payload, Payload::Style(effect));

    let mut tpl = b"8BTP\0\0\0\x03\0\0\0\x018BIMtptp".to_vec();
    let mut record = vec![0, 0, 0, 1];
    wide(&mut record, "Tool", false);
    let mut tool = object(*b"Brsh", vec![(*b"Opct", ActionValue::Integer(42))]);
    tool.class.id = ActionKey::String(b"paintbrushTool".to_vec());
    descriptor(&mut record, &tool);
    number(&mut tpl, record.len() as u32);
    tpl.extend(record);
    let decoded = decode(Kind::Tools, Format::Tpl, &tpl).unwrap();
    assert_native_fidelity(&decoded.library);
    assert_eq!(decoded.library.entries[0].name, "Tool");
    assert_eq!(decoded.library.entries[0].payload, Payload::Tool(tool));

    let stop = object(
        *b"Clrt",
        vec![
            (*b"Lctn", ActionValue::Integer(1024)),
            (*b"Mdpn", ActionValue::Integer(65)),
            (
                *b"Type",
                ActionValue::Enumerated {
                    kind: ActionKey::Code(*b"Clry"),
                    value: ActionKey::Code(*b"FrgC"),
                },
            ),
        ],
    );
    let opacity = object(
        *b"TrnS",
        vec![
            (*b"Lctn", ActionValue::Integer(0)),
            (*b"Opct", ActionValue::Double(70.0)),
        ],
    );
    let gradient = object(
        *b"Grdn",
        vec![
            (*b"Nm  ", ActionValue::Text("Ramp".into())),
            (
                *b"Clrs",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(stop))]),
            ),
            (
                *b"Trns",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(opacity))]),
            ),
        ],
    );
    let nested = object(
        *b"Grdn",
        vec![(
            *b"Grad",
            ActionValue::Descriptor(Box::new(gradient.clone())),
        )],
    );
    let mut grd = b"8BGR\0\x05".to_vec();
    descriptor(
        &mut grd,
        &object(
            *b"null",
            vec![(
                *b"GrdL",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(nested))]),
            )],
        ),
    );
    let decoded = decode(Kind::Gradients, Format::Grd, &grd).unwrap();
    assert_native_fidelity(&decoded.library);
    assert_eq!(decoded.library.entries[0].name, "Ramp");
    assert_eq!(
        decoded.library.entries[0].payload,
        Payload::Gradient(gradient)
    );

    let mut abr = vec![0, 10, 0, 1];
    let mut sample = vec![3, b't', b'i', b'p'];
    sample.extend([0; 10]);
    for value in [0, 0, 2, 2] {
        number(&mut sample, value);
    }
    word(&mut sample, 8);
    sample.extend([0, 0, 31, 200, 255]);
    let mut samples = Vec::new();
    number(&mut samples, sample.len() as u32);
    samples.extend(sample);
    samples.resize(samples.len().next_multiple_of(4), 0);
    tagged(&mut abr, b"samp", &samples);
    let mut tip = object(*b"samp", vec![(*b"Dmtr", ActionValue::Integer(32))]);
    tip.items.push((
        ActionKey::String(b"sampledData".to_vec()),
        ActionValue::Text("tip".into()),
    ));
    let brush = object(
        *b"Brsh",
        vec![
            (*b"Nm  ", ActionValue::Text("Sampled".into())),
            (*b"Brsh", ActionValue::Descriptor(Box::new(tip))),
        ],
    );
    let mut desc = Vec::new();
    descriptor(
        &mut desc,
        &object(
            *b"null",
            vec![(
                *b"Brsh",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(brush.clone()))]),
            )],
        ),
    );
    tagged(&mut abr, b"desc", &desc);
    let decoded = decode(Kind::Brushes, Format::Abr, &abr).unwrap();
    assert_native_fidelity(&decoded.library);
    let entry = &decoded.library.entries[0];
    assert_eq!(entry.name, "Sampled");
    let Payload::Brush(sample) = &entry.payload else {
        panic!("missing brush")
    };
    assert_eq!(sample.coverage, [0, 31, 200, 255]);
    assert_eq!(sample.settings, brush);
}

fn tagged(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    number(out, data.len() as u32);
    out.extend_from_slice(data);
    out.extend(std::iter::repeat_n(0, (4 - data.len() % 4) % 4));
}

#[test]
fn preset_libraries_report_partial_import() {
    let mut file = Vec::new();
    aco_colors(
        &mut file,
        2,
        &[
            (0, [0, 0, 0, 0], "First"),
            (99, [0, 0, 0, 0], "Unsupported"),
            (0, [65535, 0, 0, 0], "Last"),
        ],
    );
    let report = decode(Kind::Swatches, Format::Aco, &file).unwrap();
    assert_eq!(
        report.library.entries,
        [
            swatch("First", Color::Rgb([0.0, 0.0, 0.0])),
            swatch("Last", Color::Rgb([1.0, 0.0, 0.0])),
        ]
    );
    assert_eq!(report.diagnostics.len(), 1);
    assert!(report.diagnostics[0].message.contains("Unsupported"));

    let unsupported = object(
        *b"Clrt",
        vec![
            (*b"Lctn", ActionValue::Integer(0)),
            (
                *b"Type",
                ActionValue::Enumerated {
                    kind: ActionKey::Code(*b"Clry"),
                    value: ActionKey::Code(*b"UsrS"),
                },
            ),
            (
                *b"Clr ",
                ActionValue::Descriptor(Box::new(object(*b"CMYC", vec![]))),
            ),
        ],
    );
    let valid = object(
        *b"Clrt",
        vec![
            (*b"Lctn", ActionValue::Integer(4096)),
            (
                *b"Type",
                ActionValue::Enumerated {
                    kind: ActionKey::Code(*b"Clry"),
                    value: ActionKey::Code(*b"FrgC"),
                },
            ),
        ],
    );
    let mixed = object(
        *b"Grdn",
        vec![
            (
                *b"Clrs",
                ActionValue::List(vec![
                    ActionValue::Descriptor(Box::new(unsupported)),
                    ActionValue::Descriptor(Box::new(valid)),
                ]),
            ),
            (*b"Trns", ActionValue::List(vec![])),
        ],
    );
    let nested = object(
        *b"Grdn",
        vec![(*b"Grad", ActionValue::Descriptor(Box::new(mixed)))],
    );
    let mut grd = b"8BGR\0\x05".to_vec();
    descriptor(
        &mut grd,
        &object(
            *b"null",
            vec![(
                *b"GrdL",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(nested))]),
            )],
        ),
    );
    let mixed = decode(Kind::Gradients, Format::Grd, &grd).unwrap();
    assert_eq!(mixed.library.entries.len(), 1);
    assert!(!mixed.diagnostics.is_empty());

    let empty = object(
        *b"Grdn",
        vec![
            (*b"Clrs", ActionValue::List(vec![])),
            (*b"Trns", ActionValue::List(vec![])),
        ],
    );
    let nested = object(
        *b"Grdn",
        vec![(*b"Grad", ActionValue::Descriptor(Box::new(empty)))],
    );
    let mut grd = b"8BGR\0\x05".to_vec();
    descriptor(
        &mut grd,
        &object(
            *b"null",
            vec![(
                *b"GrdL",
                ActionValue::List(vec![ActionValue::Descriptor(Box::new(nested))]),
            )],
        ),
    );
    let empty = decode(Kind::Gradients, Format::Grd, &grd).unwrap();
    assert!(empty.library.entries.is_empty());
    assert!(!empty.diagnostics.is_empty());

    let mut tpl = b"8BTP\0\0\0\x03\0\0\0\x01".to_vec();
    let mut tools = Vec::new();
    number(&mut tools, 3);
    let mut supported = object(*b"Brsh", vec![(*b"Opct", ActionValue::Integer(37))]);
    supported.class.id = ActionKey::String(b"paintbrushTool".to_vec());
    wide(&mut tools, "Usable", false);
    descriptor(&mut tools, &supported);
    wide(&mut tools, "Bad descriptor", false);
    number(&mut tools, 15);
    wide(&mut tools, "Unsafe to resume", false);
    descriptor(&mut tools, &supported);
    tagged(&mut tpl, b"tptp", &tools);
    let report = decode(Kind::Tools, Format::Tpl, &tpl).unwrap();
    assert_eq!(report.library.entries.len(), 1);
    assert_eq!(report.library.entries[0].name, "Usable");
    assert_eq!(report.diagnostics.len(), 1);
}

#[test]
fn preset_libraries_reject_invalid_input() {
    let bad: &[(Format, &[u8])] = &[
        (Format::Aco, b"\x00\x01\xff\xff"),
        (Format::Aco, b"\x00\x03\x00\x00"),
        (Format::Ase, b"ASEF\x00\x01\x00\x00\xff\xff\xff\xff"),
        (
            Format::Ase,
            b"ASEF\x00\x01\x00\x00\x00\x00\x00\x01\x00\x01\xff\xff\xff\xff",
        ),
        (Format::Native, b"KPPRESET\x00\x01\x01\xff\xff\xff\xff"),
    ];
    for (format, bytes) in bad {
        assert!(decode(Kind::Swatches, *format, bytes).is_err());
    }
    for (kind, format, header) in [
        (Kind::Brushes, Format::Abr, &b"\0\x0a\0\x01bad!"[..]),
        (Kind::Gradients, Format::Grd, &b"8BGR\0\x05\0\0\0\x10"[..]),
        (
            Kind::Styles,
            Format::Asl,
            &b"\0\x028BSL\0\x03\xff\xff\xff\xff"[..],
        ),
        (
            Kind::Patterns,
            Format::Pat,
            &b"8BPT\0\x01\xff\xff\xff\xff"[..],
        ),
        (
            Kind::Contours,
            Format::Shc,
            &b"8BFS\0\x01\xff\xff\xff\xff"[..],
        ),
        (
            Kind::CustomShapes,
            Format::Csh,
            &b"cush\0\0\0\x02\xff\xff\xff\xff"[..],
        ),
        (
            Kind::Tools,
            Format::Tpl,
            &b"8BTP\0\0\0\x03\xff\xff\xff\xff"[..],
        ),
    ] {
        assert!(decode(kind, format, header).is_err(), "{format:?}");
    }
    assert!(decode(Kind::Gradients, Format::Aco, &[0, 1, 0, 0]).is_err());
}

#[test]
fn preset_native_sets_round_trip() {
    for kind in Kind::ALL {
        let entries = if kind == Kind::Swatches {
            vec![
                swatch("正", Color::Gray(0.35)),
                swatch("正", Color::Rgb([0.0, 0.5, 1.0])),
            ]
        } else {
            vec![
                Preset {
                    name: "正".into(),
                    payload: Payload::BuiltIn(5),
                    opaque: vec![1, 0, 255],
                },
                Preset {
                    name: "正".into(),
                    payload: Payload::BuiltIn(0),
                    opaque: Vec::new(),
                },
            ]
        };
        for entries in [entries, Vec::new()] {
            let library = Library { kind, entries };
            let bytes = encode_native(&library).unwrap();
            assert_eq!(
                decode(kind, Format::Native, &bytes).unwrap().library,
                library
            );
        }
    }
}

#[test]
fn preset_swatches_round_trip_exchange() {
    let library = Library {
        kind: Kind::Swatches,
        entries: vec![
            swatch("红", Color::Rgb([0.17, 0.98, 0.323])),
            swatch("Ink", Color::Cmyk([0.42, 0.98, 0.01, 0.165])),
            swatch("Lab", Color::Lab([50.234, -50.887, 12.125])),
            swatch("Gray", Color::Gray(0.25321)),
        ],
    };
    for format in [Format::Aco, Format::Ase] {
        let file = encode_swatches(format, &library).unwrap();
        let output = decode(Kind::Swatches, format, &file).unwrap().library;
        assert_eq!(output.entries.len(), library.entries.len());
        for (source, decoded) in library.entries.iter().zip(output.entries) {
            assert_eq!(source.name, decoded.name);
            let (Payload::Swatch(source), Payload::Swatch(decoded)) =
                (&source.payload, decoded.payload)
            else {
                unreachable!()
            };
            assert_eq!(source.role, decoded.role);
            let (before, after, aco_tolerance) = match (source.color, decoded.color) {
                (Color::Rgb(a), Color::Rgb(b)) => (a.to_vec(), b.to_vec(), 1.0 / 65535.0),
                (Color::Cmyk(a), Color::Cmyk(b)) => (a.to_vec(), b.to_vec(), 1.0 / 65535.0),
                (Color::Lab(a), Color::Lab(b)) => (a.to_vec(), b.to_vec(), 0.01),
                (Color::Gray(a), Color::Gray(b)) => (vec![a], vec![b], 0.0001),
                _ => panic!("swatch color model changed during exchange"),
            };
            let tolerance = if format == Format::Aco {
                aco_tolerance
            } else {
                0.00001
            };
            for (before, after) in before.into_iter().zip(after) {
                assert!(
                    (before - after).abs() <= tolerance,
                    "{format:?}: {before} != {after}"
                );
            }
        }
    }
    let mut invalid = library;
    let Payload::Swatch(swatch) = &mut invalid.entries[0].payload else {
        unreachable!()
    };
    swatch.alpha = 0.5;
    assert!(encode_swatches(Format::Aco, &invalid).is_err());
    assert!(encode_swatches(Format::Ase, &invalid).is_err());
}

#[test]
fn native_tool_preset_preserves_opaque_options() {
    let mut tool = object(
        *b"Brsh",
        vec![
            (*b"Opct", ActionValue::Integer(37)),
            (*b"Flw ", ActionValue::Integer(58)),
        ],
    );
    tool.class.id = ActionKey::String(b"move".to_vec());
    let options = br#"{"move":{"autoSelect":true},"type":{"size":42}}"#.to_vec();
    let library = Library {
        kind: Kind::Tools,
        entries: vec![Preset {
            name: "Move preset".into(),
            payload: Payload::Tool(tool),
            opaque: options.clone(),
        }],
    };
    let bytes = encode_native(&library).unwrap();
    let decoded = decode(Kind::Tools, Format::Native, &bytes).unwrap().library;
    assert_eq!(decoded, library);
    assert_eq!(decoded.entries[0].opaque, options);
}

#[test]
fn native_tool_preset_keeps_unknown_opaque_bytes() {
    let mut tool = object(*b"Brsh", vec![]);
    tool.class.id = ActionKey::String(b"brush".to_vec());
    let opaque = vec![0xff, 0x00, 0x7f, b'{'];
    let library = Library {
        kind: Kind::Tools,
        entries: vec![Preset {
            name: "Opaque".into(),
            payload: Payload::Tool(tool),
            opaque: opaque.clone(),
        }],
    };
    let first = encode_native(&library).unwrap();
    let decoded = decode(Kind::Tools, Format::Native, &first).unwrap().library;
    assert_eq!(decoded.entries[0].opaque, opaque);
    let second = encode_native(&decoded).unwrap();
    assert_eq!(
        decode(Kind::Tools, Format::Native, &second)
            .unwrap()
            .library
            .entries[0]
            .opaque,
        opaque
    );
}
