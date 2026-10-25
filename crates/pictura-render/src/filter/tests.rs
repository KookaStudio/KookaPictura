use super::*;
use pictura_core::{BlendMode, Channel, ColorLabel, LockFlags, PsdRect};
use pictura_filters::{FilterKind, IrisShape, NoiseDistribution};

fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn step_layer(w: i32, h: i32, alpha: u8) -> Layer {
    let n = (w * h) as usize;
    let plane: Vec<u8> = (0..n)
        .map(|i| if (i as i32 % w) < w / 2 { 40 } else { 200 })
        .collect();
    Layer {
        name: "px".into(),
        rect: rect(0, 0, h, w),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: plane.clone().into(),
            },
            Channel {
                id: 1,
                data: plane.clone().into(),
            },
            Channel {
                id: 2,
                data: plane.into(),
            },
            Channel {
                id: -1,
                data: vec![alpha; n].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn chan(layer: &Layer, id: i16) -> &[u8] {
    &layer.channels.iter().find(|c| c.id == id).unwrap().data
}

/// A non-uniform colour layer (the step in [`step_layer`] cannot expose a
/// too-small apron, because clamping replays the same flat values).
fn varied_layer(w: i32, h: i32) -> Layer {
    let n = (w * h) as usize;
    let mut layer = step_layer(w, h, 255);
    for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
        let plane: Vec<u8> = (0..n)
            .map(|i| {
                let x = (i % w as usize) as u32;
                let y = (i / w as usize) as u32;
                ((x * 7 + y * 13 + c as u32 * 53) % 256) as u8
            })
            .collect();
        ch.data = plane.into();
    }
    layer
}

#[test]
fn gaussian_blur_changes_interior_leaves_alpha_and_meta() {
    let mut layer = step_layer(32, 32, 123);
    let rect_before = layer.rect;
    let blend_before = layer.blend;
    let opacity_before = layer.opacity;
    let alpha_before = chan(&layer, -1).to_vec();
    let before = chan(&layer, 0).to_vec();

    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 5.0 },
        None,
        false,
    )
    .unwrap();

    assert_ne!(chan(&layer, 0), before.as_slice());
    let edge = 15usize * 32 + 16;
    assert_ne!(chan(&layer, 0)[edge], before[edge]);
    assert_eq!(chan(&layer, -1), alpha_before.as_slice());
    assert_eq!(layer.rect, rect_before);
    assert_eq!(layer.blend, blend_before);
    assert_eq!(layer.opacity, opacity_before);
    assert!(layer.mask.is_none());
}

#[test]
fn mask_confines_change_to_left_half() {
    let (w, h) = (8usize, 8usize);
    let mut layer = step_layer(w as i32, h as i32, 255);
    let before = chan(&layer, 0).to_vec();
    let mask = LayerMask {
        rect: rect(0, 0, h as i32, w as i32),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(
            (0..w * h)
                .map(|i| if i % w < w / 2 { 255 } else { 0 })
                .collect(),
        ),
        ..Default::default()
    };

    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 2.0 },
        Some(&mask),
        false,
    )
    .unwrap();

    let after = chan(&layer, 0);
    let inside = 3 * w + 3;
    assert_ne!(
        after[inside], before[inside],
        "left-half pixel should change"
    );
    for y in 0..h {
        for x in w / 2..w {
            let i = y * w + x;
            assert_eq!(
                after[i], before[i],
                "pixel ({x},{y}) outside coverage changed"
            );
        }
    }
}

#[test]
fn disabled_mask_reports_full_coverage() {
    let mask = LayerMask {
        rect: rect(0, 0, 4, 4),
        default_color: 0,
        disabled: true,
        flags: 0,
        data: Some(vec![0u8; 16].into()),
        ..Default::default()
    };
    assert_eq!(
        coverage(Some(&mask), 0, 0),
        255,
        "disabled mask must not gate"
    );
    assert_eq!(coverage(Some(&mask), 3, 3), 255);

    let enabled = LayerMask {
        disabled: false,
        ..mask
    };
    assert_eq!(
        coverage(Some(&enabled), 1, 1),
        0,
        "enabled mask still gates"
    );
}

