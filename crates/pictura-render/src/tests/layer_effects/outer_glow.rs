use super::*;

fn orgl(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"OrGl".to_vec(),
        items,
    }
}

fn glow_top_with(glow: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (b"OrGl".to_vec(), glow),
        ],
    }
}

/// A glowing 3x3 pixel layer at `(3, 3)` on a 9x9 canvas.
fn glow_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Glowing",
        rect(3, 3, 6, 6),
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
struct GlowSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    spread: f64,
    size: f64,
    technique: Vec<u8>,
}

impl Default for GlowSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 0.0],
            opacity: 100.0,
            spread: 0.0,
            size: 3.0,
            technique: b"SfBL".to_vec(),
        }
    }
}

fn glow_block(spec: &GlowSpec) -> LayerBlock {
    lfx2(glow_top_with(orgl(vec![
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
        (
            b"GlwT".to_vec(),
            DescValue::Enum {
                kind: b"BETE".to_vec(),
                value: spec.technique.clone(),
            },
        ),
        (b"Ckmt".to_vec(), unit(spec.spread, PXL)),
        (b"blur".to_vec(), unit(spec.size, PXL)),
    ])))
}

fn spec_glow(spec: &GlowSpec) -> Layer {
    glow_layer(Some(glow_block(spec)))
}

/// Pixels that differ from the same scene without the effect.
fn glow_pixels(with: &PixelBuffer, plain: &PixelBuffer) -> u32 {
    let mut n = 0;
    for y in 0..with.height {
        for x in 0..with.width {
            if px(with, x, y) != px(plain, x, y) {
                n += 1;
            }
        }
    }
    n
}

fn compose_glow(spec: &GlowSpec) -> PixelBuffer {
    compose(spec_glow(spec))
}

fn plain_glow_compose() -> PixelBuffer {
    compose(glow_layer(None))
}

#[test]
fn outer_glow_decodes_typed_parameters() {
    let block = lfx2(glow_top_with(orgl(vec![
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
        (
            b"GlwT".to_vec(),
            DescValue::Enum {
                kind: b"BETE".to_vec(),
                value: b"PrBL".to_vec(),
            },
        ),
        (b"Ckmt".to_vec(), unit(20.0, PXL)),
        (b"blur".to_vec(), unit(10.0, PXL)),
    ])));
    let glow = decode_outer_glow(&glow_layer(Some(block))).expect("decodes");
    assert!(glow.enabled && glow.present);
    assert_eq!(glow.blend_mode, BlendMode::Multiply);
    assert_eq!(glow.color, [200, 100, 50]);
    assert_eq!(glow.opacity, 60.0);
    assert_eq!(glow.spread, 20.0);
    assert_eq!(glow.size, 10.0);
    assert_eq!(glow.technique, GlowTechnique::Precise);
}

#[test]
fn outer_glow_missing_keys_take_defaults() {
    let block = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
    ])));
    let glow = decode_outer_glow(&glow_layer(Some(block))).expect("decodes");
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [255, 255, 190]);
    assert_eq!(glow.opacity, 75.0);
    assert_eq!(glow.spread, 0.0);
    assert_eq!(glow.size, 5.0);
    assert_eq!(glow.technique, GlowTechnique::Softer);
    // An unknown `GlwT` value falls back to Softer.
    let unknown = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"GlwT".to_vec(),
            DescValue::Enum {
                kind: b"BETE".to_vec(),
                value: b"xxxx".to_vec(),
            },
        ),
    ])));
    assert_eq!(
        decode_outer_glow(&glow_layer(Some(unknown)))
            .expect("decodes")
            .technique,
        GlowTechnique::Softer
    );
}

#[test]
fn outer_glow_doub_numeric_decodes() {
    let block = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), DescValue::Double(4.0)),
        (b"blur".to_vec(), DescValue::Double(6.0)),
    ])));
    let glow = decode_outer_glow(&glow_layer(Some(block))).expect("decodes");
    assert_eq!(glow.spread, 4.0);
    assert_eq!(glow.size, 6.0);
}

#[test]
fn outer_glow_disabled_decodes_but_is_inert() {
    let spec = GlowSpec {
        enabled: false,
        ..Default::default()
    };
    let glow = decode_outer_glow(&spec_glow(&spec)).expect("decodes");
    assert!(!glow.enabled);
    assert_eq!(
        compose_glow(&spec),
        plain_glow_compose(),
        "a disabled glow is byte-identical to no effect"
    );
}

