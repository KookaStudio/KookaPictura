use super::*;

fn sofi(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    object(b"SoFi", items)
}

/// A 4x4 red pixel layer at `(4, 4)` on a 12x12 canvas, with room on every side.
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

fn compose_overlay(overlaid: Layer) -> PixelBuffer {
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
    compose_overlay(overlay_layer(None))
}

#[derive(Clone)]
struct ColorSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
}

impl Default for ColorSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 255.0],
            opacity: 100.0,
        }
    }
}

fn spec_block(spec: &ColorSpec) -> LayerBlock {
    lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
            (b"present".to_vec(), DescValue::Bool(spec.present)),
            (b"Md  ".to_vec(), blenm(&spec.blend)),
            (
                b"Clr ".to_vec(),
                rgbc(spec.color[0], spec.color[1], spec.color[2]),
            ),
            (b"Opct".to_vec(), unit(spec.opacity, PRC)),
        ]),
    )
}

fn spec_layer(spec: &ColorSpec) -> Layer {
    overlay_layer(Some(spec_block(spec)))
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn color_overlay_decodes_typed_parameters() {
    let block = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"Md  ".to_vec(), blenm(b"Mltp")),
            (b"Clr ".to_vec(), rgbc(200.0, 100.0, 50.0)),
            (b"Opct".to_vec(), unit(60.0, PRC)),
        ]),
    );
    let overlay = decode_color_overlay(&overlay_layer(Some(block))).expect("decodes");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Multiply);
    assert_eq!(overlay.color, [200, 100, 50]);
    assert_eq!(overlay.opacity, 60.0);
}

#[test]
fn color_overlay_missing_keys_take_defaults() {
    let block = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
        ]),
    );
    let overlay = decode_color_overlay(&overlay_layer(Some(block))).expect("decodes");
    assert_eq!(overlay.blend_mode, BlendMode::Normal);
    assert_eq!(overlay.color, [255, 0, 0]);
    assert_eq!(overlay.opacity, 100.0);
}

#[test]
fn color_overlay_doub_numeric_decodes() {
    let block = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Opct".to_vec(), DescValue::Double(40.0)),
        ]),
    );
    assert_eq!(
        decode_color_overlay(&overlay_layer(Some(block)))
            .expect("decodes")
            .opacity,
        40.0
    );
}

