use super::*;

fn irgl(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"IrGl".to_vec(),
        items,
    }
}

fn inner_top_with(glow: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (b"IrGl".to_vec(), glow),
        ],
    }
}

fn src(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"IGSr".to_vec(),
        value: value.to_vec(),
    }
}

fn glwt(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"BETE".to_vec(),
        value: value.to_vec(),
    }
}

/// An inner-glowing 8x8 pixel layer at `(2, 2)` on a 12x12 canvas, leaving
/// margin on every side so the interior field has room.
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
struct InnerGlowSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    choke: f64,
    size: f64,
    source: Vec<u8>,
    technique: Vec<u8>,
}

impl Default for InnerGlowSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 255.0],
            opacity: 100.0,
            choke: 0.0,
            size: 4.0,
            source: b"SrcE".to_vec(),
            technique: b"SfBL".to_vec(),
        }
    }
}

fn spec_block(spec: &InnerGlowSpec) -> LayerBlock {
    lfx2(inner_top_with(irgl(vec![
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
        (b"Ckmt".to_vec(), unit(spec.choke, PRC)),
        (b"blur".to_vec(), unit(spec.size, PXL)),
        (b"glwS".to_vec(), src(&spec.source)),
        (b"GlwT".to_vec(), glwt(&spec.technique)),
    ])))
}

fn spec_inner(spec: &InnerGlowSpec) -> Layer {
    inner_layer(Some(spec_block(spec)))
}

fn compose_inner(glowing: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, glowing]))
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
fn inner_glow_decodes_typed_parameters() {
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnM".to_vec(),
                value: b"Scrn".to_vec(),
            },
        ),
        (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
        (b"Opct".to_vec(), unit(60.0, PRC)),
        (b"Ckmt".to_vec(), unit(20.0, PRC)),
        (b"blur".to_vec(), unit(12.0, PXL)),
        (b"glwS".to_vec(), src(b"SrcC")),
        (b"GlwT".to_vec(), glwt(b"PrBL")),
    ])));
    let glow = decode_inner_glow(&inner_layer(Some(block))).expect("decodes");
    assert!(glow.enabled && glow.present);
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [200, 100, 50]);
    assert_eq!(glow.opacity, 60.0);
    assert_eq!(glow.choke, 20.0);
    assert_eq!(glow.size, 12.0);
    assert_eq!(glow.technique, GlowTechnique::Precise);
    assert_eq!(glow.source, GlowSource::Center);
}

#[test]
fn inner_glow_missing_keys_take_defaults() {
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
    ])));
    let glow = decode_inner_glow(&inner_layer(Some(block))).expect("decodes");
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [255, 255, 255]);
    assert_eq!(glow.opacity, 75.0);
    assert_eq!(glow.choke, 0.0);
    assert_eq!(glow.size, 5.0);
    assert_eq!(glow.technique, GlowTechnique::Softer);
    assert_eq!(glow.source, GlowSource::Edge);
}

#[test]
fn inner_glow_unknown_source_and_technique_fall_back() {
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"glwS".to_vec(), src(b"xxxx")),
        (b"GlwT".to_vec(), glwt(b"yyyy")),
    ])));
    let glow = decode_inner_glow(&inner_layer(Some(block))).expect("decodes");
    assert_eq!(glow.source, GlowSource::Edge, "an unknown source is Edge");
    assert_eq!(
        glow.technique,
        GlowTechnique::Softer,
        "an unknown technique is Softer"
    );
    // A missing `glwS` also defaults to Edge.
    let no_source = lfx2(inner_top_with(irgl(vec![(
        b"enab".to_vec(),
        DescValue::Bool(true),
    )])));
    assert_eq!(
        decode_inner_glow(&inner_layer(Some(no_source)))
            .expect("decodes")
            .source,
        GlowSource::Edge
    );
}

