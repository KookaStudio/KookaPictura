//! Byte-exact digests of the row-parallel kernels (#152), taken from their
//! serial versions: parallelizing a kernel must not move a single sample.

use pictura_core::PixelBuffer;
use pictura_filters::{
    apply, BlurNoise, BlurNoiseKind, BlurPin, BokehParams, DepthMap, Filter, IrisPin, IrisShape,
    NoiseDistribution, Quality, RadialMethod, SmartBlurMode,
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
                quality: Quality::Good,
            },
        ),
        (
            "radial zoom",
            Filter::RadialBlur {
                method: RadialMethod::Zoom,
                amount: 60.0,
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
    ]
}

#[test]
fn row_parallel_kernels_keep_their_serial_bytes() {
    let expected = [
        ("gaussian 1.5", 0xf9a2_5193_6f8e_4515),
        ("gaussian 9", 0x8304_14ab_58dd_a5b3),
        ("unsharp", 0xd4f3_37f4_4ddf_f92b),
        ("median 1", 0xa87a_dfef_57b5_d176),
        ("median 4", 0x929b_65f3_1035_8689),
        ("motion 30", 0x00f7_ea4d_9a5b_f70f),
        ("motion -75", 0x9733_d43e_7acb_7c69),
        ("box 2", 0x2cd8_72fc_0780_110c),
        ("box 70", 0xc346_208f_7155_235d),
        ("shape heart 7", 0x5e7f_3ab9_03a1_f59a),
        ("shape arrow 40", 0x0d9c_2fdc_6023_46c7),
        ("smart normal 6", 0x9807_7b07_7a5e_4060),
        ("smart overlay 40", 0x90b2_060d_1532_f549),
        ("smart edges 3", 0x7d47_9518_947e_946e),
        ("maximum 2", 0x0b19_a93a_49cb_5598),
        ("minimum 5", 0x0dda_b0a3_defc_fe66),
        ("custom", 0x4a5a_022c_4c8f_6bfa),
        ("high pass", 0x366c_a481_61d8_b4dd),
        ("radial spin", 0x3c14_296d_3343_fe21),
        ("radial zoom", 0x2327_b5e8_6773_fd83),
        ("lens blur", 0x954e_772d_0d08_f422),
        ("field blur", 0x62e5_9821_9a51_6e8f),
        ("iris blur", 0x9880_82cd_fdc2_9575),
    ];
    let source = picture();
    for ((name, filter), (want_name, want)) in kernels().into_iter().zip(expected) {
        assert_eq!(name, want_name);
        let mut buf = source.clone();
        apply(&filter, &mut buf).unwrap();
        assert_eq!(digest(&buf), want, "{name} changed its output");
    }
}
