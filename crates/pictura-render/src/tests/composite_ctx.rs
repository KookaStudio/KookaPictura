use super::composite::{type_layer, type_style};
use super::smart_object::{embedded, payload_of, smart_layer};
use super::*;
use crate::{GradientKind, GradientStop};
use pictura_core::{
    BlendIf, CrsSettings, PicturaRawSettings, Samples, SourceChannels, VectorFillRule, VectorMask,
    VectorSubpath,
};

// --- Per-layer blend context ---------------------------------------------

/// Composite the same document through the resolved context and through the
/// pre-change per-pixel body.
fn assert_ctx_matches_legacy(d: &Document) {
    crate::composite_blend::set_legacy_blend(false);
    let new = composite_rgba(d);
    crate::composite_blend::set_legacy_blend(true);
    let old = composite_rgba(d);
    crate::composite_blend::set_legacy_blend(false);
    assert_eq!(
        new.data, old.data,
        "the context kernel differs from the legacy body"
    );
}

fn square_vector(feather: f64) -> VectorMask {
    let p = |x: i32, y: i32| [x * 256, y * 256];
    let mut mask = VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![p(0, 0), p(1, 0), p(1, 2), p(0, 2)],
        }],
        ..Default::default()
    };
    mask.set_feather(feather);
    mask
}

fn masked_scene(mask: Option<LayerMask>, vector: Option<VectorMask>) -> Document {
    let mut layer = solid("top", full(2, 2), (255, 0, 0), 255, BlendMode::Normal, 255);
    layer.mask = mask;
    layer.vector_mask = vector;
    doc(
        2,
        2,
        vec![
            solid(
                "base",
                full(2, 2),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    )
}

#[test]
fn ctx_kernel_matches_legacy() {
    let _guard = crate::composite_rows::serial_test_lock();
    crate::composite_blend::set_legacy_blend(false);
    let clip = full(1, 1);
    let d = doc(1, 1, Vec::new());
    let mut seed = 0x1234_5678u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / 16_777_216.0
    };
    for mode in BlendMode::LAYER_MODES {
        let mut layer = solid("top", clip, (0, 0, 0), 255, mode, 200);
        layer.fill = 64;
        let ctx = crate::composite_blend::BlendCtx::new(&layer, &d, clip);
        for _ in 0..16 {
            let backdrop = crate::composite::Px {
                r: next(),
                g: next(),
                b: next(),
                a: next(),
            };
            let cs = [next(), next(), next()];
            let a = next();
            let mut new = [backdrop];
            let mut old = [backdrop];
            crate::composite_blend::blend_into_row_ctx(
                &mut new, None, 0, &ctx, &layer, &d, clip, 0, 0, cs, a,
            );
            crate::composite_blend::blend_into_row_legacy(
                &mut old, None, 0, &layer, &d, clip, 0, 0, cs, a,
            );
            for (n, o) in [
                (new[0].r, old[0].r),
                (new[0].g, old[0].g),
                (new[0].b, old[0].b),
                (new[0].a, old[0].a),
            ] {
                assert_eq!(n.to_bits(), o.to_bits(), "{mode:?} channel bits");
            }
        }
    }
}

#[test]
fn zero_alpha_skips_mask() {
    let _guard = crate::composite_rows::serial_test_lock();
    let mut layer = solid("top", full(4, 4), (255, 0, 0), 0, BlendMode::Normal, 255);
    let mut mask = LayerMask {
        rect: full(4, 4),
        default_color: 0,
        data: Some(vec![255; 16].into()),
        ..Default::default()
    };
    mask.set_feather(2.0);
    layer.mask = Some(mask);
    let d = doc(
        4,
        4,
        vec![
            solid(
                "base",
                full(4, 4),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    );
    crate::composite_blend::reset_mask_samples();
    let out = composite_rgba(&d);
    assert_eq!(
        crate::composite_blend::mask_samples(),
        0,
        "a zero-alpha layer must not sample its mask"
    );
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(px(&out, x, y), [10, 20, 30, 255], "at {x},{y}");
        }
    }
}

#[test]
fn zero_alpha_does_not_mark_cover() {
    let _guard = crate::composite_rows::serial_test_lock();
    crate::composite_blend::set_legacy_blend(false);
    let d = doc(1, 1, Vec::new());
    let layer = solid("top", full(1, 1), (255, 0, 0), 0, BlendMode::Normal, 255);
    let clip = full(1, 1);
    let ctx = crate::composite_blend::BlendCtx::new(&layer, &d, clip);
    let mut row = [crate::composite::Px {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    }];
    let mut cover = [false];
    crate::composite_blend::blend_into_row_ctx(
        &mut row,
        Some(&mut cover[..]),
        0,
        &ctx,
        &layer,
        &d,
        clip,
        0,
        0,
        [1.0, 0.0, 0.0],
        0.0,
    );
    assert!(!cover[0], "a zero-alpha pixel is not covered");
    assert_eq!(row[0].r.to_bits(), 0.1f32.to_bits());
}

fn native_layer_16(rgb: (u8, u8, u8), low: u16) -> Layer {
    let mut layer = solid("base", full(2, 2), rgb, 255, BlendMode::Normal, 255);
    let plane = |v: u8| Samples::U16(vec![v as u16 * 257 + low; 4].into());
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        layer.rect,
        vec![
            (0, plane(rgb.0)),
            (1, plane(rgb.1)),
            (2, plane(rgb.2)),
            (-1, Samples::U16(vec![65535; 4].into())),
        ],
    ));
    layer
}

