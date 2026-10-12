use super::arities::FILTER_ARITIES;
use super::gallery::GALLERY_TAIL;
use super::*;
use pictura_core::shape::CUSTOM_SHAPE_NAMES;
use pictura_filters::{
    BlurNoise, BlurNoiseKind, BlurPin, BokehParams, DisplaceFit, GlassTexture, IrisPin, IrisShape,
    LightType, NoiseDistribution, Quality, RadialMethod, SharpenRemove, SmartBlurMode,
    TextureChannel, WaveType, WindMethod, MAX_FIELD_PINS,
};

#[test]
fn every_kind_defaults_from_empty_and_rejects_wrong_arity() {
    for (kind, n) in FILTER_ARITIES {
        assert!(
            filter_from_kind_params(kind, &[]).is_some(),
            "{kind} must default from an empty slot list"
        );
        let slots = vec![0.0f64; *n];
        assert!(
            filter_from_kind_params(kind, &slots).is_some(),
            "{kind} must accept exactly {n} slots"
        );
        let mut too_many = slots.clone();
        too_many.push(0.0);
        assert!(
            filter_from_kind_params(kind, &too_many).is_none(),
            "{kind} must refuse {} slots",
            n + 1
        );
        if *n > 1 {
            let mut too_few = slots.clone();
            too_few.pop();
            assert!(
                filter_from_kind_params(kind, &too_few).is_none(),
                "{kind} must refuse {} slots",
                n - 1
            );
        }
    }
    assert!(filter_from_kind_params("bogus", &[]).is_none());
}

#[test]
fn parameter_values_override_defaults() {
    assert_eq!(
        filter_from_kind_params("gaussian-blur", &[12.5]),
        Some(Filter::GaussianBlur { radius: 12.5 })
    );
    assert_eq!(
        filter_from_kind_params("add-noise", &[50.0, 1.0, 1.0, 7.0]),
        Some(Filter::AddNoise {
            amount: 50.0,
            distribution: NoiseDistribution::Gaussian,
            monochromatic: true,
            seed: 7,
        })
    );
    assert_eq!(
        filter_from_kind_params("radial-blur", &[1.0, 20.0, 2.0]),
        Some(Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 20.0,
            quality: Quality::Best,
        })
    );
    // Wind direction is a Choice index: 0 = "From the Right", 1 = "From the Left".
    assert_eq!(
        filter_from_kind_params("wind", &[2.0, 0.0]),
        Some(Filter::Wind {
            method: WindMethod::Stagger,
            from_right: true,
        })
    );
    assert_eq!(
        filter_from_kind_params("wind", &[2.0, 1.0]),
        Some(Filter::Wind {
            method: WindMethod::Stagger,
            from_right: false,
        })
    );
}

#[test]
fn multi_slot_offsets_decode_correctly() {
    // rgb group at base 1.
    assert_eq!(
        filter_from_kind_params("pointillize", &[3.0, 10.0, 20.0, 30.0, 9.0]),
        Some(Filter::Pointillize {
            cell_size: 3,
            background: [10, 20, 30],
            seed: 9,
        })
    );
    // A 5-slot TextureOptions block.
    assert_eq!(
        filter_from_kind_params("texturizer", &[1.0, 55.0, 7.0, 3.0, 1.0]),
        Some(Filter::Texturizer {
            texture: TextureOptions {
                surface: TextureSurface::Burlap,
                scaling: 55,
                relief: 7,
                light_direction: 3,
                invert: true,
            },
        })
    );
    // The count + control-point pairs + fill shear encoding.
    let mut shear = vec![0.0; SHEAR_SLOTS];
    shear[0] = 3.0;
    shear[1] = -1.0;
    shear[2] = -0.5;
    shear[3] = 0.25;
    shear[4] = 0.75;
    shear[5] = 1.0;
    shear[6] = 0.5;
    shear[SHEAR_SLOTS - 1] = 1.0;
    assert_eq!(
        filter_from_kind_params("shear", &shear),
        Some(Filter::Shear {
            curve: vec![(-1.0, -0.5), (0.25, 0.75), (1.0, 0.5)],
            fill: ShearFill::RepeatEdgePixels,
        })
    );
    // Wave wavelength/amplitude/scale pairs.
    assert_eq!(
        filter_from_kind_params("wave", &[4.0, 6.0, 8.0, 2.0, 3.0, 1.0, 50.0, 75.0, 11.0]),
        Some(Filter::Wave {
            generators: 4,
            wavelength: (6.0, 8.0),
            amplitude: (2.0, 3.0),
            kind: WaveType::Triangle,
            scale: (50.0, 75.0),
            seed: 11,
            repeat_edge: true,
        })
    );
    // Smart Sharpen shadow (6..9) and highlight (9..12) TonalFade blocks.
    assert_eq!(
        filter_from_kind_params(
            "smart-sharpen",
            &[50.0, 2.0, 30.0, 1.0, 45.0, 1.0, 10.0, 20.0, 3.0, 40.0, 50.0, 4.0],
        ),
        Some(Filter::SmartSharpen {
            amount: 50.0,
            radius: 2.0,
            reduce_noise: 30.0,
            remove: SharpenRemove::LensBlur,
            angle: 45.0,
            more_accurate: true,
            shadow: TonalFade {
                amount: 10,
                width: 20,
                radius: 3,
            },
            highlight: TonalFade {
                amount: 40,
                width: 50,
                radius: 4,
            },
        })
    );
}

