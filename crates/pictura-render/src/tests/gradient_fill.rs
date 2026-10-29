use super::*;
use pictura_adjust::{GradientFillParams, GradientKind, GradientStop};
use pictura_codec::{write_descriptor, DescValue};

fn desc_object(name: &str, class_id: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: name.to_string(),
        class_id: class_id.to_vec(),
        items,
    }
}

fn bw_stops() -> [GradientStop; 2] {
    [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ]
}

fn kind_bytes(kind: GradientKind) -> &'static [u8] {
    match kind {
        GradientKind::Linear => b"Lnr ",
        GradientKind::Radial => b"Rdl ",
        GradientKind::Angle => b"Angl",
        GradientKind::Reflected => b"Rflc",
        GradientKind::Diamond => b"Dmnd",
    }
}

fn clr_stop(location: f64, rgb: [f64; 3]) -> DescValue {
    desc_object(
        "",
        b"Clrt",
        vec![
            (
                b"Clr ".to_vec(),
                desc_object(
                    "",
                    b"RGBC",
                    vec![
                        (b"Rd  ".to_vec(), DescValue::Double(rgb[0])),
                        (b"Grn ".to_vec(), DescValue::Double(rgb[1])),
                        (b"Bl  ".to_vec(), DescValue::Double(rgb[2])),
                    ],
                ),
            ),
            (b"Lctn".to_vec(), DescValue::Double(location)),
        ],
    )
}

fn gradient_fill_data(
    kind: GradientKind,
    stops: &[GradientStop],
    angle: f64,
    reverse: bool,
    scale: f64,
) -> AdjustmentData {
    let grad = desc_object(
        "",
        b"Grdn",
        vec![
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (
                b"Clrs".to_vec(),
                DescValue::List(
                    stops
                        .iter()
                        .map(|s| {
                            clr_stop(
                                s.location as f64,
                                [s.color[0] as f64, s.color[1] as f64, s.color[2] as f64],
                            )
                        })
                        .collect(),
                ),
            ),
        ],
    );
    let mut items = vec![
        (b"Angl".to_vec(), DescValue::Double(angle)),
        (
            b"Type".to_vec(),
            DescValue::Enum {
                kind: b"GrdT".to_vec(),
                value: kind_bytes(kind).to_vec(),
            },
        ),
        (b"Grad".to_vec(), grad),
    ];
    if reverse {
        items.push((b"Rvrs".to_vec(), DescValue::Bool(true)));
    }
    items.push((b"Scl ".to_vec(), DescValue::Double(scale)));
    adjdata(*b"GdFl", write_descriptor(&desc_object("", b"GdFl", items)))
}

fn gradient_layer(params: &GradientFillParams, w: u32, h: u32) -> Layer {
    let mut layer = adjustment_layer(
        "gradient",
        *b"GdFl",
        gradient_fill_data(
            params.kind,
            &params.stops,
            params.angle_deg as f64,
            params.reverse,
            params.scale as f64,
        )
        .data
        .to_vec(),
        255,
        None,
    );
    layer.rect = full(w, h);
    layer
}

#[test]
fn gradient_fill_decodes_all_kinds() {
    for kind in [
        GradientKind::Linear,
        GradientKind::Radial,
        GradientKind::Angle,
        GradientKind::Reflected,
        GradientKind::Diamond,
    ] {
        let decoded = decode_adjustment(&encode_gradient_fill(kind, &bw_stops(), 0.0));
        let Some(Adjustment::GradientFill(params)) = decoded else {
            panic!("{kind:?} did not decode to GradientFill");
        };
        assert_eq!(params.kind, kind);
        assert_eq!(params.stops, bw_stops().to_vec());
        assert_eq!(params.angle_deg, 0.0);
        assert!(!params.reverse);
        assert_eq!(params.scale, 100.0);
    }
}

