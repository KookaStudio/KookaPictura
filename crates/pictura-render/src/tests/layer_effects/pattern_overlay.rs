use std::collections::HashSet;

use super::*;

fn pattern_fill(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    object(b"patternFill", items)
}

fn ptrn(pattern_id: &str) -> DescValue {
    object(
        b"Ptrn",
        vec![
            (b"Nm  ".to_vec(), DescValue::Text("Pictura\0".into())),
            (b"Idnt".to_vec(), DescValue::Text(format!("{pattern_id}\0"))),
        ],
    )
}

/// A 4x4 black pixel layer at `(1, 1)` on the 8x8 fixture document's canvas, so
/// the pattern tile origin is not a multiple of the 2x2 tile and
/// `align_with_layer` is observable.
fn overlay_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Overlaid",
        rect(1, 1, 5, 5),
        (0, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    if let Some(block) = block {
        layer.extra_blocks = vec![block];
    }
    layer
}

fn compose_pattern(overlaid: Layer) -> PixelBuffer {
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
        overlaid,
    ]);
    composite_rgba(&d)
}

fn plain() -> PixelBuffer {
    compose_pattern(overlay_layer(None))
}

#[derive(Clone)]
struct PatternSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    opacity: f64,
    pattern_id: String,
    scale: f64,
    angle: f64,
    align: bool,
}

impl Default for PatternSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            opacity: 100.0,
            pattern_id: "pictura-pattern".into(),
            scale: 100.0,
            angle: 0.0,
            align: true,
        }
    }
}

fn spec_block(spec: &PatternSpec) -> LayerBlock {
    lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
            (b"present".to_vec(), DescValue::Bool(spec.present)),
            (b"Md  ".to_vec(), blenm(&spec.blend)),
            (b"Opct".to_vec(), unit(spec.opacity, PRC)),
            (b"Ptrn".to_vec(), ptrn(&spec.pattern_id)),
            (b"Scl ".to_vec(), unit(spec.scale, PRC)),
            (b"Algn".to_vec(), DescValue::Bool(spec.align)),
            (b"Angl".to_vec(), DescValue::Double(spec.angle)),
        ]),
    )
}

fn spec_layer(spec: &PatternSpec) -> Layer {
    overlay_layer(Some(spec_block(spec)))
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn pattern_overlay_decodes_typed_parameters() {
    let spec = PatternSpec {
        blend: b"Scrn".to_vec(),
        opacity: 80.0,
        scale: 50.0,
        angle: 30.0,
        align: true,
        ..Default::default()
    };
    let overlay = decode_pattern_overlay(&spec_layer(&spec)).expect("decodes");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Screen);
    assert_eq!(overlay.opacity, 80.0);
    assert_eq!(overlay.pattern_id, "pictura-pattern");
    assert_eq!(overlay.scale, 50.0);
    assert_eq!(overlay.angle_deg, 30.0);
    assert!(overlay.align_with_layer);
}

#[test]
fn pattern_overlay_missing_keys_take_defaults() {
    let block = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
        ]),
    );
    let overlay = decode_pattern_overlay(&overlay_layer(Some(block))).expect("decodes");
    assert_eq!(overlay.blend_mode, BlendMode::Normal);
    assert_eq!(overlay.opacity, 100.0);
    assert_eq!(overlay.scale, 100.0);
    assert_eq!(overlay.angle_deg, 0.0);
    assert!(overlay.align_with_layer);
}

#[test]
fn pattern_overlay_phase_origin_decodes() {
    let block = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
            (
                b"phase".to_vec(),
                object(
                    b"Pnt ",
                    vec![
                        (b"Hrzn".to_vec(), DescValue::Double(3.0)),
                        (b"Vrtc".to_vec(), DescValue::Double(5.0)),
                    ],
                ),
            ),
        ]),
    );
    let overlay = decode_pattern_overlay(&overlay_layer(Some(block))).expect("decodes");
    assert_eq!(overlay.origin, (3, 5));
}