#[test]
fn inner_glow_source_typeid_is_igsr() {
    // The canonical `IGSr` (capital S) typeID decodes each source value.
    for (value, expected) in [(b"SrcE", GlowSource::Edge), (b"SrcC", GlowSource::Center)] {
        let block = lfx2(inner_top_with(irgl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"glwS".to_vec(),
                DescValue::Enum {
                    kind: b"IGSr".to_vec(),
                    value: value.to_vec(),
                },
            ),
        ])));
        assert_eq!(
            decode_inner_glow(&inner_layer(Some(block)))
                .expect("decodes")
                .source,
            expected,
            "canonical IGSr/{value:?}"
        );
    }
    // The legacy/mis-authored `IGsr` spelling is accepted leniently.
    let legacy = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"glwS".to_vec(),
            DescValue::Enum {
                kind: b"IGsr".to_vec(),
                value: b"SrcC".to_vec(),
            },
        ),
    ])));
    assert_eq!(
        decode_inner_glow(&inner_layer(Some(legacy)))
            .expect("decodes")
            .source,
        GlowSource::Center,
        "legacy IGsr is accepted"
    );
}

#[test]
fn inner_glow_wrong_source_typeid_is_none() {
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"glwS".to_vec(),
            DescValue::Enum {
                kind: b"XXXX".to_vec(),
                value: b"SrcE".to_vec(),
            },
        ),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(block))).is_none());
}

#[test]
fn inner_glow_doub_numeric_decodes() {
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), DescValue::Double(5.0)),
        (b"blur".to_vec(), DescValue::Double(4.0)),
    ])));
    let glow = decode_inner_glow(&inner_layer(Some(block))).expect("decodes");
    assert_eq!(glow.choke, 5.0);
    assert_eq!(glow.size, 4.0);
}

#[test]
fn malformed_or_absent_inner_glow_is_none() {
    // No lfx2 block.
    assert!(decode_inner_glow(&inner_layer(None)).is_none());
    // lfx2 without an IrGl.
    let no_irgl = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_inner_glow(&inner_layer(Some(no_irgl))).is_none());
    // An object named IrGl whose class is not IrGl.
    let wrong_class = lfx2(inner_top_with(DescValue::Object {
        name: String::new(),
        class_id: b"OrGl".to_vec(),
        items: vec![(b"enab".to_vec(), DescValue::Bool(true))],
    }));
    assert!(decode_inner_glow(&inner_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        inner_top_with(irgl(vec![(b"enab".to_vec(), DescValue::Bool(true))])),
        17,
    );
    assert!(decode_inner_glow(&inner_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnX".to_vec(),
                value: b"Scrn".to_vec(),
            },
        ),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(wrong_kind))).is_none());
    // Wrong `GlwT` typeID.
    let wrong_glwt = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"GlwT".to_vec(),
            DescValue::Enum {
                kind: b"XXXX".to_vec(),
                value: b"SfBL".to_vec(),
            },
        ),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(wrong_glwt))).is_none());
    // Wrong `glwS` typeID (the Outer Glow technique typeID does not apply).
    let wrong_glws = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"glwS".to_vec(),
            DescValue::Enum {
                kind: b"BETE".to_vec(),
                value: b"SrcE".to_vec(),
            },
        ),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(wrong_glws))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2(inner_top_with(irgl(vec![
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
    assert!(decode_inner_glow(&inner_layer(Some(wrong_color))).is_none());
    // Non-finite blur.
    let nan = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"blur".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert!(decode_inner_glow(&inner_layer(Some(nan))).is_none());
    // Wrong-typed boolean.
    let bad_bool = lfx2(inner_top_with(irgl(vec![(
        b"enab".to_vec(),
        DescValue::Double(1.0),
    )])));
    assert!(decode_inner_glow(&inner_layer(Some(bad_bool))).is_none());
    // Truncated block.
    let mut short = lfx2(inner_top_with(irgl(vec![])));
    short.data.truncate(6);
    assert!(decode_inner_glow(&inner_layer(Some(short))).is_none());
}

#[test]
fn huge_inner_glow_parameters_are_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Ckmt", b"blur", b"Opct"] {
        let block = lfx2(inner_top_with(irgl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), DescValue::Double(1e300)),
        ])));
        assert!(
            decode_inner_glow(&inner_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2(inner_top_with(irgl(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), unit(1e30, PRC)),
        (b"blur".to_vec(), unit(1e30, PXL)),
        (b"Opct".to_vec(), unit(1e30, PRC)),
    ])));
    let glow = decode_inner_glow(&inner_layer(Some(block))).expect("decodes clamped");
    assert_eq!(glow.choke, 100.0);
    assert_eq!(glow.size, 250.0);
    assert_eq!(glow.opacity, 100.0);
    // A hand-built non-finite value is a no-op, never a panic.
    let out = compose_inner(spec_inner(&InnerGlowSpec {
        choke: f64::NAN,
        size: f64::INFINITY,
        opacity: f64::NAN,
        ..Default::default()
    }));
    assert_eq!(out.width, 12);
}

