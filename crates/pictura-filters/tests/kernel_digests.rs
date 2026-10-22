//! Byte-exact digests of the row-parallel kernels (#152), taken from their
//! serial versions: parallelizing a kernel must not move a single sample.

use pictura_core::PixelBuffer;
use pictura_filters::{
    apply, BlurNoise, BlurNoiseKind, BlurPath, BlurPin, BokehParams, ChannelDetail, DepthMap,
    Filter, FilterKind, IrisPin, IrisShape, NoiseDistribution, PathMotion, Quality, RadialMethod,
    ReduceNoiseParams, SmartBlurMode, SpinPin, SpinStrobe, TiltPin,
};

/// A deterministic 4-channel picture with noise, gradients and hard edges, in
/// odd sizes so rows and column strips never divide evenly.
fn picture() -> PixelBuffer {
    let (w, h) = (97u32, 61u32);
    let mut b = PixelBuffer::new(w, h, 4);
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for (i, v) in b.data.as_mut_slice().iter_mut().enumerate() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let x = i % w as usize;
        let edge = if (x / 9).is_multiple_of(2) { 0 } else { 120 };
        *v = ((state >> 59) as usize + x * 2 + edge) as u8;
    }
    b
}

/// FNV-1a over every byte.
fn digest(buf: &PixelBuffer) -> u64 {
    buf.data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A horizontal depth ramp matching `picture`'s size.
fn lens_depth() -> DepthMap {
    let (w, h) = (97u32, 61u32);
    let samples: Vec<u8> = (0..w * h).map(|i| ((i % w) * 255 / 96) as u8).collect();
    DepthMap::new(w, h, samples).unwrap()
}

fn kernels() -> Vec<(&'static str, Filter)> {
    vec![
        ("gaussian 1.5", Filter::GaussianBlur { radius: 1.5 }),
        ("gaussian 9", Filter::GaussianBlur { radius: 9.0 }),
        (
            "unsharp",
            Filter::UnsharpMask {
                amount: 180.0,
                radius: 2.5,
                threshold: 3,
            },
        ),
        ("median 1", Filter::Median { radius: 1 }),
        ("median 4", Filter::Median { radius: 4 }),
        (
            "reduce noise",
            Filter::ReduceNoise(ReduceNoiseParams {
                reduce_color_noise: 50.0,
                sharpen_details: 40.0,
                remove_jpeg_artifacts: true,
                channels: [
                    ChannelDetail {
                        strength: 7.0,
                        preserve_details: 20.0,
                    },
                    ChannelDetail {
                        strength: 5.0,
                        preserve_details: 40.0,
                    },
                    ChannelDetail {
                        strength: 9.0,
                        preserve_details: 0.0,
                    },
                ],
            }),
        ),
        (
            "motion 30",
            Filter::MotionBlur {
                angle: 30.0,
                distance: 9,
            },
        ),
        (
            "motion -75",
            Filter::MotionBlur {
                angle: -75.0,
                distance: 40,
            },
        ),
        ("box 2", Filter::BoxBlur { radius: 2 }),
        ("box 70", Filter::BoxBlur { radius: 70 }),
        (
            "shape heart 7",
            Filter::ShapeBlur {
                shape: 1,
                radius: 7,
            },
        ),
        (
            "shape arrow 40",
            Filter::ShapeBlur {
                shape: 2,
                radius: 40,
            },
        ),
        (
            "smart normal 6",
            Filter::SmartBlur {
                radius: 6.0,
                threshold: 25.0,
                quality: Quality::Best,
                mode: SmartBlurMode::Normal,
            },
        ),
        (
            "smart overlay 40",
            Filter::SmartBlur {
                radius: 40.0,
                threshold: 60.0,
                quality: Quality::Draft,
                mode: SmartBlurMode::OverlayEdge,
            },
        ),
        (
            "smart edges 3",
            Filter::SmartBlur {
                radius: 3.0,
                threshold: 30.0,
                quality: Quality::Good,
                mode: SmartBlurMode::EdgeOnly,
            },
        ),
        ("maximum 2", Filter::Maximum { radius: 2 }),
        ("minimum 5", Filter::Minimum { radius: 5 }),
        (
            "custom",
            Filter::Custom {
                kernel: [
                    [0.0, -1.0, 0.0, 1.0, 0.0],
                    [-1.0, 2.0, -3.0, 2.0, -1.0],
                    [0.0, -3.0, 13.0, -3.0, 0.0],
                    [-1.0, 2.0, -3.0, 2.0, -1.0],
                    [0.0, 1.0, 0.0, -1.0, 0.5],
                ],
                scale: 3.0,
                offset: 7.0,
            },
        ),
        ("high pass", Filter::HighPass { radius: 3.5 }),
        (
            "radial spin",
            Filter::RadialBlur {
                method: RadialMethod::Spin,
                amount: 30.0,
                center: (0.5, 0.5),
                quality: Quality::Good,
            },
        ),
        (
            "radial zoom",
            Filter::RadialBlur {
                method: RadialMethod::Zoom,
                amount: 60.0,
                center: (0.5, 0.5),
                quality: Quality::Best,
            },
        ),
        (
            "lens blur",
            Filter::LensBlur {
                depth: Some(std::sync::Arc::new(lens_depth())),
                focal: 96,
                invert: false,
                shape: IrisShape::Hexagon,
                radius: 9,
                curvature: 40,
                rotation: 23.0,
                specular_brightness: 60,
                specular_threshold: 180,
                noise_amount: 25,
                noise_distribution: NoiseDistribution::Gaussian,
                monochromatic: false,
                faster: false,
                seed: 7,
            },
        ),
        (
            "field blur",
            Filter::FieldBlur {
                pins: vec![
                    BlurPin {
                        x: 0.2,
                        y: 0.3,
                        blur: 12.0,
                    },
                    BlurPin {
                        x: 0.8,
                        y: 0.7,
                        blur: 45.0,
                    },
                ],
                bokeh: BokehParams {
                    enabled: true,
                    light_bokeh: 30.0,
                    bokeh_color: 50.0,
                    light_range: (40, 200),
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    size: 40.0,
                    roughness: 30.0,
                    color: 25.0,
                    highlights: 10.0,
                    kind: BlurNoiseKind::Grain,
                    seed: 7,
                },
            },
        ),
        (
            "iris blur",
            Filter::IrisBlur {
                pins: vec![
                    IrisPin {
                        x: 0.4,
                        y: 0.45,
                        radius_x: 0.25,
                        radius_y: 0.15,
                        angle: 30.0,
                        squareness: 40.0,
                        feather: 0.5,
                        blur: 18.0,
                    },
                    IrisPin {
                        x: 0.8,
                        y: 0.75,
                        radius_x: 0.1,
                        radius_y: 0.1,
                        angle: 0.0,
                        squareness: 0.0,
                        feather: 0.3,
                        blur: 40.0,
                    },
                ],
                bokeh: BokehParams {
                    enabled: true,
                    light_bokeh: 30.0,
                    bokeh_color: 50.0,
                    light_range: (40, 200),
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    size: 40.0,
                    roughness: 30.0,
                    color: 25.0,
                    highlights: 10.0,
                    kind: BlurNoiseKind::Gaussian,
                    seed: 7,
                },
            },
        ),
        (
            "tilt-shift",
            Filter::TiltShift {
                pins: vec![
                    TiltPin {
                        x: 0.45,
                        y: 0.4,
                        angle: 20.0,
                        focus_above: 0.08,
                        focus_below: 0.05,
                        fade_above: 0.1,
                        fade_below: 0.2,
                        distortion: 60.0,
                        symmetric: false,
                        blur: 24.0,
                    },
                    TiltPin {
                        x: 0.7,
                        y: 0.85,
                        angle: -10.0,
                        focus_above: 0.03,
                        focus_below: 0.03,
                        fade_above: 0.05,
                        fade_below: 0.05,
                        distortion: -40.0,
                        symmetric: true,
                        blur: 12.0,
                    },
                ],
                bokeh: BokehParams {
                    enabled: true,
                    light_bokeh: 30.0,
                    bokeh_color: 50.0,
                    light_range: (40, 200),
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    size: 40.0,
                    roughness: 30.0,
                    color: 25.0,
                    highlights: 10.0,
                    kind: BlurNoiseKind::Uniform,
                    seed: 7,
                },
            },
        ),
        (
            "blur gallery",
            Filter::BlurGallery {
                field: vec![BlurPin {
                    x: 0.9,
                    y: 0.1,
                    blur: 9.0,
                }],
                iris: vec![IrisPin {
                    x: 0.2,
                    y: 0.7,
                    radius_x: 0.15,
                    radius_y: 0.1,
                    angle: 10.0,
                    squareness: 30.0,
                    feather: 0.4,
                    blur: 20.0,
                }],
                tilt: vec![
                    TiltPin {
                        x: 0.45,
                        y: 0.4,
                        angle: 20.0,
                        focus_above: 0.08,
                        focus_below: 0.05,
                        fade_above: 0.1,
                        fade_below: 0.2,
                        distortion: 60.0,
                        symmetric: false,
                        blur: 24.0,
                    },
                    TiltPin {
                        x: 0.7,
                        y: 0.85,
                        angle: -10.0,
                        focus_above: 0.03,
                        focus_below: 0.03,
                        fade_above: 0.05,
                        fade_below: 0.05,
                        distortion: -40.0,
                        symmetric: true,
                        blur: 12.0,
                    },
                ],
                path: Vec::new(),
                motion: PathMotion::default(),
                spin: Vec::new(),
                spin_strobe: SpinStrobe::default(),
                bokeh: BokehParams {
                    enabled: true,
                    light_bokeh: 30.0,
                    bokeh_color: 50.0,
                    light_range: (40, 200),
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    size: 40.0,
                    roughness: 30.0,
                    color: 25.0,
                    highlights: 10.0,
                    kind: BlurNoiseKind::Uniform,
                    seed: 7,
                },
            },
        ),
        (
            "path blur",
            Filter::PathBlur {
                paths: path_blur_paths(),
                motion: PathMotion {
                    speed: 120.0,
                    taper: 25.0,
                    centered: false,
                    rear_sync: true,
                    strobe_strength: 30.0,
                    strobe_flashes: 5,
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 15.0,
                    seed: 3,
                    ..BlurNoise::default()
                },
            },
        ),
        (
            "blur gallery paths",
            Filter::BlurGallery {
                field: vec![BlurPin {
                    x: 0.9,
                    y: 0.1,
                    blur: 9.0,
                }],
                iris: Vec::new(),
                tilt: Vec::new(),
                path: path_blur_paths(),
                motion: PathMotion::default(),
                spin: Vec::new(),
                spin_strobe: SpinStrobe::default(),
                bokeh: BokehParams {
                    enabled: true,
                    light_bokeh: 30.0,
                    bokeh_color: 50.0,
                    light_range: (40, 200),
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    seed: 7,
                    ..BlurNoise::default()
                },
            },
        ),
        (
            "spin blur",
            Filter::SpinBlur {
                spins: spin_blur_spins(),
                strobe: SpinStrobe {
                    strength: 40.0,
                    flashes: 3,
                    duration: 8.0,
                },
                noise: BlurNoise {
                    enabled: true,
                    amount: 15.0,
                    seed: 3,
                    ..BlurNoise::default()
                },
            },
        ),
        (
            "blur gallery spins",
            Filter::BlurGallery {
                field: vec![BlurPin {
                    x: 0.9,
                    y: 0.1,
                    blur: 9.0,
                }],
                iris: Vec::new(),
                tilt: Vec::new(),
                path: path_blur_paths(),
                motion: PathMotion::default(),
                spin: spin_blur_spins(),
                spin_strobe: SpinStrobe::default(),
                bokeh: BokehParams::default(),
                noise: BlurNoise {
                    enabled: true,
                    amount: 20.0,
                    seed: 7,
                    ..BlurNoise::default()
                },
            },
        ),
    ]
}

/// An elliptical spin with a moved rotation centre and a small circle.
fn spin_blur_spins() -> Vec<SpinPin> {
    vec![
        SpinPin {
            x: 0.4,
            y: 0.45,
            radius_x: 0.35,
            radius_y: 0.2,
            angle: 30.0,
            feather: 0.6,
            blur_angle: 70.0,
            pivot_x: 0.05,
            pivot_y: -0.03,
        },
        SpinPin {
            x: 0.8,
            y: 0.75,
            blur_angle: 200.0,
            ..SpinPin::default()
        },
    ]
}

/// A curved path and a straight one with unequal end speeds.
fn path_blur_paths() -> Vec<BlurPath> {
    vec![
        BlurPath {
            points: vec![(0.1, 0.8), (0.4, 0.3), (0.85, 0.55)],
            start_speed: 8.0,
            end_speed: 30.0,
        },
        BlurPath {
            points: vec![(0.7, 0.1), (0.95, 0.4)],
            start_speed: 20.0,
            end_speed: 4.0,
        },
    ]
}

/// Whether a variant's row-parallel kernel has a recorded digest. Every
/// variant is named so a new one cannot be added without a decision.
#[allow(clippy::match_like_matches_macro)]
fn digest_decision(kind: FilterKind) -> bool {
    match kind {
        FilterKind::BlurGallery
        | FilterKind::BoxBlur
        | FilterKind::Custom
        | FilterKind::FieldBlur
        | FilterKind::GaussianBlur
        | FilterKind::HighPass
        | FilterKind::IrisBlur
        | FilterKind::LensBlur
        | FilterKind::Maximum
        | FilterKind::Median
        | FilterKind::Minimum
        | FilterKind::MotionBlur
        | FilterKind::PathBlur
        | FilterKind::RadialBlur
        | FilterKind::ReduceNoise
        | FilterKind::ShapeBlur
        | FilterKind::SmartBlur
        | FilterKind::SpinBlur
        | FilterKind::TiltShift
        | FilterKind::UnsharpMask => true,
        FilterKind::Average
        | FilterKind::Blur
        | FilterKind::BlurMore
        | FilterKind::SurfaceBlur
        | FilterKind::Sharpen
        | FilterKind::SharpenMore
        | FilterKind::SharpenEdges
        | FilterKind::AddNoise
        | FilterKind::Despeckle
        | FilterKind::Offset
        | FilterKind::Emboss
        | FilterKind::FindEdges
        | FilterKind::Solarize
        | FilterKind::Diffuse
        | FilterKind::GlowingEdges
        | FilterKind::Mosaic
        | FilterKind::Crystallize
        | FilterKind::Facet
        | FilterKind::Fragment
        | FilterKind::Mezzotint
        | FilterKind::Pointillize
        | FilterKind::ColorHalftone
        | FilterKind::Twirl
        | FilterKind::Pinch
        | FilterKind::Spherize
        | FilterKind::Ripple
        | FilterKind::Wave
        | FilterKind::PolarCoordinates
        | FilterKind::Shear
        | FilterKind::Displace
        | FilterKind::Liquify
        | FilterKind::ZigZag
        | FilterKind::OceanRipple
        | FilterKind::Glass
        | FilterKind::DiffuseGlow
        | FilterKind::Clouds
        | FilterKind::DifferenceClouds
        | FilterKind::Fibers
        | FilterKind::PictureFrame
        | FilterKind::LensFlare
        | FilterKind::Lighting
        | FilterKind::Cutout
        | FilterKind::FilmGrain
        | FilterKind::NeonGlow
        | FilterKind::PosterEdges
        | FilterKind::PaintDaubs
        | FilterKind::PaletteKnife
        | FilterKind::PlasticWrap
        | FilterKind::Sponge
        | FilterKind::ColoredPencil
        | FilterKind::DryBrush
        | FilterKind::Fresco
        | FilterKind::RoughPastels
        | FilterKind::SmudgeStick
        | FilterKind::Underpainting
        | FilterKind::Watercolor
        | FilterKind::AccentedEdges
        | FilterKind::AngledStrokes
        | FilterKind::Crosshatch
        | FilterKind::DarkStrokes
        | FilterKind::InkOutlines
        | FilterKind::Spatter
        | FilterKind::SprayedStrokes
        | FilterKind::SumiE
        | FilterKind::BasRelief
        | FilterKind::ChalkCharcoal
        | FilterKind::Charcoal
        | FilterKind::Chrome
        | FilterKind::ConteCrayon
        | FilterKind::GraphicPen
        | FilterKind::HalftonePattern
        | FilterKind::NotePaper
        | FilterKind::Photocopy
        | FilterKind::Plaster
        | FilterKind::Reticulation
        | FilterKind::Stamp
        | FilterKind::TornEdges
        | FilterKind::WaterPaper
        | FilterKind::Craquelure
        | FilterKind::Grain
        | FilterKind::MosaicTiles
        | FilterKind::Patchwork
        | FilterKind::StainedGlass
        | FilterKind::Texturizer
        | FilterKind::OilPaint
        | FilterKind::DustAndScratches
        | FilterKind::Extrude
        | FilterKind::Tiles
        | FilterKind::TraceContour
        | FilterKind::Wind
        | FilterKind::SmartSharpen
        | FilterKind::HdrToning => false,
    }
}

#[test]
fn every_variant_has_a_digest_decision() {
    let recorded: std::collections::BTreeSet<&str> = kernels()
        .iter()
        .map(|(_, filter)| filter.kind().name())
        .collect();
    for &kind in FilterKind::ALL {
        assert_eq!(
            digest_decision(kind),
            recorded.contains(kind.name()),
            "{}: digest decision disagrees with the recorded kernels",
            kind.name()
        );
    }
}

#[test]
fn row_parallel_kernels_keep_their_serial_bytes() {
    // Moving kernels now run over the picture's fourth plane as well, so
    // their digests moved with the fold; value kernels (custom, high pass)
    // still leave alpha bit-identical and keep their old digests.
    // Shape, Smart Blur Normal and Lens Blur joined the fold in #406: their
    // colour bytes are unchanged, only the alpha plane now moves.
    let expected = [
        ("gaussian 1.5", 0xc400_4efa_61a4_ed25),
        ("gaussian 9", 0x7972_fd09_8cf3_00f3),
        ("unsharp", 0x034f_4a46_fc10_95bd),
        ("median 1", 0x6c43_94f5_21a7_60ef),
        ("median 4", 0x7df2_1565_3253_8682),
        ("reduce noise", 0x8ef2_36aa_9e18_5691),
        ("motion 30", 0x06c6_d5a7_5020_9627),
        ("motion -75", 0xc47c_d47d_da23_3405),
        ("box 2", 0xb4f7_9300_d71d_b971),
        ("box 70", 0x033c_5682_b051_aa87),
        ("shape heart 7", 0x9f27_8f83_1ee2_3188),
        ("shape arrow 40", 0x7b84_d94c_95db_a0c0),
        ("smart normal 6", 0xa536_c508_d93b_cf70),
        ("smart overlay 40", 0x90b2_060d_1532_f549),
        ("smart edges 3", 0x7d47_9518_947e_946e),
        ("maximum 2", 0xd5f0_6297_d8a0_bcba),
        ("minimum 5", 0x4b83_b5a6_dcb9_66a8),
        ("custom", 0x4a5a_022c_4c8f_6bfa),
        ("high pass", 0x366c_a481_61d8_b4dd),
        ("radial spin", 0x4c62_de8a_06d5_6282),
        ("radial zoom", 0x356d_0a67_96e5_c3b5),
        ("lens blur", 0x55cd_8bd2_19be_c27a),
        ("field blur", 0x28df_9133_325d_d4e6),
        ("iris blur", 0xc3bb_434e_14ef_8186),
        ("tilt-shift", 0x0c98_ef52_c090_cca3),
        ("blur gallery", 0x4ab9_0f60_c404_69f0),
        ("path blur", 0x5de5_0e6d_9851_2588),
        ("blur gallery paths", 0xfbc3_9ea9_6582_f8b9),
        ("spin blur", 0xd4c7_91b5_baea_e074),
        ("blur gallery spins", 0xac82_6ac3_4d1e_5d6b),
    ];
    let source = picture();
    for ((name, filter), (want_name, want)) in kernels().into_iter().zip(expected) {
        assert_eq!(name, want_name);
        let mut buf = source.clone();
        apply(&filter, &mut buf).unwrap();
        assert_eq!(digest(&buf), want, "{name} changed its output");
    }
}
