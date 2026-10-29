use super::*;

fn ebbl(items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: b"ebbl".to_vec(),
        items,
    }
}

fn enumerated(kind: &[u8], value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: kind.to_vec(),
        value: value.to_vec(),
    }
}

/// A beveled 6x6 pixel layer at `(3, 3)` on a 12x12 canvas, leaving a margin so
/// the interior band has room.
fn bevel_layer(block: Option<LayerBlock>) -> Layer {
    let mut layer = solid(
        "Beveled",
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

fn compose(bevel: Layer) -> PixelBuffer {
    let backdrop = solid(
        "Backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    composite_rgba(&doc(12, 12, vec![backdrop, bevel]))
}

#[derive(Clone)]
struct BevelSpec {
    enabled: bool,
    present: bool,
    style: Vec<u8>,
    technique: Vec<u8>,
    direction: Vec<u8>,
    depth: f64,
    size: f64,
    soften: f64,
    angle: f64,
    altitude: f64,
    hi_blend: Vec<u8>,
    hi_color: [f64; 3],
    hi_opacity: f64,
    sh_blend: Vec<u8>,
    sh_color: [f64; 3],
    sh_opacity: f64,
}

impl Default for BevelSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            present: true,
            style: b"InrB".to_vec(),
            technique: b"SfBL".to_vec(),
            direction: b"In  ".to_vec(),
            depth: 100.0,
            size: 4.0,
            soften: 0.0,
            angle: 120.0,
            altitude: 30.0,
            hi_blend: b"Scrn".to_vec(),
            hi_color: [255.0, 255.0, 255.0],
            hi_opacity: 75.0,
            sh_blend: b"Mltp".to_vec(),
            sh_color: [0.0, 0.0, 0.0],
            sh_opacity: 75.0,
        }
    }
}

fn bevel_block(spec: &BevelSpec) -> LayerBlock {
    lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(spec.enabled)),
            (b"present".to_vec(), DescValue::Bool(spec.present)),
            (b"showInDialog".to_vec(), DescValue::Bool(true)),
            (b"bvlS".to_vec(), enumerated(b"BESl", &spec.style)),
            (b"bvlT".to_vec(), enumerated(b"bvlT", &spec.technique)),
            (b"bvlD".to_vec(), enumerated(b"BESs", &spec.direction)),
            (b"srgR".to_vec(), unit(spec.depth, PRC)),
            (b"blur".to_vec(), unit(spec.size, PXL)),
            (b"Sftn".to_vec(), unit(spec.soften, PXL)),
            (b"lagl".to_vec(), unit(spec.angle, ANG)),
            (b"Lald".to_vec(), unit(spec.altitude, ANG)),
            (b"uglg".to_vec(), DescValue::Bool(false)),
            (b"hglM".to_vec(), enumerated(b"BlnM", &spec.hi_blend)),
            (
                b"hglC".to_vec(),
                rgbc(spec.hi_color[0], spec.hi_color[1], spec.hi_color[2]),
            ),
            (b"hglO".to_vec(), unit(spec.hi_opacity, PRC)),
            (b"sdwM".to_vec(), enumerated(b"BlnM", &spec.sh_blend)),
            (
                b"sdwC".to_vec(),
                rgbc(spec.sh_color[0], spec.sh_color[1], spec.sh_color[2]),
            ),
            (b"sdwO".to_vec(), unit(spec.sh_opacity, PRC)),
            (b"AntA".to_vec(), DescValue::Bool(true)),
        ]),
    )
}

fn spec_bevel(spec: &BevelSpec) -> Layer {
    bevel_layer(Some(bevel_block(spec)))
}

/// A decoded bevel with every field non-finite, for the composite's own clamps.
fn non_finite_bevel() -> BevelEmboss {
    let mut b = decode_bevel_emboss(&spec_bevel(&BevelSpec::default())).expect("decodes");
    b.size = f32::NAN;
    b.soften = f32::INFINITY;
    b.depth = f32::NAN;
    b.angle_deg = f32::NAN;
    b.altitude_deg = f32::NEG_INFINITY;
    b.highlight.opacity = f32::NAN;
    b.shadow.opacity = f32::NAN;
    b
}

