use super::*;
use pictura_core::{BlendIf, CharacterAttrs, Knockout, ParagraphAttrs, TextStyle, TypeTool};

// --- compositing pipeline ----------------------------------------------

#[test]
fn multi_layer_source_over_scene() {
    let d = doc(
        2,
        2,
        vec![
            solid(
                "white",
                full(2, 2),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "red",
                full(2, 2),
                (255, 0, 0),
                255,
                BlendMode::Multiply,
                255,
            ),
            solid("blue", full(2, 2), (0, 0, 255), 255, BlendMode::Screen, 255),
        ],
    );
    let out = composite_rgba(&d);
    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(px(&out, x, y), [255, 0, 255, 255], "at {x},{y}");
        }
    }
}

#[test]
fn blend_ignored_over_transparent_backdrop() {
    let d = doc(
        1,
        1,
        vec![solid(
            "top",
            full(1, 1),
            (255, 0, 0),
            255,
            BlendMode::Multiply,
            255,
        )],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [255, 0, 0, 255]);
}

#[test]
fn layer_opacity_scales_over_backdrop() {
    // 1x1: base 200 opaque, top 0/200/0 at 50% opacity, Normal.
    let d = doc(
        1,
        1,
        vec![
            solid(
                "base",
                full(1, 1),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ),
            solid("top", full(1, 1), (0, 200, 0), 255, BlendMode::Normal, 128),
        ],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [100, 150, 25, 255]);
}

#[test]
fn fill_255_is_byte_identical_to_pre_change_composites() {
    // Each expected pixel is the pre-fill value: fill 255 is an exact 1.0
    // factor, so plain, masked, group and adjustment scenes must not move.

    // Plain: the multi-layer source-over scene.
    let plain = doc(
        2,
        2,
        vec![
            solid(
                "white",
                full(2, 2),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "red",
                full(2, 2),
                (255, 0, 0),
                255,
                BlendMode::Multiply,
                255,
            ),
            solid("blue", full(2, 2), (0, 0, 255), 255, BlendMode::Screen, 255),
        ],
    );
    assert_eq!(px(&composite_rgba(&plain), 0, 0), [255, 0, 255, 255]);

    // Masked: mask zeroes the top half.
    let mut masked = solid(
        "masked",
        full(1, 2),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    masked.mask = Some(LayerMask {
        rect: full(1, 2),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![0, 255].into()),
        ..Default::default()
    });
    let masked_out = composite_rgba(&doc(1, 2, vec![masked]));
    assert_eq!(px(&masked_out, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&masked_out, 0, 1), [255, 0, 0, 255]);

    // Group: fill lives on a group and is ignored; fill 0 must equal 255.
    let group_scene = |fill: u8| {
        let mut g = group(
            "group",
            BlendMode::Multiply,
            255,
            None,
            vec![solid(
                "child",
                full(1, 1),
                (50, 50, 50),
                255,
                BlendMode::Normal,
                255,
            )],
        );
        g.fill = fill;
        composite_rgba(&doc(
            1,
            1,
            vec![
                solid(
                    "backdrop",
                    full(1, 1),
                    (200, 200, 200),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                g,
            ],
        ))
    };
    assert_eq!(px(&group_scene(255), 0, 0), [39, 39, 39, 255]);
    assert_eq!(
        group_scene(0).data,
        group_scene(255).data,
        "group fill is ignored"
    );

    // Adjustment: fill 255 leaves the invert gate at its pre-fill value.
    let base = solid(
        "base",
        full(2, 1),
        (100, 100, 100),
        255,
        BlendMode::Normal,
        255,
    );
    let mut adj = adjustment_layer("invert", *b"nvrt", Vec::new(), 255, None);
    adj.mask = Some(LayerMask {
        rect: full(2, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![255, 0].into()),
        ..Default::default()
    });
    let adj_out = composite_rgba(&doc(2, 1, vec![base, adj]));
    assert_eq!(rgb(&adj_out, 0, 0), [155, 155, 155]);
    assert_eq!(rgb(&adj_out, 1, 0), [100, 100, 100]);
}

#[test]
fn fill_below_255_scales_contribution() {
    // Over a transparent backdrop: source alpha is scaled to 128/255.
    let over_transparent = doc(
        1,
        1,
        vec![{
            let mut l = solid("top", full(1, 1), (10, 20, 30), 255, BlendMode::Normal, 255);
            l.fill = 128;
            l
        }],
    );
    assert_eq!(
        px(&composite_rgba(&over_transparent), 0, 0),
        [10, 20, 30, 128]
    );

    // Over an opaque backdrop: the same factor lerps toward the source.
    let over_opaque = doc(
        1,
        1,
        vec![
            solid(
                "base",
                full(1, 1),
                (100, 100, 100),
                255,
                BlendMode::Normal,
                255,
            ),
            {
                let mut l = solid("top", full(1, 1), (200, 0, 0), 255, BlendMode::Normal, 255);
                l.fill = 128;
                l
            },
        ],
    );
    assert_eq!(px(&composite_rgba(&over_opaque), 0, 0), [150, 50, 50, 255]);
}

#[test]
fn fill_combines_with_opacity_and_mask() {
    let mut layer = solid("top", full(2, 1), (10, 20, 30), 255, BlendMode::Normal, 255);
    layer.opacity = 200;
    layer.fill = 128;
    layer.mask = Some(LayerMask {
        rect: full(2, 1),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![255, 0].into()),
        ..Default::default()
    });
    let out = composite_rgba(&doc(2, 1, vec![layer]));
    assert_eq!(
        px(&out, 0, 0),
        [10, 20, 30, 100],
        "opacity x fill x mask on the revealed pixel"
    );
    assert_eq!(px(&out, 1, 0), [0, 0, 0, 0], "masked-off pixel is empty");
}

