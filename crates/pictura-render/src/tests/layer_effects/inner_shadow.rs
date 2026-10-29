use super::*;

fn irsh(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"IrSh".to_vec(),
        items,
    }
}

fn inner_top_with(shadow: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (b"IrSh".to_vec(), shadow),
        ],
    }
}

/// A shadowed 8x8 pixel layer at `(2, 2)` on a 12x12 canvas, leaving margin on
/// every side so offsets and chokes have room.
fn inner_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Inner",
        rect(2, 2, 10, 10),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    if let Some(block) = block {
        layer.extra_blocks = vec![block];
    }
    layer
}

#[derive(Clone)]
struct InnerSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    angle: f64,
    distance: f64,
    choke: f64,
    size: f64,
    knocks_out: bool,
}

impl Default for InnerSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 0.0],
            opacity: 100.0,
            angle: 0.0,
            distance: 2.0,
            choke: 0.0,
            size: 0.0,
            knocks_out: true,
        }
    }
}

fn spec_block(spec: &InnerSpec) -> LayerBlock {
    lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
        (b"present".to_vec(), DescValue::Bool(spec.present)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnM".to_vec(),
                value: spec.blend.clone(),
            },
        ),
        (
            b"Clr ".to_vec(),
            rgbc(spec.color[0], spec.color[1], spec.color[2]),
        ),
        (b"Opct".to_vec(), unit(spec.opacity, PRC)),
        (b"uglg".to_vec(), DescValue::Bool(false)),
        (b"lagl".to_vec(), unit(spec.angle, ANG)),
        (b"Dstn".to_vec(), unit(spec.distance, PXL)),
        (b"Ckmt".to_vec(), unit(spec.choke, PRC)),
        (b"blur".to_vec(), unit(spec.size, PXL)),
        (b"layerConceals".to_vec(), DescValue::Bool(spec.knocks_out)),
    ])))
}

fn spec_inner(spec: &InnerSpec) -> Layer {
    inner_layer(Some(spec_block(spec)))
}

fn compose_inner(shadowed: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, shadowed]))
}

fn plain_inner() -> PixelBuffer {
    compose_inner(inner_layer(None))
}

fn content_rect() -> (u32, u32, u32, u32) {
    (2, 2, 10, 10)
}

/// Mask the layer rect: columns left of `split_x` are 0, the rest are 255.
fn with_left_masked(mut layer: Layer, split_x: i32) -> Layer {
    let r = layer.rect;
    let w = r.width().max(0) as usize;
    let h = r.height().max(0) as usize;
    let mut data = vec![255u8; w * h];
    for y in 0..h {
        for x in 0..w {
            if r.left + (x as i32) < split_x {
                data[y * w + x] = 0;
            }
        }
    }
    layer.mask = Some(LayerMask {
        rect: r,
        data: Some(data.into()),
        ..Default::default()
    });
    layer
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn inner_shadow_decodes_typed_parameters() {
    let block = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnM".to_vec(),
                value: b"Mltp".to_vec(),
            },
        ),
        (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
        (b"Opct".to_vec(), unit(60.0, PRC)),
        (b"uglg".to_vec(), DescValue::Bool(false)),
        (b"lagl".to_vec(), unit(45.0, ANG)),
        (b"Dstn".to_vec(), unit(8.0, PXL)),
        (b"Ckmt".to_vec(), unit(20.0, PRC)),
        (b"blur".to_vec(), unit(12.0, PXL)),
        (b"layerConceals".to_vec(), DescValue::Bool(false)),
    ])));
    let shadow = decode_inner_shadow(&inner_layer(Some(block))).expect("decodes");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [200, 100, 50]);
    assert_eq!(shadow.opacity, 60.0);
    assert!(!shadow.use_global_angle);
    assert_eq!(shadow.angle_deg, 45.0);
    assert_eq!(shadow.distance, 8.0);
    assert_eq!(shadow.choke, 20.0);
    assert_eq!(shadow.size, 12.0);
    assert!(!shadow.knocks_out);
}

#[test]
fn inner_shadow_missing_keys_take_defaults() {
    let block = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
    ])));
    let shadow = decode_inner_shadow(&inner_layer(Some(block))).expect("decodes");
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [0, 0, 0]);
    assert_eq!(shadow.opacity, 75.0);
    assert_eq!(shadow.angle_deg, 120.0);
    assert_eq!(shadow.distance, 5.0);
    assert_eq!(shadow.choke, 0.0);
    assert_eq!(shadow.size, 5.0);
    assert!(shadow.use_global_angle);
    assert!(shadow.knocks_out);
}