#[test]
fn missing_color_channel_is_invalid_params() {
    let mut layer = step_layer(4, 4, 255);
    layer.channels.retain(|c| c.id != 2);
    let err = apply_filter(&mut layer, &Filter::Sharpen, None, false).unwrap_err();
    assert!(matches!(err, FilterError::InvalidParams(_)), "{err:?}");
}

#[test]
fn wrong_length_channel_is_invalid_params() {
    let mut layer = step_layer(4, 4, 255);
    for c in &mut layer.channels {
        if c.id == 1 {
            c.data.truncate(3);
        }
    }
    let err = apply_filter(&mut layer, &Filter::Sharpen, None, false).unwrap_err();
    assert!(matches!(err, FilterError::InvalidParams(_)), "{err:?}");
}

#[test]
fn empty_layer_rect_is_noop() {
    let mut layer = step_layer(4, 4, 255);
    layer.rect = rect(0, 0, 4, 0);
    assert!(apply_filter(&mut layer, &Filter::Median { radius: 200 }, None, false).is_ok());
    layer.rect = rect(0, 0, 0, 4);
    assert!(apply_filter(&mut layer, &Filter::Median { radius: 200 }, None, false).is_ok());
}

#[test]
fn filter_parameter_error_propagates() {
    let mut layer = step_layer(4, 4, 255);
    let err = apply_filter(&mut layer, &Filter::Median { radius: 200 }, None, false).unwrap_err();
    assert!(matches!(err, FilterError::InvalidParams(_)), "{err:?}");
}

#[test]
fn pixel_locked_layer_is_refused_and_unchanged() {
    let mut layer = step_layer(8, 8, 255);
    layer.lock = LockFlags::default().with(LockFlags::PIXELS, true);
    let before = layer.clone();
    let err = apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 2.0 },
        None,
        false,
    )
    .unwrap_err();
    assert!(matches!(err, FilterError::Locked), "{err:?}");
    assert_eq!(layer, before, "refusal leaves every channel bit-identical");
}

#[test]
fn transparency_lock_keeps_alpha_and_skips_clear_pixels() {
    let (w, h) = (4usize, 4usize);
    let mut layer = step_layer(w as i32, h as i32, 255);
    for c in &mut layer.channels {
        if c.id == -1 {
            c.data[w + 1] = 0;
            c.data[w + 2] = 180;
        }
    }
    layer.lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    let alpha_before = chan(&layer, -1).to_vec();
    let before = chan(&layer, 0).to_vec();

    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 2.0 },
        None,
        false,
    )
    .unwrap();

    assert_eq!(
        chan(&layer, -1),
        alpha_before.as_slice(),
        "alpha is bit-identical"
    );
    assert_eq!(
        chan(&layer, 0)[w + 1],
        before[w + 1],
        "a fully transparent pixel is untouched"
    );
    assert_eq!(chan(&layer, -1)[w + 2], 180);
    assert!(
        chan(&layer, 0)
            .iter()
            .enumerate()
            .any(|(i, v)| { alpha_before[i] > 0 && *v != before[i] }),
        "an opaque pixel still takes the filter"
    );
}

#[test]
fn region_apply_matches_full_inside_and_leaves_the_rest() {
    let filter = Filter::GaussianBlur { radius: 2.0 };
    let full = step_layer(32, 32, 255);
    let region = rect(8, 8, 24, 24);
    let mut partial = full.clone();
    apply_filter_region(&mut partial, &filter, None, false, region).unwrap();
    let mut whole = full.clone();
    apply_filter(&mut whole, &filter, None, false).unwrap();

    let w = 32usize;
    // The blurred step edge at the region centre matches the full apply.
    assert_eq!(
        chan(&partial, 0)[16 * w + 16],
        chan(&whole, 0)[16 * w + 16],
        "inside the region matches a full apply"
    );
    // A corner outside the region is untouched.
    assert_eq!(
        chan(&partial, 0)[0],
        chan(&full, 0)[0],
        "outside the region is not filtered"
    );
}

