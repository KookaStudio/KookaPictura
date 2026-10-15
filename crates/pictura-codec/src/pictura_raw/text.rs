//! Text form of the full Pictura Raw settings, for crossing the C++/Rust bridge.
//!
//! The bridge carries the settings as one opaque string rather than as a wide
//! argument list, so adding a tab never widens the bridge. The grammar is
//! `key=value` pairs joined by `;`, using the same 4-byte `Fltr` keys as the
//! codec (see `codec.rs`): numeric keys as decimal, text/enum keys as their
//! stored bytes, curve lists as comma-separated integers. Keys and values are
//! escaped rather than trimmed (`\`, `;`, `=` and bytes below `0x20` or at
//! `0x7f`), so a key ending in a space (`Crv `) or a value holding a trailing
//! NUL survives verbatim. An empty string means "no settings".
//!
//! `to_text` is derived from the codec's `encode_items`, and `from_text` feeds
//! `decode_items`, so the text form cannot drift from the on-disk key map.

use pictura_core::PicturaRawSettings;

use crate::descriptor::DescValue;

use super::codec::{decode_items, encode_items};

/// Serialize the full settings to the bridge text form. Default settings yield
/// the empty string.
pub fn pictura_raw_to_text(settings: &PicturaRawSettings) -> String {
    let items = encode_items(settings);
    let mut out = String::new();
    for (i, (key, value)) in items.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(&escape(key));
        out.push('=');
        out.push_str(&escape(&value_bytes(value)));
    }
    out
}

/// Parse the bridge text form. An empty string yields default settings. An
/// unparseable pair is skipped, leaving its control unset.
pub fn pictura_raw_from_text(text: &str) -> PicturaRawSettings {
    if text.is_empty() {
        return PicturaRawSettings::default();
    }
    let items: Vec<(Vec<u8>, DescValue)> = split_escaped(text, b';')
        .into_iter()
        .filter_map(|pair| {
            let eq = find_unescaped(pair, b'=')?;
            let key = unescape(&pair[..eq]);
            let value = parse_value(&key, &pair[eq + 1..]);
            Some((key, value))
        })
        .collect();
    decode_items(&items)
}

/// The raw bytes of a descriptor value as it appears on the right of `=`.
fn value_bytes(value: &DescValue) -> Vec<u8> {
    match value {
        DescValue::Long(v) => v.to_string().into_bytes(),
        DescValue::Double(v) => v.to_string().into_bytes(),
        DescValue::Bool(b) => {
            if *b {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        DescValue::Text(s) => s.as_bytes().to_vec(),
        DescValue::Enum { value, .. } => value.clone(),
        DescValue::List(list) => {
            let mut out = String::new();
            for (i, item) in list.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match item {
                    DescValue::Long(v) => out.push_str(&v.to_string()),
                    DescValue::Double(v) => out.push_str(&v.to_string()),
                    _ => {}
                }
            }
            out.into_bytes()
        }
        _ => Vec::new(),
    }
}

/// Rebuild a value for `key`. Only the non-numeric encodings need naming: every
/// other key is a number.
fn parse_value(key: &[u8], raw: &str) -> DescValue {
    match key {
        b"CtoG" => DescValue::Bool(unescape(raw).first() == Some(&b'1')),
        b"WBal" | b"Sett" => DescValue::Enum {
            kind: key.to_vec(),
            value: unescape(raw),
        },
        b"CMod" | b"CamP" | b"CP_D" => {
            DescValue::Text(String::from_utf8_lossy(&unescape(raw)).into_owned())
        }
        b"Crv " | b"CrvR" | b"CrvG" | b"CrvB" => DescValue::List(
            raw.split(',')
                .filter(|t| !t.is_empty())
                .map(number_literal)
                .collect(),
        ),
        _ => number_literal(raw),
    }
}

fn number_literal(text: &str) -> DescValue {
    match text.parse::<i32>() {
        Ok(v) => DescValue::Long(v),
        Err(_) => DescValue::Double(text.parse::<f64>().unwrap_or(0.0)),
    }
}

/// Escape `\`, `;`, `=` and any byte outside the printable ASCII range. A space
/// (`0x20`) is printable and stays literal, so a trailing-space key survives.
fn escape(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b';' => out.push_str("\\;"),
            b'=' => out.push_str("\\="),
            0x20..=0x7e => out.push(b as char),
            _ => {
                out.push_str("\\x");
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
    out
}

fn unescape(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'\\' => {
                    out.push(b'\\');
                    i += 2;
                }
                b';' => {
                    out.push(b';');
                    i += 2;
                }
                b'=' => {
                    out.push(b'=');
                    i += 2;
                }
                b'x' if i + 3 < bytes.len() => {
                    out.push(hex_value(bytes[i + 2]) * 16 + hex_value(bytes[i + 3]));
                    i += 4;
                }
                other => {
                    out.push(b'\\');
                    out.push(other);
                    i += 2;
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + nibble - 10) as char,
    }
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    }
}

