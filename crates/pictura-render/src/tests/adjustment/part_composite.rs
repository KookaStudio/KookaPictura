use super::*;

#[test]
fn invert_adjustment_layer_matches_flattened() {
    // Varied RGB backdrop so Invert is not a uniform all-zero/all-one case.
    let base = solid(
        "base",
        full(4, 2),
        (30, 90, 210),
        255,
        BlendMode::Normal,
        255,
    );
    let with_adj = doc(
        4,
        2,
        vec![
            base.clone(),
            adjustment_layer("invert", *b"nvrt", Vec::new(), 255, None),
        ],
    );
    let out = composite_rgba(&with_adj);
    // Expected: apply the destructive Adjustment to the flattened composite.
    let mut flat = composite_rgba(&doc(4, 2, vec![base]));
    pictura_adjust::apply(&Adjustment::Invert, &mut flat).unwrap();
    for y in 0..2 {
        for x in 0..4 {
            let want = rgb(&flat, x, y);
            let got = rgb(&out, x, y);
            for c in 0..3 {
                let d = got[c] as i16 - want[c] as i16;
                assert!(d.abs() <= 1, "at {x},{y}.{c}: got {got:?} want {want:?}");
            }
        }
    }
}

#[test]
fn adjustment_layer_mask_and_opacity_gate() {
    // 2x1: mask fully reveals x=0, hides x=1.
    let base = solid(
        "base",
        full(2, 1),
        (100, 100, 100),
        255,
        BlendMode::Normal,
        255,
    );
    let mut masked = adjustment_layer("invert", *b"nvrt", Vec::new(), 255, None);
    masked.mask = Some(LayerMask {
        rect: full(2, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![255, 0].into()),
        ..Default::default()
    });
    let out = composite_rgba(&doc(2, 1, vec![base, masked]));
    assert_eq!(rgb(&out, 0, 0), [155, 155, 155], "unmasked pixel inverts");
    assert_eq!(
        rgb(&out, 1, 0),
        [100, 100, 100],
        "masked-out pixel unchanged"
    );

    // Opacity 128 lerps about halfway to the inverted value.
    let base = solid(
        "base",
        full(1, 1),
        (100, 100, 100),
        255,
        BlendMode::Normal,
        255,
    );
    let out = composite_rgba(&doc(
        1,
        1,
        vec![
            base,
            adjustment_layer("invert", *b"nvrt", Vec::new(), 128, None),
        ],
    ));
    let got = rgb(&out, 0, 0)[0] as i16;
    assert!(
        (got - 128).abs() <= 1,
        "50% opacity should land near 128, got {got}"
    );
}

#[test]
fn adjustment_ignores_fill() {
    let base = || {
        solid(
            "base",
            full(1, 1),
            (100, 100, 100),
            255,
            BlendMode::Normal,
            255,
        )
    };
    for fill in [0, 128, 255] {
        let mut adj = adjustment_layer("invert", *b"nvrt", Vec::new(), 255, None);
        adj.fill = fill;
        let scene = doc(1, 1, vec![base(), adj]);
        let out = composite_rgba(&scene);
        assert_eq!(rgb(&out, 0, 0), [155, 155, 155], "fill {fill} is ignored");
        let gpu = composite_gpu_or_cpu(&scene);
        for (i, (a, b)) in out.data.iter().zip(gpu.data.iter()).enumerate() {
            assert!(
                (*a as i16 - *b as i16).abs() <= 1,
                "GPU parity at sample {i}: {a} vs {b}"
            );
        }
    }

    // Opacity still gates the adjustment; Fill stays inert.
    let mut half = adjustment_layer("invert", *b"nvrt", Vec::new(), 128, None);
    half.fill = 0;
    let out = composite_rgba(&doc(1, 1, vec![base(), half]));
    let got = rgb(&out, 0, 0)[0] as i16;
    assert!(
        (got - 128).abs() <= 1,
        "opacity 128 lands near 128, got {got}"
    );

    let mut zero = adjustment_layer("invert", *b"nvrt", Vec::new(), 0, None);
    zero.fill = 0;
    let out = composite_rgba(&doc(1, 1, vec![base(), zero]));
    assert_eq!(rgb(&out, 0, 0), [100, 100, 100], "opacity 0 is inert");
}

#[test]
fn fill_layer_keeps_fill() {
    let mut fill = adjustment_layer(
        "fill",
        *b"SoCo",
        crate::encode_solid_color_fill([255, 0, 0]).data.to_vec(),
        255,
        None,
    );
    fill.rect = full(1, 1);
    fill.fill = 128;
    let out = composite_rgba(&doc(1, 1, vec![fill]));
    assert_eq!(
        px(&out, 0, 0),
        [255, 0, 0, 128],
        "fill content still honours Fill"
    );
}

#[test]
fn unknown_adjustment_key_is_noop() {
    let base = solid(
        "base",
        full(2, 2),
        (10, 200, 60),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 2, vec![base.clone()]));
    let with_unknown = composite_rgba(&doc(
        2,
        2,
        vec![
            base,
            adjustment_layer("lookup", *b"clrL", vec![1, 2, 3], 255, None),
        ],
    ));
    assert_eq!(
        with_unknown.data, plain.data,
        "undecodable key must be a no-op"
    );
}