fn native_layer_32(rgb: [f32; 3]) -> Layer {
    let mut layer = solid("base", full(2, 2), (0, 0, 0), 255, BlendMode::Normal, 255);
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::ThirtyTwo,
        layer.rect,
        vec![
            (0, Samples::F32(vec![rgb[0]; 4].into())),
            (1, Samples::F32(vec![rgb[1]; 4].into())),
            (2, Samples::F32(vec![rgb[2]; 4].into())),
            (-1, Samples::F32(vec![1.0; 4].into())),
        ],
    ));
    layer
}

#[test]
fn ctx_high_depth_matches_legacy() {
    let _guard = crate::composite_rows::serial_test_lock();
    let mut d16 = doc(2, 2, vec![native_layer_16((100, 140, 200), 7)]);
    d16.source_depth = Some(BitDepth::Sixteen);
    let mut d16_missing = doc(2, 2, vec![native_layer_16((100, 140, 200), 7)]);
    d16_missing.source_depth = Some(BitDepth::Sixteen);
    d16_missing.layers[0]
        .source_channels
        .as_mut()
        .unwrap()
        .planes
        .retain(|(id, _)| *id != 1);
    let mut d32 = doc(2, 2, vec![native_layer_32([0.4, 0.6, 0.8])]);
    d32.source_depth = Some(BitDepth::ThirtyTwo);

    for d in [&d16, &d16_missing, &d32] {
        crate::composite_blend::set_legacy_source(false);
        let new = composite_rgba(d);
        crate::composite_blend::set_legacy_source(true);
        let old = composite_rgba(d);
        crate::composite_blend::set_legacy_source(false);
        assert_eq!(new.data, old.data, "native source sampling differs");
    }
}

#[test]
fn ctx_masks_match_legacy() {
    let _guard = crate::composite_rows::serial_test_lock();
    let mut raster = LayerMask {
        rect: full(2, 2),
        default_color: 0,
        data: Some(vec![0, 255, 128, 64].into()),
        ..Default::default()
    };
    raster.set_feather(1.5);
    let plain_raster = LayerMask {
        rect: full(2, 2),
        default_color: 0,
        data: Some(vec![0, 255, 128, 64].into()),
        ..Default::default()
    };
    let mut native_mask = LayerMask {
        rect: full(2, 2),
        default_color: 0,
        data: Some(vec![50; 4].into()),
        ..Default::default()
    };
    native_mask.density = 255;
    let mut native_layer = solid("top", full(2, 2), (255, 0, 0), 255, BlendMode::Normal, 255);
    native_layer.mask = Some(native_mask);
    native_layer.vector_mask = Some(square_vector(0.0));
    native_layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        native_layer.rect,
        vec![(-2, Samples::U16(vec![65000; 4].into()))],
    ));
    let mut native_doc = doc(
        2,
        2,
        vec![
            solid(
                "base",
                full(2, 2),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            native_layer,
        ],
    );
    native_doc.source_depth = Some(BitDepth::Sixteen);

    for d in [
        masked_scene(Some(plain_raster), None),
        masked_scene(Some(raster), None),
        masked_scene(None, Some(square_vector(1.5))),
        masked_scene(None, Some(square_vector(0.0))),
        native_doc,
    ] {
        assert_ctx_matches_legacy(&d);
    }
}

