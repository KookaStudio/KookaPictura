use pictura_core::{Knockout, LayerBlock};

use super::*;

const PRC: [u8; 4] = *b"#Prc";
const ANG: [u8; 4] = *b"#Ang";
const PXL: [u8; 4] = *b"#Pxl";

fn unit(value: f64, unit: [u8; 4]) -> DescValue {
    DescValue::UnitFloat { unit, value }
}

fn rgbc(r: f64, g: f64, b: f64) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"RGBC".to_vec(),
        items: vec![
            (b"Rd  ".to_vec(), DescValue::Double(r)),
            (b"Grn ".to_vec(), DescValue::Double(g)),
            (b"Bl  ".to_vec(), DescValue::Double(b)),
        ],
    }
}

fn drsh(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"DrSh".to_vec(),
        items,
    }
}

fn top_with(drsh: DescValue) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (b"DrSh".to_vec(), drsh),
        ],
    }
}

/// Build an `lfx2` block: a `u32` version then the version-`data_version`
/// descriptor block.
fn lfx2_with(top: DescValue, data_version: u32) -> LayerBlock {
    let mut inner = write_descriptor(&top);
    inner[0..4].copy_from_slice(&data_version.to_be_bytes());
    let mut data = 1u32.to_be_bytes().to_vec();
    data.extend_from_slice(&inner);
    LayerBlock {
        key: *b"lfx2",
        data,
    }
}

fn lfx2(top: DescValue) -> LayerBlock {
    lfx2_with(top, 16)
}

/// A descriptor object with `class_id` and `items`.
fn object(class_id: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class_id.to_vec(),
        items,
    }
}

/// The `lfx2` top-level descriptor carrying `masterFXSwitch` and one effect
/// object under `key`.
fn effect_top(key: &[u8], effect: DescValue) -> DescValue {
    object(
        b"null",
        vec![
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), effect),
        ],
    )
}

/// An `lfx2` block whose top-level object carries one effect under `key`.
fn lfx2_effect(key: &[u8], effect: DescValue) -> LayerBlock {
    lfx2(effect_top(key, effect))
}

/// A `Md  ` blend-mode enum.
fn blenm(value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: b"BlnM".to_vec(),
        value: value.to_vec(),
    }
}

/// A shadowed 3x3 pixel layer at `(3, 3)` on a 9x9 canvas.
fn shadow_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Shadowed",
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

fn compose(shadowed: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(9, 9),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(9, 9, vec![backdrop, shadowed]))
}

/// The same scene on an `n × n` canvas.
fn compose_sized(n: u32, shadowed: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(n, n),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(n, n, vec![backdrop, shadowed]))
}

fn with_mask(mut layer: Layer, value: u8) -> Layer {
    let r = layer.rect;
    let n = r.width().max(0) as usize * r.height().max(0) as usize;
    layer.mask = Some(LayerMask {
        rect: r,
        data: Some(vec![value; n].into()),
        ..Default::default()
    });
    layer
}

/// A mask zero on the left half of the layer rect and 255 on the right half.
fn half_mask(mut layer: Layer) -> Layer {
    let r = layer.rect;
    let w = r.width().max(0) as usize;
    let h = r.height().max(0) as usize;
    let mut data = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            if x * 2 >= w {
                data[y * w + x] = 255;
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

#[derive(Clone)]
struct ShadowSpec {
    enabled: bool,
    present: bool,
    blend: Vec<u8>,
    color: [f64; 3],
    opacity: f64,
    angle: f64,
    distance: f64,
    spread: f64,
    size: f64,
}

impl Default for ShadowSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            blend: b"Nrml".to_vec(),
            color: [0.0, 0.0, 0.0],
            opacity: 100.0,
            angle: 0.0,
            distance: 2.0,
            spread: 0.0,
            size: 0.0,
        }
    }
}

fn spec_block(spec: &ShadowSpec) -> LayerBlock {
    lfx2(top_with(drsh(vec![
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
        (b"uglg".to_vec(), DescValue::Bool(true)),
        (b"lagl".to_vec(), unit(spec.angle, ANG)),
        (b"Dstn".to_vec(), unit(spec.distance, PXL)),
        (b"Ckmt".to_vec(), unit(spec.spread, PRC)),
        (b"blur".to_vec(), unit(spec.size, PXL)),
        (b"layerConceals".to_vec(), DescValue::Bool(true)),
    ])))
}