#[test]
fn unlocked_layer_filters_its_transparency() {
    let (w, h) = (8usize, 8usize);
    let mut layer = step_layer(w as i32, h as i32, 255);
    for c in &mut layer.channels {
        if c.id == -1 {
            for i in 0..w * h {
                c.data[i] = if i % w < w / 2 { 255 } else { 0 };
            }
        }
    }
    let alpha_before = chan(&layer, -1).to_vec();
    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 1.5 },
        None,
        false,
    )
    .unwrap();
    assert_ne!(
        chan(&layer, -1),
        alpha_before.as_slice(),
        "an unlocked layer's alpha is filtered"
    );
}

/// Kinds whose kernel reads absolute values, replaces the planes, or adds
/// onto them, so a rerun over alpha would rewrite opacity (#248).
fn value_kernels() -> Vec<Filter> {
    use pictura_filters::{ContourEdge, LensType, NoiseDistribution, TileFill};
    vec![
        Filter::GlowingEdges {
            width: 2,
            brightness: 6,
            smoothness: 1,
        },
        Filter::Lighting {
            lighting: pictura_filters::Lighting::default(),
        },
        Filter::HdrToning(pictura_filters::HdrToningParams::default()),
        Filter::Solarize,
        Filter::Clouds {
            color_a: [0; 3],
            color_b: [255; 3],
            starker: false,
            seed: 7,
        },
        Filter::DifferenceClouds {
            color_a: [0; 3],
            color_b: [255; 3],
            starker: false,
            seed: 7,
        },
        Filter::Fibers {
            variance: 16.0,
            strength: 4.0,
            color_a: [0; 3],
            color_b: [255; 3],
            seed: 7,
        },
        Filter::LensFlare {
            brightness: 300.0,
            center: (0.75, 0.5),
            lens: LensType::Zoom,
        },
        Filter::TraceContour {
            level: 128,
            edge: ContourEdge::Upper,
        },
        Filter::Tiles {
            count: 4,
            offset: 30,
            fill: TileFill::BackgroundColor,
            foreground: [255; 3],
            background: [0; 3],
        },
        Filter::FindEdges,
        Filter::Emboss {
            angle: 135.0,
            height: 3.0,
            amount: 100.0,
        },
        Filter::HighPass { radius: 4.0 },
        Filter::AddNoise {
            amount: 25.0,
            distribution: NoiseDistribution::Uniform,
            monochromatic: false,
            seed: 7,
        },
    ]
}

/// The right half of `layer` fully transparent, the left half opaque.
fn half_clear(mut layer: Layer) -> Layer {
    let w = layer.rect.width() as usize;
    for c in &mut layer.channels {
        if c.id == -1 {
            for i in 0..c.data.len() {
                if i % w >= w / 2 {
                    c.data[i] = 0;
                }
            }
        }
    }
    layer
}

#[test]
fn value_kernels_leave_alpha_bit_identical() {
    for filter in value_kernels() {
        for mut layer in [step_layer(16, 16, 255), half_clear(step_layer(16, 16, 255))] {
            let alpha_before = chan(&layer, -1).to_vec();
            apply_filter(&mut layer, &filter, None, false).unwrap();
            assert_eq!(
                chan(&layer, -1),
                alpha_before.as_slice(),
                "{filter:?} must leave alpha bit-identical"
            );
        }
    }
}