#[test]
fn masked_layer_zeroes_masked_alpha() {
    let lw = 1u32;
    let mut top = solid(
        "masked",
        full(1, 2),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    top.mask = Some(LayerMask {
        rect: full(1, 2),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![0, 255].into()),
        ..Default::default()
    });
    let _ = lw;
    let d = doc(1, 2, vec![top]);
    let out = composite_rgba(&d);
    assert_eq!(px(&out, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&out, 0, 1), [255, 0, 0, 255]);
}

#[test]
fn disabled_mask_is_ignored() {
    let mut top = solid(
        "masked",
        full(1, 1),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    top.mask = Some(LayerMask {
        rect: full(1, 1),
        default_color: 0,
        disabled: true,
        flags: 0x02,
        data: Some(vec![0].into()),
        ..Default::default()
    });
    let d = doc(1, 1, vec![top]);
    assert_eq!(px(&composite_rgba(&d), 0, 0), [255, 0, 0, 255]);
}

#[test]
fn isolated_group_blends_as_single_layer() {
    // Backdrop 200; group Multiply containing an opaque 50 child. Isolation
    // means the child is Normal inside the group, then Multiply applies:
    // 200/255 * 50/255 = 0.1537 -> 39.
    let d = doc(
        1,
        1,
        vec![
            solid(
                "backdrop",
                full(1, 1),
                (200, 200, 200),
                255,
                BlendMode::Normal,
                255,
            ),
            group(
                "group",
                BlendMode::Multiply,
                255,
                None,
                vec![solid(
                    "child",
                    full(1, 1),
                    (50, 50, 50),
                    255,
                    BlendMode::Normal,
                    255,
                )],
            ),
        ],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [39, 39, 39, 255]);
}

#[test]
fn group_opacity_scales_isolated_result() {
    let d = doc(
        1,
        1,
        vec![group(
            "group",
            BlendMode::Normal,
            128,
            None,
            vec![solid(
                "child",
                full(1, 1),
                (255, 0, 0),
                255,
                BlendMode::Normal,
                255,
            )],
        )],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [255, 0, 0, 128]);
}

#[test]
fn pass_through_group_equals_ungrouped_stack() {
    let backdrop = solid(
        "backdrop",
        full(2, 2),
        (200, 120, 40),
        255,
        BlendMode::Normal,
        255,
    );
    let child = solid(
        "child",
        full(2, 2),
        (50, 180, 90),
        255,
        BlendMode::Multiply,
        255,
    );
    let grouped = doc(
        2,
        2,
        vec![
            backdrop.clone(),
            group(
                "group",
                BlendMode::PassThrough,
                255,
                None,
                vec![child.clone()],
            ),
        ],
    );
    let ungrouped = doc(2, 2, vec![backdrop, child]);
    assert_eq!(
        composite_rgba(&grouped).data,
        composite_rgba(&ungrouped).data,
        "pass-through group must equal the flattened stack"
    );
}

#[test]
fn pass_through_group_with_opacity_falls_back_to_isolated() {
    // Approximation: pass-through + opacity is isolated with mode Normal.
    let d = doc(
        1,
        1,
        vec![group(
            "group",
            BlendMode::PassThrough,
            128,
            None,
            vec![solid(
                "child",
                full(1, 1),
                (255, 0, 0),
                255,
                BlendMode::Normal,
                255,
            )],
        )],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [255, 0, 0, 128]);
}

#[test]
fn pass_through_group_vector_mask() {
    use pictura_core::{VectorFillRule, VectorMask, VectorSubpath};
    // The mask covers the left half of a 4x4 canvas.
    let mask = || VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![[0, 0], [512, 0], [512, 1024], [0, 1024]],
        }],
        ..Default::default()
    };
    let scene = |blend| {
        let mut g = group(
            "group",
            blend,
            255,
            None,
            vec![solid(
                "child",
                full(4, 4),
                (255, 0, 0),
                255,
                BlendMode::Normal,
                255,
            )],
        );
        g.vector_mask = Some(mask());
        doc(
            4,
            4,
            vec![
                solid(
                    "backdrop",
                    full(4, 4),
                    (0, 0, 255),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                g,
            ],
        )
    };
    let pass = composite_rgba(&scene(BlendMode::PassThrough));
    let isolated = composite_rgba(&scene(BlendMode::Normal));
    assert_eq!(
        pass.data, isolated.data,
        "a vector mask falls back to the isolated path"
    );
    for x in 0..2 {
        assert_eq!(rgb(&pass, x, 0), [255, 0, 0], "masked-in column {x}");
    }
    for x in 2..4 {
        assert_eq!(rgb(&pass, x, 0), [0, 0, 255], "masked-out column {x}");
    }
    let gpu = composite_gpu_or_cpu(&scene(BlendMode::PassThrough));
    for (i, (a, b)) in gpu.data.iter().zip(pass.data.iter()).enumerate() {
        assert!(
            (*a as i16 - *b as i16).abs() <= 1,
            "GPU parity at sample {i}: {a} vs {b}"
        );
    }
}