#[test]
fn malformed_or_absent_color_overlay_is_none() {
    // No lfx2 block.
    assert!(decode_color_overlay(&overlay_layer(None)).is_none());
    // lfx2 without a SoFi.
    let no_sofi = lfx2(object(
        b"null",
        vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    ));
    assert!(decode_color_overlay(&overlay_layer(Some(no_sofi))).is_none());
    // An object under `SoFi` whose class is not SoFi.
    let wrong_class = lfx2_effect(
        b"SoFi",
        object(b"SoCo", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    );
    assert!(decode_color_overlay(&overlay_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        effect_top(
            b"SoFi",
            sofi(vec![(b"enab".to_vec(), DescValue::Bool(true))]),
        ),
        17,
    );
    assert!(decode_color_overlay(&overlay_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Md  ".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_color_overlay(&overlay_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2_effect(
        b"SoFi",
        sofi(vec![
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
    assert!(decode_color_overlay(&overlay_layer(Some(wrong_kind))).is_none());
    // A non-`RGBC` `Clr `.
    let wrong_color = lfx2_effect(
        b"SoFi",
        sofi(vec![
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
    assert!(decode_color_overlay(&overlay_layer(Some(wrong_color))).is_none());
    // Non-finite opacity.
    let nan = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Opct".to_vec(), DescValue::Double(f64::NAN)),
        ]),
    );
    assert!(decode_color_overlay(&overlay_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2_effect(b"SoFi", sofi(vec![]));
    short.data.truncate(6);
    assert!(decode_color_overlay(&overlay_layer(Some(short))).is_none());
}

#[test]
fn huge_color_overlay_opacity_is_bounded_or_rejected() {
    // A finite f64 that fits f32 is clamped to the documented range.
    let clamped = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Opct".to_vec(), unit(1e30, PRC)),
        ]),
    );
    assert_eq!(
        decode_color_overlay(&overlay_layer(Some(clamped)))
            .expect("decodes clamped")
            .opacity,
        100.0
    );
    // A finite f64 that overflows f32 is rejected.
    let overflow = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Opct".to_vec(), DescValue::Double(1e300)),
        ]),
    );
    assert!(decode_color_overlay(&overlay_layer(Some(overflow))).is_none());
}

// --- Rendering -------------------------------------------------------------

#[test]
fn color_overlay_is_confined_to_the_content() {
    let spec = ColorSpec::default();
    let out = compose_overlay(spec_layer(&spec));
    let plain = plain();
    for y in 4..8u32 {
        for x in 4..8u32 {
            assert_eq!(rgb(&out, x, y), [0, 0, 255], "overlay colour ({x},{y})");
        }
    }
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
fn color_overlay_opacity_shapes_the_fill() {
    let full = compose_overlay(spec_layer(&ColorSpec::default()));
    let half = compose_overlay(spec_layer(&ColorSpec {
        opacity: 50.0,
        ..Default::default()
    }));
    assert_eq!(rgb(&full, 5, 5), [0, 0, 255], "opacity 100 is the colour");
    assert!(
        rgb(&half, 5, 5)[0] > rgb(&full, 5, 5)[0],
        "opacity 50 shows more of the red content"
    );
    assert_ne!(half, full);
}

#[test]
fn color_overlay_blend_mode_shapes_the_fill() {
    let normal = compose_overlay(spec_layer(&ColorSpec::default()));
    let multiply = compose_overlay(spec_layer(&ColorSpec {
        blend: b"Mltp".to_vec(),
        ..Default::default()
    }));
    assert_ne!(normal, multiply, "Normal and Multiply differ");
    // Blue multiplied over red is black.
    assert_eq!(rgb(&multiply, 5, 5), [0, 0, 0]);
}

#[test]
fn color_overlay_follows_the_layer_mask() {
    let spec = ColorSpec::default();
    let out = compose_overlay(half_mask(spec_layer(&spec)));
    let plain = compose_overlay(half_mask(overlay_layer(None)));
    assert_eq!(px(&out, 4, 5), px(&plain, 4, 5), "mask 0 hides the overlay");
    assert_ne!(px(&out, 6, 5), px(&plain, 6, 5), "mask 255 keeps it");
    assert_eq!(rgb(&out, 6, 5), [0, 0, 255]);
}

#[test]
fn small_color_overlay_layer_is_bounded() {
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
    let with = backdrop(spec_layer(&ColorSpec::default()));
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

#[test]
fn disabled_color_overlay_is_byte_identical() {
    for spec in [
        ColorSpec {
            enabled: false,
            ..Default::default()
        },
        ColorSpec {
            present: false,
            ..Default::default()
        },
    ] {
        assert_eq!(
            compose_overlay(spec_layer(&spec)),
            plain(),
            "an inert effect is byte-identical to no effect"
        );
    }
    assert_eq!(compose_overlay(overlay_layer(None)), plain());
}

#[test]
fn non_finite_or_zero_color_overlay_is_a_bounded_no_op() {
    // A non-finite `Opct` rejects the effect, so the composite is a no-op.
    let nan = lfx2_effect(
        b"SoFi",
        sofi(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Opct".to_vec(), DescValue::Double(f64::NAN)),
        ]),
    );
    assert_eq!(compose_overlay(overlay_layer(Some(nan))), plain());
    // Opacity 0 is a no-op.
    assert_eq!(
        compose_overlay(spec_layer(&ColorSpec {
            opacity: 0.0,
            ..Default::default()
        })),
        plain(),
        "opacity 0 is a no-op"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_color_overlay_layer_and_falls_back() {
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
            spec_layer(&ColorSpec::default()),
        ],
    );
    assert_gpu_accepts_and_matches_cpu(&d);
}

#[test]
fn gpu_does_not_reject_an_inert_or_undecodable_color_overlay() {
    // A disabled effect.
    let disabled = doc(
        12,
        12,
        vec![spec_layer(&ColorSpec {
            enabled: false,
            ..Default::default()
        })],
    );
    assert_gpu_accepts_and_matches_cpu(&disabled);
    // A malformed effect (wrong class id) does not decode and does not reject.
    let malformed = lfx2_effect(
        b"SoFi",
        object(b"SoCo", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    );
    let d = doc(12, 12, vec![overlay_layer(Some(malformed))]);
    assert_gpu_accepts_and_matches_cpu(&d);
}

// --- Fixture ---------------------------------------------------------------

const COLOR_OVERLAY_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/color_overlay.psd");

#[test]
fn color_overlay_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(COLOR_OVERLAY_FIXTURE).expect("color_overlay.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Colored")
        .expect("Colored layer");
    let overlay = decode_color_overlay(layer).expect("decodes the authored SoFi");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Multiply);
    assert_eq!(overlay.color, [10, 20, 30]);
    assert_eq!(overlay.opacity, 75.0);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the overlay renders");
}

pub(super) fn parallel_layer() -> Layer {
    spec_layer(&ColorSpec::default())
}
