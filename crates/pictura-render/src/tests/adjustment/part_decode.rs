use super::*;

#[test]
fn decode_adjustment_subset_and_unknown() {
    assert_eq!(
        decode_adjustment(&adjdata(*b"nvrt", vec![])),
        Some(Adjustment::Invert)
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"invr", vec![])),
        Some(Adjustment::Invert)
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"post", vec![0, 4, 0, 0])),
        Some(Adjustment::Posterize(4))
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"thrs", vec![0, 128, 0, 0])),
        Some(Adjustment::Threshold(128))
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"brit", vec![0, 10, 0, 20, 0, 0, 0, 0])),
        Some(Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 10,
            contrast: 20,
            use_legacy: false,
        }))
    );

    let mut levels = vec![0, 2];
    for v in [5u16, 250, 10, 240, 120] {
        levels.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(
        decode_adjustment(&adjdata(*b"levl", levels)),
        Some(Adjustment::Levels(LevelsParams {
            input_black: 5,
            input_white: 250,
            gamma: 1.2,
            output_black: 10,
            output_white: 240,
            red: None,
            green: None,
            blue: None,
        }))
    );

    let hue = vec![0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 20, 0, 30];
    assert_eq!(
        decode_adjustment(&adjdata(*b"hue2", hue)),
        Some(Adjustment::HueSaturation(HueSaturationParams {
            hue: 10,
            saturation: 20,
            lightness: 30,
            ranges: Vec::new(),
        }))
    );

    // Unknown key and undecodable payloads are a no-op, never an error.
    assert_eq!(decode_adjustment(&adjdata(*b"zzzz", vec![1, 2, 3])), None);
    assert_eq!(decode_adjustment(&adjdata(*b"clrL", vec![1, 2, 3])), None);
    assert_eq!(decode_adjustment(&adjdata(*b"post", vec![0, 0])), None);
    assert_eq!(decode_adjustment(&adjdata(*b"levl", vec![0, 3])), None);
}

#[test]
fn solid_fill_decodes_both_payload_forms() {
    // The in-house 4-byte straight-alpha tuple is unchanged.
    assert_eq!(
        decode_adjustment(&adjdata(*b"SoCo", vec![10, 20, 30, 40])),
        Some(Adjustment::SolidFill([10, 20, 30, 40]))
    );
    // The standard descriptor decodes with alpha forced to 255.
    assert_eq!(
        decode_adjustment(&encode_solid_color_fill([10, 20, 30])),
        Some(Adjustment::SolidFill([10, 20, 30, 255]))
    );

    // A non-4-byte, non-descriptor payload is a no-op.
    for payload in [vec![], vec![1, 2, 3], vec![1, 2, 3, 4, 5]] {
        assert_eq!(decode_adjustment(&adjdata(*b"SoCo", payload)), None);
    }
    // A descriptor missing `Clr ` is a no-op.
    let no_clr = write_descriptor(&desc_object("", b"SoCo", vec![]));
    assert_eq!(decode_adjustment(&adjdata(*b"SoCo", no_clr)), None);
    // A `Clr ` object with a wrong-typed component is a no-op.
    let wrong_type = write_descriptor(&desc_object(
        "",
        b"SoCo",
        vec![(
            b"Clr ".to_vec(),
            desc_object(
                "",
                b"RGBC",
                vec![
                    (b"Rd  ".to_vec(), DescValue::Long(10)),
                    (b"Grn ".to_vec(), DescValue::Double(20.0)),
                    (b"Bl  ".to_vec(), DescValue::Double(30.0)),
                ],
            ),
        )],
    ));
    assert_eq!(decode_adjustment(&adjdata(*b"SoCo", wrong_type)), None);
    // A non-finite component is a no-op.
    let non_finite = write_descriptor(&desc_object(
        "",
        b"SoCo",
        vec![(
            b"Clr ".to_vec(),
            desc_object(
                "",
                b"RGBC",
                vec![
                    (b"Rd  ".to_vec(), DescValue::Double(f64::NAN)),
                    (b"Grn ".to_vec(), DescValue::Double(20.0)),
                    (b"Bl  ".to_vec(), DescValue::Double(30.0)),
                ],
            ),
        )],
    ));
    assert_eq!(decode_adjustment(&adjdata(*b"SoCo", non_finite)), None);
}