#[test]
fn ctx_constants_match_legacy() {
    let _guard = crate::composite_rows::serial_test_lock();
    let mut layer = solid(
        "top",
        full(2, 2),
        (200, 100, 50),
        255,
        BlendMode::Multiply,
        200,
    );
    layer.fill = 64;
    layer.blend_if = Some(BlendIf {
        composite_source: (1000, 60000),
        composite_dest: (2000, 64000),
        channel_ranges: vec![((0, 65535), (1000, 60000))],
    });
    layer.type_tool = type_layer(None).type_tool;
    let d = doc(
        2,
        2,
        vec![
            solid(
                "base",
                full(2, 2),
                (10, 20, 30),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    );
    assert_ctx_matches_legacy(&d);
}

#[test]
fn ctx_route_callers_match_legacy() {
    let _guard = crate::composite_rows::serial_test_lock();
    let base = || {
        solid(
            "base",
            full(2, 2),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )
    };
    let pixel = doc(
        2,
        2,
        vec![
            base(),
            solid(
                "top",
                full(2, 2),
                (200, 100, 50),
                255,
                BlendMode::Multiply,
                200,
            ),
        ],
    );
    let group = doc(
        2,
        2,
        vec![
            base(),
            group(
                "g",
                BlendMode::Multiply,
                200,
                None,
                vec![solid(
                    "child",
                    full(2, 2),
                    (50, 50, 50),
                    255,
                    BlendMode::Normal,
                    255,
                )],
            ),
        ],
    );
    let adjustment = doc(
        2,
        2,
        vec![
            base(),
            adjustment_layer("invert", *b"nvrt", Vec::new(), 200, None),
        ],
    );
    let mut solid_fill = adjustment_layer("fill", *b"SoCo", vec![255, 0, 0, 255], 200, None);
    solid_fill.rect = full(2, 2);
    let solid_fill = doc(2, 2, vec![base(), solid_fill]);
    let mut gradient = adjustment_layer("gradient", *b"GdFl", Vec::new(), 200, None);
    gradient.rect = full(2, 2);
    gradient.adjustment = Some(crate::encode_gradient_fill(
        GradientKind::Linear,
        &[
            GradientStop {
                location: 0,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 4096,
                color: [255, 255, 255],
            },
        ],
        0.0,
    ));
    let gradient = doc(2, 2, vec![base(), gradient]);
    let pattern = pattern_fixture_doc();
    let smart_source = doc(
        2,
        2,
        vec![
            base(),
            smart_layer(
                "smart",
                full(2, 2),
                embedded(payload_of([255, 0, 0])),
                Vec::new(),
            ),
        ],
    );
    let mut filtered = smart_layer(
        "smart",
        full(2, 2),
        embedded(payload_of([200, 100, 50])),
        Vec::new(),
    );
    pictura_codec::attach_pictura_raw_filter(
        &mut filtered,
        &PicturaRawSettings {
            basic: CrsSettings {
                exposure: Some(1.0),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("filter attaches");
    let smart_filter = doc(2, 2, vec![base(), filtered]);
    let type_source = doc(200, 80, vec![type_layer(Some(type_style()))]);

    for d in [
        pixel,
        group,
        adjustment,
        solid_fill,
        gradient,
        pattern,
        smart_source,
        smart_filter,
        type_source,
    ] {
        assert_ctx_matches_legacy(&d);
    }
}