/// A `LUT_3D_SIZE 2` cube whose node `(r, g, b)` maps to `(1 - r, 1 - g, 1 - b)`.
fn inverting_cube() -> Vec<u8> {
    let mut s = String::from("LUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                s.push_str(&format!("{}.0 {}.0 {}.0\n", 1 - r, 1 - g, 1 - b));
            }
        }
    }
    s.into_bytes()
}

#[test]
fn color_lookup_identity_is_neutral_and_a_cube_changes_the_composite() {
    let base = solid(
        "base",
        full(2, 2),
        (10, 200, 60),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 2, vec![base.clone()]));
    let identity = adjustment_layer(
        "lookup",
        *b"clrL",
        crate::encode_color_lookup(&crate::identity_cube(), "Identity")
            .data
            .to_vec(),
        255,
        None,
    );
    let neutral = composite_rgba(&doc(2, 2, vec![base.clone(), identity]));
    assert_eq!(
        neutral.data, plain.data,
        "an identity cube must not change the backdrop"
    );

    let inverting = adjustment_layer(
        "lookup",
        *b"clrL",
        crate::encode_color_lookup(&inverting_cube(), "Invert")
            .data
            .to_vec(),
        255,
        None,
    );
    let changed = composite_rgba(&doc(2, 2, vec![base, inverting]));
    assert_ne!(
        changed.data, plain.data,
        "a non-identity cube must change the backdrop"
    );
}

#[test]
fn descriptor_solid_fill_layer_composites_to_its_color() {
    let mut fill = adjustment_layer(
        "fill",
        *b"SoCo",
        encode_solid_color_fill([10, 20, 30]).data.to_vec(),
        255,
        None,
    );
    fill.rect = full(2, 2);
    let out = composite_rgba(&doc(2, 2, vec![fill]));
    assert_eq!(rgb(&out, 0, 0), [10, 20, 30]);
    assert_eq!(rgb(&out, 1, 1), [10, 20, 30]);
    assert_eq!(px(&out, 1, 1)[3], 255, "descriptor fills are opaque");
}

#[test]
fn hidden_decoded_adjustment_is_noop() {
    // A committed key (expA) decodes, but a fully hidden mask must gate it off
    // exactly like the existing `nvrt` path.
    let base = solid(
        "base",
        full(2, 1),
        (120, 60, 200),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone()]));
    let mut hidden = adjustment_layer(
        "exposure",
        *b"expA",
        exposure_payload(2.0, 0.5, 0.5),
        255,
        None,
    );
    hidden.mask = Some(LayerMask {
        rect: full(2, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![0, 0].into()),
        ..Default::default()
    });
    let out = composite_rgba(&doc(2, 1, vec![base, hidden]));
    assert_eq!(
        out.data, plain.data,
        "a fully masked decoded adjustment layer must be a no-op"
    );
}

#[test]
fn decoded_adjustment_changes_backdrop() {
    let base = solid(
        "base",
        full(2, 2),
        (120, 60, 200),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 2, vec![base.clone()]));
    let adjusted = composite_rgba(&doc(
        2,
        2,
        vec![
            base,
            adjustment_layer(
                "exposure",
                *b"expA",
                exposure_payload(1.0, 0.0, 1.0),
                255,
                None,
            ),
        ],
    ));
    assert_ne!(
        adjusted.data, plain.data,
        "a decoded expA adjustment layer must change the backdrop"
    );
}

#[test]
fn photo_filter_layer_warms_and_preserves_luminance() {
    // Non-uniform backdrop: two different neutral grays.
    let base = solid(
        "base",
        full(2, 1),
        (120, 120, 120),
        255,
        BlendMode::Normal,
        255,
    );
    let patch = solid(
        "patch",
        rect(0, 0, 1, 1),
        (80, 80, 80),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone(), patch.clone()]));
    let filtered = composite_rgba(&doc(
        2,
        1,
        vec![
            base,
            patch,
            adjustment_layer(
                "photo-filter",
                *b"phfl",
                encode_photo_filter([255, 180, 80], 25.0, true)
                    .data
                    .to_vec(),
                255,
                None,
            ),
        ],
    ));
    assert_ne!(
        filtered.data, plain.data,
        "a Photo Filter layer must change the backdrop"
    );
    for x in 0..2 {
        let got = rgb(&filtered, x, 0);
        assert!(
            got[0] > got[2],
            "warming filter must give red > blue: {got:?}"
        );
        let before = rgb(&plain, x, 0);
        let luma = |c: [u8; 3]| 0.299 * c[0] as f64 + 0.587 * c[1] as f64 + 0.114 * c[2] as f64;
        assert!(
            (luma(got) - luma(before)).abs() <= 2.0,
            "luminance must be preserved at {x}: {:?} -> {:?}",
            before,
            got
        );
    }
}

