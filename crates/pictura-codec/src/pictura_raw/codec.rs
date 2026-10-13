//! Type-aware encode/decode of the Pictura Raw `Fltr` key map.
//!
//! Key names and value types were reverse-engineered from a reference Photoshop
//! camera-raw filter descriptor; see `docs/dev/camera-raw-cc-notes.md` §Fltr key
//! map. Encoding emits only set controls, so default settings write no keys and
//! decoding leaves a control unset for a missing, unparseable, or unmodeled key.

use pictura_core::{
    CameraCalibration, CrsSettings, Detail, Effects, Hsl, Lens, PicturaRawSettings, RawMode,
    SplitToning, ToneCurve, HSL_RANGES,
};

use crate::descriptor::{get_object_item, DescValue};

/// Decode the modeled `Fltr` controls from a filter options buffer.
pub fn decode_pictura_raw_settings(fltr_options: &[u8]) -> PicturaRawSettings {
    let Ok(DescValue::Object { items, .. }) = crate::camera_raw_options(fltr_options) else {
        return PicturaRawSettings::default();
    };
    PicturaRawSettings {
        basic: decode_basic(&items),
        legacy_blacks: integer(&items, b"BlkB"),
        mode: decode_mode(&items),
        tone_curve: decode_tone_curve(&items),
        detail: decode_detail(&items),
        hsl: decode_hsl(&items),
        split_toning: decode_split_toning(&items),
        lens: decode_lens(&items),
        effects: decode_effects(&items),
        calibration: decode_calibration(&items),
    }
}