#[test]
fn encoded_gradient_fill_is_version16_and_round_trips() {
    let stops = bw_stops();
    let encoded = encode_gradient_fill(GradientKind::Linear, &stops, 12.5);
    assert_eq!(encoded.key, *b"GdFl");
    assert_eq!(&encoded.data[0..4], &[0, 0, 0, 16], "version-16 header");
    assert_eq!(
        decode_adjustment(&encoded),
        Some(Adjustment::GradientFill(GradientFillParams {
            stops: stops.to_vec(),
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 12.5,
            scale: 100.0,
        }))
    );

    let desc = pictura_codec::read_descriptor(&encoded.data).expect("version-16 descriptor");
    let DescValue::Object { items, .. } = &desc else {
        panic!("top level is an object");
    };
    let type_item = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Type")
        .map(|(_, v)| v)
        .expect("Type item");
    assert_eq!(
        type_item,
        &DescValue::Enum {
            kind: b"GrdT".to_vec(),
            value: b"Lnr ".to_vec(),
        }
    );
    let grad = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Grad")
        .map(|(_, v)| v)
        .expect("Grad item");
    let DescValue::Object {
        items: grad_items, ..
    } = grad
    else {
        panic!("Grad is an object");
    };
    let DescValue::List(clrs) = grad_items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Clrs")
        .map(|(_, v)| v)
        .expect("Clrs item")
    else {
        panic!("Clrs is a list");
    };
    assert_eq!(clrs.len(), 2);
}

fn drop_key(v: &mut DescValue, key: &[u8]) {
    match v {
        DescValue::Object { items, .. } => {
            items.retain(|(k, _)| k.as_slice() != key);
            for (_, val) in items.iter_mut() {
                drop_key(val, key);
            }
        }
        DescValue::List(items) => items.iter_mut().for_each(|val| drop_key(val, key)),
        _ => {}
    }
}

fn replace_key(v: &mut DescValue, key: &[u8], new: &DescValue) -> bool {
    match v {
        DescValue::Object { items, .. } => {
            for (k, val) in items.iter_mut() {
                if k.as_slice() == key {
                    *val = new.clone();
                    return true;
                }
            }
            items.iter_mut().any(|(_, val)| replace_key(val, key, new))
        }
        DescValue::List(items) => items.iter_mut().any(|val| replace_key(val, key, new)),
        _ => false,
    }
}

#[test]
fn gradient_fill_decode_rejects_malformed() {
    // Colour-noise gradients (ClNs) are a no-op.
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"GdFl",
            write_descriptor(&desc_object(
                "",
                b"GdFl",
                vec![
                    (b"Angl".to_vec(), DescValue::Double(0.0)),
                    (
                        b"Type".to_vec(),
                        DescValue::Enum {
                            kind: b"GrdT".to_vec(),
                            value: b"Lnr ".to_vec(),
                        },
                    ),
                    (
                        b"Grad".to_vec(),
                        desc_object(
                            "",
                            b"Grdn",
                            vec![
                                (
                                    b"GrdF".to_vec(),
                                    DescValue::Enum {
                                        kind: b"GrdF".to_vec(),
                                        value: b"ClNs".to_vec(),
                                    },
                                ),
                                (
                                    b"Clrs".to_vec(),
                                    DescValue::List(vec![
                                        clr_stop(0.0, [0.0, 0.0, 0.0]),
                                        clr_stop(4096.0, [255.0, 255.0, 255.0]),
                                    ]),
                                ),
                            ],
                        ),
                    ),
                ],
            )),
        )),
        None
    );
    // Missing Grad.
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"GdFl",
            write_descriptor(&desc_object(
                "",
                b"GdFl",
                vec![
                    (b"Angl".to_vec(), DescValue::Double(0.0)),
                    (
                        b"Type".to_vec(),
                        DescValue::Enum {
                            kind: b"GrdT".to_vec(),
                            value: b"Lnr ".to_vec(),
                        },
                    ),
                ],
            )),
        )),
        None
    );
    // A single stop and non-increasing / out-of-range locations.
    let one = gradient_fill_data(GradientKind::Linear, &bw_stops()[..1], 0.0, false, 100.0);
    assert_eq!(decode_adjustment(&one), None);
    let decreasing = [
        GradientStop {
            location: 4096,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 0,
            color: [255, 255, 255],
        },
    ];
    assert_eq!(
        decode_adjustment(&gradient_fill_data(
            GradientKind::Linear,
            &decreasing,
            0.0,
            false,
            100.0
        )),
        None
    );
    let out_of_range = [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 5000,
            color: [255, 255, 255],
        },
    ];
    assert_eq!(
        decode_adjustment(&gradient_fill_data(
            GradientKind::Linear,
            &out_of_range,
            0.0,
            false,
            100.0
        )),
        None
    );
    // Non-finite angle.
    let mut bad_angle = gradient_fill_data(GradientKind::Linear, &bw_stops(), 0.0, false, 100.0);
    let DescValue::Object { items, .. } =
        pictura_codec::read_descriptor(&bad_angle.data).expect("descriptor")
    else {
        panic!("object");
    };
    let mut items = items;
    for (k, v) in &mut items {
        if k.as_slice() == b"Angl" {
            *v = DescValue::Double(f64::NAN);
        }
    }
    bad_angle.data = write_descriptor(&desc_object("", b"GdFl", items)).into();
    assert_eq!(decode_adjustment(&bad_angle), None);

    // A payload that is not a descriptor at all.
    assert_eq!(
        decode_adjustment(&adjdata(*b"GdFl", vec![0, 1, 2, 3, 4, 5])),
        None
    );
    // A truncated descriptor.
    let mut truncated = gradient_fill_data(GradientKind::Linear, &bw_stops(), 0.0, false, 100.0);
    truncated.data.truncate(truncated.data.len() / 2);
    assert_eq!(decode_adjustment(&truncated), None);
    // A descriptor missing Angl, Type, or the nested Clrs list.
    for key in [b"Angl".as_slice(), b"Type".as_slice(), b"Clrs".as_slice()] {
        let mut missing = gradient_fill_data(GradientKind::Linear, &bw_stops(), 0.0, false, 100.0);
        let mut obj = pictura_codec::read_descriptor(&missing.data).expect("descriptor");
        drop_key(&mut obj, key);
        missing.data = write_descriptor(&obj).into();
        assert_eq!(decode_adjustment(&missing), None, "missing {key:?}");
    }
    // A non-finite stop component.
    let mut bad_red = gradient_fill_data(GradientKind::Linear, &bw_stops(), 0.0, false, 100.0);
    let mut obj = pictura_codec::read_descriptor(&bad_red.data).expect("descriptor");
    assert!(replace_key(&mut obj, b"Rd  ", &DescValue::Double(f64::NAN)));
    bad_red.data = write_descriptor(&obj).into();
    assert_eq!(decode_adjustment(&bad_red), None);
}

