use crate::layer_content_budget::ContentAlloc;
use crate::DescValue;
use pictura_core::{
    Adjustment, BlackWhiteParams, BrightnessContrastParams, ExposureParams, HueRange,
    HueSaturationParams, LevelsChannel, LevelsParams, PhotoFilterParams, ShadowsHighlightsOptions,
    ShadowsHighlightsParams, VibranceParams,
};
use pictura_core::{AdjustmentData, FillContent};

/// `SoCo`: the standard PSD solid-color fill descriptor. A version-16
/// `DescriptorBlock` whose `Clr ` object carries `Rd  `/`Grn `/`Bl  ` `doub`
/// values on the `0..=255` scale; each is rounded and clamped, and alpha is
/// forced to 255 (the descriptor has none). Missing key, wrong type,
/// non-finite value, or a parse error is `None`, never a panic.
pub fn decode_solid_fill(d: &[u8]) -> Option<FillContent> {
    let obj = crate::read_descriptor(d).ok()?;
    solid_fill_from_desc(&obj)
}

pub fn solid_fill_from_desc(obj: &DescValue) -> Option<FillContent> {
    let clr = desc_item(obj, b"Clr ")?;
    if !matches!(clr, DescValue::Object { .. }) {
        return None;
    }
    let component = |key: &[u8]| -> Option<u8> {
        match desc_item(clr, key) {
            Some(DescValue::Double(c)) if c.is_finite() => Some(c.round().clamp(0.0, 255.0) as u8),
            _ => None,
        }
    };
    Some(FillContent::Solid([
        component(b"Rd  ")?,
        component(b"Grn ")?,
        component(b"Bl  ")?,
        255,
    ]))
}

pub fn be_u16(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at + 2)?;
    Some(u16::from_be_bytes([s[0], s[1]]))
}

pub fn be_i16(d: &[u8], at: usize) -> Option<i16> {
    Some(be_u16(d, at)? as i16)
}