/// Encode the modeled controls as a `Fltr` descriptor object. Only set controls
/// are written; an all-unset settings value yields a keyless object.
pub fn encode_pictura_raw_fltr(settings: &PicturaRawSettings) -> DescValue {
    let mut items: Vec<(Vec<u8>, DescValue)> = Vec::new();

    let b = &settings.basic;
    push_rounded(&mut items, b"Temp", b.temperature);
    push_rounded(&mut items, b"Tint", b.tint);
    push_double(&mut items, b"Ex12", b.exposure);
    push_rounded(&mut items, b"Cr12", b.contrast);
    push_rounded(&mut items, b"Hi12", b.highlights);
    push_rounded(&mut items, b"Sh12", b.shadows);
    push_rounded(&mut items, b"Wh12", b.whites);
    push_rounded(&mut items, b"Bk12", b.blacks);
    push_rounded(&mut items, b"Cl12", b.clarity);
    push_rounded(&mut items, b"Vibr", b.vibrance);
    push_rounded(&mut items, b"Strt", b.saturation);
    push_int(&mut items, b"BlkB", settings.legacy_blacks);

    let m = &settings.mode;
    push_text(&mut items, b"CMod", m.module.as_deref());
    push_enum(&mut items, b"WBal", m.white_balance);
    push_int(&mut items, b"AWBV", m.auto_wb_version);
    push_enum(&mut items, b"Sett", m.settings_mode);
    push_int(&mut items, b"PrVe", m.process_version);
    push_bool(&mut items, b"CtoG", m.grayscale);

    let t = &settings.tone_curve;
    push_int(&mut items, b"PC_S", t.parametric_shadows);
    push_int(&mut items, b"PC_D", t.parametric_darks);
    push_int(&mut items, b"PC_L", t.parametric_lights);
    push_int(&mut items, b"PC_H", t.parametric_highlights);
    push_int(&mut items, b"PC_1", t.shadow_split);
    push_int(&mut items, b"PC_2", t.midtone_split);
    push_int(&mut items, b"PC_3", t.highlight_split);
    push_curve(&mut items, b"Crv ", &t.curve_rgb);
    push_curve(&mut items, b"CrvR", &t.curve_red);
    push_curve(&mut items, b"CrvG", &t.curve_green);
    push_curve(&mut items, b"CrvB", &t.curve_blue);

    let d = &settings.detail;
    push_int(&mut items, b"Shrp", d.sharpen_amount);
    push_double(&mut items, b"ShpR", d.sharpen_radius);
    push_int(&mut items, b"ShpD", d.sharpen_detail);
    push_int(&mut items, b"ShpM", d.sharpen_masking);
    push_int(&mut items, b"LNR ", d.noise_luminance);
    push_int(&mut items, b"LNRD", d.noise_luminance_detail);
    push_int(&mut items, b"LNRC", d.noise_luminance_contrast);
    push_int(&mut items, b"CNR ", d.noise_color);
    push_int(&mut items, b"CNRD", d.noise_color_detail);
    push_int(&mut items, b"CNRS", d.noise_color_smoothness);

    let h = &settings.hsl;
    push_range(&mut items, b"HA_", &h.hue);
    push_range(&mut items, b"SA_", &h.saturation);
    push_range(&mut items, b"LA_", &h.luminance);
    push_range(&mut items, b"GM_", &h.grayscale_mix);
    push_int(&mut items, b"RHue", h.legacy_red_hue);
    push_int(&mut items, b"RSat", h.legacy_red_saturation);
    push_int(&mut items, b"GHue", h.legacy_green_hue);
    push_int(&mut items, b"GSat", h.legacy_green_saturation);
    push_int(&mut items, b"BHue", h.legacy_blue_hue);
    push_int(&mut items, b"BSat", h.legacy_blue_saturation);

    let s = &settings.split_toning;
    push_int(&mut items, b"STSH", s.shadow_hue);
    push_int(&mut items, b"STSS", s.shadow_saturation);
    push_int(&mut items, b"STHH", s.highlight_hue);
    push_int(&mut items, b"STHS", s.highlight_saturation);
    push_int(&mut items, b"STB ", s.balance);

    let l = &settings.lens;
    push_int(&mut items, b"LPEn", l.profile_enabled);
    push_int(&mut items, b"MDis", l.distortion);
    push_int(&mut items, b"VigA", l.vignette_amount);
    push_int(&mut items, b"VigM", l.vignette_midpoint);
    push_int(&mut items, b"PerV", l.perspective_vertical);
    push_int(&mut items, b"PerH", l.perspective_horizontal);
    push_int(&mut items, b"PerS", l.perspective_scale);
    push_int(&mut items, b"PerA", l.perspective_aspect);
    push_int(&mut items, b"PerU", l.perspective_upright);
    push_double(&mut items, b"PerR", l.rotate);
    push_double(&mut items, b"PerX", l.offset_x);
    push_double(&mut items, b"PerY", l.offset_y);
    push_int(&mut items, b"DfPA", l.defringe_purple_amount);
    push_int(&mut items, b"DPHL", l.defringe_purple_hue_lo);
    push_int(&mut items, b"DPPH", l.defringe_purple_hue_hi);
    push_int(&mut items, b"DfGA", l.defringe_green_amount);
    push_int(&mut items, b"DPGL", l.defringe_green_hue_lo);
    push_int(&mut items, b"DPGH", l.defringe_green_hue_hi);

    let e = &settings.effects;
    push_int(&mut items, b"GRNA", e.grain_amount);
    push_int(&mut items, b"GRNS", e.grain_size);
    push_int(&mut items, b"GRNF", e.grain_frequency);
    push_int(&mut items, b"PCVA", e.vignette_amount);
    push_int(&mut items, b"PCVM", e.vignette_midpoint);
    push_int(&mut items, b"PCVF", e.vignette_feather);
    push_int(&mut items, b"PCVR", e.vignette_roundness);
    push_int(&mut items, b"PCVS", e.vignette_style);
    push_int(&mut items, b"PCVH", e.vignette_highlights);

    let c = &settings.calibration;
    push_text(&mut items, b"CamP", c.camera_profile.as_deref());
    push_text(&mut items, b"CP_D", c.profile_digest.as_deref());

    DescValue::Object {
        name: String::new(),
        class_id: b"Adobe Camera Raw Filter".to_vec(),
        items,
    }
}

fn decode_basic(items: &[(Vec<u8>, DescValue)]) -> CrsSettings {
    CrsSettings {
        temperature: number(items, b"Temp"),
        tint: number(items, b"Tint"),
        exposure: number(items, b"Ex12"),
        contrast: number(items, b"Cr12"),
        highlights: number(items, b"Hi12"),
        shadows: number(items, b"Sh12"),
        whites: number(items, b"Wh12"),
        blacks: number(items, b"Bk12"),
        clarity: number(items, b"Cl12"),
        vibrance: number(items, b"Vibr"),
        saturation: number(items, b"Strt"),
    }
}