#[test]
fn bounds_and_negative_rect_clipping() {
    let mut d = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(vec![
        // Covers only canvas (0,0): local (1,1) of a 2x2 rect anchored at -1,-1.
        solid(
            "neg",
            rect(-1, -1, 1, 1),
            (255, 0, 0),
            255,
            BlendMode::Normal,
            255,
        ),
        // Fully off-canvas, contributes nothing and must not panic.
        solid(
            "off",
            rect(-9, -9, -1, -1),
            (0, 255, 0),
            255,
            BlendMode::Normal,
            255,
        ),
        // Zero-area layer.
        solid(
            "empty",
            rect(1, 1, 1, 1),
            (0, 0, 255),
            255,
            BlendMode::Normal,
            255,
        ),
    ]);
    let out = composite_rgba(&d);
    assert_eq!(px(&out, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&out, 1, 0), [0, 0, 0, 0]);
    assert_eq!(px(&out, 0, 1), [0, 0, 0, 0]);
    assert_eq!(px(&out, 1, 1), [0, 0, 0, 0]);
}

#[test]
fn positive_offset_rect_lands_in_canvas() {
    let d = doc(
        3,
        2,
        vec![solid(
            "offset",
            rect(1, 1, 2, 2),
            (10, 20, 30),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let out = composite_rgba(&d);
    assert_eq!(px(&out, 1, 1), [10, 20, 30, 255]);
    assert_eq!(px(&out, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&out, 2, 1), [0, 0, 0, 0]);
}

#[test]
fn empty_layer_stack_is_transparent() {
    let out = composite_rgba(&doc(2, 2, Vec::new()));
    assert_eq!(out.channels, 4);
    assert!(out.data.iter().all(|&b| b == 0));
}

#[test]
fn dissolve_is_binary_and_deterministic() {
    let d = doc(
        8,
        8,
        vec![solid(
            "noise",
            full(8, 8),
            (255, 0, 0),
            255,
            BlendMode::Dissolve,
            128,
        )],
    );
    let a = composite_rgba(&d);
    let b = composite_rgba(&d);
    assert_eq!(a.data, b.data, "dissolve must be deterministic");
    let mut sources = 0;
    for y in 0..8 {
        for x in 0..8 {
            let p = px(&a, x, y);
            assert!(
                p == [255, 0, 0, 255] || p == [0, 0, 0, 0],
                "dissolve pixel must be binary, got {p:?}"
            );
            if p == [255, 0, 0, 255] {
                sources += 1;
            }
        }
    }
    assert!(sources > 0 && sources < 64, "expected a mix, got {sources}");
}

#[test]
fn grayscale_layer_replicates_channel() {
    let mut d = Document::new(1, 1, ColorMode::Grayscale, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        name: "gray".into(),
        rect: full(1, 1),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![Channel {
            id: 0,
            data: vec![120].into(),
        }],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }]);
    assert_eq!(px(&composite_rgba(&d), 0, 0), [120, 120, 120, 255]);
}

// --- Blend If gating -----------------------------------------------------

/// A `BlendIf` view whose every range is the full `(0, 65535)` default.
fn default_blend_if() -> BlendIf {
    BlendIf {
        composite_source: (0, 65535),
        composite_dest: (0, 65535),
        channel_ranges: Vec::new(),
    }
}

#[test]
fn blend_if_absent_or_default_is_identity() {
    assert_eq!(blend_if_factor(None, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]), 1.0);
    assert_eq!(
        blend_if_factor(Some(&default_blend_if()), [0.5, 0.2, 0.9], [0.1, 0.8, 0.3]),
        1.0
    );
}