#[test]
fn malformed_or_absent_outer_glow_is_none() {
    // No lfx2 block.
    assert!(decode_outer_glow(&glow_layer(None)).is_none());
    // lfx2 without an OrGl.
    let no_orgl = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_outer_glow(&glow_layer(Some(no_orgl))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        glow_top_with(orgl(vec![(b"enab".to_vec(), DescValue::Bool(true))])),
        17,
    );
    assert!(decode_outer_glow(&glow_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_outer_glow(&glow_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnX".to_vec(),
                value: b"Scrn".to_vec(),
            },
        ),
    ])));
    assert!(decode_outer_glow(&glow_layer(Some(wrong_kind))).is_none());
    // Wrong `GlwT` typeID.
    let wrong_glwt = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"GlwT".to_vec(),
            DescValue::Enum {
                kind: b"XXXX".to_vec(),
                value: b"SfBL".to_vec(),
            },
        ),
    ])));
    assert!(decode_outer_glow(&glow_layer(Some(wrong_glwt))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2(glow_top_with(orgl(vec![
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
    assert!(decode_outer_glow(&glow_layer(Some(wrong_color))).is_none());
    // Non-finite blur.
    let nan = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"blur".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert!(decode_outer_glow(&glow_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2(glow_top_with(orgl(vec![])));
    short.data.truncate(6);
    assert!(decode_outer_glow(&glow_layer(Some(short))).is_none());
}

#[test]
fn huge_outer_glow_parameters_are_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Ckmt", b"blur"] {
        let block = lfx2(glow_top_with(orgl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), DescValue::Double(1e300)),
        ])));
        assert!(
            decode_outer_glow(&glow_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2(glow_top_with(orgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), unit(1e30, PXL)),
        (b"blur".to_vec(), unit(1e30, PXL)),
        (b"Opct".to_vec(), unit(1e30, PRC)),
    ])));
    let glow = decode_outer_glow(&glow_layer(Some(block))).expect("decodes clamped");
    assert_eq!(glow.spread, 100.0);
    assert_eq!(glow.size, 250.0);
    assert_eq!(glow.opacity, 100.0);
    // A hand-built non-finite value is a no-op, never a panic.
    let out = compose_glow(&GlowSpec {
        spread: f64::NAN,
        size: f64::INFINITY,
        opacity: f64::NAN,
        ..Default::default()
    });
    assert_eq!(out.width, 9);
}

#[test]
fn glow_surrounds_the_content() {
    let spec = GlowSpec {
        color: [0.0, 0.0, 255.0],
        size: 3.0,
        ..Default::default()
    };
    let out = compose_glow(&spec);
    let p = rgb(&out, 2, 4);
    assert_ne!(
        p,
        [255, 255, 255],
        "the backdrop next to the content is tinted"
    );
    assert!(p[2] > p[0], "tinted toward the glow colour: {p:?}");
}

#[test]
fn glow_is_exterior_and_leaves_content_unchanged() {
    let out = compose_glow(&GlowSpec::default());
    assert_eq!(
        rgb(&out, 4, 4),
        [255, 0, 0],
        "the glow is knocked out under opaque content"
    );
}

#[test]
fn spread_extends_and_hardens_the_glow() {
    let plain = plain_glow_compose();
    let soft = compose_glow(&GlowSpec {
        size: 6.0,
        ..Default::default()
    });
    let spread = compose_glow(&GlowSpec {
        spread: 50.0,
        size: 6.0,
        ..Default::default()
    });
    assert!(
        glow_pixels(&spread, &plain) >= glow_pixels(&soft, &plain),
        "a positive spread covers no fewer pixels"
    );
    // A near-opaque core count: spread 100 dilates the matte before the blur.
    let dark = |buf: &PixelBuffer| {
        let mut n = 0;
        for y in 0..buf.height {
            for x in 0..buf.width {
                if rgb(buf, x, y)[0] < 20 {
                    n += 1;
                }
            }
        }
        n
    };
    let hard = compose_glow(&GlowSpec {
        spread: 100.0,
        size: 6.0,
        ..Default::default()
    });
    assert!(
        dark(&hard) > dark(&soft),
        "spread 100 has a harder edge ({} vs {})",
        dark(&hard),
        dark(&soft)
    );
}