#[test]
fn encoded_solid_fill_is_version16_descriptor() {
    let encoded = encode_solid_color_fill([10, 20, 30]);
    assert_eq!(encoded.key, *b"SoCo");
    assert_eq!(
        decode_adjustment(&encoded),
        Some(Adjustment::SolidFill([10, 20, 30, 255]))
    );

    let desc = pictura_codec::read_descriptor(&encoded.data).expect("version-16 descriptor");
    let DescValue::Object { items, .. } = desc else {
        panic!("top level is an object");
    };
    let clr = items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Clr ")
        .map(|(_, v)| v)
        .expect("Clr item");
    let DescValue::Object { items, .. } = clr else {
        panic!("Clr is an object");
    };
    let get = |key: &[u8]| {
        items
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
            .expect("component")
    };
    assert_eq!(get(b"Rd  "), &DescValue::Double(10.0));
    assert_eq!(get(b"Grn "), &DescValue::Double(20.0));
    assert_eq!(get(b"Bl  "), &DescValue::Double(30.0));
}

#[test]
fn encode_decode_round_trips() {
    assert_eq!(
        decode_adjustment(&encode_invert()),
        Some(Adjustment::Invert)
    );
    assert_eq!(
        decode_adjustment(&encode_posterize(4)),
        Some(Adjustment::Posterize(4))
    );
    assert_eq!(
        decode_adjustment(&encode_threshold(128)),
        Some(Adjustment::Threshold(128))
    );
    assert_eq!(
        decode_adjustment(&encode_brightness_contrast(10, 20)),
        Some(Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 10,
            contrast: 20,
            use_legacy: false,
        }))
    );
    assert_eq!(
        decode_adjustment(&encode_hue_saturation(10, 20, 30)),
        Some(Adjustment::HueSaturation(HueSaturationParams {
            hue: 10,
            saturation: 20,
            lightness: 30,
            ranges: Vec::new(),
        }))
    );
    assert_eq!(
        decode_adjustment(&encode_photo_filter([255, 180, 80], 25.0, true)),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 25.0,
            preserve_luminosity: true,
        }))
    );
    assert_eq!(
        decode_adjustment(&encode_color_balance(
            [0.0; 3],
            [25.0, 0.0, 0.0],
            [0.0; 3],
            true
        )),
        Some(Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [0.0; 3],
            midtones: [25.0, 0.0, 0.0],
            highlights: [0.0; 3],
            preserve_luminosity: true,
        }))
    );

    let cm = encode_channel_mixer(
        false,
        [30.0, -10.0, 50.0],
        [10.0, 90.0, 0.0],
        [0.0, 20.0, 110.0],
        [5.0, -20.0, 40.0],
    );
    assert_eq!(cm.key, *b"mixr");
    assert_eq!(cm.data.len(), 44);
    assert_eq!(
        decode_adjustment(&cm),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [30.0, -10.0, 50.0],
            green: [10.0, 90.0, 0.0],
            blue: [0.0, 20.0, 110.0],
            constant: [5.0, -20.0, 40.0],
            ink: [
                [30.0, -10.0, 50.0, 0.0],
                [10.0, 90.0, 0.0, 0.0],
                [0.0, 20.0, 110.0, 0.0],
                [30.0, -10.0, 50.0, 0.0],
            ],
            ink_constant: [5.0, -20.0, 40.0, 5.0],
        }))
    );
    let cm_mono = encode_channel_mixer(
        true,
        [20.0, 40.0, 60.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [-15.0, 0.0, 0.0],
    );
    assert_eq!(cm_mono.data.len(), 44);
    assert_eq!(
        decode_adjustment(&cm_mono),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [20.0, 40.0, 60.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [-15.0, 0.0, 0.0],
            ..Default::default()
        }))
    );
    let cm_clamped =
        encode_channel_mixer(false, [999.0, -999.0, 0.0], [0.0; 3], [0.0; 3], [0.0; 3]);
    assert_eq!(
        decode_adjustment(&cm_clamped),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [200.0, -200.0, 0.0],
            green: [0.0; 3],
            blue: [0.0; 3],
            constant: [0.0; 3],
            ink: [
                [200.0, -200.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [200.0, -200.0, 0.0, 0.0],
            ],
            ink_constant: [0.0; 4],
        }))
    );

    // Byte formats match the psd-tools fixtures (`H2x`, `3HBx`).
    assert_eq!(encode_invert().key, *b"nvrt");
    assert!(encode_invert().data.is_empty());
    assert_eq!(encode_posterize(4).data, [0, 4, 0, 0]);
    assert_eq!(encode_threshold(128).data, [0, 128, 0, 0]);
    assert_eq!(
        encode_brightness_contrast(10, 20).data,
        [0, 10, 0, 20, 0, 0, 0, 0]
    );
    assert_eq!(encode_hue_saturation(10, 20, 30).data.len(), 100);
    assert_eq!(encode_hue_saturation(10, 20, 30).data[0..2], [0, 2]);
    let phfl = encode_photo_filter([255, 180, 80], 25.0, true);
    assert_eq!(phfl.key, *b"phfl");
    assert_eq!(phfl.data.len(), 20);
    assert_eq!(
        phfl.data,
        [0, 2, 0, 0, 0, 255, 0, 180, 0, 80, 0, 0, 0, 0, 0, 25, 1, 0, 0, 0]
    );

    let gm_stops = [
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 2048,
            color: [255, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ];
    let gm = encode_gradient_map(
        &GradientMapParams {
            stops: gm_stops.to_vec(),
            reverse: true,
            transparency: Vec::new(),
        },
        false,
    );
    assert_eq!(gm.key, *b"grdm");
    assert_eq!(
        decode_adjustment(&gm),
        Some(Adjustment::GradientMap(GradientMapParams {
            stops: gm_stops.to_vec(),
            reverse: true,
            transparency: Vec::new(),
        }))
    );
    assert_eq!(gm.data.len() % 4, 0, "block is padded to 4 bytes");

    // Out-of-range inputs are clamped to what the decoder accepts.
    assert_eq!(
        decode_adjustment(&encode_posterize(0)),
        Some(Adjustment::Posterize(2))
    );
    assert_eq!(
        decode_adjustment(&encode_threshold(0)),
        Some(Adjustment::Threshold(1))
    );
    assert_eq!(
        decode_adjustment(&encode_brightness_contrast(999, -999)),
        Some(Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 150,
            contrast: -50,
            use_legacy: false,
        }))
    );
    assert_eq!(
        decode_adjustment(&encode_photo_filter([255, 180, 80], 150.0, false)),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 100.0,
            preserve_luminosity: false,
        }))
    );
    assert_eq!(
        decode_adjustment(&encode_photo_filter([255, 180, 80], -5.0, false)),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 0.0,
            preserve_luminosity: false,
        }))
    );

    // Out-of-range Color Balance shifts are clamped and the block still decodes.
    let cb = encode_color_balance([150.0, -150.0, 0.0], [0.0; 3], [0.0; 3], false);
    assert_eq!(cb.key, *b"blnc");
    assert_eq!(cb.data.len(), 20);
    assert_eq!(
        decode_adjustment(&cb),
        Some(Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [100.0, -100.0, 0.0],
            midtones: [0.0; 3],
            highlights: [0.0; 3],
            preserve_luminosity: false,
        }))
    );
}