#[test]
fn blend_if_source_gate_hides_below_black_and_keeps_above() {
    // Source black at 0.5: a dark source is gated out, a light one blends.
    let view = BlendIf {
        composite_source: (32768, 65535),
        composite_dest: (0, 65535),
        channel_ranges: Vec::new(),
    };
    let backdrop = [0.0, 0.0, 0.0];
    assert_eq!(
        blend_if_factor(Some(&view), [0.24, 0.24, 0.24], backdrop),
        0.0
    );
    assert_eq!(blend_if_factor(Some(&view), [0.8, 0.8, 0.8], backdrop), 1.0);
}

#[test]
fn blend_if_dest_gate_gates_on_backdrop() {
    let view = BlendIf {
        composite_source: (0, 65535),
        composite_dest: (32768, 65535),
        channel_ranges: Vec::new(),
    };
    let source = [0.5, 0.5, 0.5];
    assert_eq!(blend_if_factor(Some(&view), source, [0.2, 0.2, 0.2]), 0.0);
    assert_eq!(blend_if_factor(Some(&view), source, [0.8, 0.8, 0.8]), 1.0);
}

#[test]
fn blend_if_per_channel_gate_gates_that_channel() {
    // Group 0 (R) source black above the source red value hides the layer.
    let view = BlendIf {
        composite_source: (0, 65535),
        composite_dest: (0, 65535),
        channel_ranges: vec![((32768, 65535), (0, 65535))],
    };
    let backdrop = [0.0, 0.0, 0.0];
    assert_eq!(blend_if_factor(Some(&view), [0.2, 0.9, 0.9], backdrop), 0.0);
    assert_eq!(blend_if_factor(Some(&view), [0.9, 0.2, 0.2], backdrop), 1.0);
}