/// Split on an unescaped `delim`, returning the raw (still-escaped) segments.
fn split_escaped(text: &str, delim: u8) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == delim {
            parts.push(&text[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    parts.push(&text[start..]);
    parts
}

fn find_unescaped(text: &str, delim: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == delim {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        CameraCalibration, CrsSettings, Detail, Effects, Hsl, Lens, PicturaRawSettings, RawMode,
        SplitToning, ToneCurve,
    };

    fn every_tab() -> PicturaRawSettings {
        PicturaRawSettings {
            basic: CrsSettings {
                temperature: Some(15.0),
                tint: Some(-8.0),
                exposure: Some(1.25),
                contrast: Some(30.0),
                highlights: Some(-40.0),
                shadows: Some(20.0),
                whites: Some(10.0),
                blacks: Some(-5.0),
                clarity: Some(12.0),
                vibrance: Some(25.0),
                saturation: Some(-3.0),
            },
            mode: RawMode {
                module: Some("Filter\0".into()),
                white_balance: Some(*b"Cst "),
                auto_wb_version: Some(2),
                settings_mode: Some(*b"Cst "),
                process_version: Some(2012),
                grayscale: Some(true),
            },
            tone_curve: ToneCurve {
                parametric_shadows: Some(-10),
                parametric_darks: Some(5),
                parametric_lights: Some(7),
                parametric_highlights: Some(-2),
                shadow_split: Some(25),
                midtone_split: Some(50),
                highlight_split: Some(75),
                curve_rgb: vec![0, 0, 128, 140, 255, 255],
                curve_red: vec![0, 0, 255, 255],
                curve_green: vec![0, 0, 255, 255],
                curve_blue: vec![0, 0, 255, 255],
            },
            detail: Detail {
                sharpen_amount: Some(40),
                sharpen_radius: Some(1.5),
                sharpen_detail: Some(25),
                sharpen_masking: Some(10),
                noise_luminance: Some(20),
                noise_luminance_detail: Some(50),
                noise_luminance_contrast: Some(0),
                noise_color: Some(25),
                noise_color_detail: Some(50),
                noise_color_smoothness: Some(50),
            },
            hsl: Hsl {
                hue: [Some(1), None, Some(3), None, None, None, None, None],
                saturation: [None; 8],
                luminance: [None; 8],
                grayscale_mix: [Some(0); 8],
            },
            split_toning: SplitToning {
                shadow_hue: Some(210),
                shadow_saturation: Some(20),
                highlight_hue: Some(40),
                highlight_saturation: Some(15),
                balance: Some(-10),
            },
            lens: Lens {
                profile_enabled: Some(1),
                distortion: Some(-15),
                vignette_amount: Some(-20),
                vignette_midpoint: Some(50),
                perspective_vertical: Some(0),
                perspective_horizontal: Some(0),
                perspective_scale: Some(100),
                perspective_aspect: Some(0),
                perspective_upright: Some(0),
                rotate: Some(0.5),
                offset_x: Some(0.0),
                offset_y: Some(0.0),
                defringe_purple_amount: Some(3),
                defringe_purple_hue_lo: Some(30),
                defringe_purple_hue_hi: Some(70),
                defringe_green_amount: Some(3),
                defringe_green_hue_lo: Some(40),
                defringe_green_hue_hi: Some(60),
            },
            effects: Effects {
                grain_amount: Some(15),
                grain_size: Some(25),
                grain_frequency: Some(50),
                dehaze: Some(7),
                vignette_amount: Some(-12),
                vignette_midpoint: Some(50),
                vignette_feather: Some(50),
                vignette_roundness: Some(0),
                vignette_style: Some(1),
                vignette_highlights: Some(0),
            },
            calibration: CameraCalibration {
                camera_profile: Some("Pictura Standard\0".into()),
                profile_digest: Some("digest".into()),
                red_hue: Some(5),
                red_saturation: Some(-3),
                green_hue: Some(0),
                green_saturation: Some(2),
                blue_hue: Some(-1),
                blue_saturation: Some(4),
                shadow_tint: Some(-6),
            },
        }
    }

    #[test]
    fn every_tab_round_trips_through_text() {
        let settings = every_tab();
        let text = pictura_raw_to_text(&settings);
        assert_eq!(pictura_raw_from_text(&text), settings);
    }

    #[test]
    fn default_settings_are_the_empty_string() {
        assert_eq!(pictura_raw_to_text(&PicturaRawSettings::default()), "");
        assert_eq!(pictura_raw_from_text(""), PicturaRawSettings::default());
    }

    #[test]
    fn trailing_space_key_and_nul_text_survive() {
        let settings = PicturaRawSettings {
            tone_curve: ToneCurve {
                curve_rgb: vec![0, 0, 255, 255],
                ..Default::default()
            },
            calibration: CameraCalibration {
                camera_profile: Some("Acme\0".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let text = pictura_raw_to_text(&settings);
        // `Crv ` keeps its trailing space, and the NUL is escaped, not dropped.
        assert!(text.contains("Crv =0,0,255,255"), "{text}");
        assert!(text.contains("CamP=Acme\\x00"), "{text}");
        assert_eq!(pictura_raw_from_text(&text), settings);
    }

    #[test]
    fn separators_inside_text_are_escaped() {
        let settings = PicturaRawSettings {
            calibration: CameraCalibration {
                camera_profile: Some("a;b=c\\d".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let text = pictura_raw_to_text(&settings);
        assert_eq!(split_escaped(&text, b';').len(), 1, "{text}");
        assert_eq!(pictura_raw_from_text(&text), settings);
    }

    #[test]
    fn text_keys_match_the_fltr_codec_keys() {
        let settings = every_tab();
        let mut from_text: Vec<Vec<u8>> = split_escaped(&pictura_raw_to_text(&settings), b';')
            .into_iter()
            .map(|pair| unescape(&pair[..find_unescaped(pair, b'=').unwrap()]))
            .collect();
        let mut from_codec: Vec<Vec<u8>> = encode_items(&settings)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        from_text.sort();
        from_codec.sort();
        assert_eq!(from_text, from_codec);
    }

    // The same literal is committed in the C++ `tst_acr_settings` fixture; keep
    // the two in step.
    #[test]
    fn committed_bridge_fixture_parses() {
        let text = "Temp=20;Ex12=1.5;Dhze=7;Crv =0,0,128,140,255,255;CamP=Pictura Standard\\x00";
        let settings = pictura_raw_from_text(text);
        assert_eq!(settings.basic.temperature, Some(20.0));
        assert_eq!(settings.basic.exposure, Some(1.5));
        assert_eq!(settings.effects.dehaze, Some(7));
        assert_eq!(
            settings.tone_curve.curve_rgb,
            vec![0, 0, 128, 140, 255, 255]
        );
        assert_eq!(
            settings.calibration.camera_profile.as_deref(),
            Some("Pictura Standard\0")
        );
    }
}