fn be_f32(d: &[u8], at: usize) -> Option<f32> {
    let s = d.get(at..at + 4)?;
    Some(f32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn be_u32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// `brit`: brightness (i16), contrast (i16), mean (i16), lab_only (u8), pad.
///
/// The reference's `brit` block carries no explicit "Use Legacy" flag; CS6 defaults
/// new adjustment layers to the modern curve, so decode with `use_legacy:false`.
/// ponytail: legacy-vs-modern detection is not encoded here; revisit if a CS6
/// baseline for legacy `brit` files appears.
pub fn decode_brightness_contrast(d: &[u8]) -> Option<Adjustment> {
    let brightness = be_i16(d, 0)?;
    let contrast = be_i16(d, 2)?;
    if !(-150..=150).contains(&brightness) || !(-50..=100).contains(&contrast) {
        return None;
    }
    Some(Adjustment::BrightnessContrast(BrightnessContrastParams {
        brightness,
        contrast,
        use_legacy: false,
    }))
}

/// `levl`: `u16 version` (2) then 29 five-`u16` records (input black, input
/// white, output black, output white, gamma × 100). Record 0 is the composite;
/// records 1–3 are red, green, and blue, kept when they differ from identity.
/// The rest (CMYK's fourth plate and the unused tail) are ignored.
pub fn decode_levels(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 2 {
        return None;
    }
    let composite = levels_record(d, 0)?;
    // A short block (some writers stop after the composite) or a zero-filled
    // record has no channel adjustment.
    let channel = |k: usize| {
        let record = d.get(2 + 10 * k..2 + 10 * (k + 1));
        if record.is_none_or(|bytes| bytes.iter().all(|&b| b == 0)) {
            return Some(None);
        }
        let record = levels_record(d, k)?;
        Some((record != LEVELS_IDENTITY).then_some(record))
    };
    Some(Adjustment::Levels(LevelsParams {
        input_black: composite.input_black,
        input_white: composite.input_white,
        gamma: composite.gamma,
        output_black: composite.output_black,
        output_white: composite.output_white,
        red: channel(1)?,
        green: channel(2)?,
        blue: channel(3)?,
    }))
}

const LEVELS_IDENTITY: LevelsChannel = LevelsChannel {
    input_black: 0,
    input_white: 255,
    gamma: 1.0,
    output_black: 0,
    output_white: 255,
};

/// `levl` record `k`, or `None` when it is out of range or invalid.
fn levels_record(d: &[u8], k: usize) -> Option<LevelsChannel> {
    let at = 2 + 10 * k;
    let input_black = be_u16(d, at)?;
    let input_white = be_u16(d, at + 2)?;
    let output_black = be_u16(d, at + 4)?;
    let output_white = be_u16(d, at + 6)?;
    let gamma = be_u16(d, at + 8)?;
    if input_black >= input_white || gamma == 0 {
        return None;
    }
    if [input_black, input_white, output_black, output_white]
        .into_iter()
        .any(|v| v > 255)
    {
        return None;
    }
    Some(LevelsChannel {
        input_black: input_black as u8,
        input_white: input_white as u8,
        gamma: gamma as f64 / 100.0,
        output_black: output_black as u8,
        output_white: output_white as u8,
    })
}

/// `hue2` (and legacy `hue `): version `u16`, enable `u8`, pad, colorization
/// (3×i16), the master Hue/Saturation/Lightness triplet (3×i16), then six
/// 14-byte colour-range records (Reds … Magentas): the band's four hue points
/// (4×i16) and its Hue/Saturation/Lightness (3×i16). Only ranges carrying
/// an edit are kept; a band stored as all zeros reads as CS6's default band.
pub fn decode_hue_saturation(d: &[u8]) -> Option<Adjustment> {
    decode_hue_saturation_using(d, &mut ContentAlloc::new(None))
}

pub(crate) fn decode_hue_saturation_using(
    d: &[u8],
    alloc: &mut ContentAlloc,
) -> Option<Adjustment> {
    if be_u16(d, 0)? != 2 {
        return None;
    }
    let hue = be_i16(d, 10)?;
    let saturation = be_i16(d, 12)?;
    let lightness = be_i16(d, 14)?;
    if !(-180..=180).contains(&hue)
        || !(-100..=100).contains(&saturation)
        || !(-100..=100).contains(&lightness)
    {
        return None;
    }
    let mut ranges = Vec::new();
    for (k, default) in HUE_RANGE_BANDS.iter().enumerate() {
        let at = 16 + 14 * k;
        // An older or truncated block stops after Master.
        let Some(record) = d.get(at..at + 14) else {
            break;
        };
        let v = |i: usize| i16::from_be_bytes([record[2 * i], record[2 * i + 1]]);
        let (hue, saturation, lightness) = (v(4), v(5), v(6));
        if (hue, saturation, lightness) == (0, 0, 0) {
            continue;
        }
        if !(-180..=180).contains(&hue)
            || !(-100..=100).contains(&saturation)
            || !(-100..=100).contains(&lightness)
        {
            return None;
        }
        let band = if record[..8].iter().all(|&b| b == 0) {
            *default
        } else {
            [v(0), v(1), v(2), v(3)]
        };
        alloc.push(
            &mut ranges,
            HueRange {
                begin_ramp: band[0],
                begin_sustain: band[1],
                end_sustain: band[2],
                end_ramp: band[3],
                hue,
                saturation,
                lightness,
            },
            "hue adjustment ranges",
        )?;
    }
    Some(Adjustment::HueSaturation(HueSaturationParams {
        hue,
        saturation,
        lightness,
        ranges,
    }))
}

/// CS6's default colour-range bands, Reds through Magentas, as (begin ramp,
/// begin sustain, end sustain, end ramp) in degrees.
pub(crate) const HUE_RANGE_BANDS: [[i16; 4]; 6] = [
    [315, 345, 15, 45],
    [15, 45, 75, 105],
    [75, 105, 135, 165],
    [135, 165, 195, 225],
    [195, 225, 255, 285],
    [255, 285, 315, 345],
];

/// `expA`: `u16` version (= 1), then big-endian `f32` exposure, offset, and
/// gamma.
pub fn decode_exposure(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 1 {
        return None;
    }
    let exposure = be_f32(d, 2)? as f64;
    let offset = be_f32(d, 6)? as f64;
    let gamma = be_f32(d, 10)? as f64;
    if !exposure.is_finite() || !offset.is_finite() || !gamma.is_finite() || gamma <= 0.0 {
        return None;
    }
    Some(Adjustment::Exposure(ExposureParams {
        exposure,
        offset,
        gamma,
    }))
}

/// `phfl` version 2: `u16` version, `u16` colour space (ignored), four `u16`
/// colour components (first three = R, G, B; fourth ignored), `u32` density,
/// `u8` luminosity. A colour space other than RGB is decoded as RGB.
/// Version 3: three `be_u32` CIE XYZ values (offsets 2/6/10), `u32` density
/// (14), `u8` luminosity (18).
///
/// ponytail: the version-2 block length is flexible — only the 17 field bytes
/// are read, so an unpadded 17–19 byte payload still decodes while 20-byte
/// writers round-trip.
pub fn decode_photo_filter(d: &[u8]) -> Option<Adjustment> {
    let (color, density, preserve_luminosity) = match be_u16(d, 0)? {
        2 => {
            let component = |at: usize| -> Option<u8> {
                let v = be_u16(d, at)?;
                (v <= 255).then_some(v as u8)
            };
            (
                [component(4)?, component(6)?, component(8)?],
                be_u32(d, 12)?,
                *d.get(16)? != 0,
            )
        }
        3 => (
            xyz_d50_to_srgb_u8([be_u32(d, 2)?, be_u32(d, 6)?, be_u32(d, 10)?]),
            be_u32(d, 14)?,
            *d.get(18)? != 0,
        ),
        _ => return None,
    };
    if density > 100 {
        return None;
    }
    Some(Adjustment::PhotoFilter(PhotoFilterParams {
        color,
        density: density as f64,
        preserve_luminosity,
    }))
}

// ponytail: 16.16 scale and D50 white are unproven without a CS6 v3 fixture;
// layout grounded on psd-tools. If a real file disagrees, only the scale
// constant / white adaptation changes, not the field layout.
fn xyz_d50_to_srgb_u8(xyz: [u32; 3]) -> [u8; 3] {
    let scale = |v: u32| v as f64 / 65536.0;
    crate::xyz_d50_to_srgb_u8([scale(xyz[0]), scale(xyz[1]), scale(xyz[2])])
}

/// `vibA`: descriptor block with `vibrance` and the legacy `Strt` (saturation)
/// integer keys. Missing keys are 0; wrong types and out-of-range values are
/// rejected so a corrupt file cannot silently render a different adjustment.
pub fn decode_vibrance(d: &[u8]) -> Option<Adjustment> {
    let obj = crate::read_descriptor(d).ok()?;
    vibrance_from_desc(&obj)
}

pub fn vibrance_from_desc(obj: &DescValue) -> Option<Adjustment> {
    let vibrance = desc_long_or(obj, b"vibrance", 0.0)?;
    let saturation = desc_long_or(obj, b"Strt", 0.0)?;
    if !(-100.0..=100.0).contains(&vibrance) || !(-100.0..=100.0).contains(&saturation) {
        return None;
    }
    Some(Adjustment::Vibrance(VibranceParams {
        vibrance: vibrance as i16,
        saturation: saturation as i16,
    }))
}

/// `shdH`: two big-endian `u16` amounts, Shadows then Highlights, each
/// `0..=100`. An out-of-range amount is rejected so a corrupt file cannot render
/// a different adjustment.
pub fn decode_shadows_highlights(d: &[u8]) -> Option<Adjustment> {
    let shadows = be_u16(d, 0)?;
    let highlights = be_u16(d, 2)?;
    if shadows > 100 || highlights > 100 {
        return None;
    }
    Some(Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
        shadows_amount: shadows as f64,
        highlights_amount: highlights as f64,
        local: None,
    }))
}

