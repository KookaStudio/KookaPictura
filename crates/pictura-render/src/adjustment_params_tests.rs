use super::*;
use crate::{
    encode_brightness_contrast, encode_channel_mixer, encode_color_balance, encode_curves,
    encode_hue_saturation, encode_invert, encode_photo_filter, encode_posterize,
    encode_selective_color,
};
use pictura_adjust::{SelectiveColorMethod, SelectiveRange};

fn value(data: &AdjustmentData, key: &str) -> f64 {
    adjustment_editor(data)
        .unwrap()
        .params
        .into_iter()
        .find(|p| p.key == key)
        .unwrap_or_else(|| panic!("no {key}"))
        .value
}

fn levels(input_black: u16, input_white: u16, gamma: u16) -> AdjustmentData {
    let mut data = 2u16.to_be_bytes().to_vec();
    for v in [input_black, input_white, 0, 255, gamma] {
        data.extend_from_slice(&v.to_be_bytes());
    }
    // Red, green, and blue at identity, then the 25 records the decoder does
    // not read.
    for _ in 0..3 {
        for v in [0u16, 255, 0, 255, 100] {
            data.extend_from_slice(&v.to_be_bytes());
        }
    }
    data.extend(std::iter::repeat_n(7u8, 250));
    AdjustmentData {
        key: *b"levl",
        data: data.into(),
    }
}

#[test]
fn a_patch_keeps_what_the_model_does_not_carry() {
    // brit's mean (offset 4) and Lab flag (offset 6) are not modelled.
    let mut brit = encode_brightness_contrast(20, 0);
    brit.data[4..7].copy_from_slice(&[0, 9, 1]);
    let edited = set_adjustment_param(&brit, "brightness", 55.0).unwrap();
    assert_eq!(value(&edited, "brightness"), 55.0);
    assert_eq!(&edited.data[4..7], &[0, 9, 1]);

    // hue2's Colorize byte and a colour range's band survive a Master edit.
    let mut hue = encode_hue_saturation(30, 0, 0);
    hue.data[2] = 1;
    hue.data[17] = 0x2a;
    let edited = set_adjustment_param(&hue, "hue", -40.0).unwrap();
    assert_eq!(value(&edited, "hue"), -40.0);
    assert_eq!((edited.data[2], edited.data[17]), (1, 0x2a));
    assert_eq!(edited.data.len(), hue.data.len());

    let edited = set_adjustment_param(&levels(0, 255, 100), "gamma", 1.5).unwrap();
    assert_eq!(value(&edited, "gamma"), 1.5);
    assert!(edited.data[42..].iter().all(|&b| b == 7));
}

#[test]
fn black_white_tint_and_its_colour_are_editable() {
    let bw = crate::default_adjustment_block("black-white", [0; 3], [255; 3]).unwrap();
    assert_eq!(value(&bw, "tint"), 0.0);
    assert_eq!(value(&bw, "tintColor"), f64::from(0xe1_d3_b4u32));
    let tinted = set_adjustment_param(&bw, "tint", 1.0).unwrap();
    let tinted = set_adjustment_param(&tinted, "tintColor", f64::from(0x80_40_20u32)).unwrap();
    assert_eq!(value(&tinted, "tintColor"), f64::from(0x80_40_20u32));
    let Some(pictura_adjust::Adjustment::BlackWhite(p)) = crate::decode_adjustment(&tinted) else {
        panic!("blwh decode");
    };
    assert!(p.tint);
    assert_eq!(p.tint_color, [0x80, 0x40, 0x20]);
    assert_eq!(p.red, 40.0);
}

#[test]
fn hue_saturation_ranges_are_grouped_and_decode_with_their_band() {
    let editor = adjustment_editor(&encode_hue_saturation(0, 0, 0)).unwrap();
    assert_eq!(editor.groups.len(), 7);
    let blues =
        set_adjustment_param(&encode_hue_saturation(0, 0, 0), "blues.lightness", 40.0).unwrap();
    // Blues is range 4: settings at 16 + 14*4 + 8, lightness two i16 further.
    assert_eq!(&blues.data[84..86], &40i16.to_be_bytes());
    let Some(pictura_adjust::Adjustment::HueSaturation(p)) = crate::decode_adjustment(&blues)
    else {
        panic!("hue2 decode");
    };
    assert_eq!(p.ranges.len(), 1);
    assert_eq!(
        (p.ranges[0].begin_sustain, p.ranges[0].lightness),
        (225, 40)
    );
    assert_eq!(value(&blues, "lightness"), 0.0);
}