#[test]
fn inner_shadow_unknown_blend_falls_back_to_multiply() {
    let block = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnM".to_vec(),
                value: b"zzzz".to_vec(),
            },
        ),
    ])));
    assert_eq!(
        decode_inner_shadow(&inner_layer(Some(block)))
            .expect("decodes")
            .blend_mode,
        BlendMode::Multiply
    );
}

#[test]
fn inner_shadow_doub_numeric_decodes() {
    let block = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"Dstn".to_vec(), DescValue::Double(3.0)),
        (b"Ckmt".to_vec(), DescValue::Double(5.0)),
        (b"blur".to_vec(), DescValue::Double(4.0)),
    ])));
    let shadow = decode_inner_shadow(&inner_layer(Some(block))).expect("decodes");
    assert_eq!(shadow.distance, 3.0);
    assert_eq!(shadow.choke, 5.0);
    assert_eq!(shadow.size, 4.0);
}

#[test]
fn malformed_or_absent_inner_shadow_is_none() {
    // No lfx2 block.
    assert!(decode_inner_shadow(&inner_layer(None)).is_none());
    // lfx2 without an IrSh.
    let no_irsh = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_inner_shadow(&inner_layer(Some(no_irsh))).is_none());
    // A key named IrSh whose object class is not IrSh.
    let wrong_class = lfx2(inner_top_with(DescValue::Object {
        name: String::new(),
        class_id: b"DrSh".to_vec(),
        items: vec![(b"enab".to_vec(), DescValue::Bool(true))],
    }));
    assert!(decode_inner_shadow(&inner_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        inner_top_with(irsh(vec![(b"enab".to_vec(), DescValue::Bool(true))])),
        17,
    );
    assert!(decode_inner_shadow(&inner_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_inner_shadow(&inner_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnX".to_vec(),
                value: b"Mltp".to_vec(),
            },
        ),
    ])));
    assert!(decode_inner_shadow(&inner_layer(Some(wrong_kind))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Clr ".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"LbCl".to_vec(),
                items: vec![
                    (b"Rd  ".to_vec(), DescValue::Double(1.0)),
                    (b"Grn ".to_vec(), DescValue::Double(2.0)),
                    (b"Bl  ".to_vec(), DescValue::Double(3.0)),
                ],
            },
        ),
    ])));
    assert!(decode_inner_shadow(&inner_layer(Some(wrong_color))).is_none());
    // Non-finite blur.
    let nan = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"blur".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert!(decode_inner_shadow(&inner_layer(Some(nan))).is_none());
    // Wrong-typed boolean.
    let bad_bool = lfx2(inner_top_with(irsh(vec![(
        b"enab".to_vec(),
        DescValue::Double(1.0),
    )])));
    assert!(decode_inner_shadow(&inner_layer(Some(bad_bool))).is_none());
    // Truncated block.
    let mut short = lfx2(inner_top_with(irsh(vec![])));
    short.data.truncate(6);
    assert!(decode_inner_shadow(&inner_layer(Some(short))).is_none());
}

#[test]
fn huge_inner_parameters_are_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Ckmt", b"blur", b"Dstn", b"Opct"] {
        let block = lfx2(inner_top_with(irsh(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), DescValue::Double(1e300)),
        ])));
        assert!(
            decode_inner_shadow(&inner_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2(inner_top_with(irsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), unit(1e30, PRC)),
        (b"blur".to_vec(), unit(1e30, PXL)),
        (b"Dstn".to_vec(), unit(1e30, PXL)),
        (b"Opct".to_vec(), unit(1e30, PRC)),
    ])));
    let shadow = decode_inner_shadow(&inner_layer(Some(block))).expect("decodes clamped");
    assert_eq!(shadow.choke, 100.0);
    assert_eq!(shadow.size, 250.0);
    assert_eq!(shadow.distance, 30_000.0);
    assert_eq!(shadow.opacity, 100.0);
}

// --- Rendering -------------------------------------------------------------

#[test]
fn inner_shadow_is_interior_and_leaves_exterior_unchanged() {
    let spec = InnerSpec {
        distance: 4.0,
        ..Default::default()
    };
    let with = compose_inner(spec_inner(&spec));
    let plain = plain_inner();
    let (x0, y0, x1, y1) = content_rect();
    for y in 0..12 {
        for x in 0..12 {
            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                continue;
            }
            assert_eq!(px(&with, x, y), px(&plain, x, y), "exterior ({x},{y})");
        }
    }
    assert!(
        rgb(&with, 9, 5)[0] < 200,
        "the interior is darkened: {:?}",
        rgb(&with, 9, 5)
    );
}