#[test]
fn exp_a_decodes_exposure() {
    let payload = exposure_payload(1.5, -0.25, 1.1);
    assert_eq!(
        decode_adjustment(&adjdata(*b"expA", payload.clone())),
        Some(Adjustment::Exposure(ExposureParams {
            exposure: 1.5f32 as f64,
            offset: -0.25f32 as f64,
            gamma: 1.1f32 as f64,
        }))
    );

    // Fewer than the 14 field bytes is a no-op, never an error.
    for cut in 0..payload.len() {
        assert_eq!(
            decode_adjustment(&adjdata(*b"expA", payload[..cut].to_vec())),
            None,
            "cut {cut}"
        );
    }
    assert_eq!(
        decode_adjustment(&adjdata(*b"expA", exposure_payload(1.5, -0.25, 0.0))),
        None,
        "gamma must be positive"
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"expA", exposure_payload(f32::NAN, 0.0, 1.0))),
        None,
        "non-finite exposure is rejected"
    );

    // The version word must be 1, per the committed payload layout.
    let mut wrong_version = 2u16.to_be_bytes().to_vec();
    wrong_version.extend_from_slice(&1.5f32.to_be_bytes());
    wrong_version.extend_from_slice(&(-0.25f32).to_be_bytes());
    wrong_version.extend_from_slice(&1.1f32.to_be_bytes());
    assert_eq!(
        decode_adjustment(&adjdata(*b"expA", wrong_version)),
        None,
        "version must be 1"
    );
}