/// Total absolute per-channel deviation from `plain` over the layer rect.
fn interior_delta(with: &PixelBuffer, plain: &PixelBuffer) -> u64 {
    let mut sum = 0u64;
    for y in 3..9 {
        for x in 3..9 {
            let a = px(with, x, y);
            let b = px(plain, x, y);
            for c in 0..3 {
                sum += (a[c] as i64 - b[c] as i64).unsigned_abs();
            }
        }
    }
    sum
}

/// Interior pixels differing from `plain`.
fn interior_changed(with: &PixelBuffer, plain: &PixelBuffer) -> u32 {
    let mut n = 0;
    for y in 3..9 {
        for x in 3..9 {
            if px(with, x, y) != px(plain, x, y) {
                n += 1;
            }
        }
    }
    n
}

// --- Decoder ---------------------------------------------------------------

#[test]
fn bevel_decodes_typed_parameters() {
    let spec = BevelSpec {
        style: b"OtrB".to_vec(),
        technique: b"PrBL".to_vec(),
        direction: b"Out ".to_vec(),
        depth: 250.0,
        size: 7.0,
        soften: 3.0,
        angle: 45.0,
        altitude: 60.0,
        hi_blend: b"Nrml".to_vec(),
        hi_color: [200.0, 100.0, 50.0],
        hi_opacity: 60.0,
        sh_blend: b"Scrn".to_vec(),
        sh_color: [10.0, 20.0, 30.0],
        sh_opacity: 40.0,
        ..Default::default()
    };
    let b = decode_bevel_emboss(&spec_bevel(&spec)).expect("decodes");
    assert!(b.enabled && b.present);
    assert_eq!(b.style, BevelStyle::Outer);
    assert_eq!(b.technique, BevelTechnique::ChiselHard);
    assert_eq!(b.direction, BevelDirection::Down);
    assert_eq!(b.depth, 250.0);
    assert_eq!(b.size, 7.0);
    assert_eq!(b.soften, 3.0);
    assert_eq!(b.angle_deg, 45.0);
    assert_eq!(b.altitude_deg, 60.0);
    assert!(!b.use_global_angle);
    assert_eq!(b.highlight.mode, BlendMode::Normal);
    assert_eq!(b.highlight.color, [200, 100, 50]);
    assert_eq!(b.highlight.opacity, 60.0);
    assert_eq!(b.shadow.mode, BlendMode::Screen);
    assert_eq!(b.shadow.color, [10, 20, 30]);
    assert_eq!(b.shadow.opacity, 40.0);
}

#[test]
fn bevel_missing_keys_take_defaults() {
    let block = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"present".to_vec(), DescValue::Bool(true)),
        ]),
    );
    let b = decode_bevel_emboss(&bevel_layer(Some(block))).expect("decodes");
    assert_eq!(b.style, BevelStyle::Inner);
    assert_eq!(b.technique, BevelTechnique::Smooth);
    assert_eq!(b.direction, BevelDirection::Up);
    assert_eq!(b.depth, 100.0);
    assert_eq!(b.size, 5.0);
    assert_eq!(b.soften, 0.0);
    assert_eq!(b.angle_deg, 120.0);
    assert_eq!(b.altitude_deg, 30.0);
    assert!(b.use_global_angle);
    assert_eq!(b.highlight.mode, BlendMode::Screen);
    assert_eq!(b.highlight.color, [255, 255, 255]);
    assert_eq!(b.highlight.opacity, 75.0);
    assert_eq!(b.shadow.mode, BlendMode::Multiply);
    assert_eq!(b.shadow.color, [0, 0, 0]);
    assert_eq!(b.shadow.opacity, 75.0);
}

#[test]
fn bevel_doub_numeric_decodes() {
    let block = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"srgR".to_vec(), DescValue::Double(250.0)),
            (b"blur".to_vec(), DescValue::Double(7.0)),
            (b"Sftn".to_vec(), DescValue::Double(3.0)),
            (b"Lald".to_vec(), DescValue::Double(60.0)),
            (b"hglO".to_vec(), DescValue::Double(80.0)),
        ]),
    );
    let b = decode_bevel_emboss(&bevel_layer(Some(block))).expect("decodes");
    assert_eq!(b.depth, 250.0);
    assert_eq!(b.size, 7.0);
    assert_eq!(b.soften, 3.0);
    assert_eq!(b.altitude_deg, 60.0);
    assert_eq!(b.highlight.opacity, 80.0);
}

