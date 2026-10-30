use std::collections::HashSet;

use super::*;

fn grfl(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    object(b"GrFl", items)
}

fn clrt(location: f64, color: [f64; 3]) -> DescValue {
    object(
        b"Clrt",
        vec![
            (b"Clr ".to_vec(), rgbc(color[0], color[1], color[2])),
            (b"Lctn".to_vec(), DescValue::Double(location)),
        ],
    )
}

fn grdn(stops: Vec<DescValue>) -> DescValue {
    object(
        b"Grdn",
        vec![
            (b"Nm  ".to_vec(), DescValue::Text("Gradient".into())),
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (
                b"Intr".to_vec(),
                DescValue::Enum {
                    kind: b"Intp".to_vec(),
                    value: b"Lnr ".to_vec(),
                },
            ),
            (b"Clrs".to_vec(), DescValue::List(stops)),
        ],
    )
}

fn grdt(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"GrdT".to_vec(),
        value: value.to_vec(),
    }
}

/// A 4x4 red pixel layer at `(4, 4)` on a 12x12 canvas.
fn overlay_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Overlaid",
        rect(4, 4, 8, 8),
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

fn compose_gradient(overlaid: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, overlaid]))
}

fn plain() -> PixelBuffer {
    compose_gradient(overlay_layer(None))
}

#[derive(Clone)]
struct GradSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    opacity: f64,
    stops: Vec<(f64, [f64; 3])>,
    reverse: bool,
    kind: Vec<u8>,
    angle: f64,
    scale: f64,
    align: bool,
}

impl Default for GradSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            opacity: 100.0,
            stops: vec![(0.0, [0.0, 0.0, 0.0]), (4096.0, [255.0, 255.0, 255.0])],
            reverse: false,
            kind: b"Lnr ".to_vec(),
            angle: 0.0,
            scale: 100.0,
            align: true,
        }
    }
}

fn spec_block(spec: &GradSpec) -> LayerBlock {
    lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
            (b"present".to_vec(), DescValue::Bool(spec.present)),
            (b"Md  ".to_vec(), blenm(&spec.blend)),
            (b"Opct".to_vec(), unit(spec.opacity, PRC)),
            (
                b"Grad".to_vec(),
                grdn(spec.stops.iter().map(|(l, c)| clrt(*l, *c)).collect()),
            ),
            (b"Angl".to_vec(), DescValue::Double(spec.angle)),
            (b"Type".to_vec(), grdt(&spec.kind)),
            (b"Rvrs".to_vec(), DescValue::Bool(spec.reverse)),
            (b"Scl ".to_vec(), unit(spec.scale, PRC)),
            (b"Algn".to_vec(), DescValue::Bool(spec.align)),
        ]),
    )
}

fn spec_layer(spec: &GradSpec) -> Layer {
    overlay_layer(Some(spec_block(spec)))
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn gradient_overlay_decodes_typed_parameters() {
    let spec = GradSpec {
        blend: b"Mltp".to_vec(),
        opacity: 80.0,
        stops: vec![(0.0, [0.0, 0.0, 0.0]), (4096.0, [255.0, 255.0, 255.0])],
        reverse: true,
        kind: b"Lnr ".to_vec(),
        angle: 45.0,
        scale: 150.0,
        align: false,
        ..Default::default()
    };
    let overlay = decode_gradient_overlay(&spec_layer(&spec)).expect("decodes");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Multiply);
    assert_eq!(overlay.opacity, 80.0);
    assert_eq!(overlay.stops.len(), 2);
    assert_eq!(overlay.stops[0].color, [0, 0, 0]);
    assert_eq!(overlay.stops[1].color, [255, 255, 255]);
    assert!(overlay.reverse);
    assert_eq!(overlay.kind, GradientKind::Linear);
    assert_eq!(overlay.angle_deg, 45.0);
    assert_eq!(overlay.scale, 150.0);
    assert!(!overlay.align_with_layer);
}

#[test]
fn gradient_overlay_missing_keys_take_defaults() {
    let block = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"Grad".to_vec(), black_to_white()),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
        ]),
    );
    let overlay = decode_gradient_overlay(&overlay_layer(Some(block))).expect("decodes");
    assert_eq!(overlay.blend_mode, BlendMode::Normal);
    assert_eq!(overlay.opacity, 100.0);
    assert!(!overlay.reverse);
    assert_eq!(overlay.scale, 100.0);
    assert!(overlay.align_with_layer);
}

#[test]
fn gradient_overlay_missing_angle_and_type_default() {
    // Missing `Angl` defaults to 0.
    let no_angle = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    let overlay = decode_gradient_overlay(&overlay_layer(Some(no_angle))).expect("decodes");
    assert_eq!(overlay.angle_deg, 0.0);
    assert_eq!(overlay.kind, GradientKind::Linear);
    // Missing `Type` defaults to Linear.
    let no_type = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(30.0)),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    let overlay = decode_gradient_overlay(&overlay_layer(Some(no_type))).expect("decodes");
    assert_eq!(overlay.angle_deg, 30.0);
    assert_eq!(overlay.kind, GradientKind::Linear);
    // Missing both still decodes with the defaults.
    let neither = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    let overlay = decode_gradient_overlay(&overlay_layer(Some(neither))).expect("decodes");
    assert_eq!(overlay.angle_deg, 0.0);
    assert_eq!(overlay.kind, GradientKind::Linear);
    // A present but malformed key still rejects.
    let bad_type = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (
                b"Type".to_vec(),
                DescValue::Enum {
                    kind: b"XXXX".to_vec(),
                    value: b"Lnr ".to_vec(),
                },
            ),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(bad_type))).is_none());
    let bad_angle = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(f64::NAN)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(bad_angle))).is_none());
}

