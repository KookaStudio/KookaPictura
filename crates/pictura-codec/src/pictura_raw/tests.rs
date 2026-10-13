//! Tests for the Pictura Raw settings codec and smart-filter attachment.

use super::*;
use crate::common::Reader;
use crate::descriptor::{self, get_object_item, DescValue};
use pictura_core::{
    CameraCalibration, CrsSettings, Detail, Document, Effects, Hsl, Layer, LayerBlock, Lens,
    PicturaRawSettings, RawMode, SmartFilter, SmartObject, SmartObjectKind, SplitToning, ToneCurve,
};
use std::path::PathBuf;

fn embedded_layer() -> Layer {
    Layer {
        name: "L".into(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        smart_object: Some(SmartObject {
            kind: SmartObjectKind::Embedded,
            payload: Some(b"8BPS\x00\x01not-a-real-psd".to_vec()),
            filename: "src.psd".into(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// A settings value with every modeled tab populated.
fn full_settings() -> PicturaRawSettings {
    PicturaRawSettings {
        basic: CrsSettings {
            temperature: Some(20.0),
            tint: Some(-5.0),
            exposure: Some(0.75),
            contrast: Some(-10.0),
            highlights: Some(15.0),
            shadows: Some(-8.0),
            whites: Some(12.0),
            blacks: Some(-3.0),
            clarity: Some(6.0),
            vibrance: Some(-9.0),
            saturation: Some(4.0),
        },
        mode: RawMode {
            module: Some("Filter\0".into()),
            white_balance: Some(*b"Cst "),
            auto_wb_version: Some(1),
            settings_mode: Some(*b"Cst "),
            process_version: Some(20120000),
            grayscale: Some(false),
        },
        tone_curve: ToneCurve {
            parametric_shadows: Some(-6),
            parametric_darks: Some(-12),
            parametric_lights: Some(-14),
            parametric_highlights: Some(-15),
            shadow_split: Some(20),
            midtone_split: Some(54),
            highlight_split: Some(85),
            curve_rgb: vec![0, 0, 128, 140, 255, 255],
            curve_red: vec![0, 0, 100, 120, 255, 255],
            curve_green: vec![0, 0, 110, 130, 255, 255],
            curve_blue: vec![0, 0, 120, 140, 255, 255],
        },
        detail: Detail {
            sharpen_amount: Some(13),
            sharpen_radius: Some(1.1),
            sharpen_detail: Some(31),
            sharpen_masking: Some(5),
            noise_luminance: Some(6),
            noise_luminance_detail: Some(56),
            noise_luminance_contrast: Some(13),
            noise_color: Some(6),
            noise_color_detail: Some(40),
            noise_color_smoothness: Some(40),
        },
        hsl: Hsl {
            hue: [
                Some(6),
                Some(14),
                Some(6),
                Some(8),
                Some(17),
                Some(13),
                Some(9),
                Some(15),
            ],
            saturation: [
                Some(9),
                Some(13),
                Some(18),
                Some(21),
                Some(18),
                Some(14),
                Some(15),
                Some(18),
            ],
            luminance: [
                Some(14),
                Some(20),
                Some(17),
                Some(13),
                Some(13),
                Some(13),
                Some(18),
                Some(18),
            ],
            grayscale_mix: [
                Some(-10),
                Some(-19),
                Some(-23),
                Some(-27),
                Some(-18),
                Some(9),
                Some(15),
                Some(4),
            ],
        },
        split_toning: SplitToning {
            shadow_hue: Some(21),
            shadow_saturation: Some(9),
            highlight_hue: Some(27),
            highlight_saturation: Some(9),
            balance: Some(18),
        },
        lens: Lens {
            profile_enabled: Some(0),
            distortion: Some(-23),
            vignette_amount: Some(-17),
            vignette_midpoint: Some(52),
            perspective_vertical: Some(0),
            perspective_horizontal: Some(0),
            perspective_scale: Some(100),
            perspective_aspect: Some(0),
            perspective_upright: Some(0),
            rotate: Some(0.0),
            offset_x: Some(0.0),
            offset_y: Some(0.0),
            defringe_purple_amount: Some(2),
            defringe_purple_hue_lo: Some(26),
            defringe_purple_hue_hi: Some(72),
            defringe_green_amount: Some(2),
            defringe_green_hue_lo: Some(36),
            defringe_green_hue_hi: Some(64),
        },
        effects: Effects {
            grain_amount: Some(9),
            grain_size: Some(6),
            grain_frequency: Some(13),
            dehaze: Some(14),
            vignette_amount: Some(-12),
            vignette_midpoint: Some(43),
            vignette_feather: Some(45),
            vignette_roundness: Some(-8),
            vignette_style: Some(1),
            vignette_highlights: Some(9),
        },
        calibration: CameraCalibration {
            camera_profile: Some("Embedded\0".into()),
            profile_digest: Some("0123456789ABCDEF\0".into()),
            red_hue: Some(-13),
            red_saturation: Some(-17),
            green_hue: Some(-13),
            green_saturation: Some(-15),
            blue_hue: Some(-13),
            blue_saturation: Some(-17),
            shadow_tint: Some(-7),
        },
    }
}

#[test]
fn encode_decode_round_trips_the_set_fields() {
    let settings = full_settings();
    let fltr = encode_pictura_raw_fltr(&settings);
    let bytes = descriptor::write_descriptor(&fltr);
    assert_eq!(decode_pictura_raw_settings(&bytes), settings);
}

#[test]
fn default_settings_encode_no_controls() {
    let fltr = encode_pictura_raw_fltr(&PicturaRawSettings::default());
    let DescValue::Object { items, .. } = &fltr else {
        panic!("Fltr is an object");
    };
    assert!(items.is_empty(), "default settings write no keys");
}

#[test]
fn trailing_space_keys_are_four_bytes() {
    let bytes = descriptor::write_descriptor(&encode_pictura_raw_fltr(&full_settings()));
    let DescValue::Object { items, .. } = crate::camera_raw_options(&bytes).expect("Fltr parses")
    else {
        panic!("Fltr is an object");
    };
    for key in [&b"Crv "[..], b"LNR ", b"CNR ", b"STB "] {
        assert!(
            get_object_item(&items, key).is_some(),
            "key {:?} is present",
            String::from_utf8_lossy(key)
        );
    }
    for key in [&b"Crv"[..], b"LNR", b"CNR", b"STB"] {
        assert!(
            get_object_item(&items, key).is_none(),
            "unspaced {:?} is absent",
            String::from_utf8_lossy(key)
        );
    }
}

#[test]
fn decode_is_tolerant_of_missing_and_malformed() {
    assert_eq!(
        decode_pictura_raw_settings(&[]),
        PicturaRawSettings::default()
    );
    assert_eq!(
        decode_pictura_raw_settings(b"not a descriptor"),
        PicturaRawSettings::default()
    );
    let fltr = object(
        b"Adobe Camera Raw Filter",
        vec![
            (b"Temp", DescValue::Text("warm".into())),
            (b"Ex12", DescValue::Double(f64::NAN)),
            (b"Vibr", DescValue::Long(7)),
            (b"ShpR", DescValue::Text("nope".into())),
        ],
    );
    let decoded = decode_pictura_raw_settings(&descriptor::write_descriptor(&fltr));
    assert_eq!(
        decoded.basic.temperature, None,
        "a non-numeric key is ignored"
    );
    assert_eq!(
        decoded.basic.exposure, None,
        "a non-finite value is ignored"
    );
    assert_eq!(decoded.basic.vibrance, Some(7.0));
    assert_eq!(decoded.detail.sharpen_radius, None, "wrong type ignored");
}

#[test]
fn attach_authors_then_write_read_round_trips() {
    let settings = full_settings();
    let mut layer = embedded_layer();
    attach_pictura_raw_filter(&mut layer, &settings).expect("attach authors filterFX");
    let filter = &layer.smart_object.as_ref().unwrap().smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(decode_pictura_raw_settings(&filter.options), settings);

    let mut doc = Document::new(
        2,
        2,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    doc.layers = vec![layer];
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("re-reads");
    let filter = &back.layers[0]
        .smart_object
        .as_ref()
        .expect("embedded object resolves")
        .smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(filter.name, CAMERA_RAW_FILTER_NAME);
    assert_eq!(decode_pictura_raw_settings(&filter.options), settings);
}

#[test]
fn reattach_without_a_preserved_block_updates_the_typed_filter() {
    let mut layer = embedded_layer();
    attach_pictura_raw_filter(&mut layer, &full_settings()).expect("first attach authors");
    let second = PicturaRawSettings {
        basic: CrsSettings {
            temperature: Some(-30.0),
            exposure: Some(-1.0),
            ..Default::default()
        },
        ..Default::default()
    };
    attach_pictura_raw_filter(&mut layer, &second).expect("re-attach updates in memory");
    let filter = &layer.smart_object.as_ref().unwrap().smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(decode_pictura_raw_settings(&filter.options), second);
}

#[test]
fn attach_inserts_into_a_preserved_sold_and_preserves_other_keys() {
    let so = SmartObject {
        uuid: "12345678-1234-1234-1234-123456789abc".into(),
        kind: SmartObjectKind::Embedded,
        payload: Some(b"8BPS\x00\x01x".to_vec()),
        ..Default::default()
    };
    let layer = embedded_layer();
    let data = crate::smart_writer::author_sold_block(&so, &layer, 2, 2);
    let mut layer = layer;
    layer.smart_object.as_mut().unwrap().uuid = so.uuid.clone();
    layer.extra_blocks.push(LayerBlock {
        key: *b"SoLd",
        data,
    });

    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.5),
            ..Default::default()
        },
        ..Default::default()
    };
    attach_pictura_raw_filter(&mut layer, &settings).expect("inserts filterFX");
    assert_eq!(
        decode_pictura_raw_settings(&layer.smart_object.as_ref().unwrap().smart_filters[0].options),
        settings
    );

    let bytes = crate::write_psd(&{
        let mut doc = Document::new(
            2,
            2,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers = vec![layer];
        doc
    })
    .expect("writes");
    let back = crate::read_psd(&bytes).expect("re-reads");
    let filter = &back.layers[0].smart_object.as_ref().unwrap().smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(decode_pictura_raw_settings(&filter.options), settings);
}

/// Parse a preserved `SoLd`/`SoLE` block into `filterFXList[0]`'s items.
fn config_filter_entry(config_descriptor: &[u8]) -> Vec<(Vec<u8>, DescValue)> {
    let mut r = Reader::new(&config_descriptor[8..]);
    let DescValue::Object { items, .. } =
        descriptor::read_descriptor(&mut r).expect("descriptor parses")
    else {
        panic!("descriptor is an object");
    };
    let DescValue::Object {
        items: filter_fx, ..
    } = get_object_item(&items, b"filterFX").expect("filterFX present")
    else {
        panic!("filterFX is an object");
    };
    let DescValue::List(list) = get_object_item(filter_fx, b"filterFXList").expect("list") else {
        panic!("filterFXList is a list");
    };
    let DescValue::Object { items: entry, .. } = &list[0] else {
        panic!("entry is an object");
    };
    entry.clone()
}

const PRESERVE_KEYS: [&[u8]; 7] = [
    b"Upri", b"GuUr", b"Rtch", b"REye", b"LCs ", b"AuCA", b"TMMs",
];

#[test]
fn reattach_over_a_preserved_filter_keeps_unmodeled_keys() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let mut doc = crate::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter_mut()
        .find(|l| l.name == "Layer 1 copy")
        .expect("Layer 1 copy present");

    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(1.25),
            ..Default::default()
        },
        ..Default::default()
    };
    attach_pictura_raw_filter(layer, &settings).expect("re-attach over the preserved entry");

    let written = crate::write_psd(&doc).expect("fixture writes");
    let back = crate::read_psd(&written).expect("written fixture re-reads");
    let so = back
        .layers
        .iter()
        .find(|l| l.name == "Layer 1 copy")
        .and_then(|l| l.smart_object.as_ref())
        .expect("smart object survives a write");
    assert_eq!(
        so.smart_filters.len(),
        1,
        "the entry is replaced, not duplicated"
    );
    let filter = &so.smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(
        decode_pictura_raw_settings(&filter.options).basic.exposure,
        Some(1.25)
    );

    let entry = config_filter_entry(&so.config_descriptor);
    assert!(
        get_object_item(&entry, b"blendOptions").is_some(),
        "the per-filter blendOptions survives the merge"
    );
    let DescValue::Object { items: fltr, .. } =
        get_object_item(&entry, b"Fltr").expect("Fltr present")
    else {
        panic!("Fltr is an object");
    };
    assert_eq!(
        get_object_item(fltr, b"AuCA"),
        Some(&DescValue::Long(0)),
        "an unmodeled Fltr key survives the merge"
    );
}

#[test]
fn reattach_a_generic_filter_replaces_only_its_own_entry() {
    let options =
        descriptor::write_descriptor(&object(b"null", vec![(b"Keep", DescValue::Long(7))]));
    let so = SmartObject {
        kind: SmartObjectKind::Embedded,
        payload: Some(b"8BPS\x00\x01x".to_vec()),
        smart_filters: vec![SmartFilter {
            filter_id: 4242,
            name: "First".to_string(),
            enabled: true,
            options,
        }],
        ..Default::default()
    };
    let layer0 = embedded_layer();
    let data = crate::smart_writer::author_sold_block(&so, &layer0, 2, 2);
    let mut layer = layer0;
    layer.extra_blocks.push(LayerBlock {
        key: *b"SoLd",
        data,
    });

    let replacement = SmartFilter {
        filter_id: 4242,
        name: "Second".to_string(),
        enabled: false,
        options: descriptor::write_descriptor(&object(b"null", vec![(b"New", DescValue::Long(9))])),
    };
    attach_smart_filter(&mut layer, replacement).expect("same-id attach replaces the entry");

    let doc = {
        let mut doc = Document::new(
            2,
            2,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers = vec![layer];
        doc
    };
    let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("re-reads");
    let so = back.layers[0].smart_object.as_ref().expect("resolves");
    assert_eq!(so.smart_filters.len(), 1, "still one entry");
    assert_eq!(so.smart_filters[0].name, "Second");
    assert!(!so.smart_filters[0].enabled);

    let entry = config_filter_entry(&so.config_descriptor);
    let DescValue::Object { items: fltr, .. } =
        get_object_item(&entry, b"Fltr").expect("Fltr present")
    else {
        panic!("Fltr is an object");
    };
    assert_eq!(
        get_object_item(fltr, b"New"),
        Some(&DescValue::Long(9)),
        "the replacement's keys win"
    );
    assert_eq!(
        get_object_item(fltr, b"Keep"),
        Some(&DescValue::Long(7)),
        "the previous entry's unmodeled keys survive"
    );
}

#[test]
fn attach_errors_without_a_smart_object() {
    let mut layer = Layer::default();
    assert!(attach_pictura_raw_filter(&mut layer, &full_settings()).is_err());
    assert!(layer.smart_object.is_none());
}

#[test]
fn fixture_all_eleven_basic_keys_decode() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let doc = crate::read_psd(&bytes).expect("fixture parses");
    let filter = &doc
        .layers
        .iter()
        .find(|l| l.name == "Layer 1 copy")
        .and_then(|l| l.smart_object.as_ref())
        .expect("smart object resolved")
        .smart_filters[0];
    assert_eq!(filter.filter_id, CAMERA_RAW_FILTER_ID);
    assert_eq!(
        decode_pictura_raw_settings(&filter.options).basic,
        CrsSettings {
            temperature: Some(-30.0),
            tint: Some(12.0),
            exposure: Some(-1.15),
            contrast: Some(12.0),
            highlights: Some(-15.0),
            shadows: Some(10.0),
            whites: Some(-18.0),
            blacks: Some(9.0),
            clarity: Some(-12.0),
            vibrance: Some(13.0),
            saturation: Some(-12.0),
        }
    );
}

#[test]
fn fixture_full_key_map_round_trips_and_preserves_unmodeled() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let mut doc = crate::read_psd(&bytes).expect("fixture parses");
    let layer = doc
        .layers
        .iter_mut()
        .find(|l| l.name == "Layer 1 copy")
        .expect("Layer 1 copy present");

    let filter = &layer.smart_object.as_ref().unwrap().smart_filters[0];
    let settings = decode_pictura_raw_settings(&filter.options);

    // Every modeled tab is present in the regenerated fixture.
    assert!(settings.basic.exposure.is_some(), "basic tab");
    assert!(settings.tone_curve.curve_rgb.len() >= 4, "point-curve tab");
    assert!(settings.detail.sharpen_amount.is_some(), "detail tab");
    assert!(settings.hsl.hue.iter().all(|v| v.is_some()), "hsl tab");
    assert!(
        settings.hsl.grayscale_mix.iter().all(|v| v.is_some()),
        "grayscale mix"
    );
    assert!(settings.split_toning.balance.is_some(), "split toning");
    assert!(settings.lens.vignette_midpoint.is_some(), "lens tab");
    assert!(settings.effects.grain_frequency.is_some(), "effects tab");
    assert_eq!(settings.effects.dehaze, Some(14), "dehaze modeled");
    assert!(
        settings.calibration.camera_profile.is_some(),
        "calibration tab"
    );
    assert_eq!(
        settings.calibration.red_hue,
        Some(-13),
        "calibration primaries"
    );
    assert_eq!(settings.calibration.shadow_tint, Some(-7), "shadow tint");

    // Capture the unmodeled keys as they sit in the fixture.
    let DescValue::Object {
        items: before_items,
        ..
    } = crate::camera_raw_options(&filter.options).expect("Fltr parses")
    else {
        panic!("Fltr is an object");
    };
    let preserved: Vec<DescValue> = PRESERVE_KEYS
        .iter()
        .map(|key| {
            get_object_item(&before_items, key)
                .unwrap_or_else(|| panic!("fixture carries {:?}", String::from_utf8_lossy(key)))
                .clone()
        })
        .collect();

    // Re-attach the decoded settings, write, and re-read.
    attach_pictura_raw_filter(layer, &settings).expect("re-attach");
    let written = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&written).expect("re-reads");
    let filter = &back
        .layers
        .iter()
        .find(|l| l.name == "Layer 1 copy")
        .and_then(|l| l.smart_object.as_ref())
        .expect("smart object survives")
        .smart_filters[0];

    assert_eq!(
        decode_pictura_raw_settings(&filter.options),
        settings,
        "every modeled control round-trips"
    );

    let DescValue::Object {
        items: after_items, ..
    } = crate::camera_raw_options(&filter.options).expect("Fltr parses")
    else {
        panic!("Fltr is an object");
    };
    for (key, value) in PRESERVE_KEYS.iter().zip(preserved.iter()) {
        assert_eq!(
            get_object_item(&after_items, key),
            Some(value),
            "unmodeled key {:?} survives byte-for-byte",
            String::from_utf8_lossy(key)
        );
    }
}