#[test]
fn lighting_maps_a_rig_of_any_size_up_to_the_cap() {
    let rig = [255.0, 200.0, 100.0, 10.0, 20.0, 30.0, 40.0, 2.0, 60.0];
    let spot = [
        0.0, 1.0, 255.0, 0.0, 0.0, 35.0, 69.0, 0.25, 0.75, -35.0, 0.6, 0.3, 50.0,
    ];
    let infinite = [
        2.0, 0.0, 0.0, 0.0, 255.0, -50.0, 0.0, 0.5, 0.5, 90.0, 0.2, 0.2, 30.0,
    ];
    let params: Vec<f64> = [&rig[..], &spot[..], &infinite[..]].concat();
    let Some(Filter::Lighting { lighting }) = filter_from_kind_params("lighting-effects", &params)
    else {
        panic!("a two-light rig must map");
    };
    assert_eq!(lighting.colorize, [255, 200, 100]);
    assert_eq!(lighting.texture, TextureChannel::Green);
    assert_eq!(lighting.height, 60.0);
    assert_eq!(lighting.lights.len(), 2);
    let (first, second) = (lighting.lights[0], lighting.lights[1]);
    assert_eq!(first.kind, LightType::Spot);
    assert_eq!(first.color, [255, 0, 0]);
    assert_eq!((first.center, first.width), ((0.25, 0.75), 0.3));
    assert_eq!(second.kind, LightType::Infinite);
    assert!(!second.on);
    assert_eq!((second.angle, second.elevation), (90.0, 30.0));

    let mut partial = params.clone();
    partial.pop();
    assert!(filter_from_kind_params("lighting-effects", &partial).is_none());
    let full: Vec<f64> = rig
        .iter()
        .chain(
            spot.iter()
                .cycle()
                .take(spot.len() * pictura_filters::MAX_LIGHTS),
        )
        .copied()
        .collect();
    assert!(filter_from_kind_params("lighting-effects", &full).is_some());
    let over: Vec<f64> = full.iter().chain(spot.iter()).copied().collect();
    assert!(filter_from_kind_params("lighting-effects", &over).is_none());
}

#[test]
fn gallery_distort_kinds_default_to_cs6() {
    assert_eq!(
        filter_from_kind_params("diffuse-glow", &[]),
        Some(Filter::DiffuseGlow {
            graininess: 6,
            glow_amount: 10,
            clear_amount: 15,
            seed: 1,
        })
    );
    assert_eq!(
        filter_from_kind_params("glass", &[]),
        Some(Filter::Glass {
            distortion: 5,
            smoothness: 3,
            texture: GlassTexture::Frosted,
            scaling: 100,
            invert: false,
        })
    );
    assert_eq!(
        filter_from_kind_params("glass", &[19.0, 15.0, 3.0, 162.0, 1.0]),
        Some(Filter::Glass {
            distortion: 19,
            smoothness: 15,
            texture: GlassTexture::TinyLens,
            scaling: 162,
            invert: true,
        })
    );
}

