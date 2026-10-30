use super::*;

fn chfx(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"ChFX".to_vec(),
        items,
    }
}

/// A satined 6x6 pixel layer at `(3, 3)` on a 12x12 canvas, leaving a margin on
/// every side so an interior band has room.
fn satin_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Satin",
        rect(3, 3, 9, 9),
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

fn compose(satin: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, satin]))
}

#[derive(Clone)]
struct SatinSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    angle: f64,
    distance: f64,
    size: f64,
    invert: bool,
}

impl Default for SatinSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Mltp".to_vec(),
            color: [0.0, 0.0, 0.0],
            opacity: 100.0,
            angle: 0.0,
            distance: 2.0,
            size: 0.0,
            invert: false,
        }
    }
}

fn satin_block(spec: &SatinSpec) -> LayerBlock {
    lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
            (b"present".to_vec(), DescValue::Bool(spec.present)),
            (b"showInDialog".to_vec(), DescValue::Bool(true)),
            (b"Md  ".to_vec(), blenm(&spec.blend)),
            (
                b"Clr ".to_vec(),
                rgbc(spec.color[0], spec.color[1], spec.color[2]),
            ),
            (b"Opct".to_vec(), unit(spec.opacity, PRC)),
            (b"uglg".to_vec(), DescValue::Bool(false)),
            (b"lagl".to_vec(), unit(spec.angle, ANG)),
            (b"Dstn".to_vec(), unit(spec.distance, PXL)),
            (b"blur".to_vec(), unit(spec.size, PXL)),
            (b"Invr".to_vec(), DescValue::Bool(spec.invert)),
            (b"AntA".to_vec(), DescValue::Bool(true)),
            (
                b"MpgS".to_vec(),
                object(
                    b"TrnS",
                    vec![(b"Nm  ".to_vec(), DescValue::Text("Linear".into()))],
                ),
            ),
        ]),
    )
}

fn spec_satin(spec: &SatinSpec) -> Layer {
    satin_layer(Some(satin_block(spec)))
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn satin_decodes_typed_parameters() {
    let block = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"Md  ".to_vec(), blenm(b"Mltp")),
            (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
            (b"Opct".to_vec(), unit(60.0, PRC)),
            (b"lagl".to_vec(), unit(45.0, ANG)),
            (b"Dstn".to_vec(), unit(8.0, PXL)),
            (b"blur".to_vec(), unit(12.0, PXL)),
            (b"Invr".to_vec(), DescValue::Bool(true)),
        ]),
    );
    let satin = decode_satin(&satin_layer(Some(block))).expect("decodes");
    assert!(satin.enabled && satin.present);
    assert_eq!(satin.blend_mode, BlendMode::Multiply);
    assert_eq!(satin.color, [200, 100, 50]);
    assert_eq!(satin.opacity, 60.0);
    assert_eq!(satin.angle_deg, 45.0);
    assert_eq!(satin.distance, 8.0);
    assert_eq!(satin.size, 12.0);
    assert!(satin.invert);
}

#[test]
fn satin_missing_keys_take_defaults() {
    let block = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
        ]),
    );
    let satin = decode_satin(&satin_layer(Some(block))).expect("decodes");
    assert_eq!(satin.blend_mode, BlendMode::Multiply);
    assert_eq!(satin.color, [0, 0, 0]);
    assert_eq!(satin.opacity, 50.0);
    assert_eq!(satin.angle_deg, 19.0);
    assert_eq!(satin.distance, 11.0);
    assert_eq!(satin.size, 14.0);
    assert!(!satin.invert);
}

#[test]
fn satin_doub_numeric_decodes() {
    let block = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Dstn".to_vec(), DescValue::Double(3.0)),
            (b"blur".to_vec(), DescValue::Double(4.0)),
        ]),
    );
    let satin = decode_satin(&satin_layer(Some(block))).expect("decodes");
    assert_eq!(satin.distance, 3.0);
    assert_eq!(satin.size, 4.0);
}

#[test]
fn satin_disabled_decodes_but_is_inert() {
    let spec = SatinSpec {
        enabled: false,
        ..Default::default()
    };
    let satin = decode_satin(&spec_satin(&spec)).expect("decodes");
    assert!(!satin.enabled);
    assert_eq!(
        compose(spec_satin(&spec)),
        compose(satin_layer(None)),
        "a disabled effect is byte-identical to no effect"
    );
}