#[test]
fn vib_a_decodes_descriptor() {
    let payload = write_descriptor(&desc_object(
        "",
        b"null",
        vec![
            (b"vibrance".to_vec(), DescValue::Long(40)),
            (b"Strt".to_vec(), DescValue::Long(-10)),
        ],
    ));
    assert_eq!(
        decode_adjustment(&adjdata(*b"vibA", payload)),
        Some(Adjustment::Vibrance(VibranceParams {
            vibrance: 40,
            saturation: -10,
        }))
    );

    // Garbage bytes are not a version-16 descriptor object: no-op.
    assert_eq!(decode_adjustment(&adjdata(*b"vibA", vec![1, 2, 3])), None);

    // A present key outside the slider range is rejected, not clamped.
    let out_of_range = write_descriptor(&desc_object(
        "",
        b"null",
        vec![(b"vibrance".to_vec(), DescValue::Long(200))],
    ));
    assert_eq!(decode_adjustment(&adjdata(*b"vibA", out_of_range)), None);
}

#[test]
fn blwh_decodes_descriptor() {
    let tint = desc_object(
        "tint",
        b"Clr ",
        vec![
            (b"Rd  ".to_vec(), DescValue::Double(0.5)),
            (b"Grn ".to_vec(), DescValue::Double(0.25)),
            (b"Bl  ".to_vec(), DescValue::Double(0.75)),
        ],
    );
    let payload = write_descriptor(&desc_object(
        "",
        b"null",
        vec![
            (b"Rd  ".to_vec(), DescValue::Long(40)),
            (b"Yllw".to_vec(), DescValue::Long(60)),
            (b"Grn ".to_vec(), DescValue::Long(40)),
            (b"Cyn ".to_vec(), DescValue::Long(60)),
            (b"Bl  ".to_vec(), DescValue::Long(20)),
            (b"Mgnt".to_vec(), DescValue::Long(80)),
            (b"useTint".to_vec(), DescValue::Bool(true)),
            (b"tintColor".to_vec(), tint),
        ],
    ));
    assert_eq!(
        decode_adjustment(&adjdata(*b"blwh", payload)),
        Some(Adjustment::BlackWhite(BlackWhiteParams {
            red: 40.0,
            yellow: 60.0,
            green: 40.0,
            cyan: 60.0,
            blue: 20.0,
            magenta: 80.0,
            tint: true,
            tint_color: [128, 64, 191],
        }))
    );

    // A non-descriptor payload is a no-op.
    assert_eq!(decode_adjustment(&adjdata(*b"blwh", vec![0, 16, 0])), None);

    // A truncated descriptor and an out-of-range percentage are both rejected.
    let truncated = write_descriptor(&desc_object("", b"null", vec![]));
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"blwh",
            truncated[..truncated.len() - 1].to_vec()
        )),
        None
    );
    let out_of_range = write_descriptor(&desc_object(
        "",
        b"null",
        vec![(b"Rd  ".to_vec(), DescValue::Long(400))],
    ));
    assert_eq!(decode_adjustment(&adjdata(*b"blwh", out_of_range)), None);

    // A descriptor with no channel keys falls back to the reference's defaults.
    let empty = write_descriptor(&desc_object("", b"null", vec![]));
    assert_eq!(
        decode_adjustment(&adjdata(*b"blwh", empty)),
        Some(Adjustment::BlackWhite(BlackWhiteParams {
            red: 40.0,
            yellow: 60.0,
            green: 40.0,
            cyan: 60.0,
            blue: 20.0,
            magenta: 80.0,
            tint: false,
            tint_color: [0, 0, 0],
        }))
    );
}

#[test]
fn phfl_decodes_version_two() {
    let payload = phfl_payload(2, [255, 180, 80, 0], 25, 1);
    assert_eq!(payload.len(), 20);
    assert_eq!(
        decode_adjustment(&adjdata(*b"phfl", payload.clone())),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 25.0,
            preserve_luminosity: true,
        }))
    );
    // The fourth component and colour space are ignored, and luminosity is a
    // boolean.
    assert_eq!(
        decode_adjustment(&adjdata(*b"phfl", phfl_payload(2, [1, 2, 3, 999], 0, 0))),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [1, 2, 3],
            density: 0.0,
            preserve_luminosity: false,
        }))
    );

    // Fewer than the 17 field bytes is a no-op, never an error.
    for cut in 0..17 {
        assert_eq!(
            decode_adjustment(&adjdata(*b"phfl", payload[..cut].to_vec())),
            None,
            "cut {cut}"
        );
    }
    // A component above 255 and density above 100 are rejected.
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"phfl",
            phfl_payload(2, [256, 180, 80, 0], 25, 1)
        )),
        None,
        "component must be 0..=255"
    );
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"phfl",
            phfl_payload(2, [255, 180, 80, 0], 101, 1)
        )),
        None,
        "density must be 0..=100"
    );
}