#[test]
fn inner_shadow_darkens_the_edge_away_from_the_offset() {
    let spec = InnerSpec {
        distance: 4.0,
        size: 0.0,
        ..Default::default()
    };
    let out = compose_inner(spec_inner(&spec));
    assert!(
        rgb(&out, 9, 5)[0] < 128,
        "angle 0 darkens the right interior: {:?}",
        rgb(&out, 9, 5)
    );
    assert_eq!(
        rgb(&out, 2, 5),
        [255, 0, 0],
        "the left interior is unchanged"
    );
}

#[test]
fn distance_widens_the_inner_band() {
    let changed = |buf: &PixelBuffer| {
        let (x0, y0, x1, y1) = content_rect();
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                if rgb(buf, x, y) != [255, 0, 0] {
                    n += 1;
                }
            }
        }
        n
    };
    let near = compose_inner(spec_inner(&InnerSpec {
        distance: 2.0,
        ..Default::default()
    }));
    let far = compose_inner(spec_inner(&InnerSpec {
        distance: 6.0,
        ..Default::default()
    }));
    assert!(
        changed(&far) > changed(&near),
        "distance 6 covers more of the interior ({} vs {})",
        changed(&far),
        changed(&near)
    );
}

#[test]
fn choke_narrows_the_inner_band() {
    let changed = |buf: &PixelBuffer| {
        let (x0, y0, x1, y1) = content_rect();
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                if rgb(buf, x, y) != [255, 0, 0] {
                    n += 1;
                }
            }
        }
        n
    };
    let plain = compose_inner(spec_inner(&InnerSpec {
        distance: 5.0,
        size: 6.0,
        ..Default::default()
    }));
    let choked = compose_inner(spec_inner(&InnerSpec {
        distance: 5.0,
        size: 6.0,
        choke: 50.0,
        ..Default::default()
    }));
    assert!(
        changed(&plain) > 0,
        "the unchoked shadow covers the interior"
    );
    assert!(
        changed(&choked) < changed(&plain),
        "a positive choke covers strictly less ({}) than none ({})",
        changed(&choked),
        changed(&plain)
    );
}

#[test]
fn size_softens_the_inner_edge() {
    let partial = |buf: &PixelBuffer| {
        let (x0, y0, x1, y1) = content_rect();
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = rgb(buf, x, y);
                if p != [255, 0, 0] && p != [0, 0, 0] {
                    n += 1;
                }
            }
        }
        n
    };
    let hard = compose_inner(spec_inner(&InnerSpec {
        distance: 4.0,
        size: 0.0,
        ..Default::default()
    }));
    let soft = compose_inner(spec_inner(&InnerSpec {
        distance: 4.0,
        size: 6.0,
        ..Default::default()
    }));
    assert!(
        partial(&soft) > partial(&hard),
        "blur widens the transition ({} vs {})",
        partial(&soft),
        partial(&hard)
    );
}

#[test]
fn opacity_and_colour_shape_the_inner_shadow() {
    let white = |spec: &InnerSpec| {
        let mut layer = solid(
            "InnerWhite",
            rect(2, 2, 10, 10),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        layer.extra_blocks = vec![spec_block(spec)];
        compose_inner(layer)
    };
    let spec = InnerSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 50.0,
        distance: 4.0,
        ..Default::default()
    };
    let out = white(&spec);
    let p = rgb(&out, 9, 5);
    assert!(p[0] > p[1] + 20, "a red tint, not black: {p:?}");
    let faint = white(&InnerSpec {
        opacity: 5.0,
        ..spec.clone()
    });
    assert!(
        rgb(&faint, 9, 5) != p,
        "lowering opacity approaches the content"
    );
    let zero = white(&InnerSpec {
        opacity: 0.0,
        ..spec.clone()
    });
    let plain = {
        let mut l = solid(
            "InnerWhite",
            rect(2, 2, 10, 10),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        l.extra_blocks = vec![spec_block(&InnerSpec {
            enabled: false,
            ..spec.clone()
        })];
        compose_inner(l)
    };
    assert_eq!(zero, plain, "opacity 0 writes nothing");
}

#[test]
fn layer_mask_shapes_the_inner_matte() {
    let spec = InnerSpec {
        distance: 4.0,
        ..Default::default()
    };
    // A partial mask: the left columns are masked off, the right columns are
    // fully opaque. The backdrop is visible on the left, so a shadow that
    // ignored the mask would darken it.
    let with = compose_inner(with_left_masked(spec_inner(&spec), 6));
    let plain = compose_inner(with_left_masked(inner_layer(None), 6));
    for y in 2..10 {
        for x in 2..6 {
            assert_eq!(
                px(&with, x, y),
                px(&plain, x, y),
                "the mask hides the shadow at ({x},{y})"
            );
        }
    }
    let mut right_shadowed = false;
    for y in 2..10 {
        for x in 6..10 {
            if px(&with, x, y) != px(&plain, x, y) {
                right_shadowed = true;
            }
        }
    }
    assert!(right_shadowed, "the unmasked right interior is shadowed");
}

#[test]
fn small_layer_inner_shadow_is_confined_to_the_rect() {
    let spec = InnerSpec {
        distance: 5.0,
        size: 250.0,
        ..Default::default()
    };
    let with = compose_sized(32, spec_inner(&spec));
    let plain = compose_sized(32, inner_layer(None));
    let (x0, y0, x1, y1) = content_rect();
    for y in 0..32 {
        for x in 0..32 {
            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                continue;
            }
            assert_eq!(px(&with, x, y), px(&plain, x, y), "outside ({x},{y})");
        }
    }
}