#[test]
fn moving_kernels_keep_an_opaque_layer_opaque() {
    use pictura_filters::{PolarKind, RippleSize, ShearFill};
    let filters = [
        Filter::GaussianBlur { radius: 2.0 },
        Filter::UnsharpMask {
            amount: 150.0,
            radius: 2.0,
            threshold: 0,
        },
        Filter::Median { radius: 2 },
        Filter::Mosaic { cell_size: 5 },
        Filter::Twirl { angle: 90.0 },
        Filter::Ripple {
            amount: 200.0,
            size: RippleSize::Medium,
        },
        Filter::PolarCoordinates {
            kind: PolarKind::RectangularToPolar,
        },
        Filter::Shear {
            curve: vec![(0.0, 0.0), (0.5, 0.4), (1.0, 0.0)],
            fill: ShearFill::RepeatEdgePixels,
        },
        Filter::Offset {
            horizontal: 2000,
            vertical: 2000,
            wrap: false,
            repeat: false,
            background: [0; 3],
            transparent: false,
        },
    ];
    for filter in filters {
        let mut layer = step_layer(16, 16, 255);
        apply_filter(&mut layer, &filter, None, false).unwrap();
        assert!(
            chan(&layer, -1).iter().all(|&a| a == 255),
            "{filter:?} made an opaque layer transparent"
        );
    }
}

/// A CPU-preferred kernel gives the CPU oracle's bytes even with the GPU
/// enabled.
#[test]
fn cpu_preferred_kernels_skip_the_gpu() {
    for filter in [
        Filter::GaussianBlur { radius: 3.0 },
        Filter::UnsharpMask {
            amount: 120.0,
            radius: 2.0,
            threshold: 0,
        },
        Filter::Median { radius: 2 },
        Filter::Maximum { radius: 2 },
    ] {
        assert!(filter.cpu_outruns_gpu());
        let mut gpu = varied_layer(40, 30);
        let mut cpu = gpu.clone();
        apply_filter(&mut gpu, &filter, None, true).unwrap();
        apply_filter(&mut cpu, &filter, None, false).unwrap();
        assert_eq!(gpu.channels, cpu.channels, "{filter:?}");
    }
}