#[test]
fn grdm_decodes_versions_and_rejects_malformed() {
    let stops = [
        (0u32, [0u16, 0, 0, 0]),
        (4096u32, [65535u16, 65535, 65535, 0]),
    ];
    let expected = Adjustment::GradientMap(GradientMapParams {
        stops: vec![
            GradientStop {
                location: 0,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 4096,
                color: [255, 255, 255],
            },
        ],
        reverse: false,
        transparency: Vec::new(),
    });

    let payload = grdm_payload(1, 0, 0, "Black to White", &stops);
    assert_eq!(
        decode_adjustment(&adjdata(*b"grdm", payload.clone())),
        Some(expected.clone())
    );
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"grdm",
            grdm_payload(1, 1, 0, "Black to White", &stops)
        )),
        Some(Adjustment::GradientMap(GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 0,
                    color: [0, 0, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [255, 255, 255],
                },
            ],
            reverse: true,
            transparency: Vec::new(),
        }))
    );
    // Version 3 carries a 4-byte method after the flags; the stops are the same.
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"grdm",
            grdm_payload(3, 0, 1, "Black to White", &stops)
        )),
        Some(expected.clone())
    );

    // Truncation anywhere before the last stop's colour is a no-op, never an
    // error. The final 4 bytes (stop pad + transparency count) are ignored.
    for cut in 0..payload.len() - 4 {
        assert_eq!(
            decode_adjustment(&adjdata(*b"grdm", payload[..cut].to_vec())),
            None,
            "cut {cut}"
        );
    }
    // The in-house `gdrm` spelling is accepted as an alias.
    assert_eq!(
        decode_adjustment(&adjdata(*b"gdrm", payload.clone())),
        Some(expected.clone())
    );
    // Unsupported version.
    assert_eq!(
        decode_adjustment(&adjdata(*b"grdm", grdm_payload(2, 0, 0, "", &stops))),
        None
    );
    // Fewer than two stops.
    assert_eq!(
        decode_adjustment(&adjdata(*b"grdm", grdm_payload(1, 0, 0, "", &stops[..1]))),
        None
    );
    // Non-increasing locations.
    let decreasing = [(4096u32, [0u16, 0, 0, 0]), (0u32, [0u16, 0, 0, 0])];
    assert_eq!(
        decode_adjustment(&adjdata(*b"grdm", grdm_payload(1, 0, 0, "", &decreasing))),
        None
    );
    // A location above 4096.
    let out_of_range = [(0u32, [0u16, 0, 0, 0]), (5000u32, [0u16, 0, 0, 0])];
    assert_eq!(
        decode_adjustment(&adjdata(*b"grdm", grdm_payload(1, 0, 0, "", &out_of_range))),
        None
    );
}

#[test]
fn fixture_solid_fill_decodes_descriptor() {
    let bytes = include_bytes!("../../../../pictura-codec/tests/fixtures/solid_fill.psd");
    let d = pictura_codec::read_psd(bytes).expect("fixture parses");
    let fill = d
        .layers
        .iter()
        .find(|l| l.name == "Solid Fill")
        .expect("Solid Fill layer");
    assert_eq!(
        decode_adjustment(fill.adjustment.as_ref().expect("SoCo block")),
        Some(Adjustment::SolidFill([10, 20, 30, 255]))
    );
}

#[test]
fn phfl_decodes_version_three() {
    // D50 white at 16.16: converts to sRGB white.
    let white = [63191, 65536, 54081];
    let decoded = decode_adjustment(&adjdata(*b"phfl", phfl_v3_payload(white, 25, 1)));
    let params = match decoded {
        Some(Adjustment::PhotoFilter(p)) => p,
        other => panic!("version-3 phfl must decode, got {other:?}"),
    };
    assert_eq!(params.color, [255, 255, 255]);
    assert_eq!(params.density, 25.0);
    assert!(params.preserve_luminosity);

    // A warm XYZ converts to a stable non-white colour.
    assert_eq!(
        decode_adjustment(&adjdata(
            *b"phfl",
            phfl_v3_payload([68813, 65536, 32768], 40, 0)
        )),
        Some(Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 244, 196],
            density: 40.0,
            preserve_luminosity: false,
        }))
    );

    // Round-trip: v3 decode → v2 encode → same params.
    let reencoded = encode_photo_filter(params.color, params.density, params.preserve_luminosity);
    assert_eq!(reencoded.data[0..2], [0, 2], "encoder stays version 2");
    assert_eq!(
        decode_adjustment(&reencoded),
        Some(Adjustment::PhotoFilter(params))
    );

    // Truncated before luminosity, density above 100, unknown version: None.
    let full = phfl_v3_payload(white, 25, 1);
    assert_eq!(
        decode_adjustment(&adjdata(*b"phfl", full[..18].to_vec())),
        None,
        "truncated before luminosity"
    );
    assert_eq!(
        decode_adjustment(&adjdata(*b"phfl", phfl_v3_payload(white, 101, 1))),
        None,
        "density must be 0..=100"
    );
    let mut v4 = phfl_v3_payload(white, 25, 1);
    v4[1] = 4;
    assert_eq!(
        decode_adjustment(&adjdata(*b"phfl", v4)),
        None,
        "version must be 2 or 3"
    );
}