#[test]
fn size_softens_the_glow() {
    let plain = plain_glow_compose();
    let hard = compose_glow(&GlowSpec {
        size: 0.0,
        ..Default::default()
    });
    let soft = compose_glow(&GlowSpec {
        size: 6.0,
        ..Default::default()
    });
    assert_eq!(
        glow_pixels(&hard, &plain),
        0,
        "size 0 produces no glow (spread 0)"
    );
    assert!(
        glow_pixels(&soft, &plain) > 0,
        "blur spreads the glow across more pixels"
    );
}

#[test]
fn opacity_and_colour_shape_the_glow() {
    let spec = GlowSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 50.0,
        size: 3.0,
        ..Default::default()
    };
    let out = compose_glow(&spec);
    let p = rgb(&out, 2, 4);
    assert_eq!(p[0], 255, "red tint keeps red");
    assert!(p[1] < 255, "green attenuated to {}", p[1]);
    assert!(p[0] > p[1], "red tint, not black: {p:?}");
    // Lowering the opacity approaches the white backdrop.
    let faint = compose_glow(&GlowSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 10.0,
        size: 3.0,
        ..Default::default()
    });
    assert!(
        rgb(&faint, 2, 4)[1] >= p[1],
        "opacity 10 is closer to the backdrop than 50"
    );
}

#[test]
fn layer_mask_shapes_the_glow() {
    let spec = GlowSpec::default();
    let full = with_mask(spec_glow(&spec), 255);
    let empty = with_mask(spec_glow(&spec), 0);
    assert!(
        left_strip_changed(&compose(full)) > 0,
        "mask 255 keeps the glow"
    );
    assert_eq!(
        left_strip_changed(&compose(empty)),
        0,
        "mask 0 hides the glow"
    );
}

#[test]
fn absent_or_disabled_glow_is_byte_identical() {
    let plain = plain_glow_compose();
    let disabled = compose_glow(&GlowSpec {
        enabled: false,
        ..Default::default()
    });
    let not_present = compose_glow(&GlowSpec {
        present: false,
        ..Default::default()
    });
    assert_eq!(plain, disabled);
    assert_eq!(plain, not_present);
}

#[test]
fn small_layer_glow_is_confined_and_far_pixels_are_plain() {
    let spec = GlowSpec {
        size: 3.0,
        ..Default::default()
    };
    let with = compose_sized(32, spec_glow(&spec));
    let plain = compose_sized(32, glow_layer(None));
    assert!(rgb(&with, 2, 4)[0] < 255, "glow near the content");
    assert_eq!(rgb(&with, 4, 4), [255, 0, 0], "content unchanged");
    for y in 10..32 {
        for x in 10..32 {
            assert_eq!(px(&with, x, y), px(&plain, x, y), "far pixel ({x},{y})");
        }
    }
}

