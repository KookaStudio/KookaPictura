use super::*;
use crate::composite::{Canvas, Px};
use crate::composite_native::mask_alpha_unit;
use pictura_adjust::{Adjustment, AutoKind, ExposureGamma};
use pictura_core::{LayerMask, Samples, SourceChannels, SourcePlanes};

fn levels_layer(output_white: u16, opacity: u8) -> Layer {
    let mut data = 2u16.to_be_bytes().to_vec();
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&255u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&output_white.to_be_bytes());
    data.extend_from_slice(&100u16.to_be_bytes());
    adjustment_layer("levels", *b"levl", data, opacity, None)
}

fn base_layer() -> Layer {
    solid(
        "base",
        full(4, 4),
        (100, 140, 200),
        255,
        BlendMode::Normal,
        255,
    )
}

/// A depth-16 document with a covering pixel layer plus the Levels adjustment
/// layer at `opacity`.
fn levels_opacity_doc(output_white: u16, opacity: u8) -> Document {
    let mut d = doc(
        4,
        4,
        vec![base_layer(), levels_layer(output_white, opacity)],
    );
    d.source_depth = Some(BitDepth::Sixteen);
    d
}

fn native_u16(d: &Document) -> Vec<u16> {
    match composite_native(d) {
        Some(Samples::U16(v)) => v,
        other => panic!("expected a u16 native composite, got {other:?}"),
    }
}

#[test]
fn depth16_native_composite_keeps_sub_8bit_adjustment_samples() {
    let d = levels_opacity_doc(200, 255);

    let plane = (d.width * d.height) as usize;
    let v = native_u16(&d);
    assert_eq!(v.len(), plane * 4);

    let rgba = composite_rgba(&d);
    let differs = (0..3).any(|c| {
        let eight = &rgba.data[c * plane..(c + 1) * plane];
        (0..plane).any(|i| v[c * plane + i] != eight[i] as u16 * 257)
    });
    assert!(
        differs,
        "native adjustment must not collapse to the 8-bit widen"
    );
}

#[test]
fn adjustment_opacity_is_honored_at_native_depth() {
    let mut base_doc = doc(4, 4, vec![base_layer()]);
    base_doc.source_depth = Some(BitDepth::Sixteen);
    let base = native_u16(&base_doc);
    let full = native_u16(&levels_opacity_doc(200, 255));
    let mid = native_u16(&levels_opacity_doc(200, 128));

    let plane = (base_doc.width * base_doc.height) as usize;
    for c in 0..3 {
        for i in 0..plane {
            let at = c * plane + i;
            let (b, f, m) = (base[at], full[at], mid[at]);
            assert_ne!(
                m, f,
                "opacity 128 must differ from opacity 255 (channel {c}, px {i})"
            );
            assert_ne!(m, b, "opacity 128 must differ from the unadjusted backdrop");
            assert!(
                b.min(f) <= m && m <= b.max(f),
                "opacity 128 must land between backdrop {b} and full {f} (got {m})"
            );
        }
    }
}

#[test]
fn native_opacity_255_matches_the_8bit_composite_within_one_lsb() {
    let d = levels_opacity_doc(200, 255);
    let narrowed = composite_native(&d)
        .expect("depth-16 composite")
        .narrow_to_u8();
    let rgba = composite_rgba(&d);
    assert_eq!(narrowed.len(), rgba.data.len());
    for (i, (native, eight)) in narrowed.iter().zip(rgba.data.iter()).enumerate() {
        assert!(
            (*native as i32 - *eight as i32).abs() <= 1,
            "sample {i}: native {native} vs 8-bit {eight}"
        );
    }
}