#[test]
fn color_lookup_decodes_an_embedded_cube() {
    let encoded = crate::encode_color_lookup(&crate::identity_cube(), "Identity");
    let Some(Adjustment::ColorLookup(params)) = decode_adjustment(&encoded) else {
        panic!("clrL must decode to ColorLookup");
    };
    assert_eq!(params.kind, pictura_adjust::ColorLookupKind::ThreeDLut);
    let lut = params.lookup.expect("identity cube parses");
    assert_eq!(lut.size, 2);
    assert_eq!(lut.points.len(), 8);
}

#[test]
fn blnc_decodes_and_rejects_malformed() {
    let payload = blnc_payload([-100, 5, 0], [25, 0, -30], [100, 0, 0], 1);
    assert_eq!(payload.len(), 20);
    assert_eq!(
        decode_adjustment(&adjdata(*b"blnc", payload.clone())),
        Some(Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [-100.0, 5.0, 0.0],
            midtones: [25.0, 0.0, -30.0],
            highlights: [100.0, 0.0, 0.0],
            preserve_luminosity: true,
        }))
    );

    // The luminosity flag is a boolean and the trailing pad is ignored.
    assert_eq!(
        decode_adjustment(&adjdata(*b"blnc", blnc_payload([0; 3], [0; 3], [0; 3], 2))),
        Some(Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [0.0; 3],
            midtones: [0.0; 3],
            highlights: [0.0; 3],
            preserve_luminosity: true,
        }))
    );

    // Truncation anywhere before the luminosity byte is a no-op, never an error.
    for cut in 0..19 {
        assert_eq!(
            decode_adjustment(&adjdata(*b"blnc", payload[..cut].to_vec())),
            None,
            "cut {cut}"
        );
    }

    // A shift outside -100..=100 is rejected, not clamped.
    for bad in [101i16, -101] {
        assert_eq!(
            decode_adjustment(&adjdata(
                *b"blnc",
                blnc_payload([bad, 0, 0], [0; 3], [0; 3], 1)
            )),
            None,
            "out-of-range {bad}"
        );
    }
}

#[test]
fn mixr_decodes_channels_and_ignores_gray_and_trailing() {
    let mut payload = mixr_payload(
        false,
        &[
            ([30, -10, 50], 5),
            ([10, 90, 0], -20),
            ([0, 20, 110], 40),
            ([100, 0, 0], 0),
        ],
    );
    payload.extend_from_slice(&[9, 9, 9, 9]);
    assert_eq!(
        decode_adjustment(&adjdata(*b"mixr", payload)),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [30.0, -10.0, 50.0],
            green: [10.0, 90.0, 0.0],
            blue: [0.0, 20.0, 110.0],
            constant: [5.0, -20.0, 40.0],
            ink: [
                [30.0, -10.0, 50.0, 0.0],
                [10.0, 90.0, 0.0, 0.0],
                [0.0, 20.0, 110.0, 0.0],
                [100.0, 0.0, 0.0, 0.0],
            ],
            ink_constant: [5.0, -20.0, 40.0, 0.0],
        }))
    );

    let mono = mixr_payload(true, &[([20, 40, 60], -15)]);
    assert_eq!(mono.len(), 14);
    assert_eq!(
        decode_adjustment(&adjdata(*b"mixr", mono)),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [20.0, 40.0, 60.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [-15.0, 0.0, 0.0],
            ..Default::default()
        }))
    );
}