#[test]
fn displace_reads_its_slots_and_refuses_a_map_it_no_longer_has() {
    // CS6's defaults: 10% both ways, Stretch To Fit, Repeat Edge Pixels, and
    // no map yet, which moves nothing.
    assert_eq!(
        filter_from_kind_params("displace", &[]),
        Some(Filter::Displace {
            map: std::sync::Arc::new(pictura_filters::DisplaceMap::neutral()),
            horizontal_scale: 10.0,
            vertical_scale: 10.0,
            fit: DisplaceFit::StretchToFit,
            wrap: false,
        })
    );
    let Some(Filter::Displace {
        horizontal_scale,
        vertical_scale,
        fit,
        wrap,
        ..
    }) = filter_from_kind_params("displace", &[-40.0, 250.0, 1.0, 0.0, 0.0])
    else {
        panic!("displace did not decode");
    };
    assert_eq!((horizontal_scale, vertical_scale), (-40.0, 250.0));
    assert_eq!((fit, wrap), (DisplaceFit::Tile, true));
    assert!(filter_from_kind_params("displace", &[10.0, 10.0, 0.0, 1.0, 4e9]).is_none());
}

#[test]
fn liquify_names_a_kept_mesh_and_refuses_one_it_no_longer_has() {
    // No mesh yet (a dialog's default slot) moves nothing.
    let Some(Filter::Liquify { mesh }) = filter_from_kind_params("liquify", &[]) else {
        panic!("liquify did not default");
    };
    assert!(mesh.is_identity());
    assert!(filter_from_kind_params("liquify", &[4e9]).is_none());
    assert!(filter_from_kind_params("liquify", &[1.0, 2.0]).is_none());
}

#[test]
fn custom_reads_its_grid_row_by_row_then_scale_and_offset() {
    let mut identity = [[0.0; 5]; 5];
    identity[2][2] = 1.0;
    assert_eq!(
        filter_from_kind_params("custom", &[]),
        Some(Filter::Custom {
            kernel: identity,
            scale: 1.0,
            offset: 0.0,
        })
    );
    let mut params: Vec<f64> = (0..25).map(|i| i as f64 - 12.0).collect();
    params.extend([9.0, 128.0]);
    let Some(Filter::Custom {
        kernel,
        scale,
        offset,
    }) = filter_from_kind_params("custom", &params)
    else {
        panic!("custom did not decode");
    };
    assert_eq!(kernel[0][1], -11.0);
    assert_eq!(kernel[1][0], -7.0);
    assert_eq!(kernel[4][4], 12.0);
    assert_eq!((scale, offset), (9.0, 128.0));
    assert!(filter_from_kind_params("custom", &params[..26]).is_none());
}

#[test]
fn shape_blur_reads_radius_then_shape() {
    assert_eq!(
        filter_from_kind_params("shape-blur", &[]),
        Some(Filter::ShapeBlur {
            shape: 0,
            radius: 10,
        })
    );
    assert_eq!(
        filter_from_kind_params("shape-blur", &[46.0, 1.0]),
        Some(Filter::ShapeBlur {
            shape: 1,
            radius: 46,
        })
    );
    let Some(Filter::ShapeBlur { shape, .. }) = filter_from_kind_params("shape-blur", &[5.0, 99.0])
    else {
        panic!("shape-blur did not decode");
    };
    assert_eq!(
        shape,
        CUSTOM_SHAPE_NAMES.len() - 1,
        "an unknown shape clamps"
    );
    assert!(filter_from_kind_params("shape-blur", &[5.0]).is_none());
}

#[test]
fn smart_blur_reads_radius_threshold_quality_mode() {
    assert_eq!(
        filter_from_kind_params("smart-blur", &[]),
        Some(Filter::SmartBlur {
            radius: 3.0,
            threshold: 25.0,
            quality: Quality::Draft,
            mode: SmartBlurMode::Normal,
        })
    );
    assert_eq!(
        filter_from_kind_params("smart-blur", &[53.1, 25.0, 1.0, 2.0]),
        Some(Filter::SmartBlur {
            radius: 53.1,
            threshold: 25.0,
            quality: Quality::Good,
            mode: SmartBlurMode::OverlayEdge,
        })
    );
    assert_eq!(
        filter_from_kind_params("smart-blur", &[0.0, 400.0, 2.0, 1.0]),
        Some(Filter::SmartBlur {
            radius: 0.1,
            threshold: 100.0,
            quality: Quality::Best,
            mode: SmartBlurMode::EdgeOnly,
        }),
        "radius and threshold clamp into 0.1..=100"
    );
    assert!(filter_from_kind_params("smart-blur", &[3.0, 25.0, 0.0]).is_none());
}