#[test]
fn levels_channels_are_grouped_and_edit_their_own_record() {
    let editor = adjustment_editor(&levels(0, 255, 100)).unwrap();
    assert_eq!(editor.groups, ["RGB", "Red", "Green", "Blue"]);
    let green = editor
        .params
        .iter()
        .find(|p| p.key == "green.inputWhite")
        .unwrap();
    assert_eq!((green.group, green.value), (Some(2), 255.0));

    // Green's input white is record 2's second u16, at 2 + 20 + 2.
    let edited = set_adjustment_param(&levels(0, 255, 100), "green.inputWhite", 200.0).unwrap();
    assert_eq!(&edited.data[24..26], &200u16.to_be_bytes());
    assert_eq!(value(&edited, "inputWhite"), 255.0);
    let Some(pictura_adjust::Adjustment::Levels(p)) = crate::decode_adjustment(&edited) else {
        panic!("levels decode");
    };
    assert_eq!((p.red, p.blue), (None, None));
    assert_eq!(p.green.map(|g| g.input_white), Some(200));
    // A channel's black past its white is refused, like the composite's.
    assert!(set_adjustment_param(&edited, "green.inputBlack", 230.0).is_none());
}

#[test]
fn values_clamp_and_invalid_results_are_refused() {
    let brit = encode_brightness_contrast(0, 0);
    let clamped = set_adjustment_param(&brit, "contrast", 500.0).unwrap();
    assert_eq!(value(&clamped, "contrast"), 100.0);
    assert!(set_adjustment_param(&brit, "nonsense", 1.0).is_none());
    assert!(set_adjustment_param(&encode_invert(), "anything", 1.0).is_none());

    // Levels needs input black below input white.
    let narrow = set_adjustment_param(&levels(0, 255, 100), "inputWhite", 10.0).unwrap();
    assert!(set_adjustment_param(&narrow, "inputBlack", 20.0).is_none());
    let posterize = set_adjustment_param(&encode_posterize(4), "levels", 1.0).unwrap();
    assert_eq!(value(&posterize, "levels"), 2.0);
}

#[test]
fn descriptor_items_are_edited_and_added_in_place() {
    let obj = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (b"vibrance".to_vec(), DescValue::Long(10)),
            (b"Xtra".to_vec(), DescValue::Bool(true)),
        ],
    };
    let vib = AdjustmentData {
        key: *b"vibA",
        data: pictura_codec::write_descriptor(&obj).into(),
    };
    assert_eq!(
        value(&vib, "saturation"),
        0.0,
        "missing reads as the default"
    );
    let edited = set_adjustment_param(&vib, "saturation", 25.0).unwrap();
    assert_eq!(value(&edited, "saturation"), 25.0);
    assert_eq!(value(&edited, "vibrance"), 10.0);
    let back = pictura_codec::read_descriptor(&edited.data).unwrap();
    assert_eq!(desc_item(&back, b"Xtra"), Some(&DescValue::Bool(true)));
}