fn spec_shadow(spec: &ShadowSpec) -> Layer {
    shadow_layer(Some(spec_block(spec)))
}

/// Pixels in the strip left of the content that differ from the white backdrop.
fn left_strip_changed(buf: &PixelBuffer) -> u32 {
    let mut n = 0;
    for y in 0..buf.height {
        for x in 0..3 {
            if rgb(buf, x, y) != [255, 255, 255] {
                n += 1;
            }
        }
    }
    n
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn drop_shadow_decodes_typed_parameters() {
    let block = lfx2(top_with(drsh(vec![
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
    let shadow = decode_drop_shadow(&shadow_layer(Some(block))).expect("decodes");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [200, 100, 50]);
    assert_eq!(shadow.opacity, 60.0);
    assert!(!shadow.use_global_angle);
    assert_eq!(shadow.angle_deg, 45.0);
    assert_eq!(shadow.distance, 8.0);
    assert_eq!(shadow.spread, 20.0);
    assert_eq!(shadow.size, 12.0);
    assert!(!shadow.knocks_out);
}

#[test]
fn drop_shadow_missing_keys_take_defaults() {
    let block = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
    ])));
    let shadow = decode_drop_shadow(&shadow_layer(Some(block))).expect("decodes");
    assert_eq!(shadow.blend_mode, BlendMode::Normal);
    assert_eq!(shadow.color, [0, 0, 0]);
    assert_eq!(shadow.opacity, 75.0);
    assert_eq!(shadow.angle_deg, 120.0);
    assert_eq!(shadow.distance, 5.0);
    assert_eq!(shadow.spread, 0.0);
    assert_eq!(shadow.size, 5.0);
    assert!(shadow.use_global_angle);
    assert!(shadow.knocks_out);
}

#[test]
fn drop_shadow_doub_numeric_decodes() {
    let block = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"Dstn".to_vec(), DescValue::Double(3.0)),
        (b"blur".to_vec(), DescValue::Double(4.0)),
    ])));
    let shadow = decode_drop_shadow(&shadow_layer(Some(block))).expect("decodes");
    assert_eq!(shadow.distance, 3.0);
    assert_eq!(shadow.size, 4.0);
}

#[test]
fn drop_shadow_disabled_decodes_but_is_inert() {
    let spec = ShadowSpec {
        enabled: false,
        ..Default::default()
    };
    let shadow = decode_drop_shadow(&spec_shadow(&spec)).expect("decodes");
    assert!(!shadow.enabled);
    assert_eq!(
        compose(spec_shadow(&spec)),
        compose(shadow_layer(None)),
        "a disabled effect is byte-identical to no effect"
    );
}

#[test]
fn malformed_or_absent_effects_are_none() {
    // No lfx2 block.
    assert!(decode_drop_shadow(&shadow_layer(None)).is_none());
    // lfx2 without a DrSh.
    let no_drsh = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_drop_shadow(&shadow_layer(Some(no_drsh))).is_none());
    // Unknown data version.
    let wrong = lfx2_with(
        top_with(drsh(vec![(b"enab".to_vec(), DescValue::Bool(true))])),
        17,
    );
    assert!(decode_drop_shadow(&shadow_layer(Some(wrong))).is_none());
    // Wrong-typed `Md  `.
    let wrong_md = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Md  ".to_vec(), DescValue::Double(1.0)),
    ])));
    assert!(decode_drop_shadow(&shadow_layer(Some(wrong_md))).is_none());
    // Wrong `Md  ` enum typeID.
    let wrong_kind = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (
            b"Md  ".to_vec(),
            DescValue::Enum {
                kind: b"BlnX".to_vec(),
                value: b"Mltp".to_vec(),
            },
        ),
    ])));
    assert!(decode_drop_shadow(&shadow_layer(Some(wrong_kind))).is_none());
    // Wrong `Clr ` class id.
    let wrong_color = lfx2(top_with(drsh(vec![
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
    assert!(decode_drop_shadow(&shadow_layer(Some(wrong_color))).is_none());
    // Non-finite distance.
    let nan = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Dstn".to_vec(), DescValue::Double(f64::NAN)),
    ])));
    assert!(decode_drop_shadow(&shadow_layer(Some(nan))).is_none());
    // Truncated block.
    let mut short = lfx2(top_with(drsh(vec![])));
    short.data.truncate(6);
    assert!(decode_drop_shadow(&shadow_layer(Some(short))).is_none());
}