/// `shd2`: the amounts and version followed by fixed-width local settings.
pub fn decode_shadows_highlights_local(d: &[u8]) -> Option<Adjustment> {
    if d.len() != 24 || be_u16(d, 4)? != 1 {
        return None;
    }
    let unsigned = |offset| be_u16(d, offset).map(f64::from);
    let signed = |offset| be_u16(d, offset).map(|v| f64::from(v as i16));
    let params = ShadowsHighlightsParams {
        shadows_amount: unsigned(0)?,
        highlights_amount: unsigned(2)?,
        local: Some(ShadowsHighlightsOptions {
            shadows_width: unsigned(6)?,
            shadows_radius: unsigned(8)?,
            highlights_width: unsigned(10)?,
            highlights_radius: unsigned(12)?,
            brightness: signed(14)?,
            color_correction: signed(16)?,
            midtone_contrast: signed(18)?,
            black_clip: unsigned(20)? / 100.0,
            white_clip: unsigned(22)? / 100.0,
        }),
    };
    params
        .is_valid()
        .then_some(Adjustment::ShadowsHighlights(params))
}

/// `blwh`: descriptor block with the six channel-percentage longs, a `useTint`
/// bool, and a nested `Clr ` `tintColor` object whose `Rd  `/`Grn `/`Bl  `
/// doubles are 0..1. Missing numeric keys default to the reference's Black & White
/// defaults; absent `tintColor` is black.
pub fn decode_black_white(d: &[u8]) -> Option<Adjustment> {
    let obj = crate::read_descriptor(d).ok()?;
    black_white_from_desc(&obj)
}