/// One of every kind [`pictura_filters::Filter::moves_alpha`] names, at
/// settings that visibly move a 24×24 picture.
fn moving_kernels() -> Vec<Filter> {
    use pictura_filters::{
        BlurNoise, BlurPath, BlurPin, BokehParams, DisplaceFit, DisplaceMap, FrameOptions,
        GlassTexture, IrisPin, LiquifyBrush, LiquifyMesh, LiquifyTool, PathMotion, PolarKind,
        Quality, RadialMethod, RippleSize, SharpenRemove, ShearFill, SmartBlurMode, SpherizeMode,
        SpinPin, SpinStrobe, TiltPin, TonalFade, WaveType, ZigZagStyle,
    };
    let pin = BlurPin {
        x: 0.5,
        y: 0.5,
        blur: 6.0,
    };
    let iris = IrisPin {
        radius_x: 0.1,
        radius_y: 0.1,
        ..IrisPin::default()
    };
    let path = BlurPath {
        points: vec![(0.3, 0.5), (0.7, 0.5)],
        start_speed: 12.0,
        end_speed: 12.0,
    };
    let spin = SpinPin {
        radius_x: 0.45,
        radius_y: 0.45,
        feather: 1.0,
        blur_angle: 120.0,
        ..SpinPin::default()
    };
    let mut mesh = LiquifyMesh::new(24, 24);
    let brush = LiquifyBrush::new(LiquifyTool::ForwardWarp, 16.0);
    mesh.stroke_begin(&brush, [6.0, 12.0, 1.0]);
    mesh.stroke_to(&brush, [6.0, 12.0, 1.0], [14.0, 12.0, 1.0]);
    vec![
        Filter::GaussianBlur { radius: 2.0 },
        Filter::BoxBlur { radius: 3 },
        Filter::MotionBlur {
            angle: 30.0,
            distance: 9,
        },
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 20.0,
            center: (0.5, 0.5),
            quality: Quality::Good,
        },
        Filter::Average,
        Filter::Blur,
        Filter::BlurMore,
        Filter::SurfaceBlur {
            radius: 4,
            threshold: 20,
        },
        Filter::LensBlur {
            depth: None,
            focal: 128,
            invert: false,
            shape: IrisShape::Triangle,
            radius: 6,
            curvature: 50,
            rotation: 30.0,
            specular_brightness: 100,
            specular_threshold: 0,
            noise_amount: 100,
            noise_distribution: NoiseDistribution::Gaussian,
            monochromatic: false,
            faster: false,
            seed: 3,
        },
        Filter::Sharpen,
        Filter::SharpenMore,
        Filter::SharpenEdges,
        Filter::UnsharpMask {
            amount: 150.0,
            radius: 2.0,
            threshold: 0,
        },
        Filter::SmartSharpen {
            amount: 150.0,
            radius: 2.0,
            reduce_noise: 10.0,
            remove: SharpenRemove::GaussianBlur,
            angle: 0.0,
            more_accurate: true,
            shadow: TonalFade::default(),
            highlight: TonalFade::default(),
        },
        Filter::Median { radius: 2 },
        Filter::Despeckle,
        Filter::Maximum { radius: 2 },
        Filter::Minimum { radius: 2 },
        Filter::DustAndScratches {
            radius: 2,
            threshold: 0,
        },
        Filter::Offset {
            horizontal: 5,
            vertical: -3,
            wrap: false,
            repeat: false,
            background: [0; 3],
            transparent: false,
        },
        Filter::Mosaic { cell_size: 5 },
        Filter::Crystallize {
            cell_size: 6,
            seed: 7,
        },
        Filter::Fragment,
        Filter::Twirl { angle: 90.0 },
        Filter::Pinch { amount: 60.0 },
        Filter::Spherize {
            amount: 80.0,
            mode: SpherizeMode::Normal,
        },
        Filter::Ripple {
            amount: 200.0,
            size: RippleSize::Medium,
        },
        Filter::Wave {
            generators: 3,
            wavelength: (10.0, 60.0),
            amplitude: (5.0, 20.0),
            kind: WaveType::Sine,
            scale: (100.0, 100.0),
            seed: 7,
            repeat_edge: false,
        },
        Filter::PolarCoordinates {
            kind: PolarKind::RectangularToPolar,
        },
        Filter::Shear {
            curve: vec![(0.0, 0.0), (0.5, 0.4), (1.0, 0.0)],
            fill: ShearFill::RepeatEdgePixels,
        },
        Filter::ZigZag {
            amount: 40.0,
            ridges: 5,
            style: ZigZagStyle::PondRipples,
        },
        Filter::OceanRipple {
            size: 9,
            magnitude: 9,
            seed: 7,
        },
        Filter::Glass {
            distortion: 5,
            smoothness: 3,
            texture: GlassTexture::Frosted,
            scaling: 100,
            invert: false,
        },
        Filter::ShapeBlur {
            shape: 0,
            radius: 3,
        },
        Filter::SmartBlur {
            radius: 4.0,
            threshold: 100.0,
            quality: Quality::Good,
            mode: SmartBlurMode::Normal,
        },
        Filter::FieldBlur {
            pins: vec![pin],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::IrisBlur {
            pins: vec![iris],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::TiltShift {
            pins: vec![TiltPin::default()],
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::PathBlur {
            paths: vec![path.clone()],
            motion: PathMotion::default(),
            noise: BlurNoise::default(),
        },
        Filter::SpinBlur {
            spins: vec![spin],
            strobe: SpinStrobe::default(),
            noise: BlurNoise::default(),
        },
        Filter::BlurGallery {
            field: vec![pin],
            iris: vec![iris],
            tilt: vec![TiltPin::default()],
            path: vec![path],
            motion: PathMotion::default(),
            spin: vec![spin],
            spin_strobe: SpinStrobe::default(),
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        },
        Filter::Displace {
            map: std::sync::Arc::new(
                DisplaceMap::new(2, 2, vec![0, 255, 255, 0], vec![255, 0, 0, 255]).unwrap(),
            ),
            horizontal_scale: 10.0,
            vertical_scale: 10.0,
            fit: DisplaceFit::StretchToFit,
            wrap: false,
        },
        Filter::Liquify {
            mesh: std::sync::Arc::new(mesh),
        },
        // A long lens at 150 % covers the frame, so opaque stays opaque.
        Filter::AdaptiveWideAngle {
            params: pictura_filters::AwaParams {
                correction: pictura_filters::AwaCorrection::Perspective,
                scale: 1.5,
                focal_length: 15.31,
                crop_factor: 10.0,
                ..Default::default()
            },
        },
        // Grid lines across the soft disc's edge.
        Filter::VanishingPoint {
            edit: std::sync::Arc::new(pictura_filters::VpEdit {
                patch: None,
                lines: vec![[(0.0, 12.0), (24.0, 12.0)], [(12.0, 0.0), (12.0, 24.0)]],
            }),
        },
        Filter::PictureFrame {
            options: FrameOptions::default(),
        },
    ]
}

/// The fixture holds exactly the kinds the moving-alpha fact selects.
#[test]
fn moving_kernel_fixture_matches_the_moving_set() {
    let fixture = moving_kernels();
    let names: std::collections::BTreeSet<&str> =
        fixture.iter().map(|filter| filter.kind().name()).collect();
    let expected: std::collections::BTreeSet<&str> = FilterKind::ALL
        .iter()
        .filter(|kind| kind.moves_alpha())
        .map(|kind| kind.name())
        .collect();
    assert_eq!(
        names, expected,
        "the fixture must hold exactly the moving-alpha variants"
    );
    assert_eq!(fixture.len(), names.len(), "the fixture repeats a variant");
}

/// An unlocked layer folds its alpha into the working buffer, so every moving
/// kernel's run over a flat 255 plane must map it back to 255 — otherwise an
/// opaque layer would gain transparency it never had.
#[test]
fn moving_kernels_map_opaque_to_opaque() {
    for filter in moving_kernels() {
        assert!(filter.moves_alpha(), "{filter:?}");
        let mut buf = PixelBuffer::new(24, 24, 4);
        buf.data.as_mut_slice().fill(255);
        pictura_filters::apply(&filter, &mut buf).unwrap();
        let n = buf.pixel_count();
        assert!(
            buf.data[3 * n..].iter().all(|&a| a == 255),
            "{filter:?} does not map opaque to opaque"
        );
    }
}

/// Every moving kernel actually runs over an unlocked layer's alpha: a soft
/// disc with a stray speck must come out changed (#406). A kernel that capped
/// at three planes would hand the fold its alpha back untouched.
#[test]
fn moving_kernels_move_a_soft_layer_edge() {
    let (w, h) = (24usize, 24usize);
    let mut layer = varied_layer(w as i32, h as i32);
    let alpha: Vec<u8> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f64 - 11.5, (i / w) as f64 - 11.5);
            let soft = ((7.0 - x.hypot(y)) * 48.0 + 128.0).clamp(0.0, 255.0) as u8;
            if i == 2 * w + 2 {
                255
            } else {
                soft
            }
        })
        .collect();
    layer.channels.iter_mut().find(|c| c.id == -1).unwrap().data = alpha.clone().into();
    for filter in moving_kernels() {
        let mut out = layer.clone();
        apply_filter(&mut out, &filter, None, false).unwrap();
        assert_ne!(
            chan(&out, -1),
            alpha.as_slice(),
            "{filter:?} left the layer edge where it was"
        );
    }
}