#[test]
fn grouped_pages_address_each_group() {
    let balance = encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true);
    let editor = adjustment_editor(&balance).unwrap();
    assert_eq!(editor.groups, ["Shadows", "Midtones", "Highlights"]);
    assert_eq!(editor.params.len(), 10);
    let edited = set_adjustment_param(&balance, "midtones.cyanRed", 40.0).unwrap();
    match decode_adjustment(&edited) {
        Some(Adjustment::ColorBalance(p)) => assert_eq!(p.midtones, [40.0, 0.0, 0.0]),
        other => panic!("{other:?}"),
    }

    let selective = encode_selective_color(
        SelectiveColorMethod::Relative,
        &[SelectiveRange::default(); 9],
    );
    let editor = adjustment_editor(&selective).unwrap();
    assert_eq!(editor.groups.len(), 9);
    let edited = set_adjustment_param(&selective, "blues.black", -30.0).unwrap();
    let edited = set_adjustment_param(&edited, "method", 1.0).unwrap();
    match decode_adjustment(&edited) {
        Some(Adjustment::SelectiveColor(p)) => {
            assert_eq!(p.ranges[4].k, -30);
            assert_eq!(p.method, SelectiveColorMethod::Absolute);
        }
        other => panic!("{other:?}"),
    }

    let mixer = encode_channel_mixer(
        false,
        [100.0, 0.0, 0.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [0.0; 3],
    );
    let edited = set_adjustment_param(&mixer, "1.red", 30.0).unwrap();
    match decode_adjustment(&edited) {
        Some(Adjustment::ChannelMixer(p)) => assert_eq!(p.green, [30.0, 100.0, 0.0]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn photo_filter_colour_is_editable_in_version_two_only() {
    let filter = encode_photo_filter([255, 180, 80], 25.0, true);
    assert_eq!(value(&filter, "color"), f64::from(0xff_b4_50u32));
    let edited = set_adjustment_param(&filter, "color", f64::from(0x10_20_30u32)).unwrap();
    match decode_adjustment(&edited) {
        Some(Adjustment::PhotoFilter(p)) => assert_eq!(p.color, [0x10, 0x20, 0x30]),
        other => panic!("{other:?}"),
    }
    let mut v3 = vec![0, 3];
    for v in [0u32, 0, 0, 40] {
        v3.extend_from_slice(&v.to_be_bytes());
    }
    v3.extend_from_slice(&[1, 0, 0]);
    let v3 = AdjustmentData {
        key: *b"phfl",
        data: v3.into(),
    };
    let editor = adjustment_editor(&v3).unwrap();
    assert!(editor.params.iter().all(|p| p.key != "color"));
    assert_eq!(value(&v3, "density"), 40.0);
}

#[test]
fn curves_edit_per_channel() {
    let curves = encode_curves(&[(0, 0), (255, 255)], None, None, None);
    assert!(adjustment_editor(&curves).unwrap().curves);
    let red = [(0, 0), (128, 160), (255, 255)];
    let edited = set_curve_points(&curves, 1, &red).unwrap();
    assert_eq!(curve_points(&edited, 1).unwrap(), red);
    assert_eq!(curve_points(&edited, 2).unwrap(), [(0, 0), (255, 255)]);
    // Back to the identity, the channel curve is dropped.
    let flat = set_curve_points(&edited, 1, &[(0, 0), (255, 255)]).unwrap();
    assert!(matches!(decode_adjustment(&flat), Some(Adjustment::Curves(c)) if c.red.is_none()));
    assert!(set_curve_points(&curves, 0, &[(10, 0), (5, 255)]).is_none());
    assert!(set_curve_points(&curves, 4, &red).is_none());
}

#[test]
fn reset_restores_every_default() {
    let brit = encode_brightness_contrast(60, -20);
    let reset = reset_adjustment(&brit).unwrap();
    assert_eq!(
        (value(&reset, "brightness"), value(&reset, "contrast")),
        (0.0, 0.0)
    );
    let curves = encode_curves(&[(0, 30), (255, 200)], None, None, None);
    let reset = reset_adjustment(&curves).unwrap();
    assert_eq!(curve_points(&reset, 0).unwrap(), [(0, 0), (255, 255)]);
    let note = adjustment_editor(&encode_invert()).unwrap();
    assert!(note.params.is_empty() && note.note.is_some());
}

#[test]
fn exposure_specs_carry_the_dialog_precision() {
    let block = crate::default_adjustment_block("exposure", [0; 3], [255; 3]).unwrap();
    let editor = adjustment_editor(&block).unwrap();
    let spec = |key: &str| {
        let param = editor.params.iter().find(|p| p.key == key).unwrap();
        match param.kind {
            ParamKind::Slider { min, max, decimals } => (min, max, decimals),
            _ => panic!("{key} is a slider"),
        }
    };
    assert_eq!(spec("exposure"), (-20.0, 20.0, 2));
    assert_eq!(spec("offset"), (-0.5, 0.5, 4));
    assert_eq!(spec("gamma"), (0.01, 9.99, 2));
}