#[test]
fn huge_numeric_parameters_are_bounded_or_rejected() {
    // A finite f64 that overflows f32 is rejected.
    for key in [b"Ckmt", b"blur", b"Dstn"] {
        let block = lfx2(top_with(drsh(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (key.to_vec(), DescValue::Double(1e300)),
        ])));
        assert!(
            decode_drop_shadow(&shadow_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    // A finite f64 that fits f32 is clamped to the documented range.
    let block = lfx2(top_with(drsh(vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"Ckmt".to_vec(), unit(1e30, PRC)),
        (b"blur".to_vec(), unit(1e30, PXL)),
        (b"Dstn".to_vec(), unit(1e30, PXL)),
        (b"Opct".to_vec(), unit(1e30, PRC)),
    ])));
    let shadow = decode_drop_shadow(&shadow_layer(Some(block))).expect("decodes clamped");
    assert_eq!(shadow.spread, 100.0);
    assert_eq!(shadow.size, 250.0);
    assert_eq!(shadow.distance, 30_000.0);
    assert_eq!(shadow.opacity, 100.0);
    // Rendering the clamped effect does not panic and stays bounded.
    let out = compose(spec_shadow(&ShadowSpec {
        spread: 1e30,
        size: 1e30,
        distance: 1e30,
        ..Default::default()
    }));
    assert_eq!(out.width, 9);
    // A hand-built non-finite value is a no-op, never a panic.
    let out = compose(spec_shadow(&ShadowSpec {
        spread: f64::NAN,
        size: f64::INFINITY,
        distance: f64::NAN,
        ..Default::default()
    }));
    assert_eq!(out.width, 9);
}

// --- Rendering -------------------------------------------------------------

#[test]
fn shadow_offset_follows_the_angle() {
    let left = ShadowSpec {
        angle: 0.0,
        distance: 2.0,
        ..Default::default()
    };
    let out = compose(spec_shadow(&left));
    assert!(
        rgb(&out, 1, 4)[0] < 128,
        "angle 0 casts left: {}",
        rgb(&out, 1, 4)[0]
    );
    assert_eq!(rgb(&out, 8, 4), [255, 255, 255], "nothing casts right");

    let right = ShadowSpec {
        angle: 180.0,
        distance: 2.0,
        ..Default::default()
    };
    let out = compose(spec_shadow(&right));
    assert!(rgb(&out, 7, 4)[0] < 128, "angle 180 casts right");
    assert_eq!(rgb(&out, 1, 4), [255, 255, 255], "nothing casts left");
}

#[test]
fn distance_scales_the_shadow_extent() {
    let near = ShadowSpec {
        distance: 1.0,
        ..Default::default()
    };
    let far = ShadowSpec {
        distance: 3.0,
        ..Default::default()
    };
    let near_changed = left_strip_changed(&compose(spec_shadow(&near)));
    let far_changed = left_strip_changed(&compose(spec_shadow(&far)));
    assert!(
        far_changed > near_changed,
        "distance 3 darkens more of the strip than distance 1 ({far_changed} vs {near_changed})"
    );
}

#[test]
fn size_softens_the_shadow() {
    let hard = ShadowSpec {
        distance: 0.0,
        size: 0.0,
        ..Default::default()
    };
    let soft = ShadowSpec {
        distance: 0.0,
        size: 6.0,
        ..Default::default()
    };
    let partial = |buf: &PixelBuffer| {
        let mut n = 0;
        for y in 0..buf.height {
            for x in 0..3 {
                let r = rgb(buf, x, y)[0];
                if (1..=254).contains(&r) {
                    n += 1;
                }
            }
        }
        n
    };
    let hard_n = partial(&compose(spec_shadow(&hard)));
    let soft_n = partial(&compose(spec_shadow(&soft)));
    assert!(
        soft_n > hard_n,
        "blur widens the transition ({soft_n} vs {hard_n})"
    );
}