#[test]
fn offset_moves_the_layer_edge_and_fills_what_it_exposes_opaque() {
    let mut layer = half_clear(step_layer(16, 16, 255));
    let filter = Filter::Offset {
        horizontal: 4,
        vertical: 0,
        wrap: false,
        repeat: false,
        background: [0; 3],
        transparent: false,
    };
    apply_filter(&mut layer, &filter, None, false).unwrap();
    let alpha = chan(&layer, -1);
    for (x, expected) in [(0, 255), (3, 255), (4, 255), (11, 255), (12, 0), (15, 0)] {
        assert_eq!(alpha[x], expected, "alpha at x = {x}");
    }
}

#[test]
fn offset_in_transparent_mode_exposes_clear_pixels() {
    let mut layer = half_clear(step_layer(16, 16, 255));
    let filter = Filter::Offset {
        horizontal: 4,
        vertical: 0,
        wrap: false,
        repeat: false,
        background: [10, 20, 30],
        transparent: true,
    };
    apply_filter(&mut layer, &filter, None, false).unwrap();
    let alpha = chan(&layer, -1);
    let red = chan(&layer, 0);
    for x in 0..4 {
        assert_eq!(alpha[x], 0, "exposed pixel {x} must be clear");
        assert_eq!(red[x], 10, "exposed pixel {x} takes the background colour");
    }
    assert_eq!(alpha[4], 255, "the moved opaque half stays opaque");
    assert_eq!(red[4], 40, "the moved half keeps its colour");
}