#[test]
fn huge_glow_is_bounded() {
    // A legit maximum size/spread on a small layer is confined to the content
    // bbox plus reach, not the whole 512px canvas.
    let spec = GlowSpec {
        size: 250.0,
        spread: 100.0,
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let out = compose_sized(512, spec_glow(&spec));
    let elapsed = start.elapsed();
    assert_eq!(out.width, 512);
    assert!(
        elapsed.as_secs_f64() < 15.0,
        "a small layer at max size is bounded ({elapsed:?})"
    );
}

#[test]
fn gpu_rejects_an_outer_glow_layer_and_falls_back() {
    let d = doc(
        9,
        9,
        vec![
            solid(
                "Backdrop",
                full(9, 9),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            spec_glow(&GlowSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_does_not_reject_a_disabled_or_not_present_glow() {
    for spec in [
        GlowSpec {
            enabled: false,
            ..Default::default()
        },
        GlowSpec {
            present: false,
            ..Default::default()
        },
    ] {
        let d = doc(9, 9, vec![spec_glow(&spec)]);
        assert!(
            !matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
            "an inert glow must not reject the GPU"
        );
    }
}

const OUTER_GLOW_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/outer_glow.psd");

#[test]
fn outer_glow_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(OUTER_GLOW_FIXTURE).expect("outer_glow.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Glowing")
        .expect("Glowing layer");
    let glow = decode_outer_glow(layer).expect("decodes the authored OrGl");
    assert!(glow.enabled && glow.present);
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [40, 80, 120]);
    assert_eq!(glow.opacity, 60.0);
    assert_eq!(glow.spread, 20.0);
    assert_eq!(glow.size, 10.0);
    assert_eq!(glow.technique, GlowTechnique::Precise);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the glow renders");
}

// --- Glow matte sources ----------------------------------------------------

#[test]
fn solid_fill_glow_follows_payload_alpha() {
    let fill = |alpha: u8| {
        let mut layer = Layer {
            name: "Solid".into(),
            rect: rect(3, 3, 6, 6),
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![255, 0, 0, alpha],
            }),
            ..Default::default()
        };
        layer.extra_blocks = vec![glow_block(&GlowSpec::default())];
        compose(layer)
    };
    assert!(left_strip_changed(&fill(255)) > 0, "an opaque fill glows");
    assert_eq!(
        left_strip_changed(&fill(0)),
        0,
        "a transparent fill glows nothing"
    );
}

#[test]
fn pattern_fill_glow_is_confined_to_the_fill() {
    let params = PatternFillParams {
        pattern_id: "pictura-pattern".into(),
        scale: 100.0,
        link_with_layer: true,
        origin: (0, 0),
    };
    let mut layer = Layer {
        name: "Pattern".into(),
        rect: rect(3, 3, 6, 6),
        adjustment: Some(ptfl(&params)),
        ..Default::default()
    };
    layer.extra_blocks = vec![glow_block(&GlowSpec {
        size: 2.0,
        ..Default::default()
    })];
    let mut d = pattern_fixture_doc();
    let backdrop = solid(
        "Backdrop",
        full(8, 8),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    d.layers = pictura_core::LayerStack::from_layers(vec![backdrop, layer]);
    let out = composite_rgba(&d);
    let near = rgb(&out, 2, 4);
    assert_ne!(near, [255, 255, 255], "the glow is near the fill: {near:?}");
    assert_eq!(rgb(&out, 0, 4), [255, 255, 255], "no glow off the fill");
}

#[test]
fn channel_less_smart_object_glow_covers_the_layer_rect() {
    let mut layer = Layer {
        name: "Smart".into(),
        rect: rect(3, 3, 6, 6),
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
    layer.extra_blocks = vec![glow_block(&GlowSpec::default())];
    let out = compose(layer);
    assert!(left_strip_changed(&out) > 0, "the layer rect glows");
}

// --- Edge cases ------------------------------------------------------------

#[test]
fn zero_opacity_glow_is_byte_identical() {
    let plain = plain_glow_compose();
    let zero = compose_glow(&GlowSpec {
        opacity: 0.0,
        ..Default::default()
    });
    assert_eq!(plain, zero, "opacity 0 writes nothing");
}

#[test]
fn off_canvas_or_zero_area_glow_is_a_no_op() {
    let backdrop = || {
        composite_rgba(&doc(
            9,
            9,
            vec![solid(
                "Backdrop",
                full(9, 9),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            )],
        ))
    };
    let spec = GlowSpec {
        size: 250.0,
        spread: 100.0,
        ..Default::default()
    };
    // Fully off-canvas.
    let mut off = glow_layer(None);
    off.rect = rect(-50, -50, -40, -40);
    off.extra_blocks = vec![glow_block(&spec)];
    assert_eq!(compose(off), backdrop(), "an off-canvas layer is a no-op");
    // Zero area.
    let mut empty = glow_layer(Some(glow_block(&spec)));
    empty.rect = rect(3, 3, 3, 3);
    assert_eq!(compose(empty), backdrop(), "a zero-area layer is a no-op");
}

#[test]
fn softer_and_precise_glow_render_identically() {
    let softer = compose_glow(&GlowSpec {
        technique: b"SfBL".to_vec(),
        size: 4.0,
        spread: 40.0,
        ..Default::default()
    });
    let precise = compose_glow(&GlowSpec {
        technique: b"PrBL".to_vec(),
        size: 4.0,
        spread: 40.0,
        ..Default::default()
    });
    assert_eq!(
        softer, precise,
        "Precise is rendered as the Softer blur (declared ceiling)"
    );
}

#[test]
fn gpu_reports_the_effect_for_a_glow_fill_layer() {
    let mut layer = Layer {
        name: "Solid".into(),
        rect: rect(3, 3, 6, 6),
        adjustment: Some(AdjustmentData {
            key: *b"SoCo",
            data: vec![255, 0, 0, 255],
        }),
        ..Default::default()
    };
    layer.extra_blocks = vec![glow_block(&GlowSpec::default())];
    let d = doc(9, 9, vec![layer]);
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "the effect error wins over UnsupportedAdjustment"
    );
}