#[test]
fn malformed_or_absent_pattern_overlay_is_none() {
    assert!(decode_pattern_overlay(&overlay_layer(None)).is_none());
    // lfx2 without a patternFill.
    let none = lfx2(object(
        b"null",
        vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    ));
    assert!(decode_pattern_overlay(&overlay_layer(Some(none))).is_none());
    // Wrong class id.
    let wrong_class = lfx2_effect(b"patternFill", object(b"PtFl", vec![]));
    assert!(decode_pattern_overlay(&overlay_layer(Some(wrong_class))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        effect_top(
            b"patternFill",
            pattern_fill(vec![(b"Ptrn".to_vec(), ptrn("pictura-pattern"))]),
        ),
        17,
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(wrong))).is_none());
    // Wrong `Md  ` typeID.
    let wrong_md = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Md  ".to_vec(),
                DescValue::Enum {
                    kind: b"BlnX".to_vec(),
                    value: b"Nrml".to_vec(),
                },
            ),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(wrong_md))).is_none());
    // A `Ptrn` that is not an object.
    let wrong_ptrn = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(wrong_ptrn))).is_none());
    // A missing `Idnt`.
    let no_id = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"Ptrn".to_vec(),
                object(
                    b"Ptrn",
                    vec![(b"Nm  ".to_vec(), DescValue::Text("x".into()))],
                ),
            ),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(no_id))).is_none());
    // Non-finite `Scl `.
    let nan = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
            (b"Scl ".to_vec(), DescValue::Double(f64::NAN)),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![(b"Ptrn".to_vec(), ptrn("pictura-pattern"))]),
    );
    short.data.truncate(6);
    assert!(decode_pattern_overlay(&overlay_layer(Some(short))).is_none());
}

#[test]
fn huge_pattern_overlay_scale_is_bounded_or_rejected() {
    // A finite f64 that fits f32 is accepted (scale has no documented cap).
    let big = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
            (b"Scl ".to_vec(), unit(1e30, PRC)),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(big))).is_some());
    // A finite f64 that overflows f32 is rejected.
    let overflow = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
            (b"Scl ".to_vec(), DescValue::Double(1e300)),
        ]),
    );
    assert!(decode_pattern_overlay(&overlay_layer(Some(overflow))).is_none());
}

// --- Rendering -------------------------------------------------------------

fn interior_colors(buf: &PixelBuffer) -> HashSet<[u8; 3]> {
    let mut set = HashSet::new();
    for y in 1..5u32 {
        for x in 1..5u32 {
            set.insert(rgb(buf, x, y));
        }
    }
    set
}

#[test]
fn pattern_overlay_is_confined_to_the_content() {
    let out = compose_pattern(spec_layer(&PatternSpec::default()));
    let plain = plain();
    // The 2x2 tile shows its four colours over the black content.
    assert_eq!(rgb(&out, 1, 1), [255, 0, 0], "red tile cell");
    assert_eq!(rgb(&out, 2, 1), [0, 255, 0], "green tile cell");
    assert_eq!(rgb(&out, 1, 2), [0, 0, 255], "blue tile cell");
    assert_eq!(rgb(&out, 2, 2), [255, 255, 255], "white tile cell");
    assert!(interior_colors(&out).len() > 1, "the tile varies");
    for y in 0..8u32 {
        for x in 0..8u32 {
            if (1..5).contains(&x) && (1..5).contains(&y) {
                continue;
            }
            assert_eq!(px(&out, x, y), px(&plain, x, y), "exterior ({x},{y})");
        }
    }
}

#[test]
fn missing_pattern_renders_the_placeholder() {
    let spec = PatternSpec {
        pattern_id: "not-in-the-library".into(),
        ..Default::default()
    };
    let out = compose_pattern(spec_layer(&spec));
    assert_eq!(rgb(&out, 2, 2), [128, 128, 128], "the grey placeholder");
    assert_ne!(out, plain(), "the placeholder is not a no-op");
}

#[test]
fn pattern_scale_and_align_shape_the_tile() {
    let base = compose_pattern(spec_layer(&PatternSpec::default()));
    let scaled = compose_pattern(spec_layer(&PatternSpec {
        scale: 200.0,
        ..Default::default()
    }));
    assert_ne!(scaled, base, "scale changes the tiling");
    let unaligned = compose_pattern(spec_layer(&PatternSpec {
        align: false,
        ..Default::default()
    }));
    assert_ne!(unaligned, base, "align_with_layer moves the tile origin");
}

