//! The Properties panel's view of an adjustment layer: per-kind parameter
//! descriptors (key, label, range, default, group) and in-place edits. Ported
//! from photorust's `PropertiesPanel` descriptor design (`addSlider` /
//! `Adjustment::value` / `set_value`).
//!
//! An edit patches only the bytes (or the descriptor item) that hold the
//! parameter, then requires the block to still decode, so whatever the decoder
//! does not model — a Hue/Saturation layer's Colorize and per-range settings,
//! Brightness/Contrast's legacy flag, Black & White's tint — survives the edit
//! and a later save. Curves alone is re-encoded from its decoded points.
//!
//! ponytail: Gradient Map and fill layers have no controls; Channel Mixer's
//! Monochrome, Hue/Saturation's Colorize and ranges, and Black & White's tint
//! are not editable here. Color Lookup edits only its preset (the embedded
//! `.CUBE` is rebuilt, not patched).

use pictura_adjust::Adjustment;
use pictura_codec::DescValue;
use pictura_core::AdjustmentData;

use crate::composite::desc_item;
use crate::decode_adjustment;

/// What a control looks like.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamKind {
    Slider {
        min: f64,
        max: f64,
        decimals: u8,
    },
    Check,
    /// A menu whose index is the value.
    Choice(&'static [&'static str]),
    /// An RGB colour as `0xRRGGBB`.
    Color,
}

/// One control of an adjustment's Properties page.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustmentParam {
    pub key: String,
    pub label: String,
    pub kind: ParamKind,
    /// Index into [`AdjustmentEditor::groups`]; `None` shows in every group.
    pub group: Option<usize>,
    pub value: f64,
    /// What the panel's Reset restores.
    pub default: f64,
}

/// An adjustment layer's Properties page.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustmentEditor {
    /// The adjustment's CS6 name ("Brightness/Contrast"), also used for the
    /// "Modify … Layer" history state.
    pub title: &'static str,
    /// A menu that switches which group's controls show (Color Balance's
    /// tones, Channel Mixer's output channel, Selective Color's colours).
    pub groups: Vec<&'static str>,
    pub params: Vec<AdjustmentParam>,
    /// The page carries the Curves editor (composite, red, green, blue).
    pub curves: bool,
    /// A line of explanation where there is nothing (or not everything) to edit.
    pub note: Option<&'static str>,
}