#[test]
fn blend_if_channel_group_beyond_rgb_is_ignored() {
    // A 4th group (alpha in the reference's layout) must not gate the pixel.
    let view = BlendIf {
        composite_source: (0, 65535),
        composite_dest: (0, 65535),
        channel_ranges: vec![
            ((0, 65535), (0, 65535)),
            ((0, 65535), (0, 65535)),
            ((0, 65535), (0, 65535)),
            ((50000, 65535), (50000, 65535)),
        ],
    };
    assert_eq!(
        blend_if_factor(Some(&view), [0.5, 0.5, 0.5], [0.5, 0.5, 0.5]),
        1.0
    );
}

#[test]
fn blend_if_source_gate_changes_composite_output() {
    let scene = |blend_if: Option<BlendIf>| {
        let mut top = solid(
            "top",
            full(1, 1),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        );
        top.blend_if = blend_if;
        doc(
            1,
            1,
            vec![
                solid(
                    "base",
                    full(1, 1),
                    (64, 64, 64),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                top,
            ],
        )
    };
    // Absent or full-default ranges: the white layer covers the backdrop.
    assert_eq!(
        px(&composite_rgba(&scene(None)), 0, 0),
        [255, 255, 255, 255]
    );
    assert_eq!(
        px(&composite_rgba(&scene(Some(default_blend_if()))), 0, 0),
        [255, 255, 255, 255]
    );
    // A source white endpoint below the source gray hides the layer entirely.
    let hidden = BlendIf {
        composite_source: (0, 40000),
        composite_dest: (0, 65535),
        channel_ranges: Vec::new(),
    };
    assert_eq!(
        px(&composite_rgba(&scene(Some(hidden))), 0, 0),
        [64, 64, 64, 255]
    );
}

// --- Live type compositing ----------------------------------------------

pub(crate) fn type_style() -> TextStyle {
    TextStyle {
        font: Some("Arial".into()),
        character: CharacterAttrs {
            size: 48.0,
            fill_color: [1.0, 0.0, 0.0, 0.0],
            ..CharacterAttrs::default()
        },
        paragraph: ParagraphAttrs::default(),
        ..TextStyle::default()
    }
}

pub(crate) fn type_layer(style: Option<TextStyle>) -> Layer {
    Layer {
        name: "text".into(),
        rect: rect(0, 0, 80, 200),
        type_tool: Some(TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: "Hi".into(),
            bounds: [0, 0, 80, 200],
            text_desc: Vec::new(),
            warp_desc: Vec::new(),
            fonts: vec!["Arial".into()],
            style,
            vertical: false,
            box_bounds: None,
            path: None,
        }),
        ..Default::default()
    }
}

#[test]
fn proxy_less_type_layer_composites_live_coverage() {
    let layer = type_layer(Some(type_style()));
    assert!(
        layer.channels.is_empty(),
        "proxy-less type carries no channel"
    );
    let d = doc(200, 80, vec![layer]);
    let out = composite_rgba(&d);
    let plane = (200 * 80) as usize;
    let covered = out.data[3 * plane..4 * plane]
        .iter()
        .filter(|&&a| a > 0)
        .count();
    assert!(covered > 0, "live type must paint coverage");
    assert!(
        covered < plane,
        "a proxy-less type layer is not an opaque black rect"
    );
    let (active, backend) = composite_active(&d, true);
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(active, out);
}

#[test]
fn type_layer_with_channel_composites_from_the_raster() {
    let n = (200 * 80) as usize;
    let mut layer = type_layer(Some(type_style()));
    layer.channels = vec![
        Channel {
            id: 0,
            data: vec![10; n].into(),
        },
        Channel {
            id: 1,
            data: vec![20; n].into(),
        },
        Channel {
            id: 2,
            data: vec![30; n].into(),
        },
        Channel {
            id: -1,
            data: vec![255; n].into(),
        },
    ];
    let out = composite_rgba(&doc(200, 80, vec![layer]));
    for y in 0..80 {
        for x in 0..200 {
            assert_eq!(px(&out, x, y), [10, 20, 30, 255], "at {x},{y}");
        }
    }
}

