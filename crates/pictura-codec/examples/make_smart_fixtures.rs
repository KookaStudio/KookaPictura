//! Author the synthetic smart-object PSD fixtures committed under `assets/`.
//!
//! Regenerate with:
//!
//! ```bash
//! cargo run -p pictura-codec --example make_smart_fixtures
//! ```
//!
//! Both fixtures mirror the structure the old reference files carried, but every
//! byte is authored here by `pictura-codec` itself:
//!
//! - `test_with_smart_object01.psd`: layer `Layer 1` with one embedded smart
//!   object and no smart filter.
//! - `test_with_smart_object02.psd`: layer `Layer 1 copy` with the same
//!   embedded smart object plus one Camera Raw smart filter (`filterID 2683`).
//!
//! The embedded payload is a minimal version-2 `8BPS` document (`write_psb`), so
//! the fixtures stay a few KB; the output is deterministic.

use std::path::Path;

use pictura_codec::{encode_pictura_raw_fltr, write_descriptor, write_psb, write_psd};
use pictura_core::{
    BitDepth, CameraCalibration, Channel, ColorMode, CrsSettings, Detail, Document, Effects, Hsl,
    Layer, Lens, PicturaRawSettings, PsdRect, RawMode, SmartFilter, SmartObject, SmartObjectKind,
    SplitToning, ToneCurve,
};

const EMBEDDED_FILENAME: &str = "Layer 1.psb";

fn embedded_psb() -> Vec<u8> {
    write_psb(&Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight)).expect("embedded PSB writes")
}

fn smart_layer(name: &str, payload: Vec<u8>, smart_filters: Vec<SmartFilter>) -> Layer {
    Layer {
        name: name.to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        // A placed object normally keeps its rendered proxy; a 4x4 red square.
        channels: vec![
            Channel {
                id: 0,
                data: vec![255; 16].into(),
            },
            Channel {
                id: 1,
                data: vec![0; 16].into(),
            },
            Channel {
                id: 2,
                data: vec![0; 16].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 16].into(),
            },
        ],
        smart_object: Some(SmartObject {
            kind: SmartObjectKind::Embedded,
            payload: Some(payload),
            filename: EMBEDDED_FILENAME.to_string(),
            filetype: *b"8BPB",
            creator: *b"8BIM",
            smart_filters,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// The Camera Raw `Fltr` options the tests assert on: every modeled tab, plus a
/// set of keys the engine does not model (so a filter edit proves unmodeled keys
/// survive). All values are authored here; none come from a reference file.
fn camera_raw_options() -> Vec<u8> {
    let settings = PicturaRawSettings {
        basic: CrsSettings {
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
    };
    let mut fltr = encode_pictura_raw_fltr(&settings);
    if let pictura_codec::DescValue::Object { items, .. } = &mut fltr {
        let unmodeled: [(&[u8], pictura_codec::DescValue); 7] = [
            (b"AuCA", pictura_codec::DescValue::Long(0)),
            (b"TMMs", pictura_codec::DescValue::Long(0)),
            (b"LCs ", pictura_codec::DescValue::Text("\0".into())),
            (b"REye", pictura_codec::DescValue::Text("\0".into())),
            (b"Rtch", pictura_codec::DescValue::Text("\0".into())),
            (
                b"Upri",
                pictura_codec::DescValue::Text("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>\0".into()),
            ),
            (
                b"GuUr",
                pictura_codec::DescValue::Text("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>\0".into()),
            ),
        ];
        for (key, value) in unmodeled {
            items.push((key.to_vec(), value));
        }
    }
    write_descriptor(&fltr)
}

fn camera_raw_filter() -> SmartFilter {
    SmartFilter {
        filter_id: pictura_codec::CAMERA_RAW_FILTER_ID,
        name: pictura_codec::CAMERA_RAW_FILTER_NAME.to_string(),
        enabled: true,
        options: camera_raw_options(),
    }
}

fn document(layer: Layer) -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    for (i, byte) in doc.composite.data.iter_mut().enumerate() {
        *byte = (i * 7 + 3) as u8;
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let payload = embedded_psb();

    let fixtures = [
        (
            "test_with_smart_object01.psd",
            document(smart_layer("Layer 1", payload.clone(), Vec::new())),
        ),
        (
            "test_with_smart_object02.psd",
            document(smart_layer(
                "Layer 1 copy",
                payload,
                vec![camera_raw_filter()],
            )),
        ),
    ];

    for (name, doc) in fixtures {
        let path = assets.join(name);
        let bytes = write_psd(&doc).expect("fixture writes");
        std::fs::write(&path, &bytes).expect("fixture writes to disk");
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
}