fn decode_mode(items: &[(Vec<u8>, DescValue)]) -> RawMode {
    RawMode {
        module: text(items, b"CMod"),
        white_balance: enum_code(items, b"WBal"),
        auto_wb_version: integer(items, b"AWBV"),
        settings_mode: enum_code(items, b"Sett"),
        process_version: integer(items, b"PrVe"),
        grayscale: boolean(items, b"CtoG"),
    }
}

fn decode_tone_curve(items: &[(Vec<u8>, DescValue)]) -> ToneCurve {
    ToneCurve {
        parametric_shadows: integer(items, b"PC_S"),
        parametric_darks: integer(items, b"PC_D"),
        parametric_lights: integer(items, b"PC_L"),
        parametric_highlights: integer(items, b"PC_H"),
        shadow_split: integer(items, b"PC_1"),
        midtone_split: integer(items, b"PC_2"),
        highlight_split: integer(items, b"PC_3"),
        curve_rgb: curve(items, b"Crv "),
        curve_red: curve(items, b"CrvR"),
        curve_green: curve(items, b"CrvG"),
        curve_blue: curve(items, b"CrvB"),
    }
}

fn decode_detail(items: &[(Vec<u8>, DescValue)]) -> Detail {
    Detail {
        sharpen_amount: integer(items, b"Shrp"),
        sharpen_radius: number(items, b"ShpR"),
        sharpen_detail: integer(items, b"ShpD"),
        sharpen_masking: integer(items, b"ShpM"),
        noise_luminance: integer(items, b"LNR "),
        noise_luminance_detail: integer(items, b"LNRD"),
        noise_luminance_contrast: integer(items, b"LNRC"),
        noise_color: integer(items, b"CNR "),
        noise_color_detail: integer(items, b"CNRD"),
        noise_color_smoothness: integer(items, b"CNRS"),
    }
}

fn decode_hsl(items: &[(Vec<u8>, DescValue)]) -> Hsl {
    Hsl {
        hue: range(items, b"HA_"),
        saturation: range(items, b"SA_"),
        luminance: range(items, b"LA_"),
        legacy_red_hue: integer(items, b"RHue"),
        legacy_red_saturation: integer(items, b"RSat"),
        legacy_green_hue: integer(items, b"GHue"),
        legacy_green_saturation: integer(items, b"GSat"),
        legacy_blue_hue: integer(items, b"BHue"),
        legacy_blue_saturation: integer(items, b"BSat"),
        grayscale_mix: range(items, b"GM_"),
    }
}

fn decode_split_toning(items: &[(Vec<u8>, DescValue)]) -> SplitToning {
    SplitToning {
        shadow_hue: integer(items, b"STSH"),
        shadow_saturation: integer(items, b"STSS"),
        highlight_hue: integer(items, b"STHH"),
        highlight_saturation: integer(items, b"STHS"),
        balance: integer(items, b"STB "),
    }
}

fn decode_lens(items: &[(Vec<u8>, DescValue)]) -> Lens {
    Lens {
        profile_enabled: integer(items, b"LPEn"),
        distortion: integer(items, b"MDis"),
        vignette_amount: integer(items, b"VigA"),
        vignette_midpoint: integer(items, b"VigM"),
        perspective_vertical: integer(items, b"PerV"),
        perspective_horizontal: integer(items, b"PerH"),
        perspective_scale: integer(items, b"PerS"),
        perspective_aspect: integer(items, b"PerA"),
        perspective_upright: integer(items, b"PerU"),
        rotate: number(items, b"PerR"),
        offset_x: number(items, b"PerX"),
        offset_y: number(items, b"PerY"),
        defringe_purple_amount: integer(items, b"DfPA"),
        defringe_purple_hue_lo: integer(items, b"DPHL"),
        defringe_purple_hue_hi: integer(items, b"DPPH"),
        defringe_green_amount: integer(items, b"DfGA"),
        defringe_green_hue_lo: integer(items, b"DPGL"),
        defringe_green_hue_hi: integer(items, b"DPGH"),
    }
}

fn decode_effects(items: &[(Vec<u8>, DescValue)]) -> Effects {
    Effects {
        grain_amount: integer(items, b"GRNA"),
        grain_size: integer(items, b"GRNS"),
        grain_frequency: integer(items, b"GRNF"),
        vignette_amount: integer(items, b"PCVA"),
        vignette_midpoint: integer(items, b"PCVM"),
        vignette_feather: integer(items, b"PCVF"),
        vignette_roundness: integer(items, b"PCVR"),
        vignette_style: integer(items, b"PCVS"),
        vignette_highlights: integer(items, b"PCVH"),
    }
}