fn black_to_white() -> DescValue {
    grdn(vec![
        clrt(0.0, [0.0, 0.0, 0.0]),
        clrt(4096.0, [255.0, 255.0, 255.0]),
    ])
}

fn full_grfl(mut extra: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    let mut items = vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Angl".to_vec(), DescValue::Double(0.0)),
        (b"Type".to_vec(), grdt(b"Lnr ")),
        (b"Grad".to_vec(), black_to_white()),
    ];
    items.append(&mut extra);
    grfl(items)
}

#[test]
fn malformed_or_absent_gradient_overlay_is_none() {
    assert!(decode_gradient_overlay(&overlay_layer(None)).is_none());
    // lfx2 without a GrFl.
    let no_grfl = lfx2(object(
        b"null",
        vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    ));
    assert!(decode_gradient_overlay(&overlay_layer(Some(no_grfl))).is_none());
    // Wrong class id.
    let wrong_class = lfx2_effect(b"GrFl", object(b"GdFl", vec![]));
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(effect_top(b"GrFl", full_grfl(vec![])), 17);
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong))).is_none());
    // Wrong `Md  ` typeID.
    let wrong_md = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Md  ".to_vec(),
                DescValue::Enum {
                    kind: b"BlnX".to_vec(),
                    value: b"Nrml".to_vec(),
                },
            ),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong_md))).is_none());
    // Wrong `Type` typeID.
    let wrong_type = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Type".to_vec(),
                DescValue::Enum {
                    kind: b"XXXX".to_vec(),
                    value: b"Lnr ".to_vec(),
                },
            ),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong_type))).is_none());
    // A `GrdF` that is not `CstS`.
    let wrong_form = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (
                b"Grad".to_vec(),
                object(
                    b"Grdn",
                    vec![(
                        b"GrdF".to_vec(),
                        DescValue::Enum {
                            kind: b"GrdF".to_vec(),
                            value: b"FStS".to_vec(),
                        },
                    )],
                ),
            ),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong_form))).is_none());
    // A `Grad` that is not an object.
    let wrong_grad = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(wrong_grad))).is_none());
    // A single stop.
    let one_stop = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), grdn(vec![clrt(0.0, [0.0, 0.0, 0.0])])),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(one_stop))).is_none());
    // Non-increasing stop locations.
    let flat = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (
                b"Grad".to_vec(),
                grdn(vec![
                    clrt(0.0, [0.0, 0.0, 0.0]),
                    clrt(0.0, [255.0, 255.0, 255.0]),
                ]),
            ),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(flat))).is_none());
    // Non-finite `Angl`.
    let nan = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(f64::NAN)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2_effect(b"GrFl", full_grfl(vec![]));
    short.data.truncate(6);
    assert!(decode_gradient_overlay(&overlay_layer(Some(short))).is_none());
}

#[test]
fn huge_gradient_overlay_numerics_are_bounded_or_rejected() {
    // A finite f64 that fits f32 is clamped.
    let clamped = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
            (b"Opct".to_vec(), unit(1e30, PRC)),
        ]),
    );
    assert_eq!(
        decode_gradient_overlay(&overlay_layer(Some(clamped)))
            .expect("decodes clamped")
            .opacity,
        100.0
    );
    // A finite f64 that overflows f32 is rejected.
    let overflow = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(1e300)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert!(decode_gradient_overlay(&overlay_layer(Some(overflow))).is_none());
}

// --- Rendering -------------------------------------------------------------

fn interior_colors(buf: &PixelBuffer) -> HashSet<[u8; 3]> {
    let mut set = HashSet::new();
    for y in 4..8u32 {
        for x in 4..8u32 {
            set.insert(rgb(buf, x, y));
        }
    }
    set
}

#[test]
fn gradient_overlay_is_confined_to_the_content() {
    let out = compose_gradient(spec_layer(&GradSpec::default()));
    let plain = plain();
    assert!(
        interior_colors(&out).len() > 1,
        "the gradient varies across the layer"
    );
    assert_ne!(rgb(&out, 4, 4), rgb(&plain, 4, 4), "the gradient renders");
    for y in 0..12u32 {
        for x in 0..12u32 {
            if (4..8).contains(&x) && (4..8).contains(&y) {
                continue;
            }
            assert_eq!(px(&out, x, y), px(&plain, x, y), "exterior ({x},{y})");
        }
    }
}