#[test]
fn malformed_or_absent_satin_is_none() {
    // No lfx2 block.
    assert!(decode_satin(&satin_layer(None)).is_none());
    // lfx2 without a ChFX.
    let no_chfx = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_satin(&satin_layer(Some(no_chfx))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        effect_top(
            b"ChFX",
            chfx(vec![(b"enab".to_vec(), DescValue::Bool(true))]),
        ),
        17,
    );
    assert!(decode_satin(&satin_layer(Some(wrong))).is_none());
    // Wrong class id under the `ChFX` key.
    let wrong_id = lfx2_effect(
        b"ChFX",
        object(b"XXXX", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    );
    assert!(decode_satin(&satin_layer(Some(wrong_id))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Md  ".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_satin(&satin_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Md  ".to_vec(),
                DescValue::Enum {
                    kind: b"BlnX".to_vec(),
                    value: b"Mltp".to_vec(),
                },
            ),
        ]),
    );
    assert!(decode_satin(&satin_layer(Some(wrong_kind))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Clr ".to_vec(),
                object(
                    b"LbCl",
                    vec![
                        (b"Rd  ".to_vec(), DescValue::Double(1.0)),
                        (b"Grn ".to_vec(), DescValue::Double(2.0)),
                        (b"Bl  ".to_vec(), DescValue::Double(3.0)),
                    ],
                ),
            ),
        ]),
    );
    assert!(decode_satin(&satin_layer(Some(wrong_color))).is_none());
    // Non-finite numerics.
    for key in [b"Opct", b"Dstn", b"blur"] {
        let nan = lfx2_effect(
            b"ChFX",
            chfx(vec![
                (b"enab".to_vec(), DescValue::Bool(true)),
                (key.to_vec(), DescValue::Double(f64::NAN)),
            ]),
        );
        assert!(
            decode_satin(&satin_layer(Some(nan))).is_none(),
            "{key:?} NAN rejects"
        );
    }
    // Truncated block.
    let mut short = satin_block(&SatinSpec::default());
    short.data.truncate(6);
    assert!(decode_satin(&satin_layer(Some(short))).is_none());
}

#[test]
fn satin_huge_numeric_parameters_are_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Opct", b"Dstn", b"blur"] {
        let block = lfx2_effect(
            b"ChFX",
            chfx(vec![
                (b"enab".to_vec(), DescValue::Bool(true)),
                (key.to_vec(), DescValue::Double(1e300)),
            ]),
        );
        assert!(
            decode_satin(&satin_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"blur".to_vec(), unit(1e30, PXL)),
            (b"Dstn".to_vec(), unit(1e30, PXL)),
            (b"Opct".to_vec(), unit(1e30, PRC)),
        ]),
    );
    let satin = decode_satin(&satin_layer(Some(block))).expect("decodes clamped");
    assert_eq!(satin.size, 250.0);
    assert_eq!(satin.distance, 30_000.0);
    assert_eq!(satin.opacity, 100.0);
    // Rendering the clamped effect does not panic, stays bounded, and matches
    // the explicitly-clamped reference (which is off-canvas, so a no-op).
    let out = compose(spec_satin(&SatinSpec {
        size: 1e30,
        distance: 1e30,
        opacity: 1e30,
        ..Default::default()
    }));
    assert_eq!(
        out,
        compose(spec_satin(&SatinSpec {
            size: 250.0,
            distance: 30_000.0,
            opacity: 100.0,
            ..Default::default()
        })),
        "the huge finite input matches the clamped reference"
    );
    assert_eq!(
        out,
        compose(satin_layer(None)),
        "the clamped satin is a no-op"
    );
    assert_eq!(out.width, 12);
}