#[test]
fn absent_or_disabled_inner_shadow_is_byte_identical() {
    let plain = plain_inner();
    let disabled = spec_inner(&InnerSpec {
        enabled: false,
        ..Default::default()
    });
    let not_present = spec_inner(&InnerSpec {
        present: false,
        ..Default::default()
    });
    // The effect still decodes, so the inert flag (not a decoder miss) is what
    // makes the composite a no-op.
    let decoded = decode_inner_shadow(&disabled).expect("disabled decodes");
    assert!(!decoded.enabled, "enabled == false");
    let decoded = decode_inner_shadow(&not_present).expect("not present decodes");
    assert!(!decoded.present, "present == false");
    assert_eq!(plain, compose_inner(disabled));
    assert_eq!(plain, compose_inner(not_present));
}

#[test]
fn knocks_out_is_decoded_but_inert() {
    let conceal = spec_inner(&InnerSpec {
        knocks_out: true,
        ..Default::default()
    });
    let reveal = spec_inner(&InnerSpec {
        knocks_out: false,
        ..Default::default()
    });
    assert!(
        decode_inner_shadow(&conceal).expect("decodes").knocks_out,
        "layerConceals true decodes"
    );
    assert!(
        !decode_inner_shadow(&reveal).expect("decodes").knocks_out,
        "layerConceals false decodes"
    );
    assert_eq!(
        compose_inner(conceal),
        compose_inner(reveal),
        "the decoded flag must not change the render"
    );
}

#[test]
fn huge_inner_shadow_is_bounded() {
    // A crafted maximum renders without panic, stays confined to the content,
    // and completes within a generous wall-clock bound.
    let spec = InnerSpec {
        distance: 1e30,
        size: 1e30,
        choke: 1e30,
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let out = compose_sized(512, spec_inner(&spec));
    let elapsed = start.elapsed();
    let plain = compose_sized(512, inner_layer(None));
    let (x0, y0, x1, y1) = content_rect();
    for y in 0..512u32 {
        for x in 0..512u32 {
            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                continue;
            }
            assert_eq!(px(&out, x, y), px(&plain, x, y), "outside ({x},{y})");
        }
    }
    assert!(
        elapsed.as_secs_f64() < 15.0,
        "a crafted effect is bounded ({elapsed:?})"
    );
}

#[test]
fn zero_opacity_or_off_canvas_inner_shadow_is_a_no_op() {
    let plain = plain_inner();
    let zero = compose_inner(spec_inner(&InnerSpec {
        opacity: 0.0,
        ..Default::default()
    }));
    assert_eq!(plain, zero, "opacity 0 writes nothing");
    let mut off = inner_layer(None);
    off.rect = rect(-50, -50, -40, -40);
    let mut off_effect = off.clone();
    off_effect.extra_blocks = vec![spec_block(&InnerSpec {
        size: 250.0,
        distance: 5.0,
        ..Default::default()
    })];
    assert_eq!(
        compose_inner(off_effect),
        compose_inner(off),
        "off-canvas is a no-op"
    );
    let mut empty = inner_layer(None);
    empty.rect = rect(2, 2, 2, 2);
    let mut empty_effect = empty.clone();
    empty_effect.extra_blocks = vec![spec_block(&InnerSpec::default())];
    assert_eq!(
        compose_inner(empty_effect),
        compose_inner(empty),
        "zero area is a no-op"
    );
}