#[test]
fn opacity_and_colour_shape_the_shadow() {
    let spec = ShadowSpec {
        color: [255.0, 0.0, 0.0],
        opacity: 50.0,
        ..Default::default()
    };
    let out = compose(spec_shadow(&spec));
    let p = rgb(&out, 1, 4);
    assert_eq!(p[0], 255, "red tint keeps red");
    assert!((80..=180).contains(&p[1]), "green attenuated to {}", p[1]);
    assert!(p[0] > p[1] + 50, "red tint, not black: {p:?}");
}

#[test]
fn spread_dilates_the_shadow() {
    // Darkness sum over the left strip: a dilate covers more, so the strip is
    // darker (the pixel count saturates on the small canvas).
    let dark = |buf: &PixelBuffer| -> u32 {
        let mut sum = 0;
        for y in 0..buf.height {
            for x in 0..3 {
                sum += 255 - rgb(buf, x, y)[0] as u32;
            }
        }
        sum
    };
    let none = ShadowSpec {
        size: 5.0,
        ..Default::default()
    };
    let spread = ShadowSpec {
        spread: 50.0,
        size: 5.0,
        ..Default::default()
    };
    let none_dark = dark(&compose(spec_shadow(&none)));
    let spread_dark = dark(&compose(spec_shadow(&spread)));
    assert!(none_dark > 0, "the unspread shadow covers the strip");
    assert!(
        spread_dark > none_dark,
        "spread darkens more of the strip ({spread_dark} vs {none_dark})"
    );
}

#[test]
fn spread_hardens_the_edge() {
    // A near-opaque core count: spread dilates the matte before the blur, so
    // more of the shadow reaches full opacity.
    let core = |buf: &PixelBuffer| {
        let mut n = 0;
        for y in 0..buf.height {
            for x in 0..3 {
                if rgb(buf, x, y)[0] < 10 {
                    n += 1;
                }
            }
        }
        n
    };
    let soft = ShadowSpec {
        distance: 0.0,
        size: 6.0,
        ..Default::default()
    };
    let hard = ShadowSpec {
        distance: 0.0,
        size: 6.0,
        spread: 100.0,
        ..Default::default()
    };
    let soft_n = core(&compose(spec_shadow(&soft)));
    let hard_n = core(&compose(spec_shadow(&hard)));
    assert!(
        hard_n > soft_n,
        "spread 100 has a harder edge ({hard_n} vs {soft_n})"
    );
}

#[test]
fn layer_mask_shapes_the_matte() {
    let spec = ShadowSpec::default();
    let full = with_mask(spec_shadow(&spec), 255);
    let empty = with_mask(spec_shadow(&spec), 0);
    assert!(
        left_strip_changed(&compose(full)) > 0,
        "mask 255 keeps the shadow"
    );
    assert_eq!(
        left_strip_changed(&compose(empty)),
        0,
        "mask 0 hides the shadow"
    );
}

#[test]
fn shadow_composites_behind_the_content() {
    let spec = ShadowSpec {
        distance: 1.0,
        ..Default::default()
    };
    let out = compose(spec_shadow(&spec));
    // The offset overlaps the content at its left edge; the content wins.
    assert_eq!(
        rgb(&out, 3, 4),
        [255, 0, 0],
        "content draws over its shadow"
    );
}

#[test]
fn pixel_layer_without_alpha_channel_casts_a_shadow() {
    let mut layer = shadow_layer(Some(spec_block(&ShadowSpec::default())));
    layer.channels.retain(|c| c.id != -1);
    let out = compose(layer);
    assert!(
        left_strip_changed(&out) > 0,
        "a missing -1 channel is opaque, matching the content path"
    );
}

#[test]
fn solid_fill_shadow_follows_payload_alpha() {
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
        layer.extra_blocks = vec![spec_block(&ShadowSpec::default())];
        compose(layer)
    };
    assert!(left_strip_changed(&fill(255)) > 0, "an opaque fill casts");
    assert_eq!(
        left_strip_changed(&fill(0)),
        0,
        "a transparent fill casts nothing"
    );
}