#[test]
fn mixr_rejects_malformed() {
    let full = mixr_payload(
        false,
        &[
            ([10, 20, 30], 5),
            ([40, 50, 60], -5),
            ([70, 80, 90], 10),
            ([0, 0, 0], 0),
        ],
    );
    assert_eq!(full.len(), 44);
    for cut in 0..full.len() {
        assert_eq!(
            decode_adjustment(&adjdata(*b"mixr", full[..cut].to_vec())),
            None,
            "cut {cut}"
        );
    }

    let mut wrong_version = full.clone();
    wrong_version[1] = 2;
    assert_eq!(decode_adjustment(&adjdata(*b"mixr", wrong_version)), None);

    for (rgb, constant) in [([201, 0, 0], 0), ([0, 0, 0], -201)] {
        let payload = mixr_payload(
            false,
            &[
                (rgb, constant),
                ([0, 0, 0], 0),
                ([0, 0, 0], 0),
                ([0, 0, 0], 0),
            ],
        );
        assert_eq!(
            decode_adjustment(&adjdata(*b"mixr", payload)),
            None,
            "out-of-range {rgb:?} {constant}"
        );
    }

    let mono = mixr_payload(true, &[([0, 0, 0], 201)]);
    assert_eq!(decode_adjustment(&adjdata(*b"mixr", mono)), None);
}

#[test]
fn fixture_channel_mixer_decodes() {
    let bytes = include_bytes!("../../../../pictura-codec/tests/fixtures/channel_mixer.psd");
    let d = pictura_codec::read_psd(bytes).expect("fixture parses");
    let color = d
        .layers
        .iter()
        .find(|l| l.name == "Channel Mixer")
        .expect("Channel Mixer layer");
    assert_eq!(
        decode_adjustment(color.adjustment.as_ref().expect("mixr block")),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [30.0, -10.0, 50.0],
            green: [10.0, 90.0, 0.0],
            blue: [0.0, 20.0, 110.0],
            constant: [5.0, -20.0, 40.0],
            ink: [
                [30.0, -10.0, 50.0, 0.0],
                [10.0, 90.0, 0.0, 0.0],
                [0.0, 20.0, 110.0, 0.0],
                [100.0, 0.0, 0.0, 0.0],
            ],
            ink_constant: [5.0, -20.0, 40.0, 0.0],
        }))
    );

    let mono = d
        .layers
        .iter()
        .find(|l| l.name == "Channel Mixer Mono")
        .expect("Channel Mixer Mono layer");
    assert_eq!(
        decode_adjustment(mono.adjustment.as_ref().expect("mixr block")),
        Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [20.0, 40.0, 60.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [-15.0, 0.0, 0.0],
            ..Default::default()
        }))
    );
}

#[test]
fn gradient_map_opacity_stops_round_trip() {
    let params = GradientMapParams {
        stops: vec![
            GradientStop {
                location: 0,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 4096,
                color: [255, 255, 255],
            },
        ],
        reverse: false,
        transparency: vec![
            pictura_adjust::OpacityStop {
                location: 0,
                opacity: 100,
            },
            pictura_adjust::OpacityStop {
                location: 4096,
                opacity: 25,
            },
        ],
    };
    let block = encode_gradient_map(&params, true);
    assert_eq!(block.data[3], 1, "the dither flag");
    assert_eq!(
        decode_adjustment(&block),
        Some(Adjustment::GradientMap(params.clone()))
    );
    // All-opaque stops decode as an opaque map.
    let opaque = GradientMapParams {
        transparency: vec![pictura_adjust::OpacityStop {
            location: 0,
            opacity: 100,
        }],
        ..params.clone()
    };
    let decoded = decode_adjustment(&encode_gradient_map(&opaque, false));
    assert_eq!(
        decoded,
        Some(Adjustment::GradientMap(GradientMapParams {
            transparency: Vec::new(),
            ..params
        }))
    );
}

fn ink_identity() -> [[f64; 4]; 4] {
    [
        [100.0, 0.0, 0.0, 0.0],
        [0.0, 100.0, 0.0, 0.0],
        [0.0, 0.0, 100.0, 0.0],
        [0.0, 0.0, 0.0, 100.0],
    ]
}