// --- Rendering -------------------------------------------------------------

#[test]
fn inner_glow_is_interior_and_leaves_exterior_unchanged() {
    let with = compose_inner(spec_inner(&InnerGlowSpec::default()));
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
    assert_ne!(
        rgb(&with, 2, 6),
        [255, 0, 0],
        "the interior edge is tinted: {:?}",
        rgb(&with, 2, 6)
    );
}

#[test]
fn edge_and_center_sources_produce_different_fields() {
    let edge = compose_inner(spec_inner(&InnerGlowSpec {
        source: b"SrcE".to_vec(),
        ..Default::default()
    }));
    let center = compose_inner(spec_inner(&InnerGlowSpec {
        source: b"SrcC".to_vec(),
        ..Default::default()
    }));
    assert_ne!(edge, center, "Edge and Center render differently");
    // Edge tints the band next to the content edge; Center lights the interior.
    assert_ne!(rgb(&edge, 2, 6), [255, 0, 0], "Edge tints the edge band");
    assert!(
        rgb(&center, 6, 6)[2] > rgb(&edge, 6, 6)[2],
        "Center lights the interior more than Edge: center={:?} edge={:?}",
        rgb(&center, 6, 6),
        rgb(&edge, 6, 6)
    );
}

#[test]
fn size_zero_edge_is_a_no_op_and_size_six_tints_the_edge() {
    let plain = plain_inner();
    let zero = compose_inner(spec_inner(&InnerGlowSpec {
        size: 0.0,
        choke: 0.0,
        source: b"SrcE".to_vec(),
        ..Default::default()
    }));
    assert_eq!(zero, plain, "size 0 with choke 0 under Edge is a no-op");
    let soft = compose_inner(spec_inner(&InnerGlowSpec {
        size: 6.0,
        source: b"SrcE".to_vec(),
        ..Default::default()
    }));
    assert_ne!(rgb(&soft, 2, 6), [255, 0, 0], "size 6 tints the edge band");
}

#[test]
fn size_zero_edge_is_a_no_op_for_partial_alpha_content() {
    // A partial-alpha matte would otherwise pick up an `M · (1 - M)` tint from
    // the size-0 Edge field; the documented no-op must short-circuit first.
    let build = |block: Option<LayerBlock>| {
        let mut layer = solid(
            "Inner",
            rect(2, 2, 10, 10),
            (255, 0, 0),
            128,
            BlendMode::Normal,
            255,
        );
        if let Some(block) = block {
            layer.extra_blocks = vec![block];
        }
        compose_inner(layer)
    };
    let plain = build(None);
    let zero = build(Some(spec_block(&InnerGlowSpec {
        size: 0.0,
        choke: 0.0,
        source: b"SrcE".to_vec(),
        ..Default::default()
    })));
    assert_eq!(
        plain, zero,
        "size 0 with choke 0 under Edge is a no-op for partial alpha"
    );
}