#[test]
fn lens_blur_defaults_and_refuses_a_missing_depth_map() {
    let Some(Filter::LensBlur {
        depth,
        focal,
        invert,
        shape,
        radius,
        curvature,
        rotation,
        specular_brightness,
        specular_threshold,
        noise_amount,
        noise_distribution,
        monochromatic,
        faster,
        seed,
    }) = filter_from_kind_params("lens-blur", &[])
    else {
        panic!("lens blur did not decode");
    };
    assert!(depth.is_none(), "Source None carries no map");
    assert_eq!((focal, invert), (126, false));
    assert_eq!(shape, IrisShape::Triangle);
    assert_eq!((radius, curvature, rotation), (50, 33, 191.0));
    assert_eq!((specular_brightness, specular_threshold), (44, 93));
    assert_eq!((noise_amount, monochromatic), (47, false));
    assert_eq!(noise_distribution, NoiseDistribution::Gaussian);
    assert!(faster, "Preview defaults to Faster");
    assert_eq!(seed, 1);

    // A real source whose id names no kept map is refused...
    let mut params = vec![0.0; LENS_BLUR_SLOTS];
    params[1] = 1.0;
    assert!(filter_from_kind_params("lens-blur", &params).is_none());
    // ...and a registered map resolves, with the accurate commit flag.
    let id = crate::cxxqt_object::filter_tools::lens_blur_depths::register(
        pictura_filters::DepthMap::new(1, 1, vec![128]).unwrap(),
    );
    params[0] = 1.0;
    params[13] = f64::from(id);
    let Some(Filter::LensBlur {
        depth: Some(map),
        faster,
        ..
    }) = filter_from_kind_params("lens-blur", &params)
    else {
        panic!("a registered depth map did not resolve");
    };
    assert_eq!(map.width(), 1);
    assert!(!faster, "More Accurate decodes to faster = false");
}