#[test]
fn gradient_geometry_follows_the_parameters() {
    let base = compose_gradient(spec_layer(&GradSpec::default()));
    for spec in [
        GradSpec {
            angle: 90.0,
            ..Default::default()
        },
        GradSpec {
            scale: 150.0,
            ..Default::default()
        },
        GradSpec {
            kind: b"Rdl ".to_vec(),
            ..Default::default()
        },
        GradSpec {
            reverse: true,
            ..Default::default()
        },
    ] {
        assert_ne!(
            compose_gradient(spec_layer(&spec)),
            base,
            "a changed parameter changes the composite"
        );
    }
}

#[test]
fn gradient_align_with_layer_moves_the_origin() {
    let aligned = compose_gradient(spec_layer(&GradSpec {
        align: true,
        ..Default::default()
    }));
    let canvas = compose_gradient(spec_layer(&GradSpec {
        align: false,
        ..Default::default()
    }));
    assert_ne!(
        aligned, canvas,
        "the gradient is anchored to the layer rect or the canvas"
    );
}

#[test]
fn gradient_overlay_opacity_and_blend_shape_the_fill() {
    let full = compose_gradient(spec_layer(&GradSpec::default()));
    let half = compose_gradient(spec_layer(&GradSpec {
        opacity: 50.0,
        ..Default::default()
    }));
    assert_ne!(half, full);
    assert!(
        rgb(&half, 4, 5)[0] > rgb(&full, 4, 5)[0],
        "lower opacity shows more of the red content"
    );
    let multiply = compose_gradient(spec_layer(&GradSpec {
        blend: b"Mltp".to_vec(),
        ..Default::default()
    }));
    assert_ne!(multiply, full, "Normal and Multiply differ");
}

#[test]
fn gradient_overlay_follows_the_layer_mask() {
    let out = compose_gradient(half_mask(spec_layer(&GradSpec::default())));
    let plain = compose_gradient(half_mask(overlay_layer(None)));
    assert_eq!(px(&out, 4, 5), px(&plain, 4, 5), "mask 0 hides the overlay");
    assert_ne!(px(&out, 6, 5), px(&plain, 6, 5), "mask 255 keeps it");
}

#[test]
fn disabled_or_absent_gradient_overlay_is_byte_identical() {
    for spec in [
        GradSpec {
            enabled: false,
            ..Default::default()
        },
        GradSpec {
            present: false,
            ..Default::default()
        },
    ] {
        assert_eq!(compose_gradient(spec_layer(&spec)), plain());
    }
    assert_eq!(compose_gradient(overlay_layer(None)), plain());
}

#[test]
fn non_finite_gradient_overlay_is_a_bounded_no_op() {
    let nan = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(f64::NAN)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), black_to_white()),
        ]),
    );
    assert_eq!(compose_gradient(overlay_layer(Some(nan))), plain());
}

#[test]
fn small_gradient_overlay_layer_is_bounded() {
    let backdrop = |layer: Layer| {
        let backdrop = solid(
            "Backdrop",
            full(32, 32),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        composite_rgba(&doc(32, 32, vec![backdrop, layer]))
    };
    let with = backdrop(spec_layer(&GradSpec {
        align: false,
        ..Default::default()
    }));
    let plain_big = backdrop(overlay_layer(None));
    for y in 0..32u32 {
        for x in 0..32u32 {
            if (4..8).contains(&x) && (4..8).contains(&y) {
                continue;
            }
            assert_eq!(px(&with, x, y), px(&plain_big, x, y), "far ({x},{y})");
        }
    }
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_gradient_overlay_layer_and_falls_back() {
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
            spec_layer(&GradSpec::default()),
        ],
    );
    assert_gpu_accepts_and_matches_cpu(&d);
}

#[test]
fn gpu_does_not_reject_an_inert_or_undecodable_gradient_overlay() {
    let disabled = doc(
        12,
        12,
        vec![spec_layer(&GradSpec {
            enabled: false,
            ..Default::default()
        })],
    );
    assert_gpu_accepts_and_matches_cpu(&disabled);
    // A non-object `Grad` is undecodable and must not reject.
    let malformed = lfx2_effect(
        b"GrFl",
        grfl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Angl".to_vec(), DescValue::Double(0.0)),
            (b"Type".to_vec(), grdt(b"Lnr ")),
            (b"Grad".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    let d = doc(12, 12, vec![overlay_layer(Some(malformed))]);
    assert_gpu_accepts_and_matches_cpu(&d);
}

// --- Fixture ---------------------------------------------------------------

const GRADIENT_OVERLAY_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/gradient_overlay.psd");

#[test]
fn gradient_overlay_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(GRADIENT_OVERLAY_FIXTURE).expect("gradient_overlay.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Gradient")
        .expect("Gradient layer");
    let overlay = decode_gradient_overlay(layer).expect("decodes the authored GrFl");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Multiply);
    assert_eq!(overlay.opacity, 80.0);
    assert_eq!(overlay.stops.len(), 2);
    assert!(overlay.reverse);
    assert_eq!(overlay.kind, GradientKind::Linear);
    assert_eq!(overlay.angle_deg, 45.0);
    assert_eq!(overlay.scale, 150.0);
    assert!(!overlay.align_with_layer);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the overlay renders");
}