#[test]
fn bevel_enums_decode_every_value() {
    let styles = [
        (b"InrB".to_vec(), BevelStyle::Inner),
        (b"OtrB".to_vec(), BevelStyle::Outer),
        (b"Embs".to_vec(), BevelStyle::Emboss),
        (b"PlEb".to_vec(), BevelStyle::Pillow),
        (b"strokeEmboss".to_vec(), BevelStyle::Stroke),
    ];
    for (raw, want) in styles {
        let spec = BevelSpec {
            style: raw,
            ..Default::default()
        };
        assert_eq!(decode_bevel_emboss(&spec_bevel(&spec)).unwrap().style, want);
    }
    let techniques = [
        (b"SfBL".to_vec(), BevelTechnique::Smooth),
        (b"PrBL".to_vec(), BevelTechnique::ChiselHard),
        (b"Slmt".to_vec(), BevelTechnique::ChiselSoft),
    ];
    for (raw, want) in techniques {
        let spec = BevelSpec {
            technique: raw,
            ..Default::default()
        };
        assert_eq!(
            decode_bevel_emboss(&spec_bevel(&spec)).unwrap().technique,
            want
        );
    }
    let directions = [
        (b"In  ".to_vec(), BevelDirection::Up),
        (b"Out ".to_vec(), BevelDirection::Down),
    ];
    for (raw, want) in directions {
        let spec = BevelSpec {
            direction: raw,
            ..Default::default()
        };
        assert_eq!(
            decode_bevel_emboss(&spec_bevel(&spec)).unwrap().direction,
            want
        );
    }
    // An unknown enum value falls back to the reference default.
    let spec = BevelSpec {
        style: b"XXXX".to_vec(),
        technique: b"XXXX".to_vec(),
        direction: b"XXXX".to_vec(),
        ..Default::default()
    };
    let b = decode_bevel_emboss(&spec_bevel(&spec)).unwrap();
    assert_eq!(
        (b.style, b.technique, b.direction),
        (
            BevelStyle::Inner,
            BevelTechnique::Smooth,
            BevelDirection::Up
        )
    );
}

#[test]
fn bevel_disabled_decodes_but_is_inert() {
    let spec = BevelSpec {
        enabled: false,
        ..Default::default()
    };
    let b = decode_bevel_emboss(&spec_bevel(&spec)).expect("decodes");
    assert!(!b.enabled);
    assert_eq!(
        compose(spec_bevel(&spec)),
        compose(bevel_layer(None)),
        "a disabled effect is byte-identical to no effect"
    );
}

#[test]
fn bevel_malformed_or_absent_is_none() {
    assert!(decode_bevel_emboss(&bevel_layer(None)).is_none());
    let no_ebbl = lfx2(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![(b"masterFXSwitch".to_vec(), DescValue::Bool(true))],
    });
    assert!(decode_bevel_emboss(&bevel_layer(Some(no_ebbl))).is_none());
    let wrong = lfx2_with(
        effect_top(
            b"ebbl",
            ebbl(vec![(b"enab".to_vec(), DescValue::Bool(true))]),
        ),
        17,
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong))).is_none());
    let wrong_id = lfx2_effect(
        b"ebbl",
        object(b"XXXX", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong_id))).is_none());
    // Wrong-typed numeric and wrong enum typeID.
    let wrong_num = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"srgR".to_vec(), DescValue::Bool(true)),
        ]),
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong_num))).is_none());
    let wrong_kind = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"bvlS".to_vec(), enumerated(b"BlnM", b"InrB")),
        ]),
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong_kind))).is_none());
    let wrong_blend = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"hglM".to_vec(), DescValue::Double(1.0)),
        ]),
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong_blend))).is_none());
    // Non-RGBC colour.
    let wrong_color = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (
                b"hglC".to_vec(),
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
    assert!(decode_bevel_emboss(&bevel_layer(Some(wrong_color))).is_none());
    // Non-finite and truncated.
    let nan = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"blur".to_vec(), DescValue::Double(f64::NAN)),
        ]),
    );
    assert!(decode_bevel_emboss(&bevel_layer(Some(nan))).is_none());
    let mut short = bevel_block(&BevelSpec::default());
    short.data.truncate(6);
    assert!(decode_bevel_emboss(&bevel_layer(Some(short))).is_none());
}