fn decode_calibration(items: &[(Vec<u8>, DescValue)]) -> CameraCalibration {
    CameraCalibration {
        camera_profile: text(items, b"CamP"),
        profile_digest: text(items, b"CP_D"),
    }
}

/// A numeric key as `f64`, accepting `Long` or a finite `Double`.
fn number(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Option<f64> {
    match get_object_item(items, key)? {
        DescValue::Double(v) if v.is_finite() => Some(*v),
        DescValue::Long(v) => Some(*v as f64),
        _ => None,
    }
}

/// An integer key, accepting `Long` or a finite `Double` (rounded).
fn integer(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Option<i32> {
    match get_object_item(items, key)? {
        DescValue::Long(v) => Some(*v),
        DescValue::Double(v) if v.is_finite() => Some(v.round() as i32),
        _ => None,
    }
}

fn boolean(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Option<bool> {
    match get_object_item(items, key)? {
        DescValue::Bool(v) => Some(*v),
        _ => None,
    }
}

fn text(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Option<String> {
    match get_object_item(items, key)? {
        DescValue::Text(v) => Some(v.clone()),
        _ => None,
    }
}

fn enum_code(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Option<[u8; 4]> {
    match get_object_item(items, key)? {
        DescValue::Enum { value, .. } if value.len() == 4 => {
            let mut code = [0u8; 4];
            code.copy_from_slice(value);
            Some(code)
        }
        _ => None,
    }
}

/// A point-curve list as flattened input/output pairs; empty when absent.
fn curve(items: &[(Vec<u8>, DescValue)], key: &[u8]) -> Vec<i32> {
    match get_object_item(items, key) {
        Some(DescValue::List(list)) => list
            .iter()
            .filter_map(|value| match value {
                DescValue::Long(v) => Some(*v),
                DescValue::Double(v) if v.is_finite() => Some(v.round() as i32),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The eight ranged values for a `XX_` prefix (`HA_`, `SA_`, `LA_`, `GM_`).
fn range(items: &[(Vec<u8>, DescValue)], prefix: &[u8; 3]) -> [Option<i32>; 8] {
    let mut out = [None; 8];
    for (i, code) in HSL_RANGES.iter().enumerate() {
        let key = [prefix[0], prefix[1], prefix[2], *code];
        out[i] = integer(items, &key);
    }
    out
}

fn push_int(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<i32>) {
    if let Some(v) = value {
        items.push((key.to_vec(), DescValue::Long(v)));
    }
}

/// Push an `f64` Basic control rounded to the integer type the reference uses.
fn push_rounded(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<f64>) {
    if let Some(v) = value.filter(|v| v.is_finite()) {
        items.push((key.to_vec(), DescValue::Long(v.round() as i32)));
    }
}

fn push_double(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<f64>) {
    if let Some(v) = value.filter(|v| v.is_finite()) {
        items.push((key.to_vec(), DescValue::Double(v)));
    }
}

fn push_bool(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<bool>) {
    if let Some(v) = value {
        items.push((key.to_vec(), DescValue::Bool(v)));
    }
}

/// Push a text control verbatim; an empty string still writes the key so an
/// empty-but-present value round-trips.
fn push_text(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<&str>) {
    if let Some(v) = value {
        items.push((key.to_vec(), DescValue::Text(v.to_string())));
    }
}

fn push_enum(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: Option<[u8; 4]>) {
    if let Some(code) = value {
        items.push((
            key.to_vec(),
            DescValue::Enum {
                kind: key.to_vec(),
                value: code.to_vec(),
            },
        ));
    }
}

fn push_curve(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: &[i32]) {
    if !value.is_empty() {
        items.push((
            key.to_vec(),
            DescValue::List(value.iter().map(|v| DescValue::Long(*v)).collect()),
        ));
    }
}

fn push_range(items: &mut Vec<(Vec<u8>, DescValue)>, prefix: &[u8; 3], values: &[Option<i32>; 8]) {
    for (i, code) in HSL_RANGES.iter().enumerate() {
        if let Some(v) = values[i] {
            let key = [prefix[0], prefix[1], prefix[2], *code];
            items.push((key.to_vec(), DescValue::Long(v)));
        }
    }
}