#[test]
fn satin_ignores_uglg_antialias_and_contour() {
    // `satin_block` always carries `uglg`, `AntA` and `MpgS`; a full decode
    // still succeeds, so the keys are inert. A wrong-typed `Invr` rejects.
    assert!(decode_satin(&spec_satin(&SatinSpec::default())).is_some());
    let wrong_invr = lfx2_effect(
        b"ChFX",
        chfx(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Invr".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_satin(&satin_layer(Some(wrong_invr))).is_none());
}

// --- Rendering -------------------------------------------------------------

#[test]
fn satin_is_interior_and_exterior_is_unchanged() {
    let with = compose(spec_satin(&SatinSpec::default()));
    let plain = compose(satin_layer(None));
    // Every pixel outside the content rect `(3,3)-(9,9)` is byte-identical.
    for y in 0..12 {
        for x in 0..12 {
            if !(3..9).contains(&x) || !(3..9).contains(&y) {
                assert_eq!(px(&with, x, y), px(&plain, x, y), "exterior ({x},{y})");
            }
        }
    }
    let mut diff = 0;
    for y in 3..9 {
        for x in 3..9 {
            if px(&with, x, y) != px(&plain, x, y) {
                diff += 1;
            }
        }
    }
    assert!(diff > 0, "at least one interior pixel differs: {diff}");
    assert_eq!(rgb(&with, 3, 4), [0, 0, 0], "the black band darkens");
}

#[test]
fn satin_angle_and_distance_shape_the_band() {
    let a0 = compose(spec_satin(&SatinSpec {
        angle: 0.0,
        ..Default::default()
    }));
    let a90 = compose(spec_satin(&SatinSpec {
        angle: 90.0,
        ..Default::default()
    }));
    assert_ne!(a0, a90, "angle rotates the band");

    let near = compose(spec_satin(&SatinSpec {
        distance: 1.0,
        ..Default::default()
    }));
    let far = compose(spec_satin(&SatinSpec {
        distance: 3.0,
        ..Default::default()
    }));
    assert_ne!(near, far, "distance shifts the band");
}

#[test]
fn satin_size_softens_the_band() {
    let count = |buf: &PixelBuffer| {
        let plain = compose(satin_layer(None));
        let mut n = 0;
        for y in 3..9 {
            for x in 3..9 {
                if px(buf, x, y) != px(&plain, x, y) {
                    n += 1;
                }
            }
        }
        n
    };
    let hard = compose(spec_satin(&SatinSpec {
        distance: 2.0,
        size: 0.0,
        ..Default::default()
    }));
    let soft = compose(spec_satin(&SatinSpec {
        distance: 2.0,
        size: 6.0,
        ..Default::default()
    }));
    let hard_n = count(&hard);
    let soft_n = count(&soft);
    assert_ne!(soft, hard, "size changes the rendered band");
    assert!(
        soft_n > hard_n,
        "a larger size spreads the band over more interior pixels ({soft_n} vs {hard_n})"
    );
}

#[test]
fn satin_invert_flips_the_field() {
    let plain = compose(satin_layer(None));
    let band = compose(spec_satin(&SatinSpec {
        distance: 2.0,
        invert: false,
        ..Default::default()
    }));
    let flipped = compose(spec_satin(&SatinSpec {
        distance: 2.0,
        invert: true,
        ..Default::default()
    }));
    assert_ne!(band, flipped, "invert uses the complement");
    assert_ne!(
        plain, flipped,
        "the inverted satin still tints the interior"
    );
    // Invert with zero distance is a uniform interior tint of `M · opacity`.
    let uniform = compose(spec_satin(&SatinSpec {
        distance: 0.0,
        invert: true,
        ..Default::default()
    }));
    assert_ne!(plain, uniform);
}

#[test]
fn satin_opacity_colour_and_blend_mode_shape_it() {
    let opaque = compose(spec_satin(&SatinSpec {
        color: [0.0, 0.0, 255.0],
        opacity: 100.0,
        ..Default::default()
    }));
    let mixed = compose(spec_satin(&SatinSpec {
        color: [0.0, 0.0, 255.0],
        opacity: 50.0,
        ..Default::default()
    }));
    let screen = compose(spec_satin(&SatinSpec {
        color: [0.0, 0.0, 255.0],
        opacity: 100.0,
        blend: b"Scrn".to_vec(),
        ..Default::default()
    }));
    // The band pixel is black under multiply but keeps some red at half opacity.
    assert_eq!(rgb(&opaque, 3, 4), [0, 0, 0], "multiply blue over red");
    assert!(
        rgb(&mixed, 3, 4)[0] > rgb(&opaque, 3, 4)[0],
        "half opacity approaches the unsatined red"
    );
    assert_ne!(opaque, screen, "the blend mode changes the band");
}

#[test]
fn satin_layer_mask_shapes_the_matte() {
    let spec = SatinSpec::default();
    let plain = compose(half_mask(satin_layer(None)));
    let with = compose(half_mask(spec_satin(&spec)));
    // The mask zeroes the left half of the layer rect: no satin there.
    for y in 3..9 {
        for x in 3..6 {
            assert_eq!(px(&with, x, y), px(&plain, x, y), "mask 0 at ({x},{y})");
        }
    }
    let mut diff = 0;
    for y in 3..9 {
        for x in 6..9 {
            if px(&with, x, y) != px(&plain, x, y) {
                diff += 1;
            }
        }
    }
    assert!(diff > 0, "the satin shows where the mask is 255");
}

#[test]
fn satin_is_bounded_to_the_content_rect() {
    // A small content layer on a much larger canvas: the satin never paints
    // outside the layer rect, whatever the distance and size.
    let make = |block: Option<LayerBlock>| {
        let mut layer = solid(
            "Small",
            rect(2, 2, 6, 6),
            (255, 0, 0),
            255,
            BlendMode::Normal,
            255,
        );
        if let Some(block) = block {
            layer.extra_blocks = vec![block];
        }
        let backdrop = solid(
            "Backdrop",
            full(48, 48),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        composite_rgba(&doc(48, 48, vec![backdrop, layer]))
    };
    let with = make(Some(satin_block(&SatinSpec {
        distance: 3.0,
        size: 8.0,
        ..Default::default()
    })));
    let plain = make(None);
    for y in 0..48 {
        for x in 0..48 {
            if !(2..6).contains(&x) || !(2..6).contains(&y) {
                assert_eq!(px(&with, x, y), px(&plain, x, y), "outside rect ({x},{y})");
            }
        }
    }
}

#[test]
fn absent_disabled_or_zero_distance_satin_is_a_no_op() {
    let plain = compose(satin_layer(None));
    assert_eq!(plain, compose(satin_layer(None)), "absent satin");
    assert_eq!(
        plain,
        compose(spec_satin(&SatinSpec {
            enabled: false,
            ..Default::default()
        })),
        "disabled satin"
    );
    assert_eq!(
        plain,
        compose(spec_satin(&SatinSpec {
            present: false,
            ..Default::default()
        })),
        "not-present satin"
    );
    assert_eq!(
        plain,
        compose(spec_satin(&SatinSpec {
            distance: 0.0,
            invert: false,
            ..Default::default()
        })),
        "zero distance with invert false"
    );
    assert_eq!(
        plain,
        compose(spec_satin(&SatinSpec {
            opacity: 0.0,
            ..Default::default()
        })),
        "zero opacity"
    );
}

#[test]
fn non_finite_descriptor_is_rejected_at_decode() {
    // A descriptor carrying non-finite values never reaches the renderer:
    // `decode_satin` rejects it, so the composite is a bounded no-op.
    let out = compose(spec_satin(&SatinSpec {
        opacity: f64::NAN,
        distance: f64::INFINITY,
        size: f64::NAN,
        angle: f64::NAN,
        ..Default::default()
    }));
    assert_eq!(
        out,
        compose(satin_layer(None)),
        "the malformed satin is inert"
    );
    assert_eq!(out.width, 12);
}

#[test]
fn composite_satin_clamps_hand_built_non_finite_parameters() {
    // A `Satin` built by hand (not via decode) reaches `composite_satin` with
    // non-finite fields; its clamp_finite guards keep the result bounded.
    let layer = satin_layer(None);
    let d = doc(12, 12, vec![layer.clone()]);
    let all_bad = Satin {
        enabled: true,
        present: true,
        blend_mode: BlendMode::Multiply,
        color: [0, 0, 0],
        opacity: f32::NAN,
        angle_deg: f32::NAN,
        distance: f32::INFINITY,
        size: f32::NAN,
        invert: false,
    };
    let out = crate::layer_effects::composite_satin_for_test(&layer, &d, &all_bad);
    assert_eq!(out.width, 12);
    assert!(
        out.data.iter().all(|&b| b == 0),
        "opacity NaN clamps to 0 and paints nothing"
    );

    // With finite opacity, non-finite distance/size/angle clamp to their zero
    // references; invert makes the distance-0 field paint the interior.
    let clamped = Satin {
        opacity: 100.0,
        angle_deg: 0.0,
        distance: 0.0,
        size: 0.0,
        invert: true,
        ..all_bad
    };
    let unbounded = Satin {
        opacity: 100.0,
        invert: true,
        ..all_bad
    };
    assert_eq!(
        crate::layer_effects::composite_satin_for_test(&layer, &d, &unbounded),
        crate::layer_effects::composite_satin_for_test(&layer, &d, &clamped),
        "non-finite distance/size/angle behave as their clamped references"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_satin_layer_and_falls_back() {
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
            spec_satin(&SatinSpec::default()),
        ],
    );
    assert_gpu_accepts_and_matches_cpu(&d);
}

#[test]
fn gpu_does_not_reject_a_disabled_or_malformed_satin() {
    let disabled = spec_satin(&SatinSpec {
        enabled: false,
        ..Default::default()
    });
    assert_gpu_accepts_and_matches_cpu(&doc(12, 12, vec![disabled]));
    let malformed = satin_layer(Some(lfx2_effect(
        b"ChFX",
        object(b"XXXX", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    )));
    assert_gpu_accepts_and_matches_cpu(&doc(12, 12, vec![malformed]));
}

// --- Fixture ---------------------------------------------------------------

const SATIN_FIXTURE: &[u8] = include_bytes!("../../../../pictura-codec/tests/fixtures/satin.psd");

#[test]
fn satin_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(SATIN_FIXTURE).expect("satin.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Satin")
        .expect("Satin layer");
    let satin = decode_satin(layer).expect("decodes the authored ChFX");
    assert!(satin.enabled && satin.present);
    assert_eq!(satin.blend_mode, BlendMode::Multiply);
    assert_eq!(satin.color, [10, 20, 30]);
    assert_eq!(satin.opacity, 50.0);
    assert_eq!(satin.angle_deg, 120.0);
    assert_eq!(satin.distance, 8.0);
    assert_eq!(satin.size, 6.0);
    assert!(satin.invert);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the satin renders");
}

pub(super) fn parallel_layer() -> Layer {
    spec_satin(&SatinSpec {
        size: 3.0,
        ..Default::default()
    })
}