#[test]
fn picture_frame_adds_opacity_over_a_clear_pixel() {
    let mut layer = step_layer(64, 64, 0);
    apply_filter(
        &mut layer,
        &Filter::PictureFrame {
            options: pictura_filters::FrameOptions::default(),
        },
        None,
        false,
    )
    .unwrap();
    assert!(
        chan(&layer, -1).iter().any(|&alpha| alpha > 0),
        "the composited frame must add opacity over clear pixels"
    );
}

#[test]
fn lens_blur_apron_covers_its_radius() {
    let filter = Filter::LensBlur {
        depth: None,
        focal: 0,
        invert: false,
        shape: IrisShape::Triangle,
        radius: 40,
        curvature: 0,
        rotation: 0.0,
        specular_brightness: 0,
        specular_threshold: 255,
        noise_amount: 0,
        noise_distribution: NoiseDistribution::Uniform,
        monochromatic: false,
        faster: false,
        seed: 1,
    };
    assert_eq!(
        filter.preview_apron(),
        40,
        "the apron covers the largest CoC"
    );
}

#[test]
fn hdr_toning_apron_covers_its_blur_support() {
    let (w, h) = (512i32, 64i32);
    let filter = Filter::HdrToning(pictura_filters::HdrToningParams {
        radius: 40.0,
        gamma: 1.0,
        ..Default::default()
    });
    assert_eq!(filter.preview_apron(), 120, "apron must be 3 * sigma");

    let viewport = rect(16, 160, 48, 352);
    let base = varied_layer(w, h);
    let mut whole = base.clone();
    apply_filter(&mut whole, &filter, None, false).unwrap();

    let apron = filter.preview_apron();
    let expanded = rect(
        viewport.top - apron,
        viewport.left - apron,
        viewport.bottom + apron,
        viewport.right + apron,
    );
    let mut region = base.clone();
    apply_filter_region(&mut region, &filter, None, false, expanded).unwrap();

    let lw = w as usize;
    for y in viewport.top..viewport.bottom {
        for x in viewport.left..viewport.right {
            let i = y as usize * lw + x as usize;
            assert_eq!(
                chan(&region, 0)[i],
                chan(&whole, 0)[i],
                "expanded-region preview differs from a full apply at ({x},{y})"
            );
        }
    }
}

#[test]
fn an_adjustment_changes_colour_inside_the_region_and_never_alpha() {
    let (w, h) = (8usize, 8usize);
    let mut layer = step_layer(w as i32, h as i32, 255);
    for c in &mut layer.channels {
        if c.id == -1 {
            for i in 0..w * h {
                c.data[i] = if i % w < w / 2 { 255 } else { 90 };
            }
        }
    }
    let alpha_before = chan(&layer, -1).to_vec();
    let colour_before = chan(&layer, 0).to_vec();
    let left = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: (w / 2) as i32,
    };
    apply_adjustment_region(&mut layer, &Adjustment::Invert, None, left).unwrap();
    assert_eq!(chan(&layer, -1), alpha_before.as_slice(), "alpha untouched");
    let after = chan(&layer, 0);
    assert_eq!(
        after[0],
        255 - colour_before[0],
        "inside the region inverts"
    );
    assert_eq!(after[w - 1], colour_before[w - 1], "outside it does not");
}