// --- Matte sources ---------------------------------------------------------

#[test]
fn solid_fill_inner_shadow_follows_payload_alpha() {
    let fill = |alpha: u8| {
        let mut layer = Layer {
            name: "Solid".into(),
            rect: rect(2, 2, 10, 10),
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![255, 0, 0, alpha].into(),
            }),
            ..Default::default()
        };
        layer.extra_blocks = vec![spec_block(&InnerSpec {
            distance: 4.0,
            ..Default::default()
        })];
        compose_inner(layer)
    };
    assert!(rgb(&fill(255), 9, 5)[0] < 200, "an opaque fill is shadowed");
    assert_ne!(fill(255), fill(0), "a transparent fill shadows nothing");
}

#[test]
fn pattern_fill_inner_shadow_is_confined_to_the_fill() {
    let params = PatternFillParams {
        pattern_id: "pictura-pattern".into(),
        scale: 100.0,
        link_with_layer: true,
        origin: (0, 0),
    };
    let build = |effect: bool| {
        let mut layer = Layer {
            name: "Pattern".into(),
            rect: rect(3, 3, 6, 6),
            adjustment: Some(ptfl(&params)),
            ..Default::default()
        };
        if effect {
            layer.extra_blocks = vec![spec_block(&InnerSpec {
                distance: 1.0,
                size: 2.0,
                ..Default::default()
            })];
        }
        let mut d = pattern_fixture_doc();
        d.layers = pictura_core::LayerStack::from_layers(vec![
            solid(
                "Backdrop",
                full(8, 8),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ]);
        composite_rgba(&d)
    };
    let with = build(true);
    let plain = build(false);
    let mut changed_inside = false;
    for y in 0..8 {
        for x in 0..8 {
            if px(&with, x, y) != px(&plain, x, y) {
                assert!(
                    (3..6).contains(&x) && (3..6).contains(&y),
                    "change outside the fill at ({x},{y})"
                );
                changed_inside = true;
            }
        }
    }
    assert!(changed_inside, "the inner shadow renders inside the fill");
}

#[test]
fn channel_less_smart_object_inner_shadow_covers_the_rect() {
    let mut layer = Layer {
        name: "Smart".into(),
        rect: rect(2, 2, 10, 10),
        channels: Vec::new(),
        smart_object: Some(
            {
                let mut so = pictura_core::SmartObject::default();
                so.kind = pictura_core::SmartObjectKind::Embedded;
                so
            }
            .with_payload(vec![0; 4]),
        ),
        ..Default::default()
    };
    layer.extra_blocks = vec![spec_block(&InnerSpec {
        distance: 4.0,
        ..Default::default()
    })];
    let out = compose_inner(layer);
    assert!(
        rgb(&out, 9, 5)[0] < 200,
        "the layer rect interior is shadowed"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_an_inner_shadow_layer_and_falls_back() {
    let d = doc(
        12,
        12,
        vec![
            solid(
                "Backdrop",
                full(12, 12),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            spec_inner(&InnerSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_does_not_reject_a_disabled_or_not_present_inner_shadow() {
    for spec in [
        InnerSpec {
            enabled: false,
            ..Default::default()
        },
        InnerSpec {
            present: false,
            ..Default::default()
        },
    ] {
        let d = doc(12, 12, vec![spec_inner(&spec)]);
        assert!(
            !matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
            "an inert effect must not reject the GPU"
        );
    }
}

// --- Fixture ---------------------------------------------------------------

const INNER_SHADOW_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/inner_shadow.psd");

#[test]
fn inner_shadow_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(INNER_SHADOW_FIXTURE).expect("inner_shadow.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Inner")
        .expect("Inner layer");
    let shadow = decode_inner_shadow(layer).expect("decodes the authored IrSh");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [10, 20, 30]);
    assert_eq!(shadow.opacity, 75.0);
    assert_eq!(shadow.angle_deg, 120.0);
    assert_eq!(shadow.distance, 5.0);
    assert_eq!(shadow.choke, 0.0);
    assert_eq!(shadow.size, 5.0);
    assert!(!shadow.use_global_angle);
    assert!(shadow.knocks_out);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the shadow renders");
}

pub(super) fn parallel_layer() -> Layer {
    spec_inner(&InnerSpec {
        size: 3.0,
        ..Default::default()
    })
}