/// The sum of the blue glow channel on the pixels touching the content edge.
fn edge_band_blue(buf: &PixelBuffer) -> u32 {
    let (x0, y0, x1, y1) = content_rect();
    let mut sum = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if x == x0 || x == x1 - 1 || y == y0 || y == y1 - 1 {
                sum += rgb(buf, x, y)[2] as u32;
            }
        }
    }
    sum
}

#[test]
fn choke_strengthens_the_glow_toward_the_edge() {
    let plain = compose_inner(spec_inner(&InnerGlowSpec {
        choke: 0.0,
        size: 6.0,
        ..Default::default()
    }));
    let choked = compose_inner(spec_inner(&InnerGlowSpec {
        choke: 30.0,
        size: 6.0,
        ..Default::default()
    }));
    assert!(
        edge_band_blue(&plain) > 0,
        "the unchoked glow tints the edge"
    );
    assert!(
        edge_band_blue(&choked) > edge_band_blue(&plain),
        "a positive choke is strictly stronger in the edge band ({} vs {})",
        edge_band_blue(&choked),
        edge_band_blue(&plain)
    );
}

#[test]
fn opacity_and_colour_shape_the_glow() {
    let white = |spec: &InnerGlowSpec| {
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
    let spec = InnerGlowSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 50.0,
        ..Default::default()
    };
    let out = white(&spec);
    let p = rgb(&out, 2, 6);
    assert!(p[0] > p[1] + 20, "a red tint, not white: {p:?}");
    let faint = white(&InnerGlowSpec {
        opacity: 5.0,
        ..spec.clone()
    });
    assert!(
        rgb(&faint, 2, 6)[1] > p[1],
        "opacity 5 is closer to the content than 50"
    );
    let zero = white(&InnerGlowSpec {
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
        l.extra_blocks = vec![spec_block(&InnerGlowSpec {
            enabled: false,
            ..spec.clone()
        })];
        compose_inner(l)
    };
    assert_eq!(zero, plain, "opacity 0 writes nothing");
}

#[test]
fn layer_mask_shapes_the_matte() {
    let spec = InnerGlowSpec::default();
    let with = compose_inner(with_left_masked(spec_inner(&spec), 6));
    let plain = compose_inner(with_left_masked(inner_layer(None), 6));
    for y in 2..10 {
        for x in 2..6 {
            assert_eq!(
                px(&with, x, y),
                px(&plain, x, y),
                "the mask hides the glow at ({x},{y})"
            );
        }
    }
    let mut right_glowing = false;
    for y in 2..10 {
        for x in 6..10 {
            if px(&with, x, y) != px(&plain, x, y) {
                right_glowing = true;
            }
        }
    }
    assert!(right_glowing, "the unmasked right interior glows");
}

#[test]
fn small_layer_inner_glow_is_confined_to_the_rect() {
    let spec = InnerGlowSpec {
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
fn absent_or_disabled_inner_glow_is_byte_identical() {
    let plain = plain_inner();
    let disabled = spec_inner(&InnerGlowSpec {
        enabled: false,
        ..Default::default()
    });
    let not_present = spec_inner(&InnerGlowSpec {
        present: false,
        ..Default::default()
    });
    // The effect still decodes, so the inert flag (not a decoder miss) is what
    // makes the composite a no-op.
    assert!(
        !decode_inner_glow(&disabled)
            .expect("disabled decodes")
            .enabled,
        "enabled == false"
    );
    assert!(
        !decode_inner_glow(&not_present)
            .expect("not present decodes")
            .present,
        "present == false"
    );
    assert_eq!(plain, compose_inner(disabled));
    assert_eq!(plain, compose_inner(not_present));
}

#[test]
fn huge_inner_glow_is_bounded() {
    let spec = InnerGlowSpec {
        choke: 1e30,
        size: 1e30,
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
fn zero_opacity_or_off_canvas_inner_glow_is_a_no_op() {
    let plain = plain_inner();
    let zero = compose_inner(spec_inner(&InnerGlowSpec {
        opacity: 0.0,
        ..Default::default()
    }));
    assert_eq!(plain, zero, "opacity 0 writes nothing");
    let mut off = inner_layer(None);
    off.rect = rect(-50, -50, -40, -40);
    let mut off_effect = off.clone();
    off_effect.extra_blocks = vec![spec_block(&InnerGlowSpec {
        size: 250.0,
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
    empty_effect.extra_blocks = vec![spec_block(&InnerGlowSpec::default())];
    assert_eq!(
        compose_inner(empty_effect),
        compose_inner(empty),
        "zero area is a no-op"
    );
}

#[test]
fn softer_and_precise_inner_glow_render_identically() {
    let softer = compose_inner(spec_inner(&InnerGlowSpec {
        technique: b"SfBL".to_vec(),
        size: 6.0,
        ..Default::default()
    }));
    let precise = compose_inner(spec_inner(&InnerGlowSpec {
        technique: b"PrBL".to_vec(),
        size: 6.0,
        ..Default::default()
    }));
    assert_eq!(
        softer, precise,
        "Precise is rendered as the Softer blur (declared ceiling)"
    );
}

// --- Matte sources ---------------------------------------------------------

#[test]
fn solid_fill_inner_glow_follows_payload_alpha() {
    let fill = |alpha: u8| {
        let mut layer = Layer {
            name: "Solid".into(),
            rect: rect(2, 2, 10, 10),
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![255, 0, 0, alpha],
            }),
            ..Default::default()
        };
        layer.extra_blocks = vec![spec_block(&InnerGlowSpec::default())];
        compose_inner(layer)
    };
    assert_ne!(
        rgb(&fill(255), 2, 6),
        [255, 0, 0],
        "an opaque fill glows at its edge"
    );
    assert_ne!(fill(255), fill(0), "a transparent fill glows nothing");
}

#[test]
fn pattern_fill_inner_glow_is_confined_to_the_fill() {
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
            layer.extra_blocks = vec![spec_block(&InnerGlowSpec {
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
    assert!(changed_inside, "the inner glow renders inside the fill");
}

#[test]
fn channel_less_smart_object_inner_glow_covers_the_rect() {
    let mut layer = Layer {
        name: "Smart".into(),
        rect: rect(2, 2, 10, 10),
        channels: Vec::new(),
        smart_object: Some(pictura_core::SmartObject {
            kind: pictura_core::SmartObjectKind::Embedded,
            payload: Some(vec![0; 4]),
            ..Default::default()
        }),
        ..Default::default()
    };
    layer.extra_blocks = vec![spec_block(&InnerGlowSpec::default())];
    let out = compose_inner(layer);
    assert_ne!(
        rgb(&out, 2, 6),
        [255, 0, 0],
        "the layer rect interior glows"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_an_inner_glow_layer_and_falls_back() {
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
            spec_inner(&InnerGlowSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_does_not_reject_a_disabled_or_not_present_inner_glow() {
    for spec in [
        InnerGlowSpec {
            enabled: false,
            ..Default::default()
        },
        InnerGlowSpec {
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

const INNER_GLOW_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/inner_glow.psd");

#[test]
fn inner_glow_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(INNER_GLOW_FIXTURE).expect("inner_glow.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Glow")
        .expect("Glow layer");
    let glow = decode_inner_glow(layer).expect("decodes the authored IrGl");
    assert!(glow.enabled && glow.present);
    assert_eq!(glow.blend_mode, BlendMode::Screen);
    assert_eq!(glow.color, [255, 255, 255]);
    assert_eq!(glow.opacity, 75.0);
    assert_eq!(glow.choke, 0.0);
    assert_eq!(glow.size, 5.0);
    assert_eq!(glow.technique, GlowTechnique::Softer);
    assert_eq!(glow.source, GlowSource::Edge);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the glow renders");
}