#[test]
fn gradient_map_layer_changes_non_uniform_backdrop() {
    // Non-uniform, non-neutral backdrop: black-to-white maps each pixel to the
    // grey of its luminance, changing the colour.
    let base = solid(
        "base",
        full(2, 1),
        (30, 90, 210),
        255,
        BlendMode::Normal,
        255,
    );
    let patch = solid(
        "patch",
        rect(0, 0, 1, 1),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone(), patch.clone()]));
    let stops = [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ];
    let graded = composite_rgba(&doc(
        2,
        1,
        vec![
            base,
            patch,
            adjustment_layer(
                "gradient-map",
                *b"grdm",
                encode_gradient_map(
                    &GradientMapParams {
                        stops: stops.to_vec(),
                        reverse: false,
                        transparency: Vec::new(),
                    },
                    false,
                )
                .data
                .to_vec(),
                255,
                None,
            ),
        ],
    ));
    assert_ne!(
        graded.data, plain.data,
        "a Gradient Map layer must change the backdrop"
    );
    for x in 0..2 {
        let before = rgb(&plain, x, 0);
        let got = rgb(&graded, x, 0);
        let luma = 0.299 * before[0] as f64 + 0.587 * before[1] as f64 + 0.114 * before[2] as f64;
        assert!(
            got[0].abs_diff(got[1]) <= 1 && got[1].abs_diff(got[2]) <= 1,
            "black-to-white output must be neutral at {x}: {got:?}"
        );
        assert!(
            (got[0] as f64 - luma).abs() <= 2.0,
            "the grey should follow the backdrop's luminance at {x}: {before:?} -> {got:?}"
        );
    }
}

#[test]
fn color_balance_layer_changes_non_uniform_backdrop() {
    // Non-uniform, non-neutral backdrop so the luma-weighted bands differ.
    let base = solid(
        "base",
        full(2, 1),
        (30, 90, 210),
        255,
        BlendMode::Normal,
        255,
    );
    let patch = solid(
        "patch",
        rect(0, 0, 1, 1),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone(), patch.clone()]));
    let balanced = composite_rgba(&doc(
        2,
        1,
        vec![
            base,
            patch,
            adjustment_layer(
                "color-balance",
                *b"blnc",
                encode_color_balance([0.0; 3], [25.0, 0.0, 0.0], [0.0; 3], true)
                    .data
                    .to_vec(),
                255,
                None,
            ),
        ],
    ));
    assert_ne!(
        balanced.data, plain.data,
        "a non-neutral Color Balance layer must change the backdrop"
    );
}

#[test]
fn channel_mixer_layer_changes_non_uniform_backdrop() {
    let base = solid(
        "base",
        full(2, 1),
        (30, 90, 210),
        255,
        BlendMode::Normal,
        255,
    );
    let patch = solid(
        "patch",
        rect(0, 0, 1, 1),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone(), patch.clone()]));
    let mixed = composite_rgba(&doc(
        2,
        1,
        vec![
            base,
            patch,
            adjustment_layer(
                "channel-mixer",
                *b"mixr",
                encode_channel_mixer(
                    false,
                    [0.0, 100.0, 0.0],
                    [0.0, 100.0, 0.0],
                    [0.0, 0.0, 100.0],
                    [0.0; 3],
                )
                .data
                .to_vec(),
                255,
                None,
            ),
        ],
    ));
    assert_ne!(
        mixed.data, plain.data,
        "a non-neutral Channel Mixer layer must change the backdrop"
    );
}

#[test]
fn curves_layer_changes_backdrop_and_per_channel_isolates() {
    let base = solid(
        "base",
        full(2, 1),
        (100, 100, 100),
        255,
        BlendMode::Normal,
        255,
    );
    let patch = solid(
        "patch",
        rect(0, 0, 1, 1),
        (60, 140, 200),
        255,
        BlendMode::Normal,
        255,
    );
    let plain = composite_rgba(&doc(2, 1, vec![base.clone(), patch.clone()]));

    let points = [(0u8, 0u8), (64, 32), (192, 224), (255, 255)];
    let composite = encode_curves(&points, None, None, None).data;
    let graded = composite_rgba(&doc(
        2,
        1,
        vec![
            base.clone(),
            patch.clone(),
            adjustment_layer("curves", *b"curv", composite.to_vec(), 255, None),
        ],
    ));
    assert_ne!(
        graded.data, plain.data,
        "a non-identity Curves layer must change the backdrop"
    );

    let red = [(0u8, 0u8), (128, 255), (255, 255)];
    let red_only = encode_curves(&[(0, 0), (255, 255)], Some(&red), None, None).data;
    let adjusted = composite_rgba(&doc(
        2,
        1,
        vec![
            base,
            patch,
            adjustment_layer("red-curve", *b"curv", red_only.to_vec(), 255, None),
        ],
    ));
    for x in 0..2 {
        let before = rgb(&plain, x, 0);
        let after = rgb(&adjusted, x, 0);
        assert_eq!(before[1], after[1], "green untouched at {x}");
        assert_eq!(before[2], after[2], "blue untouched at {x}");
        assert!(after[0] > before[0], "red follows its curve at {x}");
    }
}