#[test]
fn bevel_huge_numeric_parameters_are_bounded_or_rejected() {
    for key in [b"srgR", b"blur", b"Sftn", b"Lald", b"hglO", b"sdwO"] {
        let block = lfx2_effect(
            b"ebbl",
            ebbl(vec![
                (b"enab".to_vec(), DescValue::Bool(true)),
                (key.to_vec(), DescValue::Double(1e300)),
            ]),
        );
        assert!(
            decode_bevel_emboss(&bevel_layer(Some(block))).is_none(),
            "{key:?} 1e300 overflows f32"
        );
    }
    let block = lfx2_effect(
        b"ebbl",
        ebbl(vec![
            (b"enab".to_vec(), DescValue::Bool(true)),
            (b"srgR".to_vec(), unit(1e30, PRC)),
            (b"blur".to_vec(), unit(1e30, PXL)),
            (b"Sftn".to_vec(), unit(1e30, PXL)),
            (b"Lald".to_vec(), unit(1e30, ANG)),
            (b"hglO".to_vec(), unit(1e30, PRC)),
            (b"sdwO".to_vec(), unit(1e30, PRC)),
        ]),
    );
    let b = decode_bevel_emboss(&bevel_layer(Some(block))).expect("decodes clamped");
    assert_eq!(b.depth, 1000.0);
    assert_eq!(b.size, 250.0);
    assert_eq!(b.soften, 250.0);
    assert_eq!(b.altitude_deg, 90.0);
    assert_eq!(b.highlight.opacity, 100.0);
    assert_eq!(b.shadow.opacity, 100.0);
    let out = compose(spec_bevel(&BevelSpec {
        depth: 1e30,
        size: 1e30,
        soften: 1e30,
        altitude: 1e30,
        hi_opacity: 1e30,
        sh_opacity: 1e30,
        ..Default::default()
    }));
    assert_eq!(out.width, 12, "rendering the clamped bevel does not panic");
}

// --- Rendering -------------------------------------------------------------

#[test]
fn bevel_is_interior_and_exterior_is_unchanged() {
    let with = compose(spec_bevel(&BevelSpec::default()));
    let plain = compose(bevel_layer(None));
    for y in 0..12 {
        for x in 0..12 {
            if !(3..9).contains(&x) || !(3..9).contains(&y) {
                assert_eq!(px(&with, x, y), px(&plain, x, y), "exterior ({x},{y})");
            }
        }
    }
    assert!(
        interior_changed(&with, &plain) > 0,
        "at least one interior pixel differs"
    );
    // Angle 120 lights the upper-left interior edge and shades the lower-right.
    assert!(
        rgb(&with, 3, 4)[1] > rgb(&plain, 3, 4)[1],
        "the left interior edge is highlighted: {:?}",
        rgb(&with, 3, 4)
    );
    assert!(
        rgb(&with, 8, 4)[0] < rgb(&plain, 8, 4)[0],
        "the right interior edge is shaded: {:?}",
        rgb(&with, 8, 4)
    );
}

#[test]
fn bevel_angle_and_altitude_shape_the_shading() {
    let a120 = compose(spec_bevel(&BevelSpec {
        angle: 120.0,
        ..Default::default()
    }));
    let a300 = compose(spec_bevel(&BevelSpec {
        angle: 300.0,
        ..Default::default()
    }));
    assert_ne!(a120, a300, "angle moves the light");

    let low = compose(spec_bevel(&BevelSpec {
        altitude: 10.0,
        ..Default::default()
    }));
    let high = compose(spec_bevel(&BevelSpec {
        altitude: 75.0,
        ..Default::default()
    }));
    assert_ne!(low, high, "altitude changes the shading");
}