#[test]
fn field_blur_defaults_and_accepts_a_variable_pin_list() {
    let default_pin = vec![BlurPin {
        x: 0.5,
        y: 0.5,
        blur: 0.0,
    }];
    assert_eq!(
        filter_from_kind_params("field-blur", &[]),
        Some(Filter::FieldBlur {
            pins: default_pin.clone(),
            bokeh: BokehParams::default(),
            noise: BlurNoise::default(),
        }),
        "an empty list is one centred pin with the default effects"
    );
    assert_eq!(
        filter_from_kind_params("field-blur", &[0.0; FIELD_BLUR_SLOTS]),
        Some(Filter::FieldBlur {
            pins: default_pin,
            bokeh: BokehParams {
                enabled: false,
                light_bokeh: 0.0,
                bokeh_color: 0.0,
                light_range: (0, 0),
            },
            noise: BlurNoise {
                enabled: false,
                amount: 0.0,
                size: 0.0,
                roughness: 0.0,
                color: 0.0,
                highlights: 0.0,
                kind: BlurNoiseKind::Gaussian,
                seed: 0,
            },
        }),
        "the canonical all-defaults list is one centred pin"
    );

    // One pin with the whole effects tail set.
    let mut one = vec![0.0; FIELD_BLUR_SLOTS];
    one[..17].copy_from_slice(&[
        1.0, 0.25, 0.75, 12.0, // pin
        1.0, 30.0, 50.0, 40.0, 200.0, // bokeh
        1.0, 20.0, 40.0, 30.0, 25.0, 10.0, 1.0, 7.0, // noise
    ]);
    assert_eq!(
        filter_from_kind_params("field-blur", &one),
        Some(Filter::FieldBlur {
            pins: vec![BlurPin {
                x: 0.25,
                y: 0.75,
                blur: 12.0
            }],
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
        })
    );

    // Two pins move the tail behind their six slots.
    let mut two = vec![0.0; 1 + 6 + GALLERY_TAIL];
    two[..7].copy_from_slice(&[2.0, 0.1, 0.2, 0.0, 0.9, 0.8, 40.0]);
    two[7..].copy_from_slice(&one[4..]);
    assert_eq!(
        filter_from_kind_params("field-blur", &two),
        Some(Filter::FieldBlur {
            pins: vec![
                BlurPin {
                    x: 0.1,
                    y: 0.2,
                    blur: 0.0
                },
                BlurPin {
                    x: 0.9,
                    y: 0.8,
                    blur: 40.0
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
        })
    );

    // A count that does not match the list length (including the old
    // no-tail layout), a list with no count, and more pins than the cap are
    // all refused.
    assert!(filter_from_kind_params("field-blur", &[2.0, 0.1, 0.2, 0.0]).is_none());
    assert!(filter_from_kind_params("field-blur", &[1.0, 0.5, 0.5, 10.0]).is_none());
    assert!(filter_from_kind_params("field-blur", &[1.0, 0.5, 0.5]).is_none());
    let mut too_many = vec![0.0f64; 1 + 3 * (MAX_FIELD_PINS + 1) + GALLERY_TAIL];
    too_many[0] = (MAX_FIELD_PINS + 1) as f64;
    assert!(filter_from_kind_params("field-blur", &too_many).is_none());
}

#[test]
fn iris_blur_defaults_and_accepts_a_variable_pin_list() {
    let iris = |params: &[f64]| match filter_from_kind_params("iris-blur", params) {
        Some(Filter::IrisBlur { pins, bokeh, noise }) => Some((pins, bokeh, noise)),
        Some(other) => panic!("iris-blur mapped to {other:?}"),
        None => None,
    };
    let (pins, bokeh, noise) = iris(&[]).unwrap();
    assert_eq!(pins, vec![IrisPin::default()]);
    assert_eq!(bokeh, BokehParams::default());
    assert_eq!(noise, BlurNoise::default());
    let (pins, bokeh, _) = iris(&[0.0; IRIS_BLUR_SLOTS]).unwrap();
    assert_eq!(
        pins,
        vec![IrisPin::default()],
        "the canonical list is one default pin"
    );
    assert!(!bokeh.enabled);

    // Two pins, then the tail.
    let mut two = vec![2.0];
    two.extend([0.3, 0.4, 0.2, 0.1, 30.0, 60.0, 0.5, 12.0]);
    two.extend([0.8, 0.7, 0.1, 0.1, 0.0, 0.0, 0.3, 40.0]);
    two.extend([1.0, 30.0, 50.0, 200.0, 40.0]);
    two.extend([1.0, 20.0, 40.0, 30.0, 25.0, 10.0, 2.0, 7.0]);
    assert_eq!(two.len(), 1 + 16 + GALLERY_TAIL);
    let (pins, bokeh, noise) = iris(&two).unwrap();
    assert_eq!(
        pins,
        vec![
            IrisPin {
                x: 0.3,
                y: 0.4,
                radius_x: 0.2,
                radius_y: 0.1,
                angle: 30.0,
                squareness: 60.0,
                feather: 0.5,
                blur: 12.0,
            },
            IrisPin {
                x: 0.8,
                y: 0.7,
                radius_x: 0.1,
                radius_y: 0.1,
                angle: 0.0,
                squareness: 0.0,
                feather: 0.3,
                blur: 40.0,
            },
        ]
    );
    assert_eq!(bokeh.light_range, (40, 200), "a reversed range is ordered");
    assert_eq!(noise.kind, BlurNoiseKind::Grain);
    assert_eq!(noise.seed, 7);

    // A Field Blur stride, a short list, and more pins than the cap are
    // refused.
    let mut field_stride = vec![1.0, 0.5, 0.5, 10.0];
    field_stride.extend([0.0; GALLERY_TAIL]);
    assert!(iris(&field_stride).is_none());
    assert!(iris(&two[..two.len() - 1]).is_none());
    let mut too_many = vec![0.0f64; 1 + 8 * (MAX_FIELD_PINS + 1) + GALLERY_TAIL];
    too_many[0] = (MAX_FIELD_PINS + 1) as f64;
    assert!(iris(&too_many).is_none());
}