#[test]
fn composite_native_is_none_without_depth_and_f32_at_32() {
    let d = doc(
        2,
        2,
        vec![solid(
            "base",
            full(2, 2),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    assert!(composite_native(&d).is_none());

    let mut d32 = d.clone();
    d32.source_depth = Some(BitDepth::ThirtyTwo);
    match composite_native(&d32) {
        Some(Samples::F32(v)) => assert_eq!(v.len(), 2 * 2 * 4),
        other => panic!("depth-32 document must yield f32 samples, got {other:?}"),
    }
}

#[test]
fn depth32_conversion_to_16_tones_above_one_and_keeps_extras() {
    let mut d = doc(1, 1, Vec::new());
    d.source_depth = Some(BitDepth::ThirtyTwo);
    d.source_planes = Some(SourcePlanes {
        depth: BitDepth::ThirtyTwo,
        width: 1,
        height: 1,
        samples: Samples::F32(vec![4.0, 0.5, 0.25, 0.9]),
    });

    // gain = 2^-2 = 0.25; 4.0 -> 1.0 -> 65535. A pre-clamp at 1.0 would give
    // 0.25 -> 16384, so the retained store (not `emit_native`) supplied the 4.0.
    convert_depth_exposure_gamma(
        &mut d,
        BitDepth::Sixteen,
        ExposureGamma {
            exposure_ev: -2.0,
            gamma: 1.0,
        },
    )
    .unwrap();

    assert_eq!(d.depth, BitDepth::Eight, "the working model stays 8-bit");
    assert_eq!(d.source_depth, Some(BitDepth::Sixteen));
    assert!(d.retains_source_depth());
    let Samples::U16(v) = &d.source_planes.as_ref().unwrap().samples else {
        panic!("expected a u16 store, got {:?}", d.source_planes);
    };
    assert_eq!(v[0], 65535, "4.0 tone-mapped then clamped");
    assert_ne!(v[0], 16384, "the pre-clamped 1.0 would quantize to 16384");
    assert_eq!(v[1], 8192, "0.5 * 0.25 -> 8192");
    assert_eq!(v[2], 4096, "0.25 * 0.25 -> 4096");
    assert_eq!(v[3], 58982, "the extra plane is copied untoned");
    assert_eq!(d.composite.channels, 3);
    assert_eq!(d.composite.data, vec![255, 32, 16]);
}

#[test]
fn depth32_conversion_to_8_clears_the_retained_store() {
    let mut d = doc(1, 1, Vec::new());
    d.source_depth = Some(BitDepth::ThirtyTwo);
    d.source_planes = Some(SourcePlanes {
        depth: BitDepth::ThirtyTwo,
        width: 1,
        height: 1,
        samples: Samples::F32(vec![0.0, 0.5, 1.0]),
    });

    convert_depth_exposure_gamma(&mut d, BitDepth::Eight, ExposureGamma::default()).unwrap();

    assert_eq!(d.depth, BitDepth::Eight);
    assert_eq!(d.source_depth, None);
    assert!(d.source_planes.is_none());
    assert!(!d.retains_source_depth());
    assert_eq!(d.composite.data, vec![0, 128, 255]);
}

#[test]
fn conversion_refusals_leave_the_document_untouched() {
    let mut eight = doc(1, 1, Vec::new());
    let before = eight.clone();
    assert!(
        convert_depth_exposure_gamma(&mut eight, BitDepth::Sixteen, ExposureGamma::default())
            .is_err()
    );
    assert_eq!(eight, before, "an 8-bit source is refused");

    let mut converted = doc(1, 1, Vec::new());
    converted.source_depth = Some(BitDepth::ThirtyTwo);
    converted.source_mode = Some(ColorMode::Cmyk);
    let before = converted.clone();
    assert!(convert_depth_exposure_gamma(
        &mut converted,
        BitDepth::Sixteen,
        ExposureGamma::default()
    )
    .is_err());
    assert_eq!(converted, before, "a converted mode is refused");

    let mut thirty_two = doc(1, 1, Vec::new());
    thirty_two.source_depth = Some(BitDepth::ThirtyTwo);
    thirty_two.source_planes = Some(SourcePlanes {
        depth: BitDepth::ThirtyTwo,
        width: 1,
        height: 1,
        samples: Samples::F32(vec![0.5, 0.5, 0.5]),
    });
    let before = thirty_two.clone();
    assert!(convert_depth_exposure_gamma(
        &mut thirty_two,
        BitDepth::ThirtyTwo,
        ExposureGamma::default()
    )
    .is_err());
    assert_eq!(thirty_two, before, "a 32-bit output is refused");
}

#[test]
fn auto_adjustment_on_depth16_uses_the_native_path() {
    let mut d = doc(4, 4, Vec::new());
    d.source_depth = Some(BitDepth::Sixteen);

    let mut canvas = Canvas::new(4, 4);
    for (i, p) in canvas.px.iter_mut().enumerate() {
        let v = (80.0 + i as f32 * 6.0) / 255.0;
        *p = Px {
            r: v,
            g: v,
            b: v,
            a: 1.0,
        };
    }
    let before: Vec<f32> = canvas.px.iter().map(|p| p.r).collect();

    let layer = adjustment_layer("auto", *b"auto", Vec::new(), 255, None);
    crate::composite_native::composite_adjustment(
        &mut canvas,
        &layer,
        &d,
        &Adjustment::Auto(AutoKind::Color),
    );

    assert!(
        canvas
            .px
            .iter()
            .zip(&before)
            .any(|(p, b)| (p.r - b).abs() > 1.0 / 255.0),
        "auto is in the native set and must adjust the depth-16 canvas"
    );
}

/// A 4x4 pixel layer whose retained native planes are the 8-bit values plus
/// `low`, so they are deliberately *not* the `v * 257` widening.
fn native_store_layer(rgb: (u8, u8, u8), low: u16) -> Layer {
    let mut layer = solid("base", full(4, 4), rgb, 255, BlendMode::Normal, 255);
    let n = 16;
    let plane = |v: u8| Samples::U16(vec![v as u16 * 257 + low; n]);
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![
            (0, plane(rgb.0)),
            (1, plane(rgb.1)),
            (2, plane(rgb.2)),
            (-1, Samples::U16(vec![65535; n])),
        ],
    ));
    layer
}

fn assert_native_is_widen(d: &Document) {
    let native = native_u16(d);
    let rgba = composite_rgba(d);
    let plane = (d.width * d.height) as usize;
    for c in 0..4 {
        for i in 0..plane {
            assert_eq!(
                native[c * plane + i],
                rgba.data[c * plane + i] as u16 * 257,
                "channel {c} px {i} must match the 8-bit widening"
            );
        }
    }
}

#[test]
fn depth16_rgb_layer_keeps_native_content() {
    let mut d = doc(4, 4, vec![native_store_layer((100, 140, 200), 7)]);
    d.source_depth = Some(BitDepth::Sixteen);

    let native = native_u16(&d);
    let rgba = composite_rgba(&d);
    let plane = (d.width * d.height) as usize;
    let differs = (0..3).any(|c| {
        (0..plane).any(|i| native[c * plane + i] != rgba.data[c * plane + i] as u16 * 257)
    });
    assert!(
        differs,
        "native layer content must not collapse to the 8-bit widening"
    );
}

#[test]
fn converted_mode_document_uses_the_8bit_path() {
    let mut d = doc(4, 4, vec![native_store_layer((100, 140, 200), 7)]);
    d.source_depth = Some(BitDepth::Sixteen);
    d.source_mode = Some(ColorMode::Cmyk);
    assert_native_is_widen(&d);
}

#[test]
fn depth16_layer_without_matching_store_falls_back() {
    let mut layer = native_store_layer((100, 140, 200), 7);
    layer.source_channels.as_mut().unwrap().rect = rect(0, 0, 2, 2);
    let mut d = doc(4, 4, vec![layer]);
    d.source_depth = Some(BitDepth::Sixteen);
    assert_native_is_widen(&d);

    let mut none = doc(
        4,
        4,
        vec![solid(
            "base",
            full(4, 4),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    none.source_depth = Some(BitDepth::Sixteen);
    assert_native_is_widen(&none);
}

/// A 1x1 red layer carrying `mask_byte` as its 8-bit mask, and a native `-2`
/// sample when `native` is `Some`.
fn masked_red(mask_byte: u8, native: Option<u16>) -> Layer {
    let mut layer = solid(
        "masked",
        full(1, 1),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    layer.mask = Some(LayerMask {
        rect: full(1, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![mask_byte].into()),
        ..Default::default()
    });
    if let Some(sample) = native {
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            layer.rect,
            vec![(-2, Samples::U16(vec![sample]))],
        ));
    }
    layer
}

#[test]
fn depth16_native_mask_gates_differently_from_the_widened_8bit_mask() {
    let mut native_doc = doc(1, 1, vec![masked_red(50, Some(65000))]);
    native_doc.source_depth = Some(BitDepth::Sixteen);
    let mut widened_doc = doc(1, 1, vec![masked_red(50, None)]);
    widened_doc.source_depth = Some(BitDepth::Sixteen);

    assert_ne!(
        composite_rgba(&native_doc).data,
        composite_rgba(&widened_doc).data,
        "the native mask must gate differently from the widened 8-bit mask"
    );
}

#[test]
fn mask_alpha_unit_matches_the_eight_bit_fallbacks() {
    let plain = solid("l", full(1, 1), (10, 20, 30), 255, BlendMode::Normal, 255);
    let mut d = doc(1, 1, vec![plain.clone()]);
    d.source_depth = Some(BitDepth::Sixteen);

    assert_eq!(
        mask_alpha_unit(&d, &plain, 0, 0),
        1.0,
        "no mask is full coverage"
    );

    let mut disabled = plain.clone();
    disabled.mask = Some(LayerMask {
        rect: full(1, 1),
        default_color: 0,
        disabled: true,
        flags: 0,
        data: Some(vec![0].into()),
        ..Default::default()
    });
    assert_eq!(mask_alpha_unit(&d, &disabled, 0, 0), 1.0, "disabled mask");

    let mut no_data = plain.clone();
    no_data.mask = Some(LayerMask {
        rect: full(1, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: None,
        ..Default::default()
    });
    assert_eq!(mask_alpha_unit(&d, &no_data, 0, 0), 1.0, "data-less mask");

    let mut out_of_rect = plain.clone();
    out_of_rect.mask = Some(LayerMask {
        rect: rect(5, 5, 6, 6),
        default_color: 200,
        disabled: false,
        flags: 0,
        data: Some(vec![7].into()),
        ..Default::default()
    });
    assert_eq!(
        mask_alpha_unit(&d, &out_of_rect, 0, 0),
        200.0 / 255.0,
        "outside the mask rect uses the default colour"
    );

    let eight = doc(1, 1, vec![plain.clone()]);
    assert_eq!(
        mask_alpha_unit(&eight, &plain, 0, 0),
        crate::composite::mask_alpha(&plain, full(1, 1), 0, 0) as f32 / 255.0,
        "an 8-bit document keeps the rounded mask_alpha path"
    );
}