/// Where a parameter lives in its block.
#[derive(Debug, Clone, Copy)]
enum Field {
    I16(usize),
    U16(usize),
    /// A `u16` holding the value times 100 (Levels' gamma).
    U16Hundredths(usize),
    U32(usize),
    F32(usize),
    /// A byte, 0 or 1.
    Flag(usize),
    /// Three `u16` colour components (Photo Filter v2).
    Rgb16(usize),
    /// A descriptor `long` item, written when absent.
    DescLong(&'static [u8]),
    /// A descriptor `bool` item, written when absent.
    DescBool(&'static [u8]),
    /// A descriptor `RGBC` colour object (`Rd  `/`Grn `/`Bl  ` doubles,
    /// 0..1 as the Black & White decoder reads them), written when absent.
    DescColor(&'static [u8]),
    /// A Color Lookup named preset: the index of the block's `Nm  ` name in
    /// [`crate::color_lookup_presets::COLOR_LOOKUP_PRESETS`]. Not written here;
    /// [`set_adjustment_param`] rebuilds the block instead.
    Preset,
}

struct Spec {
    key: String,
    label: String,
    field: Field,
    kind: ParamKind,
    group: Option<usize>,
    default: f64,
}

fn slider(key: &str, label: &str, field: Field, (min, max): (f64, f64), default: f64) -> Spec {
    let decimals = match field {
        Field::U16Hundredths(_) => 2,
        Field::F32(_) => 2,
        _ => 0,
    };
    Spec {
        key: key.into(),
        label: label.into(),
        field,
        kind: ParamKind::Slider { min, max, decimals },
        group: None,
        default,
    }
}

/// `spec` shown with `decimals` places instead of its field's default.
fn with_decimals(spec: Spec, decimals: u8) -> Spec {
    match spec.kind {
        ParamKind::Slider { min, max, .. } => Spec {
            kind: ParamKind::Slider { min, max, decimals },
            ..spec
        },
        _ => spec,
    }
}

fn check(key: &str, label: &str, field: Field, default: bool) -> Spec {
    Spec {
        key: key.into(),
        label: label.into(),
        field,
        kind: ParamKind::Check,
        group: None,
        default: f64::from(u8::from(default)),
    }
}

fn grouped(mut spec: Spec, group: usize) -> Spec {
    spec.group = Some(group);
    spec
}

/// The page layout for a decodable block: title, groups, fields, curves, note.
type Layout = (
    &'static str,
    Vec<&'static str>,
    Vec<Spec>,
    bool,
    Option<&'static str>,
);

fn layout(data: &AdjustmentData, adjustment: &Adjustment) -> Option<Layout> {
    use Field::*;
    let plain = |title, specs| Some((title, Vec::new(), specs, false, None));
    match adjustment {
        Adjustment::BrightnessContrast(_) => plain(
            "Brightness/Contrast",
            vec![
                slider("brightness", "Brightness", I16(0), (-150.0, 150.0), 0.0),
                slider("contrast", "Contrast", I16(2), (-50.0, 100.0), 0.0),
            ],
        ),
        Adjustment::Levels(_) => {
            // `levl` record k (0 composite, 1–3 red/green/blue) starts at
            // 2 + 10k: input black, input white, output black, output white,
            // gamma × 100. The composite keys stay unprefixed.
            let mut specs = Vec::new();
            for (k, prefix) in ["", "red.", "green.", "blue."].into_iter().enumerate() {
                let at = 2 + 10 * k;
                for spec in [
                    slider("inputBlack", "Input Black", U16(at), (0.0, 253.0), 0.0),
                    slider(
                        "inputWhite",
                        "Input White",
                        U16(at + 2),
                        (2.0, 255.0),
                        255.0,
                    ),
                    slider("gamma", "Gamma", U16Hundredths(at + 8), (0.1, 9.99), 1.0),
                    slider(
                        "outputBlack",
                        "Output Black",
                        U16(at + 4),
                        (0.0, 255.0),
                        0.0,
                    ),
                    slider(
                        "outputWhite",
                        "Output White",
                        U16(at + 6),
                        (0.0, 255.0),
                        255.0,
                    ),
                ] {
                    let key = format!("{prefix}{}", spec.key);
                    specs.push(grouped(Spec { key, ..spec }, k));
                }
            }
            Some((
                "Levels",
                vec!["RGB", "Red", "Green", "Blue"],
                specs,
                false,
                None,
            ))
        }
        Adjustment::Exposure(_) => plain(
            "Exposure",
            vec![
                slider("exposure", "Exposure", F32(2), (-20.0, 20.0), 0.0),
                // CS6 shows Offset to four places ("0.0000").
                with_decimals(slider("offset", "Offset", F32(6), (-0.5, 0.5), 0.0), 4),
                slider("gamma", "Gamma Correction", F32(10), (0.01, 9.99), 1.0),
            ],
        ),
        Adjustment::Vibrance(_) => plain(
            "Vibrance",
            vec![
                slider(
                    "vibrance",
                    "Vibrance",
                    DescLong(b"vibrance"),
                    (-100.0, 100.0),
                    0.0,
                ),
                slider(
                    "saturation",
                    "Saturation",
                    DescLong(b"Strt"),
                    (-100.0, 100.0),
                    0.0,
                ),
            ],
        ),
        Adjustment::ShadowsHighlights(_) => plain(
            "Shadows/Highlights",
            vec![
                slider("shadowAmount", "Shadows Amount", U16(0), (0.0, 100.0), 0.0),
                slider(
                    "highlightAmount",
                    "Highlights Amount",
                    U16(2),
                    (0.0, 100.0),
                    0.0,
                ),
            ],
        ),
        Adjustment::HueSaturation(_) => {
            // Master at 10..16, then range k's Hue/Saturation/Lightness at
            // 16 + 14k + 8. The Master keys stay unprefixed.
            let mut specs = Vec::new();
            for (k, prefix) in [
                "",
                "reds.",
                "yellows.",
                "greens.",
                "cyans.",
                "blues.",
                "magentas.",
            ]
            .into_iter()
            .enumerate()
            {
                let at = if k == 0 { 10 } else { 16 + 14 * (k - 1) + 8 };
                for (i, (key, label, max)) in [
                    ("hue", "Hue", 180.0),
                    ("saturation", "Saturation", 100.0),
                    ("lightness", "Lightness", 100.0),
                ]
                .into_iter()
                .enumerate()
                {
                    let spec = slider(key, label, I16(at + 2 * i), (-max, max), 0.0);
                    specs.push(grouped(
                        Spec {
                            key: format!("{prefix}{key}"),
                            ..spec
                        },
                        k,
                    ));
                }
            }
            Some((
                "Hue/Saturation",
                vec![
                    "Master", "Reds", "Yellows", "Greens", "Cyans", "Blues", "Magentas",
                ],
                specs,
                false,
                Some("Colorize is kept as it is."),
            ))
        }
        Adjustment::ColorBalance(_) => {
            let mut specs = Vec::new();
            for tone in 0..3 {
                for (c, (key, label)) in [
                    ("cyanRed", "Cyan / Red"),
                    ("magentaGreen", "Magenta / Green"),
                    ("yellowBlue", "Yellow / Blue"),
                ]
                .into_iter()
                .enumerate()
                {
                    let field = I16((tone * 3 + c) * 2);
                    let key = format!("{}.{key}", ["shadows", "midtones", "highlights"][tone]);
                    specs.push(grouped(
                        slider(&key, label, field, (-100.0, 100.0), 0.0),
                        tone,
                    ));
                }
            }
            specs.push(check(
                "preserveLuminosity",
                "Preserve Luminosity",
                Flag(18),
                true,
            ));
            Some((
                "Color Balance",
                vec!["Shadows", "Midtones", "Highlights"],
                specs,
                false,
                None,
            ))
        }
        Adjustment::BlackWhite(_) => plain(
            "Black & White",
            [
                ("reds", "Reds", b"Rd  ", 40.0),
                ("yellows", "Yellows", b"Yllw", 60.0),
                ("greens", "Greens", b"Grn ", 40.0),
                ("cyans", "Cyans", b"Cyn ", 60.0),
                ("blues", "Blues", b"Bl  ", 20.0),
                ("magentas", "Magentas", b"Mgnt", 80.0),
            ]
            .into_iter()
            .map(|(key, label, item, default)| {
                slider(key, label, DescLong(item), (-200.0, 300.0), default)
            })
            .chain([
                check("tint", "Tint", DescBool(b"useTint"), false),
                Spec {
                    key: "tintColor".into(),
                    label: "Tint Color".into(),
                    field: DescColor(b"tintColor"),
                    kind: ParamKind::Color,
                    group: None,
                    // photorust's default tint, hue 42° saturation 20%.
                    default: f64::from(0xe1_d3_b4u32),
                },
            ])
            .collect(),
        ),
        Adjustment::PhotoFilter(_) => {
            // Version 3 stores the colour as CIE XYZ: its density and
            // luminosity are editable, its colour is not.
            let v3 = data.data.get(..2) == Some(&[0, 3]);
            let mut specs = Vec::new();
            if !v3 {
                specs.push(Spec {
                    key: "color".into(),
                    label: "Color".into(),
                    field: Rgb16(4),
                    kind: ParamKind::Color,
                    group: None,
                    default: f64::from(0xec_8a_00u32),
                });
            }
            specs.push(slider(
                "density",
                "Density",
                U32(if v3 { 14 } else { 12 }),
                (1.0, 100.0),
                25.0,
            ));
            specs.push(check(
                "preserveLuminosity",
                "Preserve Luminosity",
                Flag(if v3 { 18 } else { 16 }),
                true,
            ));
            plain("Photo Filter", specs)
        }
        Adjustment::ChannelMixer(params) => {
            let mut specs = Vec::new();
            let outputs: Vec<&'static str> = if params.monochrome {
                vec!["Gray"]
            } else {
                vec!["Red", "Green", "Blue"]
            };
            for (output, _) in outputs.iter().enumerate() {
                let at = 4 + output * 10;
                let identity = |source: usize| f64::from(u8::from(source == output) * 100);
                for (source, label) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                    let key = format!("{output}.{}", label.to_lowercase());
                    let field = I16(at + source * 2);
                    let default = if params.monochrome {
                        0.0
                    } else {
                        identity(source)
                    };
                    specs.push(grouped(
                        slider(&key, label, field, (-200.0, 200.0), default),
                        output,
                    ));
                }
                let key = format!("{output}.constant");
                specs.push(grouped(
                    slider(&key, "Constant", I16(at + 8), (-200.0, 200.0), 0.0),
                    output,
                ));
            }
            Some((
                "Channel Mixer",
                outputs,
                specs,
                false,
                params
                    .monochrome
                    .then_some("Monochrome: edits the gray output."),
            ))
        }
        Adjustment::SelectiveColor(_) => {
            let colours = [
                "Reds", "Yellows", "Greens", "Cyans", "Blues", "Magentas", "Whites", "Neutrals",
                "Blacks",
            ];
            let mut specs = Vec::new();
            for (range, colour) in colours.iter().enumerate() {
                for (ink, label) in ["Cyan", "Magenta", "Yellow", "Black"]
                    .into_iter()
                    .enumerate()
                {
                    let key = format!("{}.{}", colour.to_lowercase(), label.to_lowercase());
                    let field = I16(12 + range * 8 + ink * 2);
                    specs.push(grouped(
                        slider(&key, label, field, (-100.0, 100.0), 0.0),
                        range,
                    ));
                }
            }
            specs.push(Spec {
                key: "method".into(),
                label: "Method".into(),
                // Stored 0 Relative, 1 Absolute; the menu lists Relative first.
                field: U16(2),
                kind: ParamKind::Choice(&["Relative", "Absolute"]),
                group: None,
                default: 0.0,
            });
            Some(("Selective Color", colours.to_vec(), specs, false, None))
        }
        Adjustment::Posterize(_) => plain(
            "Posterize",
            vec![slider("levels", "Levels", U16(0), (2.0, 255.0), 4.0)],
        ),
        Adjustment::Threshold(_) => plain(
            "Threshold",
            vec![slider(
                "level",
                "Threshold Level",
                U16(0),
                (1.0, 255.0),
                128.0,
            )],
        ),
        Adjustment::Curves(_) => Some(("Curves", Vec::new(), Vec::new(), true, None)),
        Adjustment::Invert => Some((
            "Invert",
            Vec::new(),
            Vec::new(),
            false,
            Some("Invert has no settings."),
        )),
        Adjustment::GradientMap(_) => note("Gradient Map"),
        Adjustment::ColorLookup(_) => Some((
            "Color Lookup",
            Vec::new(),
            vec![Spec {
                key: "preset".into(),
                label: "Preset".into(),
                field: Preset,
                kind: ParamKind::Choice(&crate::color_lookup_presets::COLOR_LOOKUP_PRESETS),
                group: None,
                default: 0.0,
            }],
            false,
            None,
        )),
        Adjustment::SolidFill(_) => note("Solid Color"),
        Adjustment::GradientFill(_) => note("Gradient Fill"),
        Adjustment::PatternFill(_) => note("Pattern Fill"),
        Adjustment::Desaturate
        | Adjustment::Equalize
        | Adjustment::Auto(_)
        | Adjustment::ReplaceColor(_)
        | Adjustment::MatchColor(_) => None,
    }
}

fn note(title: &'static str) -> Option<Layout> {
    Some((
        title,
        Vec::new(),
        Vec::new(),
        false,
        Some("These settings cannot be edited here yet."),
    ))
}

fn be_bytes<const N: usize>(d: &[u8], at: usize) -> Option<[u8; N]> {
    d.get(at..at + N)?.try_into().ok()
}

fn read(data: &AdjustmentData, field: Field) -> Option<f64> {
    let d = &data.data;
    Some(match field {
        Field::I16(at) => i16::from_be_bytes(be_bytes(d, at)?) as f64,
        Field::U16(at) => u16::from_be_bytes(be_bytes(d, at)?) as f64,
        Field::U16Hundredths(at) => u16::from_be_bytes(be_bytes(d, at)?) as f64 / 100.0,
        Field::U32(at) => u32::from_be_bytes(be_bytes(d, at)?) as f64,
        Field::F32(at) => f32::from_be_bytes(be_bytes(d, at)?) as f64,
        Field::Flag(at) => f64::from(u8::from(*d.get(at)? != 0)),
        Field::Rgb16(at) => {
            let c = |i: usize| -> Option<u32> {
                Some(u16::from_be_bytes(be_bytes(d, at + i * 2)?).min(255) as u32)
            };
            ((c(0)? << 16) | (c(1)? << 8) | c(2)?) as f64
        }
        Field::DescLong(key) | Field::DescBool(key) | Field::DescColor(key) => {
            let obj = pictura_codec::read_descriptor(d).ok()?;
            // A missing item reads as the spec's default, as the decoder does.
            match desc_item(&obj, key)? {
                DescValue::Long(n) => *n as f64,
                DescValue::Bool(b) => f64::from(u8::from(*b)),
                color @ DescValue::Object { .. } => {
                    let c = |k: &[u8]| match desc_item(color, k) {
                        Some(DescValue::Double(v)) => (v * 255.0).round().clamp(0.0, 255.0) as u32,
                        _ => 0,
                    };
                    ((c(b"Rd  ") << 16) | (c(b"Grn ") << 8) | c(b"Bl  ")) as f64
                }
                _ => return None,
            }
        }
        Field::Preset => crate::color_lookup::color_lookup_preset_index(data)? as f64,
    })
}

fn write(data: &mut AdjustmentData, field: Field, value: f64) -> Option<()> {
    let d = &mut data.data;
    let mut put = |at: usize, bytes: &[u8]| -> Option<()> {
        d.get_mut(at..at + bytes.len())?.copy_from_slice(bytes);
        Some(())
    };
    match field {
        Field::I16(at) => put(at, &(value.round() as i16).to_be_bytes()),
        Field::U16(at) => put(at, &(value.round().max(0.0) as u16).to_be_bytes()),
        Field::U16Hundredths(at) => {
            put(at, &((value * 100.0).round().max(0.0) as u16).to_be_bytes())
        }
        Field::U32(at) => put(at, &(value.round().max(0.0) as u32).to_be_bytes()),
        Field::F32(at) => put(at, &(value as f32).to_be_bytes()),
        Field::Flag(at) => put(at, &[u8::from(value != 0.0)]),
        Field::Rgb16(at) => {
            let rgb = value as u32;
            for (i, shift) in [16, 8, 0].into_iter().enumerate() {
                put(at + i * 2, &(((rgb >> shift) & 0xff) as u16).to_be_bytes())?;
            }
            Some(())
        }
        Field::DescLong(key) | Field::DescBool(key) | Field::DescColor(key) => {
            let mut obj = pictura_codec::read_descriptor(d).ok()?;
            let DescValue::Object { items, .. } = &mut obj else {
                return None;
            };
            let item = match field {
                Field::DescBool(_) => DescValue::Bool(value != 0.0),
                Field::DescColor(_) => {
                    let rgb = value as u32;
                    let c =
                        |shift: u32| DescValue::Double(f64::from((rgb >> shift) & 0xff) / 255.0);
                    DescValue::Object {
                        name: String::new(),
                        class_id: b"RGBC".to_vec(),
                        items: vec![
                            (b"Rd  ".to_vec(), c(16)),
                            (b"Grn ".to_vec(), c(8)),
                            (b"Bl  ".to_vec(), c(0)),
                        ],
                    }
                }
                _ => DescValue::Long(value.round() as i32),
            };
            match items.iter_mut().find(|(k, _)| k.as_slice() == key) {
                Some((_, slot)) => *slot = item,
                None => items.push((key.to_vec(), item)),
            }
            *d = pictura_codec::write_descriptor(&obj);
            Some(())
        }
        // Rebuilt wholesale by `set_adjustment_param`, never patched in place.
        Field::Preset => None,
    }
}

/// Read a field, falling back to its default where the block omits it (a
/// descriptor item the decoder also defaults).
fn value_of(data: &AdjustmentData, spec: &Spec) -> f64 {
    read(data, spec.field).unwrap_or(spec.default)
}

/// The Properties page for an adjustment or fill layer's block; `None` when
/// the block does not decode.
pub fn adjustment_editor(data: &AdjustmentData) -> Option<AdjustmentEditor> {
    let adjustment = decode_adjustment(data)?;
    let (title, groups, specs, curves, note) = layout(data, &adjustment)?;
    let params = specs
        .iter()
        .map(|spec| AdjustmentParam {
            key: spec.key.clone(),
            label: spec.label.clone(),
            kind: spec.kind.clone(),
            group: spec.group,
            value: value_of(data, spec),
            default: spec.default,
        })
        .collect();
    Some(AdjustmentEditor {
        title,
        groups,
        params,
        curves,
        note,
    })
}

/// `data` with parameter `key` set to `value` (clamped to its range), or
/// `None` for an unknown key or a result that would not decode (Levels'
/// input black past its white).
pub fn set_adjustment_param(
    data: &AdjustmentData,
    key: &str,
    value: f64,
) -> Option<AdjustmentData> {
    let adjustment = decode_adjustment(data)?;
    // A Color Lookup preset is not a scalar field: rebuild the embedded `.CUBE`.
    if key == "preset" && matches!(adjustment, Adjustment::ColorLookup(_)) {
        let last = (crate::color_lookup_presets::COLOR_LOOKUP_PRESETS.len() - 1) as f64;
        return crate::set_color_lookup_preset(data, value.clamp(0.0, last).round() as usize);
    }
    let (_, _, specs, _, _) = layout(data, &adjustment)?;
    let spec = specs.into_iter().find(|s| s.key == key)?;
    let value = match spec.kind {
        ParamKind::Slider { min, max, .. } => value.clamp(min, max),
        ParamKind::Check => f64::from(u8::from(value != 0.0)),
        ParamKind::Choice(options) => value.round().clamp(0.0, (options.len() - 1) as f64),
        ParamKind::Color => value.clamp(0.0, f64::from(0xff_ff_ffu32)),
    };
    let mut out = data.clone();
    write(&mut out, spec.field, value)?;
    decode_adjustment(&out).map(|_| out)
}

/// Every parameter back to its default, and a Curves layer back to the
/// identity line.
pub fn reset_adjustment(data: &AdjustmentData) -> Option<AdjustmentData> {
    let editor = adjustment_editor(data)?;
    let mut out = data.clone();
    if editor.curves {
        out = crate::encode_curves(&[(0, 0), (255, 255)], None, None, None);
    }
    for param in &editor.params {
        out = set_adjustment_param(&out, &param.key, param.default).unwrap_or(out);
    }
    Some(out)
}

/// A Curves layer's points for `channel` (0 composite, 1 red, 2 green,
/// 3 blue); an absent channel curve is the identity line.
pub fn curve_points(data: &AdjustmentData, channel: usize) -> Option<Vec<(u8, u8)>> {
    let Some(Adjustment::Curves(curves)) = decode_adjustment(data) else {
        return None;
    };
    let identity = || vec![(0, 0), (255, 255)];
    Some(match channel {
        0 => curves.points,
        1 => curves.red.unwrap_or_else(identity),
        2 => curves.green.unwrap_or_else(identity),
        3 => curves.blue.unwrap_or_else(identity),
        _ => return None,
    })
}

/// A Curves layer with `channel`'s points replaced (2..=14 strictly
/// increasing inputs). An identity channel curve is dropped rather than
/// stored. `None` for another kind or invalid points.
pub fn set_curve_points(
    data: &AdjustmentData,
    channel: usize,
    points: &[(u8, u8)],
) -> Option<AdjustmentData> {
    let Some(Adjustment::Curves(mut curves)) = decode_adjustment(data) else {
        return None;
    };
    if !(2..=14).contains(&points.len()) || points.windows(2).any(|w| w[0].0 >= w[1].0) {
        return None;
    }
    let channel_curve = (points != [(0, 0), (255, 255)]).then(|| points.to_vec());
    match channel {
        0 => curves.points = points.to_vec(),
        1 => curves.red = channel_curve,
        2 => curves.green = channel_curve,
        3 => curves.blue = channel_curve,
        _ => return None,
    }
    let out = crate::encode_curves(
        &curves.points,
        curves.red.as_deref(),
        curves.green.as_deref(),
        curves.blue.as_deref(),
    );
    decode_adjustment(&out).map(|_| out)
}

#[cfg(test)]
#[path = "adjustment_params_tests.rs"]
mod tests;