#[test]
fn gpu_declines_a_proxy_less_type_layer() {
    let d = doc(200, 80, vec![type_layer(Some(type_style()))]);
    assert!(matches!(composite_gpu(&d), Err(GpuError::UnsupportedText)));
}

// --- Knockout ------------------------------------------------------------

/// 1x1 stack: opaque red background, opaque green, and a half-fill blue top
/// carrying `knockout`.
fn knockout_stack(knockout: Knockout) -> Document {
    let mut top = solid("blue", full(1, 1), (0, 0, 255), 255, BlendMode::Normal, 255);
    top.fill = 128;
    top.knockout = knockout;
    doc(
        1,
        1,
        vec![
            solid("red", full(1, 1), (255, 0, 0), 255, BlendMode::Normal, 255),
            solid(
                "green",
                full(1, 1),
                (0, 255, 0),
                255,
                BlendMode::Normal,
                255,
            ),
            top,
        ],
    )
}

#[test]
fn none_knockout_shows_the_intermediate_layer() {
    let out = px(&composite_rgba(&knockout_stack(Knockout::None)), 0, 0);
    assert!(out[1] > 0, "green survives with no knockout, got {out:?}");
    assert!(out[2] > 0, "blue is present, got {out:?}");
}

#[test]
fn deep_knockout_punches_through_to_the_background() {
    let out = px(&composite_rgba(&knockout_stack(Knockout::Deep)), 0, 0);
    assert_eq!(out[1], 0, "the intermediate green is punched through");
    assert!(out[0] > 0, "the red background shows through, got {out:?}");
    assert!(out[2] > 0, "the blue knockout layer is still present");
}

#[test]
fn shallow_equals_deep_at_the_root() {
    assert_eq!(
        composite_rgba(&knockout_stack(Knockout::Shallow)).data,
        composite_rgba(&knockout_stack(Knockout::Deep)).data,
    );
}

#[test]
fn none_knockout_is_byte_identical_to_no_field() {
    let with_field = composite_rgba(&knockout_stack(Knockout::None));
    let mut layers = knockout_stack(Knockout::None).layers;
    layers[2].knockout = Knockout::None;
    let without = composite_rgba(&doc(1, 1, layers.to_vec()));
    assert_eq!(with_field.data, without.data);
}

#[test]
fn transparent_knockout_pixel_leaves_the_backdrop() {
    // A zero-alpha knockout layer covers no pixels, so the intermediate green
    // running backdrop survives.
    let mut top = solid("top", full(1, 1), (0, 0, 255), 0, BlendMode::Normal, 255);
    top.knockout = Knockout::Deep;
    let d = doc(
        1,
        1,
        vec![
            solid("red", full(1, 1), (255, 0, 0), 255, BlendMode::Normal, 255),
            solid(
                "green",
                full(1, 1),
                (0, 255, 0),
                255,
                BlendMode::Normal,
                255,
            ),
            top,
        ],
    );
    assert_eq!(px(&composite_rgba(&d), 0, 0), [0, 255, 0, 255]);
}

#[test]
fn gpu_declines_a_knockout_layer() {
    assert!(matches!(
        composite_gpu(&knockout_stack(Knockout::Deep)),
        Err(GpuError::UnsupportedAdvancedBlending)
    ));
}

/// 1x1 stack: opaque red background, then a group of opaque green and a
/// half-fill blue top carrying `knockout`. `group_blend` selects pass-through
/// (the knockout reaches the background) or an isolated mode (it stays inert).
fn knockout_group_stack(knockout: Knockout, group_blend: BlendMode) -> Document {
    let mut top = solid("blue", full(1, 1), (0, 0, 255), 255, BlendMode::Normal, 255);
    top.fill = 128;
    top.knockout = knockout;
    doc(
        1,
        1,
        vec![
            solid("red", full(1, 1), (255, 0, 0), 255, BlendMode::Normal, 255),
            group(
                "group",
                group_blend,
                255,
                None,
                vec![
                    solid(
                        "green",
                        full(1, 1),
                        (0, 255, 0),
                        255,
                        BlendMode::Normal,
                        255,
                    ),
                    top,
                ],
            ),
        ],
    )
}