#[test]
fn gradient_fill_reads_optional_reverse_and_scale() {
    let reversed = gradient_fill_data(GradientKind::Radial, &bw_stops(), 45.0, true, 50.0);
    let Some(Adjustment::GradientFill(params)) = decode_adjustment(&reversed) else {
        panic!("decodes");
    };
    assert!(params.reverse);
    assert_eq!(params.scale, 50.0);
    assert_eq!(params.kind, GradientKind::Radial);
    assert_eq!(params.angle_deg, 45.0);
}

#[test]
fn gradient_fill_layer_composites_ramp_and_differs() {
    let backdrop = solid(
        "base",
        full(8, 1),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(8, 1, vec![backdrop.clone()]));
    let params = GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse: false,
        kind: GradientKind::Linear,
        angle_deg: 0.0,
        scale: 100.0,
    };
    let out = composite_rgba(&doc(8, 1, vec![backdrop, gradient_layer(&params, 8, 1)]));
    assert_ne!(out.data, plain.data, "the fill must change the composite");
    assert!(rgb(&out, 0, 0)[0] < 8, "left edge is black");
    assert!(rgb(&out, 7, 0)[0] > 247, "right edge is white");
    assert_eq!(px(&out, 3, 0)[3], 255, "gradient output is opaque");
    let mut previous = 0u8;
    for x in 0..8 {
        let v = rgb(&out, x, 0)[0];
        assert!(v >= previous, "ramp must be non-decreasing at {x}: {v}");
        previous = v;
    }
}

#[test]
fn gradient_fill_reverse_flips_ramp() {
    let params = |reverse| GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse,
        kind: GradientKind::Linear,
        angle_deg: 0.0,
        scale: 100.0,
    };
    let forward = composite_rgba(&doc(8, 1, vec![gradient_layer(&params(false), 8, 1)]));
    let backward = composite_rgba(&doc(8, 1, vec![gradient_layer(&params(true), 8, 1)]));
    assert!(rgb(&forward, 0, 0)[0] < rgb(&forward, 7, 0)[0]);
    assert!(rgb(&backward, 0, 0)[0] > rgb(&backward, 7, 0)[0]);
}

#[test]
fn gradient_fill_masked_out_is_noop() {
    let backdrop = solid(
        "base",
        full(2, 2),
        (40, 80, 120),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 2, vec![backdrop.clone()]));
    let params = GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse: false,
        kind: GradientKind::Linear,
        angle_deg: 0.0,
        scale: 100.0,
    };
    let mut hidden = gradient_layer(&params, 2, 2);
    hidden.mask = Some(LayerMask {
        rect: full(2, 2),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![0, 0, 0, 0].into()),
        ..Default::default()
    });
    let out = composite_rgba(&doc(2, 2, vec![backdrop, hidden]));
    assert_eq!(out.data, plain.data, "a masked-out fill is a no-op");
}