#[test]
fn pattern_overlay_opacity_and_blend_shape_the_fill() {
    let full = compose_pattern(spec_layer(&PatternSpec::default()));
    let half = compose_pattern(spec_layer(&PatternSpec {
        opacity: 50.0,
        ..Default::default()
    }));
    assert_ne!(half, full);
    assert!(
        rgb(&half, 1, 1)[0] < rgb(&full, 1, 1)[0],
        "lower opacity shows more of the black content"
    );
    let multiply = compose_pattern(spec_layer(&PatternSpec {
        blend: b"Mltp".to_vec(),
        ..Default::default()
    }));
    assert_ne!(multiply, full, "Normal and Multiply differ");
}

#[test]
fn pattern_overlay_follows_the_layer_mask() {
    let out = compose_pattern(half_mask(spec_layer(&PatternSpec::default())));
    let plain = compose_pattern(half_mask(overlay_layer(None)));
    assert_eq!(px(&out, 1, 2), px(&plain, 1, 2), "mask 0 hides the overlay");
    assert_ne!(px(&out, 3, 2), px(&plain, 3, 2), "mask 255 keeps it");
}

#[test]
fn disabled_or_absent_pattern_overlay_is_byte_identical() {
    for spec in [
        PatternSpec {
            enabled: false,
            ..Default::default()
        },
        PatternSpec {
            present: false,
            ..Default::default()
        },
    ] {
        assert_eq!(compose_pattern(spec_layer(&spec)), plain());
    }
    assert_eq!(compose_pattern(overlay_layer(None)), plain());
}

#[test]
fn non_finite_pattern_overlay_is_a_bounded_no_op() {
    let nan = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), ptrn("pictura-pattern")),
            (b"Scl ".to_vec(), DescValue::Double(f64::NAN)),
        ]),
    );
    assert_eq!(compose_pattern(overlay_layer(Some(nan))), plain());
}

#[test]
fn small_pattern_overlay_layer_on_a_larger_canvas_is_bounded() {
    let build = |layer: Layer, effect: bool| {
        let mut d = pattern_fixture_doc();
        d.width = 32;
        d.height = 32;
        let mut layer = layer;
        if !effect {
            layer.extra_blocks.clear();
        }
        d.layers = pictura_core::LayerStack::from_layers(vec![
            solid(
                "Backdrop",
                full(32, 32),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ]);
        composite_rgba(&d)
    };
    let with = build(spec_layer(&PatternSpec::default()), true);
    let plain_big = build(overlay_layer(None), false);
    for y in 0..32u32 {
        for x in 0..32u32 {
            if (1..5).contains(&x) && (1..5).contains(&y) {
                continue;
            }
            assert_eq!(px(&with, x, y), px(&plain_big, x, y), "far ({x},{y})");
        }
    }
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_pattern_overlay_layer_and_falls_back() {
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
        spec_layer(&PatternSpec::default()),
    ]);
    assert_gpu_accepts_and_matches_cpu(&d);
}

#[test]
fn gpu_does_not_reject_an_inert_or_undecodable_pattern_overlay() {
    let d = doc(
        8,
        8,
        vec![spec_layer(&PatternSpec {
            enabled: false,
            ..Default::default()
        })],
    );
    assert_gpu_accepts_and_matches_cpu(&d);
    // A missing `Idnt` is undecodable and must not reject.
    let malformed = lfx2_effect(
        b"patternFill",
        pattern_fill(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"Ptrn".to_vec(), object(b"Ptrn", vec![])),
        ]),
    );
    let d = doc(8, 8, vec![overlay_layer(Some(malformed))]);
    assert_gpu_accepts_and_matches_cpu(&d);
}

// --- Fixture ---------------------------------------------------------------

const PATTERN_OVERLAY_FIXTURE: &[u8] =
    include_bytes!("../../../../pictura-codec/tests/fixtures/pattern_overlay.psd");

#[test]
fn pattern_overlay_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(PATTERN_OVERLAY_FIXTURE).expect("pattern_overlay.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Patterned")
        .expect("Patterned layer");
    let overlay = decode_pattern_overlay(layer).expect("decodes the authored patternFill");
    assert!(overlay.enabled && overlay.present);
    assert_eq!(overlay.blend_mode, BlendMode::Screen);
    assert_eq!(overlay.opacity, 80.0);
    assert_eq!(overlay.pattern_id, "pictura-pattern");
    assert_eq!(overlay.scale, 50.0);
    assert_eq!(overlay.angle_deg, 30.0);
    assert!(overlay.align_with_layer);
    assert_eq!(
        pictura_codec::decode_patterns(&d).len(),
        1,
        "the Patt pattern"
    );

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the overlay renders");
}