#[test]
fn bevel_size_soften_and_depth_shape_the_shading() {
    let plain = compose(bevel_layer(None));
    let small = compose(spec_bevel(&BevelSpec {
        size: 1.0,
        ..Default::default()
    }));
    let large = compose(spec_bevel(&BevelSpec {
        size: 6.0,
        ..Default::default()
    }));
    assert!(
        interior_changed(&large, &plain) >= interior_changed(&small, &plain),
        "a larger size spans at least as many shaded pixels"
    );
    assert_ne!(small, large);

    let hard = compose(spec_bevel(&BevelSpec {
        soften: 0.0,
        ..Default::default()
    }));
    let soft = compose(spec_bevel(&BevelSpec {
        soften: 4.0,
        ..Default::default()
    }));
    assert!(
        interior_changed(&soft, &plain) >= interior_changed(&hard, &plain),
        "a larger soften spans at least as many transition pixels"
    );
    assert_ne!(hard, soft);

    let shallow = compose(spec_bevel(&BevelSpec {
        depth: 40.0,
        ..Default::default()
    }));
    let deep = compose(spec_bevel(&BevelSpec {
        depth: 400.0,
        ..Default::default()
    }));
    assert!(
        interior_delta(&deep, &plain) > interior_delta(&shallow, &plain),
        "a larger depth increases the shading magnitude"
    );
    assert_eq!(
        compose(spec_bevel(&BevelSpec {
            depth: 0.0,
            ..Default::default()
        })),
        plain,
        "depth 0 is byte-identical to no effect"
    );
}

#[test]
fn bevel_direction_flips_the_highlight_and_shadow() {
    let plain = compose(bevel_layer(None));
    let up = compose(spec_bevel(&BevelSpec {
        direction: b"In  ".to_vec(),
        ..Default::default()
    }));
    let down = compose(spec_bevel(&BevelSpec {
        direction: b"Out ".to_vec(),
        ..Default::default()
    }));
    assert_ne!(up, down, "direction flips the bevel");
    assert!(
        rgb(&up, 3, 4)[1] > rgb(&plain, 3, 4)[1],
        "Up highlights left"
    );
    assert!(rgb(&up, 8, 4)[0] < rgb(&plain, 8, 4)[0], "Up shades right");
    assert!(
        rgb(&down, 3, 4)[0] < rgb(&plain, 3, 4)[0],
        "Down shades left"
    );
    assert!(
        rgb(&down, 8, 4)[1] > rgb(&plain, 8, 4)[1],
        "Down highlights right"
    );
}

#[test]
fn bevel_highlight_and_shadow_colour_opacity_mode_shape_it() {
    let plain = compose(bevel_layer(None));
    let tinted = compose(spec_bevel(&BevelSpec {
        hi_color: [0.0, 0.0, 255.0],
        sh_color: [0.0, 255.0, 0.0],
        ..Default::default()
    }));
    assert_ne!(plain, tinted, "the highlight/shadow colours show");

    let strong = compose(spec_bevel(&BevelSpec {
        hi_opacity: 100.0,
        sh_opacity: 100.0,
        ..Default::default()
    }));
    let weak = compose(spec_bevel(&BevelSpec {
        hi_opacity: 20.0,
        sh_opacity: 20.0,
        ..Default::default()
    }));
    assert!(
        interior_delta(&weak, &plain) < interior_delta(&strong, &plain),
        "lowering an opacity approaches the un-beveled content"
    );

    let normal = compose(spec_bevel(&BevelSpec {
        hi_blend: b"Nrml".to_vec(),
        sh_blend: b"Nrml".to_vec(),
        ..Default::default()
    }));
    assert_ne!(strong, normal, "a non-Normal blend mode changes the result");
}

#[test]
fn bevel_layer_mask_shapes_the_matte() {
    let spec = BevelSpec::default();
    let plain = compose(half_mask(bevel_layer(None)));
    let with = compose(half_mask(spec_bevel(&spec)));
    for y in 3..9 {
        for x in 3..6 {
            assert_eq!(px(&with, x, y), px(&plain, x, y), "mask 0 at ({x},{y})");
        }
    }
    assert!(
        interior_changed(&with, &plain) > 0,
        "the bevel shows where the mask is 255"
    );
}