#[test]
fn pattern_fill_shadow_is_confined_to_the_fill() {
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
    layer.extra_blocks = vec![spec_block(&ShadowSpec::default())];
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
    assert!(rgb(&out, 2, 4)[0] < 200, "the shadow is near the fill");
    assert_eq!(rgb(&out, 0, 4), [255, 255, 255], "no shadow off the fill");
}

#[test]
fn channel_less_smart_object_shadow_covers_the_layer_rect() {
    let mut layer = Layer {
        name: "Smart".into(),
        rect: rect(3, 3, 6, 6),
        channels: Vec::new(),
        smart_object: Some(pictura_core::SmartObject {
            kind: pictura_core::SmartObjectKind::Embedded,
            payload: Some(vec![0; 4]),
            ..Default::default()
        }),
        ..Default::default()
    };
    layer.extra_blocks = vec![spec_block(&ShadowSpec::default())];
    let out = compose(layer);
    assert!(
        left_strip_changed(&out) > 0,
        "the layer rect casts a shadow"
    );
}

#[test]
fn absent_or_disabled_effect_is_byte_identical() {
    let plain = compose(shadow_layer(None));
    let disabled = compose(spec_shadow(&ShadowSpec {
        enabled: false,
        ..Default::default()
    }));
    let not_present = compose(spec_shadow(&ShadowSpec {
        present: false,
        ..Default::default()
    }));
    assert_eq!(plain, disabled);
    assert_eq!(plain, not_present);
}