#[test]
fn auto_tone_uses_selection_histogram() {
    let w = 100usize;
    let mut layer = step_layer(w as i32, 1, 255);
    for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
        for x in 0..w {
            ch.data[x] = if x < w / 2 {
                (10 + x * 50 / (w / 2 - 1)) as u8
            } else {
                (200 + (x - w / 2) * 50 / (w / 2 - 1)) as u8
            };
        }
    }
    let mask = LayerMask {
        rect: rect(0, 0, 1, w as i32),
        default_color: 0,
        data: Some(
            (0..w)
                .map(|x| if x < w / 2 { 255u8 } else { 0 })
                .collect::<Vec<_>>()
                .into(),
        ),
        ..Default::default()
    };
    let right_before: Vec<u8> = chan(&layer, 0)[w / 2..].to_vec();
    apply_adjustment_region(
        &mut layer,
        &Adjustment::Auto(pictura_adjust::AutoKind::Tone),
        Some(&mask),
        rect(0, 0, 1, w as i32),
    )
    .unwrap();
    let out = chan(&layer, 0);
    assert!(
        out[0] <= 10,
        "selected dark end did not stretch: {}",
        out[0]
    );
    assert!(
        out[w / 2 - 1] >= 245,
        "selected light end did not stretch: {}",
        out[w / 2 - 1]
    );
    assert_eq!(&out[w / 2..], &right_before[..], "unselected ramp changed");
}

#[test]
fn position_and_transparency_locks_do_not_block_a_filter() {
    for flag in [LockFlags::POSITION, LockFlags::TRANSPARENCY] {
        let mut layer = step_layer(8, 8, 255);
        layer.lock = LockFlags::default().with(flag, true);
        assert!(
            apply_filter(
                &mut layer,
                &Filter::GaussianBlur { radius: 2.0 },
                None,
                false
            )
            .is_ok(),
            "flag {flag} must not refuse an opaque-preserving filter"
        );
    }
}

#[test]
fn grayscale_single_channel_layer_filters_channel_zero() {
    let (w, h) = (32usize, 32usize);
    let mut layer = step_layer(w as i32, h as i32, 123);
    layer.channels.retain(|c| c.id == 0 || c.id == -1);
    let before = chan(&layer, 0).to_vec();
    let alpha_before = chan(&layer, -1).to_vec();

    apply_filter(
        &mut layer,
        &Filter::GaussianBlur { radius: 3.0 },
        None,
        false,
    )
    .unwrap();

    assert_ne!(
        chan(&layer, 0),
        before.as_slice(),
        "grayscale plane changes"
    );
    assert_eq!(chan(&layer, -1), alpha_before.as_slice(), "alpha untouched");
    assert_eq!(layer.channels.len(), 2, "no color planes are added");
}

#[test]
fn missing_channel_zero_is_invalid_params() {
    let mut layer = step_layer(4, 4, 255);
    layer.channels.retain(|c| c.id == -1);
    let err = apply_filter(&mut layer, &Filter::Sharpen, None, false).unwrap_err();
    assert!(matches!(err, FilterError::InvalidParams(_)), "{err:?}");
}

#[test]
fn mixed_color_and_missing_channel_is_invalid_params() {
    // Channel 1 present but channel 2 absent is malformed, not Grayscale.
    let mut layer = step_layer(4, 4, 255);
    layer.channels.retain(|c| c.id != 2);
    let err = apply_filter(&mut layer, &Filter::Sharpen, None, false).unwrap_err();
    assert!(matches!(err, FilterError::InvalidParams(_)), "{err:?}");
}