#[test]
fn channel_mixer_ink_roundtrip() {
    let ink = [
        [20.0, 0.0, 0.0, 0.0],
        [0.0, 30.0, 0.0, 0.0],
        [0.0, 0.0, 40.0, 0.0],
        [0.0, 0.0, 0.0, 50.0],
    ];
    let block = encode_channel_mixer_ink(ink, [5.0, -10.0, 15.0, -20.0]);
    assert_eq!(block.key, *b"mixr");
    assert_eq!(block.data.len(), 44);
    let Some(Adjustment::ChannelMixer(params)) = decode_adjustment(&block) else {
        panic!("the ink block decodes");
    };
    assert_eq!(params.ink, ink);
    assert_eq!(params.ink_constant, [5.0, -10.0, 15.0, -20.0]);
    assert_eq!(params.red, [20.0, 0.0, 0.0]);
    assert_eq!(params.green, [0.0, 30.0, 0.0]);
    assert_eq!(params.blue, [0.0, 0.0, 40.0]);
}

#[test]
fn channel_mixer_ink_reads_k_source() {
    let mut data = vec![0, 1, 0, 0];
    for row in [
        [10i16, 20, 30, 40, 5],
        [0, 100, 0, 0, 0],
        [0, 0, 100, 0, 0],
        [0, 0, 0, 100, 0],
    ] {
        for value in row {
            data.extend_from_slice(&value.to_be_bytes());
        }
    }
    let Some(Adjustment::ChannelMixer(params)) = decode_adjustment(&adjdata(*b"mixr", data)) else {
        panic!("the hand-built block decodes");
    };
    assert_eq!(params.ink[0], [10.0, 20.0, 30.0, 40.0]);
    assert_eq!(params.ink_constant[0], 5.0);
    assert_eq!(
        params.red,
        [10.0, 20.0, 30.0],
        "RGB view keeps three sources"
    );
}

#[test]
fn channel_mixer_ink_clamps() {
    let block = encode_channel_mixer_ink(
        [
            [999.0, -999.0, 0.0, 0.0],
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 0.0, 100.0, 0.0],
            [0.0, 0.0, 0.0, 100.0],
        ],
        [0.0; 4],
    );
    let Some(Adjustment::ChannelMixer(params)) = decode_adjustment(&block) else {
        panic!("the clamped block decodes");
    };
    assert_eq!(params.ink[0], [200.0, -200.0, 0.0, 0.0]);
}

#[test]
fn channel_mixer_cmyk_page() {
    let block = encode_channel_mixer_ink(ink_identity(), [0.0; 4]);
    let editor = adjustment_editor_in_mode(&block, true).expect("a CMYK page");
    assert_eq!(editor.groups, vec!["Cyan", "Magenta", "Yellow", "Black"]);
    assert_eq!(
        editor.params.len(),
        20,
        "four rows of four sources and a constant"
    );
    assert!(editor.params.iter().all(|p| p.key != "monochrome"));
    let cyan = editor
        .params
        .iter()
        .find(|p| p.key == "0.cyan")
        .expect("cyan");
    assert_eq!(cyan.value, 100.0);
    assert!(editor.params.iter().any(|p| p.key == "3.black"));
    assert!(editor.params.iter().any(|p| p.key == "3.constant"));
}

#[test]
fn channel_mixer_monochrome_toggle() {
    let block = encode_channel_mixer(
        false,
        [100.0, 0.0, 0.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [0.0; 3],
    );
    let rgb = adjustment_editor_in_mode(&block, false).expect("an RGB page");
    assert!(rgb.params.iter().any(|p| p.key == "monochrome"));

    let on = set_adjustment_param_in_mode(&block, "monochrome", 1.0, false).expect("on");
    let editor = adjustment_editor_in_mode(&on, false).expect("gray page");
    assert_eq!(editor.groups, vec!["Gray"]);
    let red = editor
        .params
        .iter()
        .find(|p| p.key == "0.red")
        .expect("red");
    assert_eq!(red.value, 40.0);
    let green = editor
        .params
        .iter()
        .find(|p| p.key == "0.green")
        .expect("green");
    assert_eq!(green.value, 40.0);

    let off = set_adjustment_param_in_mode(&on, "monochrome", 0.0, false).expect("off");
    let editor = adjustment_editor_in_mode(&off, false).expect("rgb page");
    assert_eq!(editor.groups, vec!["Red", "Green", "Blue"]);
    let value = |key: &str| {
        editor
            .params
            .iter()
            .find(|p| p.key == key)
            .map_or(f64::NAN, |p| p.value)
    };
    assert_eq!(value("0.red"), 40.0);
    assert_eq!(value("0.green"), 40.0);
    assert_eq!(value("0.blue"), 20.0);
    assert_eq!(value("1.green"), 100.0);
    assert_eq!(value("2.blue"), 100.0);
}