#[test]
fn off_canvas_shadow_is_bounded_and_a_no_op() {
    // A crafted `Dstn`/`blur`/`Ckmt` clamps to 30000/250/100 and moves the
    // shadow fully off-canvas; the matte and blur must early-out rather than
    // run over the whole canvas.
    let spec = ShadowSpec {
        distance: 1e30,
        size: 1e30,
        spread: 1e30,
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let out = compose_sized(512, spec_shadow(&spec));
    let elapsed = start.elapsed();
    assert_eq!(
        out,
        compose_sized(512, shadow_layer(None)),
        "an off-canvas shadow is a no-op"
    );
    assert!(
        elapsed.as_secs_f64() < 15.0,
        "an off-canvas shadow must early-out ({elapsed:?})"
    );
}

#[test]
fn small_layer_shadow_is_confined_and_far_pixels_are_plain() {
    let spec = ShadowSpec {
        distance: 2.0,
        ..Default::default()
    };
    let with = compose_sized(32, spec_shadow(&spec));
    let plain = compose_sized(32, shadow_layer(None));
    // Content at (3,3)-(6,6); angle 0 casts 2 px left with a hard edge.
    assert_eq!(rgb(&with, 1, 4), [0, 0, 0], "shadow left of the content");
    assert_eq!(rgb(&with, 5, 4), [255, 0, 0], "content unchanged");
    // Outside the effect's reach the result is byte-identical to no effect.
    for y in 10..32 {
        for x in 10..32 {
            assert_eq!(px(&with, x, y), px(&plain, x, y), "far pixel ({x},{y})");
        }
    }
}

#[test]
fn bbox_shadow_matches_expected_adjacent_pixels() {
    // Golden: distance 2, angle 0, size 0, spread 0, black, opaque.
    let spec = ShadowSpec {
        distance: 2.0,
        ..Default::default()
    };
    let out = compose_sized(32, spec_shadow(&spec));
    assert_eq!(rgb(&out, 0, 4), [255, 255, 255], "outside the reach");
    assert_eq!(rgb(&out, 1, 4), [0, 0, 0], "shadow edge");
    assert_eq!(rgb(&out, 2, 4), [0, 0, 0], "shadow core");
    assert_eq!(rgb(&out, 3, 4), [255, 0, 0], "content over the shadow");
    assert_eq!(rgb(&out, 5, 4), [255, 0, 0], "content right edge");
    assert_eq!(rgb(&out, 6, 4), [255, 255, 255], "nothing casts right");
}

#[test]
fn max_size_small_layer_is_bounded() {
    // A legit maximum `size` is confined to the content bbox + reach, not the
    // 1024px canvas. (The wall-clock bound is generous; the work is bounded.)
    let spec = ShadowSpec {
        distance: 5.0,
        size: 250.0,
        spread: 100.0,
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let out = compose_sized(1024, spec_shadow(&spec));
    let elapsed = start.elapsed();
    assert_eq!(out.width, 1024);
    assert!(
        elapsed.as_secs_f64() < 30.0,
        "a small layer at max size is bounded ({elapsed:?})"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_drop_shadow_layer_and_falls_back() {
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
            spec_shadow(&ShadowSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_does_not_reject_a_disabled_or_not_present_effect() {
    for spec in [
        ShadowSpec {
            enabled: false,
            ..Default::default()
        },
        ShadowSpec {
            present: false,
            ..Default::default()
        },
    ] {
        let d = doc(9, 9, vec![spec_shadow(&spec)]);
        assert!(
            !matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
            "an inert effect must not reject the GPU"
        );
    }
}

#[test]
fn gpu_reports_the_effect_for_a_fill_layer() {
    let mut layer = Layer {
        name: "Solid".into(),
        rect: rect(3, 3, 6, 6),
        adjustment: Some(AdjustmentData {
            key: *b"SoCo",
            data: vec![255, 0, 0, 255],
        }),
        ..Default::default()
    };
    layer.extra_blocks = vec![spec_block(&ShadowSpec::default())];
    let d = doc(9, 9, vec![layer]);
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "the effect error wins over UnsupportedAdjustment"
    );
}

// --- Fixture ---------------------------------------------------------------

const DROP_SHADOW_FIXTURE: &[u8] =
    include_bytes!("../../../pictura-codec/tests/fixtures/drop_shadow.psd");

#[test]
fn drop_shadow_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(DROP_SHADOW_FIXTURE).expect("drop_shadow.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Shadowed")
        .expect("Shadowed layer");
    let shadow = decode_drop_shadow(layer).expect("decodes the authored DrSh");
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [10, 20, 30]);
    assert_eq!(shadow.opacity, 75.0);
    assert_eq!(shadow.angle_deg, 120.0);
    assert_eq!(shadow.distance, 5.0);
    assert_eq!(shadow.spread, 0.0);
    assert_eq!(shadow.size, 5.0);
    assert!(!shadow.use_global_angle);
    assert!(!shadow.knocks_out);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the shadow renders");
}

/// The CPU composite is byte-identical whether its rows run serially or on
/// rayon. Representative stacks: a blurred drop shadow (a neighborhood kernel)
/// and a knockout stack (the `cover` canvas).
#[test]
fn parallel_composite_equals_sequential() {
    use crate::composite_rows::FORCE_SERIAL;
    use std::sync::atomic::Ordering;

    let effect = ShadowSpec {
        distance: 1.0,
        size: 2.0,
        ..Default::default()
    };
    let effect_backdrop = solid(
        "backdrop",
        full(9, 9),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let effect_doc = doc(9, 9, vec![effect_backdrop, spec_shadow(&effect)]);
    let backdrop = solid(
        "backdrop",
        full(16, 12),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    let mut mid = solid(
        "mid",
        full(16, 12),
        (0, 255, 0),
        220,
        BlendMode::Multiply,
        200,
    );
    mid.fill = 200;
    let mut top = solid(
        "top",
        full(16, 12),
        (0, 0, 255),
        200,
        BlendMode::Normal,
        255,
    );
    top.fill = 128;
    top.knockout = Knockout::Deep;
    let knockout_doc = doc(16, 12, vec![backdrop, mid, top]);

    let stacks = [effect_doc, knockout_doc];
    for d in stacks {
        FORCE_SERIAL.store(true, Ordering::Relaxed);
        let sequential = composite_rgba(&d);
        FORCE_SERIAL.store(false, Ordering::Relaxed);
        let parallel = composite_rgba(&d);
        assert_eq!(
            sequential.data, parallel.data,
            "parallel rows must equal the sequential composite"
        );
    }
}

mod outer_glow;

mod inner_shadow;

mod inner_glow;

mod stroke;

mod stroke_fills;

mod color_overlay;

mod gradient_overlay;

mod pattern_overlay;

mod satin;

mod bevel;

mod legacy;

mod blend_mode;