pub fn black_white_from_desc(obj: &DescValue) -> Option<Adjustment> {
    let red = desc_long_or(obj, b"Rd  ", 40.0)?;
    let yellow = desc_long_or(obj, b"Yllw", 60.0)?;
    let green = desc_long_or(obj, b"Grn ", 40.0)?;
    let cyan = desc_long_or(obj, b"Cyn ", 60.0)?;
    let blue = desc_long_or(obj, b"Bl  ", 20.0)?;
    let magenta = desc_long_or(obj, b"Mgnt", 80.0)?;
    if [red, yellow, green, cyan, blue, magenta]
        .into_iter()
        .any(|v| !(-200.0..=300.0).contains(&v))
    {
        return None;
    }
    let tint = match desc_item(obj, b"useTint") {
        Some(DescValue::Bool(b)) => *b,
        Some(_) => return None,
        None => false,
    };
    let tint_color = match desc_item(obj, b"tintColor") {
        Some(value) => decode_tint_color(value)?,
        None => [0, 0, 0],
    };
    Some(Adjustment::BlackWhite(BlackWhiteParams {
        red,
        yellow,
        green,
        cyan,
        blue,
        magenta,
        tint,
        tint_color,
    }))
}

pub fn decode_tint_color(value: &DescValue) -> Option<[u8; 3]> {
    if !matches!(value, DescValue::Object { .. }) {
        return None;
    }
    let component = |key: &[u8]| -> Option<u8> {
        match desc_item(value, key) {
            None => Some(0),
            Some(DescValue::Double(c)) => Some((c * 255.0).round().clamp(0.0, 255.0) as u8),
            Some(_) => None,
        }
    };
    Some([
        component(b"Rd  ")?,
        component(b"Grn ")?,
        component(b"Bl  ")?,
    ])
}

pub fn desc_item<'a>(obj: &'a DescValue, key: &[u8]) -> Option<&'a DescValue> {
    let DescValue::Object { items, .. } = obj else {
        return None;
    };
    items
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v)
}

/// Read a `long` item as `f64`; `None` when the key is present but not a long.
fn desc_long_or(obj: &DescValue, key: &[u8], default: f64) -> Option<f64> {
    match desc_item(obj, key) {
        None => Some(default),
        Some(DescValue::Long(n)) => Some(*n as f64),
        Some(_) => None,
    }
}

// --- Encoders for the same subset ------------------------------------------
//
// These build the raw `AdjustmentData` the decoder above reads, so the app can
// create adjustment layers in memory. Byte layouts mirror psd-tools' adjustment
// structs (the independent oracle the codec fixtures come from); each is the
// minimal block for the key, not a full re-implementation of the reference's writer.

/// `nvrt`: Invert carries no payload.
pub fn encode_invert() -> AdjustmentData {
    AdjustmentData {
        key: *b"nvrt",
        data: Vec::new().into(),
    }
}

/// `post`: a `u16` levels value (2..=255) plus 2 pad bytes (psd-tools
/// `ShortIntegerElement`, `H2x`). Out-of-range input is clamped.
pub fn encode_posterize(levels: u8) -> AdjustmentData {
    encode_short(*b"post", levels.clamp(2, 255) as u16)
}

/// `thrs`: a `u16` level (1..=255) plus 2 pad bytes. Out-of-range input is clamped.
pub fn encode_threshold(level: u8) -> AdjustmentData {
    encode_short(*b"thrs", level.clamp(1, 255) as u16)
}