#[test]
fn every_gradient_fill_kind_renders() {
    for kind in [
        GradientKind::Linear,
        GradientKind::Radial,
        GradientKind::Angle,
        GradientKind::Reflected,
        GradientKind::Diamond,
    ] {
        let params = GradientFillParams {
            stops: bw_stops().to_vec(),
            reverse: false,
            kind,
            angle_deg: 30.0,
            scale: 100.0,
        };
        let out = composite_rgba(&doc(8, 8, vec![gradient_layer(&params, 8, 8)]));
        let distinct = out
            .data
            .chunks(64)
            .next()
            .map(|plane| plane.iter().any(|&v| v != plane[0]))
            .unwrap_or(false);
        assert!(distinct, "{kind:?} must produce a non-empty gradient");
    }
}

#[test]
fn fixture_gradient_fill_decodes_params_and_ramp() {
    let bytes = include_bytes!("../../../pictura-codec/tests/fixtures/gradient_fill.psd");
    let d = pictura_codec::read_psd(bytes).expect("fixture parses");
    let fill = d
        .layers
        .iter()
        .find(|l| l.name == "Gradient Fill")
        .expect("Gradient Fill layer");
    assert_eq!(
        decode_adjustment(fill.adjustment.as_ref().expect("GdFl block")),
        Some(Adjustment::GradientFill(GradientFillParams {
            stops: bw_stops().to_vec(),
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        }))
    );

    // psd-tools' `composite()` ramp is `0,36,72,109,145,182,218,255` (it
    // truncates); our `to_u8` rounds, so allow 1/255 on the interior samples and
    // pin the endpoints exactly.
    let out = composite_rgba(&d);
    let psd_tools = [0u8, 36, 72, 109, 145, 182, 218, 255];
    assert_eq!(rgb(&out, 0, 0)[0], psd_tools[0], "left endpoint");
    assert_eq!(rgb(&out, 7, 0)[0], psd_tools[7], "right endpoint");
    for (x, expected) in psd_tools.iter().enumerate() {
        assert!(
            rgb(&out, x as u32, 0)[0].abs_diff(*expected) <= 1,
            "ramp sample {x}: got {} want {expected}",
            rgb(&out, x as u32, 0)[0]
        );
    }
}

fn has_descriptor_item(data: &[u8], key: &[u8]) -> bool {
    let Ok(DescValue::Object { items, .. }) = pictura_codec::read_descriptor(data) else {
        return false;
    };
    items.iter().any(|(item_key, _)| item_key.as_slice() == key)
}

#[test]
fn encode_gradient_fill_params_round_trips() {
    let source = GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse: true,
        kind: GradientKind::Radial,
        angle_deg: 45.0,
        scale: 60.0,
    };
    let data = crate::encode_gradient_fill_params(&source);
    assert_eq!(
        decode_adjustment(&data),
        Some(Adjustment::GradientFill(source))
    );
}

#[test]
fn encode_gradient_fill_params_omits_defaults() {
    let source = GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse: false,
        kind: GradientKind::Linear,
        angle_deg: 0.0,
        scale: 100.0,
    };
    let data = crate::encode_gradient_fill_params(&source);
    assert!(!has_descriptor_item(&data.data, b"Rvrs"));
    assert!(!has_descriptor_item(&data.data, b"Scl "));
    assert_eq!(
        decode_adjustment(&data),
        Some(Adjustment::GradientFill(source))
    );
}

#[test]
fn gradient_region_matches_the_full_buffer_slice() {
    let params = GradientFillParams {
        stops: bw_stops().to_vec(),
        reverse: false,
        kind: GradientKind::Linear,
        angle_deg: 33.0,
        scale: 100.0,
    };
    let full = crate::fill::gradient_rgba(40, 30, &params);
    let region = (5, 7, 25, 20);
    let part = crate::fill::gradient_rgba_region(40, 30, region, &params);
    let sw = (region.2 - region.0) as usize;
    for y in region.1..region.3 {
        for x in region.0..region.2 {
            let fi = y as usize * 40 + x as usize;
            let pi = (y - region.1) as usize * sw + (x - region.0) as usize;
            assert_eq!(part[pi], full[fi], "({x},{y})");
        }
    }
}