#[test]
fn pass_through_group_knockout_punches_green_through() {
    let out = px(
        &composite_rgba(&knockout_group_stack(
            Knockout::Deep,
            BlendMode::PassThrough,
        )),
        0,
        0,
    );
    assert_eq!(
        out[1], 0,
        "the group's green is punched through, got {out:?}"
    );
    assert!(out[0] > 0, "the red background shows through, got {out:?}");
    assert!(
        out[2] > 0,
        "the blue knockout layer is still present, got {out:?}"
    );
}

#[test]
fn isolated_group_knockout_punches_green_through() {
    // The isolated group's knockout child bases on the group's transparent
    // initial backdrop, so the group's green is punched through; the group
    // (blue over transparency) then composites over the red background.
    let out = px(
        &composite_rgba(&knockout_group_stack(Knockout::Deep, BlendMode::Normal)),
        0,
        0,
    );
    assert_eq!(
        out[1], 0,
        "the group's green is punched through, got {out:?}"
    );
    assert!(out[0] > 0, "the red background shows through, got {out:?}");
    assert!(out[2] > 0, "blue knockout layer is present, got {out:?}");
}

#[test]
fn isolated_group_without_knockout_is_unchanged() {
    // No child carries a knockout, so the isolated branch threads `None` and the
    // group composites as before the change: blue half-fill over green inside
    // the group, then over red.
    let out = px(
        &composite_rgba(&knockout_group_stack(Knockout::None, BlendMode::Normal)),
        0,
        0,
    );
    assert_eq!(out, [0, 127, 128, 255]);
}

/// 1x1 stack: opaque red background, an opaque yellow intervening layer, then a
/// pass-through group of opaque green and a half-fill blue carrying `knockout`.
/// The group's entry backdrop is the opaque red-plus-yellow stack.
fn knockout_shallow_group_stack(knockout: Knockout) -> Document {
    let mut top = solid("blue", full(1, 1), (0, 0, 255), 255, BlendMode::Normal, 255);
    top.fill = 128;
    top.knockout = knockout;
    doc(
        1,
        1,
        vec![
            solid("red", full(1, 1), (255, 0, 0), 255, BlendMode::Normal, 255),
            solid(
                "yellow",
                full(1, 1),
                (255, 255, 0),
                255,
                BlendMode::Normal,
                255,
            ),
            group(
                "group",
                BlendMode::PassThrough,
                255,
                None,
                vec![
                    solid(
                        "green",
                        full(1, 1),
                        (0, 255, 0),
                        255,
                        BlendMode::Normal,
                        255,
                    ),
                    top,
                ],
            ),
        ],
    )
}

#[test]
fn shallow_group_knockout_stops_at_the_group_backdrop() {
    // Shallow stops at the backdrop current when the group began (red+yellow =
    // opaque yellow), so the half-fill blue composites over yellow: the group's
    // green is punched through, yet the yellow below still contributes.
    let shallow = px(
        &composite_rgba(&knockout_shallow_group_stack(Knockout::Shallow)),
        0,
        0,
    );
    assert_eq!(
        shallow,
        [127, 127, 128, 255],
        "shallow reveals the group backdrop, got {shallow:?}"
    );
}

#[test]
fn deep_group_knockout_punches_through_the_intervening_layer() {
    // Deep reaches the document background (red), punching both the group's
    // green and the intervening yellow through: the same stack reveals red.
    let deep = px(
        &composite_rgba(&knockout_shallow_group_stack(Knockout::Deep)),
        0,
        0,
    );
    assert_eq!(
        deep,
        [127, 0, 128, 255],
        "deep reveals the document background, got {deep:?}"
    );
}