#[test]
fn bevel_is_bounded_to_the_content_rect() {
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
    let with = make(Some(bevel_block(&BevelSpec {
        size: 8.0,
        soften: 4.0,
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
fn absent_disabled_or_stroke_emboss_bevel_is_a_no_op() {
    let plain = compose(bevel_layer(None));
    assert_eq!(plain, compose(bevel_layer(None)), "absent bevel");
    assert_eq!(
        plain,
        compose(spec_bevel(&BevelSpec {
            enabled: false,
            ..Default::default()
        })),
    );
    assert_eq!(
        plain,
        compose(spec_bevel(&BevelSpec {
            present: false,
            ..Default::default()
        })),
    );
    assert_eq!(
        plain,
        compose(spec_bevel(&BevelSpec {
            hi_opacity: 0.0,
            sh_opacity: 0.0,
            ..Default::default()
        })),
        "both opacities 0"
    );
    assert_eq!(
        plain,
        compose(spec_bevel(&BevelSpec {
            style: b"strokeEmboss".to_vec(),
            ..Default::default()
        })),
        "Stroke Emboss without a stroke band is a no-op"
    );
}

/// How many pixels inside / outside the 6x6 content square differ from `plain`.
fn changed_inside_outside(out: &PixelBuffer, plain: &PixelBuffer) -> (usize, usize) {
    let (mut inside, mut outside) = (0, 0);
    for y in 0..12 {
        for x in 0..12 {
            if px(out, x, y) != px(plain, x, y) {
                if (3..9).contains(&x) && (3..9).contains(&y) {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
    }
    (inside, outside)
}

#[test]
fn each_bevel_style_lights_its_side_of_the_edge() {
    let plain = compose(bevel_layer(None));
    let style = |code: &[u8]| {
        changed_inside_outside(
            &compose(spec_bevel(&BevelSpec {
                style: code.to_vec(),
                ..Default::default()
            })),
            &plain,
        )
    };
    let (inner_in, inner_out) = style(b"InrB");
    assert!(
        inner_in > 0 && inner_out == 0,
        "Inner lights only the content"
    );
    let (outer_in, outer_out) = style(b"OtrB");
    assert!(outer_in == 0 && outer_out > 0, "Outer lights only outside");
    for code in [b"Embs".as_slice(), b"PlEb"] {
        let (inside, outside) = style(code);
        assert!(inside > 0 && outside > 0, "{code:?} straddles the edge");
    }
    // Pillow sinks the outside: the outer ring is the inverse of an Emboss's.
    let emboss = compose(spec_bevel(&BevelSpec {
        style: b"Embs".to_vec(),
        ..Default::default()
    }));
    let pillow = compose(spec_bevel(&BevelSpec {
        style: b"PlEb".to_vec(),
        ..Default::default()
    }));
    assert_ne!(px(&emboss, 1, 6), px(&pillow, 1, 6));
}

#[test]
fn each_technique_rounds_the_chamfer_differently() {
    let render = |technique: &[u8]| {
        compose(spec_bevel(&BevelSpec {
            technique: technique.to_vec(),
            size: 4.0,
            ..Default::default()
        }))
    };
    let (smooth, hard, soft) = (render(b"SfBL"), render(b"PrBL"), render(b"Slmt"));
    assert_ne!(smooth, hard);
    assert_ne!(smooth, soft);
    assert_ne!(hard, soft);
}

#[test]
fn non_finite_descriptor_is_rejected_at_decode() {
    let out = compose(spec_bevel(&BevelSpec {
        size: f64::NAN,
        soften: f64::INFINITY,
        depth: f64::NAN,
        angle: f64::NAN,
        altitude: f64::NAN,
        ..Default::default()
    }));
    assert_eq!(
        out,
        compose(bevel_layer(None)),
        "the malformed bevel is inert"
    );
    assert_eq!(out.width, 12);
}

#[test]
fn composite_bevel_emboss_clamps_hand_built_non_finite_parameters() {
    let layer = bevel_layer(None);
    let d = doc(12, 12, vec![layer.clone()]);
    let all_bad = non_finite_bevel();
    let out = crate::layer_effects::composite_bevel_emboss_for_test(&layer, &d, &all_bad);
    assert_eq!(out.width, 12);
    assert!(
        out.data.iter().all(|&b| b == 0),
        "NaN opacities clamp to 0 and paint nothing"
    );

    let mut clamped = non_finite_bevel();
    clamped.highlight.opacity = 100.0;
    clamped.shadow.opacity = 100.0;
    clamped.angle_deg = 0.0;
    clamped.size = 0.0;
    clamped.soften = 0.0;
    clamped.depth = 0.0;
    clamped.altitude_deg = 0.0;
    let mut unbounded = clamped;
    unbounded.angle_deg = f32::NAN;
    unbounded.size = f32::INFINITY;
    unbounded.depth = f32::NAN;
    unbounded.altitude_deg = f32::INFINITY;
    assert_eq!(
        crate::layer_effects::composite_bevel_emboss_for_test(&layer, &d, &unbounded),
        crate::layer_effects::composite_bevel_emboss_for_test(&layer, &d, &clamped),
        "non-finite fields behave as their clamped references"
    );
}

// --- GPU fallback ----------------------------------------------------------

#[test]
fn gpu_rejects_a_bevel_layer_and_falls_back() {
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
            spec_bevel(&BevelSpec::default()),
        ],
    );
    assert!(
        matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
        "check_supported rejects before dispatch"
    );
    assert_eq!(composite_gpu_or_cpu(&d), composite_rgba(&d));
}

#[test]
fn gpu_does_not_reject_a_disabled_or_stroke_emboss_bevel() {
    for spec in [
        BevelSpec {
            enabled: false,
            ..Default::default()
        },
        BevelSpec {
            present: false,
            ..Default::default()
        },
        BevelSpec {
            style: b"strokeEmboss".to_vec(),
            ..Default::default()
        },
    ] {
        let d = doc(12, 12, vec![spec_bevel(&spec)]);
        assert!(
            !matches!(composite_gpu(&d), Err(GpuError::UnsupportedLayerEffect)),
            "an inert bevel must not reject the GPU"
        );
    }
    let malformed = bevel_layer(Some(lfx2_effect(
        b"ebbl",
        object(b"XXXX", vec![(b"enab".to_vec(), DescValue::Bool(true))]),
    )));
    assert!(
        !matches!(
            composite_gpu(&doc(12, 12, vec![malformed])),
            Err(GpuError::UnsupportedLayerEffect)
        ),
        "a malformed bevel must not reject the GPU"
    );
}

// --- Fixture ---------------------------------------------------------------

const BEVEL_FIXTURE: &[u8] = include_bytes!("../../../../pictura-codec/tests/fixtures/bevel.psd");

#[test]
fn bevel_fixture_decodes_and_composites() {
    let d = pictura_codec::read_psd(BEVEL_FIXTURE).expect("bevel.psd parses");
    let layer = d
        .layers
        .iter()
        .find(|l| l.name == "Beveled")
        .expect("Beveled layer");
    let b = decode_bevel_emboss(layer).expect("decodes the authored ebbl");
    assert!(b.enabled && b.present);
    assert_eq!(b.style, BevelStyle::Inner);
    assert_eq!(b.technique, BevelTechnique::Smooth);
    assert_eq!(b.direction, BevelDirection::Up);
    assert_eq!(b.depth, 250.0);
    assert_eq!(b.size, 7.0);
    assert_eq!(b.soften, 3.0);
    assert_eq!(b.angle_deg, 120.0);
    assert_eq!(b.altitude_deg, 30.0);
    assert!(!b.use_global_angle);
    assert_eq!(b.highlight.mode, BlendMode::Screen);
    assert_eq!(b.highlight.color, [250, 240, 230]);
    assert_eq!(b.highlight.opacity, 80.0);
    assert_eq!(b.shadow.mode, BlendMode::Multiply);
    assert_eq!(b.shadow.color, [10, 20, 30]);
    assert_eq!(b.shadow.opacity, 70.0);

    let with_effect = composite_rgba(&d);
    let mut plain = d.clone();
    for l in plain.layers.iter_mut() {
        l.extra_blocks.clear();
    }
    assert_ne!(with_effect, composite_rgba(&plain), "the bevel renders");
}

pub(super) fn parallel_layer() -> Layer {
    spec_bevel(&BevelSpec::default())
}