/// `brit`: brightness (i16), contrast (i16), mean (i16), lab_only (u8), pad
/// (psd-tools `3HBx`). Inputs are clamped to the decoder's accepted ranges
/// (brightness -150..=150, contrast -50..=100); `use_legacy` is not representable
/// and the decoder always uses the modern curve.
pub fn encode_brightness_contrast(brightness: i16, contrast: i16) -> AdjustmentData {
    let b = brightness.clamp(-150, 150);
    let c = contrast.clamp(-50, 100);
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&b.to_be_bytes());
    data.extend_from_slice(&c.to_be_bytes());
    data.extend_from_slice(&0i16.to_be_bytes());
    data.push(0);
    data.push(0);
    AdjustmentData {
        key: *b"brit",
        data: data.into(),
    }
}

/// `hue2`: version (2), enable (1), pad, colorization (3×i16), then the master
/// Hue/Saturation/Lightness triplet (3×i16), followed by the six per-band range
/// records (6 × 7 i16) that the reference stores. The decoder only reads the version
/// and the master triplet; the trailing records are zeroed so the block matches
/// the real 100-byte layout.
pub fn encode_hue_saturation(hue: i16, saturation: i16, lightness: i16) -> AdjustmentData {
    let hue = hue.clamp(-180, 180);
    let saturation = saturation.clamp(-100, 100);
    let lightness = lightness.clamp(-100, 100);
    let mut data = Vec::with_capacity(100);
    data.extend_from_slice(&2u16.to_be_bytes());
    data.push(1);
    data.push(0);
    data.extend_from_slice(&[0u8; 6]);
    data.extend_from_slice(&hue.to_be_bytes());
    data.extend_from_slice(&saturation.to_be_bytes());
    data.extend_from_slice(&lightness.to_be_bytes());
    // Each colour range: its default band, no edit.
    for band in HUE_RANGE_BANDS {
        for v in band.into_iter().chain([0; 3]) {
            data.extend_from_slice(&v.to_be_bytes());
        }
    }
    AdjustmentData {
        key: *b"hue2",
        data: data.into(),
    }
}

/// `phfl`: the version-2 fixed struct the decoder reads. `density` clamps to
/// `0..=100`; the colour space and fourth component are zero, and three pad
/// bytes round the block to 20.
pub fn encode_photo_filter(
    color: [u8; 3],
    density: f64,
    preserve_luminosity: bool,
) -> AdjustmentData {
    let density = density.clamp(0.0, 100.0).round() as u32;
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&2u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());
    for c in color {
        data.extend_from_slice(&(c as u16).to_be_bytes());
    }
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&density.to_be_bytes());
    data.push(u8::from(preserve_luminosity));
    data.extend_from_slice(&[0u8; 3]);
    AdjustmentData {
        key: *b"phfl",
        data: data.into(),
    }
}

/// `SoCo`: the standard PSD solid-color fill descriptor. `Clr ` is an
/// `RGBC` object carrying the three components as `doub` values on the
/// `0..=255` scale. The descriptor has no alpha, so fills authored through
/// this encoder are opaque; use `encode_fill_content` when alpha must survive.
pub fn encode_solid_color_fill(color: [u8; 3]) -> AdjustmentData {
    let clr = DescValue::Object {
        name: String::new(),
        class_id: b"RGBC".to_vec(),
        items: [b"Rd  ", b"Grn ", b"Bl  "]
            .into_iter()
            .zip(color)
            .map(|(key, c)| (key.to_vec(), DescValue::Double(c as f64)))
            .collect(),
    };
    let desc = DescValue::Object {
        name: String::new(),
        class_id: b"SoCo".to_vec(),
        items: vec![(b"Clr ".to_vec(), clr)],
    };
    AdjustmentData {
        key: *b"SoCo",
        data: crate::write_descriptor(&desc).into(),
    }
}

/// `H2x` payload: a big-endian `u16` value plus two zero pad bytes.
fn encode_short(key: [u8; 4], value: u16) -> AdjustmentData {
    let mut data = value.to_be_bytes().to_vec();
    data.extend_from_slice(&[0, 0]);
    AdjustmentData {
        key,
        data: data.into(),
    }
}
