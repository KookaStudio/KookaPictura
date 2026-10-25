//! Layer-style authoring: the Layer Style dialog's controls read and written as
//! `"<effect>.<field>"` numbers over the layer's `lfx2` descriptor, plus the
//! Layer > Layer Style commands (Copy / Paste / Clear, Hide / Show All Effects,
//! Scale Effects).
//!
//! The `lfx2` descriptor stays the model: the compositor already decodes and
//! draws every effect from it and the codec round-trips it verbatim, so an edit
//! here is a descriptor rewrite and nothing else. Numbers are the dialog's
//! units: percent, pixels and degrees as shown; a colour packs as `0xRRGGBB`;
//! a blend mode is its index in [`BlendMode::LAYER_MODES`]; a choice is its
//! index in the field's option list; a flag is `0` or `1`.
//!
//! `blending.*` keys are the Blending Options page and write the layer record
//! itself (mode, opacity, fill, knockout, the two group flags).
//!
//! ponytail: a layer carrying only a legacy `lrFX` block starts a fresh `lfx2`
//! on its first edit, which then wins over the legacy set; the stroke is always
//! a colour fill; a Pattern Overlay can be edited but not created (no pattern
//! preset system yet); Global Light, Create Layers and Blend If are not
//! authored here.

use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Knockout, Layer, LayerBlock};

use super::paths::{flatten_rows, is_background, resolve_path_mut};
use crate::composite::desc_item;

#[path = "layer_style_fields.rs"]
mod fields;

use fields::{Effect, Field, Kind, EFFECTS};

/// The dialog key prefixes of the ten effects, in CS6's list order.
pub fn layer_style_effect_names() -> Vec<&'static str> {
    EFFECTS.iter().map(|e| e.name).collect()
}

/// The `BlnM` value Photoshop writes for an effect blend mode (a char code for
/// the classic modes, a string id for the newer ones); the inverse of
/// `layer_effects::effect_blend_mode`.
fn effect_blend_key(mode: BlendMode) -> &'static [u8] {
    match mode {
        BlendMode::Dissolve => b"Dslv",
        BlendMode::Darken => b"Drkn",
        BlendMode::Multiply => b"Mltp",
        BlendMode::ColorBurn => b"CBrn",
        BlendMode::LinearBurn => b"linearBurn",
        BlendMode::DarkerColor => b"darkerColor",
        BlendMode::Lighten => b"Lghn",
        BlendMode::Screen => b"Scrn",
        BlendMode::ColorDodge => b"CDdg",
        BlendMode::LinearDodge => b"linearDodge",
        BlendMode::LighterColor => b"lighterColor",
        BlendMode::Overlay => b"Ovrl",
        BlendMode::SoftLight => b"SftL",
        BlendMode::HardLight => b"HrdL",
        BlendMode::VividLight => b"vividLight",
        BlendMode::LinearLight => b"linearLight",
        BlendMode::PinLight => b"pinLight",
        BlendMode::HardMix => b"hardMix",
        BlendMode::Difference => b"Dfrn",
        BlendMode::Exclusion => b"Xclu",
        BlendMode::Subtract => b"blendSubtraction",
        BlendMode::Divide => b"blendDivide",
        BlendMode::Hue => b"H   ",
        BlendMode::Saturation => b"Strt",
        BlendMode::Color => b"Clr ",
        BlendMode::Luminosity => b"Lmns",
        BlendMode::Normal | BlendMode::PassThrough => b"Nrml",
    }
}

fn mode_index(mode: BlendMode) -> Option<usize> {
    BlendMode::LAYER_MODES.iter().position(|m| *m == mode)
}

fn mode_at(value: f64) -> Option<BlendMode> {
    let i = value.round();
    (0.0..BlendMode::LAYER_MODES.len() as f64)
        .contains(&i)
        .then(|| BlendMode::LAYER_MODES[i as usize])
}

fn object(class: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class.to_vec(),
        items,
    }
}

fn rgbc(packed: f64) -> DescValue {
    let v = packed.round().clamp(0.0, 16_777_215.0) as u32;
    object(
        b"RGBC",
        [(b"Rd  ", 16), (b"Grn ", 8), (b"Bl  ", 0)]
            .into_iter()
            .map(|(k, shift)| {
                (
                    k.to_vec(),
                    DescValue::Double(f64::from((v >> shift) & 0xff)),
                )
            })
            .collect(),
    )
}

fn packed(color: &DescValue) -> Option<f64> {
    let c = |key: &[u8]| match desc_item(color, key)? {
        DescValue::Double(v) => Some(v.round().clamp(0.0, 255.0) as u32),
        DescValue::UnitFloat { value, .. } => Some(value.round().clamp(0.0, 255.0) as u32),
        _ => None,
    };
    Some(f64::from(
        (c(b"Rd  ")? << 16) | (c(b"Grn ")? << 8) | c(b"Bl  ")?,
    ))
}

/// A linear contour (`ShpC`), which every contour-bearing effect carries.
fn linear_contour() -> DescValue {
    let point = |v: f64| {
        object(
            b"CrPt",
            vec![
                (b"Hrzn".to_vec(), DescValue::Double(v)),
                (b"Vrtc".to_vec(), DescValue::Double(v)),
            ],
        )
    };
    object(
        b"ShpC",
        vec![
            (b"Nm  ".to_vec(), DescValue::Text("Linear".into())),
            (
                b"Crv ".to_vec(),
                DescValue::List(vec![point(0.0), point(255.0)]),
            ),
        ],
    )
}

fn two_stop_gradient(from: f64, to: f64) -> DescValue {
    let stop = |color: f64, location: f64| {
        object(
            b"Clrt",
            vec![
                (b"Clr ".to_vec(), rgbc(color)),
                (
                    b"Type".to_vec(),
                    DescValue::Enum {
                        kind: b"Clry".to_vec(),
                        value: b"UsrS".to_vec(),
                    },
                ),
                (b"Lctn".to_vec(), DescValue::Long(location as i32)),
                (b"Mdpn".to_vec(), DescValue::Long(50)),
            ],
        )
    };
    let opacity = |location: f64| {
        object(
            b"TrnS",
            vec![
                (
                    b"Opct".to_vec(),
                    DescValue::UnitFloat {
                        unit: *b"#Prc",
                        value: 100.0,
                    },
                ),
                (b"Lctn".to_vec(), DescValue::Long(location as i32)),
                (b"Mdpn".to_vec(), DescValue::Long(50)),
            ],
        )
    };
    object(
        b"Grdn",
        vec![
            (b"Nm  ".to_vec(), DescValue::Text("Custom".into())),
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (b"Intr".to_vec(), DescValue::Double(4096.0)),
            (
                b"Clrs".to_vec(),
                DescValue::List(vec![stop(from, 0.0), stop(to, 4096.0)]),
            ),
            (
                b"Trns".to_vec(),
                DescValue::List(vec![opacity(0.0), opacity(4096.0)]),
            ),
        ],
    )
}

/// The stop list of a gradient object, mutably.
fn stops_mut(grad: &mut DescValue) -> Option<&mut Vec<DescValue>> {
    let DescValue::Object { items, .. } = grad else {
        return None;
    };
    match items.iter_mut().find(|(k, _)| k.as_slice() == b"Clrs")? {
        (_, DescValue::List(stops)) if !stops.is_empty() => Some(stops),
        _ => None,
    }
}

/// Read a field off an effect object; absent is the field default.
fn read_field(obj: &DescValue, f: &Field) -> Option<f64> {
    let Some(value) = desc_item(obj, f.key) else {
        return Some(f.default);
    };
    match (f.kind, value) {
        (Kind::Bool, DescValue::Bool(b)) => Some(f64::from(u8::from(*b))),
        (Kind::Unit(..), DescValue::UnitFloat { value, .. } | DescValue::Double(value)) => {
            value.is_finite().then_some(*value)
        }
        (Kind::Unit(..), DescValue::Long(v)) => Some(f64::from(*v)),
        (Kind::Color, color) => packed(color),
        (Kind::Blend, DescValue::Enum { value, .. }) => mode_index(
            crate::layer_effects::effect_blend_mode(value, BlendMode::Normal),
        )
        .map(|i| i as f64),
        (Kind::Choice(_, options), DescValue::Enum { value, .. }) => options
            .iter()
            .position(|o| *o == value.as_slice())
            .map(|i| i as f64),
        (Kind::GradientStop(last), grad) => {
            let DescValue::List(stops) = desc_item(grad, b"Clrs")? else {
                return None;
            };
            let stop = if last { stops.last() } else { stops.first() }?;
            packed(desc_item(stop, b"Clr ")?)
        }
        _ => None,
    }
}

/// The descriptor value a field's number writes, or `None` for a gradient stop
/// (edited in place) or an out-of-range choice.
fn field_value(f: &Field, value: f64) -> Option<DescValue> {
    Some(match f.kind {
        Kind::Bool => DescValue::Bool(value >= 0.5),
        Kind::Unit(unit, min, max) => DescValue::UnitFloat {
            unit: *unit,
            value: value.clamp(min, max),
        },
        Kind::Color => rgbc(value),
        Kind::Blend => DescValue::Enum {
            kind: b"BlnM".to_vec(),
            value: effect_blend_key(mode_at(value)?).to_vec(),
        },
        Kind::Choice(kind, options) => DescValue::Enum {
            kind: kind.to_vec(),
            value: options
                .get(usize::try_from(value.round() as i64).ok()?)?
                .to_vec(),
        },
        Kind::GradientStop(_) => return None,
    })
}

/// A new, absent and disabled effect object at CS6's defaults.
fn default_effect(effect: &Effect) -> DescValue {
    let mut items = vec![
        (b"enab".to_vec(), DescValue::Bool(false)),
        (b"present".to_vec(), DescValue::Bool(false)),
        (b"showInDialog".to_vec(), DescValue::Bool(true)),
    ];
    for f in effect.fields {
        if let Some(v) = field_value(f, f.default) {
            items.push((f.key.to_vec(), v));
        }
    }
    match effect.key {
        b"GrFl" => items.push((b"Grad".to_vec(), two_stop_gradient(0.0, 16_777_215.0))),
        b"FrFX" => items.push((
            b"PntT".to_vec(),
            DescValue::Enum {
                kind: b"FrFl".to_vec(),
                value: b"SClr".to_vec(),
            },
        )),
        b"ChFX" => items.push((b"MpgS".to_vec(), linear_contour())),
        b"ebbl" => {
            items.push((b"TrnS".to_vec(), linear_contour()));
            items.push((b"MpgS".to_vec(), linear_contour()));
        }
        b"DrSh" | b"IrSh" | b"OrGl" | b"IrGl" => items.push((b"TrnS".to_vec(), linear_contour())),
        _ => {}
    }
    object(effect.key, items)
}

/// The layer's `lfx2` top-level object, or `None` when it has none (or it does
/// not parse).
fn read_lfx2(layer: &Layer) -> Option<DescValue> {
    let data = layer.extra_block(b"lfx2")?.data.get(4..)?;
    pictura_codec::read_descriptor(data)
        .ok()
        .filter(|d| matches!(d, DescValue::Object { .. }))
}

/// Replace the layer's `lfx2` with `items` (dropping it when no effect is
/// left), keeping the block's place among the others.
fn write_lfx2(layer: &mut Layer, items: Vec<(Vec<u8>, DescValue)>) {
    let has_effects = items
        .iter()
        .any(|(k, _)| !matches!(k.as_slice(), b"Scl " | b"masterFXSwitch"));
    let at = layer.extra_blocks.iter().position(|b| &b.key == b"lfx2");
    layer.extra_blocks.retain(|b| &b.key != b"lfx2");
    if !has_effects {
        return;
    }
    let mut data = 0u32.to_be_bytes().to_vec();
    data.extend_from_slice(&pictura_codec::write_descriptor(&object(b"null", items)));
    let block = LayerBlock {
        key: *b"lfx2",
        data,
    };
    match at {
        Some(i) => layer.extra_blocks.insert(i, block),
        None => layer.extra_blocks.push(block),
    }
}

fn lfx2_items(layer: &Layer) -> Vec<(Vec<u8>, DescValue)> {
    match read_lfx2(layer) {
        Some(DescValue::Object { items, .. }) => items,
        _ => vec![
            (
                b"Scl ".to_vec(),
                DescValue::UnitFloat {
                    unit: *b"#Prc",
                    value: 100.0,
                },
            ),
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
        ],
    }
}

fn set_item(items: &mut Vec<(Vec<u8>, DescValue)>, key: &[u8], value: DescValue) {
    match items.iter_mut().find(|(k, _)| k.as_slice() == key) {
        Some(slot) => slot.1 = value,
        None => items.push((key.to_vec(), value)),
    }
}

fn effect_named(name: &str) -> Option<&'static Effect> {
    EFFECTS.iter().find(|e| e.name == name)
}

/// Whether a layer can carry a style: not a group (the compositor draws no
/// group effects yet) and not the Background.
fn styleable(layer: &Layer) -> bool {
    !layer.is_group && !layer.background
}

/// The value of `key` on `layer`: an effect field (its default when the effect
/// or field is absent), `<effect>.on` (enabled and present), the read-only
/// `<effect>.exists` (on or off), `fx.visible` (the master switch), or a
/// `blending.*` option. `None` for an unknown key.
pub fn layer_style_value(layer: &Layer, key: &str) -> Option<f64> {
    let (group, name) = key.split_once('.')?;
    let flag = |b: bool| Some(f64::from(u8::from(b)));
    match group {
        "blending" => blending_value(layer, name),
        "fx" if name == "visible" => {
            let top = read_lfx2(layer);
            match top.as_ref().and_then(|t| desc_item(t, b"masterFXSwitch")) {
                Some(DescValue::Bool(b)) => flag(*b),
                _ => flag(true),
            }
        }
        _ => {
            let effect = effect_named(group)?;
            let top = read_lfx2(layer);
            let obj = top.as_ref().and_then(|t| desc_item(t, effect.key));
            if name == "exists" {
                return flag(obj.is_some());
            }
            if effect.key == b"patternFill" && name == "pattern" {
                return Some(obj.map_or(0.0, |o| pattern_index(o).map_or(-1.0, |i| i as f64)));
            }
            if name == "on" {
                let on = |k: &[u8]| {
                    matches!(
                        obj.and_then(|o| desc_item(o, k)),
                        Some(DescValue::Bool(true))
                    )
                };
                return flag(on(b"enab") && on(b"present"));
            }
            let f = effect.fields.iter().find(|f| f.name == name)?;
            match obj {
                Some(obj) => read_field(obj, f),
                None => Some(f.default),
            }
        }
    }
}

/// Set `key` on `layer` (see [`layer_style_value`]). Writing a field of an
/// absent effect adds it, off, at CS6's defaults; `<effect>.on` switches it
/// on (enabled and present) or off. False when the key is unknown, the layer
/// cannot carry a style, or nothing changed.
pub fn set_layer_style_value(layer: &mut Layer, key: &str, value: f64) -> bool {
    if !value.is_finite() || layer.background {
        return false;
    }
    let Some((group, name)) = key.split_once('.') else {
        return false;
    };
    // Groups carry Blending Options now; their effects wait for the group
    // effects renderer.
    if layer.is_group {
        return group == "blending" && set_blending_value(layer, name, value);
    }
    if group == "blending" {
        return set_blending_value(layer, name, value);
    }
    if layer_style_value(layer, key).is_some_and(|v| (v - value).abs() < 1e-9) {
        return false;
    }
    let mut items = lfx2_items(layer);
    if group == "fx" {
        if name != "visible" || layer.extra_block(b"lfx2").is_none() {
            return false;
        }
        set_item(&mut items, b"masterFXSwitch", DescValue::Bool(value >= 0.5));
        write_lfx2(layer, items);
        return true;
    }
    let Some(effect) = effect_named(group) else {
        return false;
    };
    let existing = items
        .iter()
        .find(|(k, _)| k.as_slice() == effect.key)
        .map(|(_, v)| v.clone());
    // A Pattern Overlay needs a pattern in the document first, which takes the
    // document: see `set_document_layer_style_value`.
    if existing.is_none() && effect.key == b"patternFill" {
        return false;
    }
    let mut obj = existing.unwrap_or_else(|| default_effect(effect));
    let DescValue::Object { items: fields, .. } = &mut obj else {
        return false;
    };
    if name == "on" {
        set_item(fields, b"enab", DescValue::Bool(value >= 0.5));
        set_item(fields, b"present", DescValue::Bool(value >= 0.5));
    } else {
        let Some(f) = effect.fields.iter().find(|f| f.name == name) else {
            return false;
        };
        if let Kind::GradientStop(last) = f.kind {
            if desc_item(&obj, b"Grad").is_none() {
                let DescValue::Object { items: fields, .. } = &mut obj else {
                    return false;
                };
                set_item(fields, b"Grad", two_stop_gradient(0.0, 16_777_215.0));
            }
            let DescValue::Object { items: fields, .. } = &mut obj else {
                return false;
            };
            let Some((_, grad)) = fields.iter_mut().find(|(k, _)| k.as_slice() == b"Grad") else {
                return false;
            };
            let Some(stops) = stops_mut(grad) else {
                return false;
            };
            let stop = if last {
                stops.last_mut()
            } else {
                stops.first_mut()
            };
            let Some(DescValue::Object { items: stop, .. }) = stop else {
                return false;
            };
            set_item(stop, b"Clr ", rgbc(value));
        } else {
            let Some(v) = field_value(f, value) else {
                return false;
            };
            set_item(fields, f.key, v);
        }
    }
    set_item(&mut items, effect.key, obj);
    write_lfx2(layer, items);
    true
}

fn blending_value(layer: &Layer, name: &str) -> Option<f64> {
    let pct = |v: u8| (f64::from(v) * 100.0 / 255.0).round();
    let flag = |b: bool| f64::from(u8::from(b));
    Some(match name {
        "mode" => mode_index(layer.blend)? as f64,
        "opacity" => pct(layer.opacity),
        "fillOpacity" => pct(layer.fill),
        "knockout" => f64::from(layer.knockout.to_byte()),
        "blendInterior" => flag(layer.blend_interior),
        "blendClipped" => flag(layer.blend_clipping),
        _ => return None,
    })
}

fn set_blending_value(layer: &mut Layer, name: &str, value: f64) -> bool {
    let byte = |pct: f64| (pct.clamp(0.0, 100.0) * 255.0 / 100.0).round() as u8;
    fn update<T: PartialEq>(slot: &mut T, next: T) -> bool {
        let changed = *slot != next;
        *slot = next;
        changed
    }
    match name {
        "mode" => match mode_at(value) {
            Some(mode) => update(&mut layer.blend, mode),
            None => false,
        },
        "opacity" => update(&mut layer.opacity, byte(value)),
        "fillOpacity" => update(&mut layer.fill, byte(value)),
        "knockout" => update(
            &mut layer.knockout,
            Knockout::from_byte(value.round().clamp(0.0, 2.0) as u8),
        ),
        "blendInterior" => update(&mut layer.blend_interior, value >= 0.5),
        "blendClipped" => update(&mut layer.blend_clipping, value >= 0.5),
        _ => false,
    }
}

/// The built-in patterns a Pattern Overlay offers (the Pattern Stamp's set),
/// in picker order.
pub fn layer_style_pattern_names() -> Vec<&'static str> {
    pictura_paint::pattern::PATTERN_NAMES.to_vec()
}

/// The id a built-in pattern is stored under in the document's `Patt` block.
fn builtin_pattern_id(index: usize) -> String {
    format!("kooka-builtin-pattern-{index}")
}

/// The built-in pattern a `patternFill` object names, if it is one.
fn pattern_index(obj: &DescValue) -> Option<usize> {
    let DescValue::Text(id) = desc_item(desc_item(obj, b"Ptrn")?, b"Idnt")? else {
        return None;
    };
    let id = id.trim_end_matches('\0');
    (0..pictura_paint::pattern::PATTERN_NAMES.len()).find(|&i| builtin_pattern_id(i) == id)
}

/// Make sure built-in pattern `index` is in the document's `Patt` block.
fn ensure_builtin_pattern(doc: &mut Document, index: usize) -> Option<String> {
    let tile = pictura_paint::pattern::tile(index)?;
    let id = builtin_pattern_id(index);
    let rgba: Vec<u8> = tile.data.concat();
    let record = pictura_codec::encode_rgb_pattern(
        pictura_paint::pattern::PATTERN_NAMES[index],
        &id,
        tile.width as u32,
        tile.height as u32,
        &rgba,
    )?;
    pictura_codec::add_document_pattern(doc, &id, &record);
    Some(id)
}

/// [`set_layer_style_value`] on the layer at `path`, plus what needs the
/// document: `patternOverlay.pattern` picks a built-in pattern (embedding it
/// in the document, and adding the overlay, off, when absent), and switching
/// on an absent Pattern Overlay gives it the first pattern.
pub fn set_document_layer_style_value(
    doc: &mut Document,
    path: &str,
    key: &str,
    value: f64,
) -> bool {
    if is_background(doc, path) {
        return false;
    }
    let pattern_key = key == "patternOverlay.pattern";
    let switching_on = key == "patternOverlay.on" && value >= 0.5;
    let mut changed = false;
    if pattern_key || switching_on {
        let Some(layer) = resolve_path_mut(doc, path).filter(|l| styleable(l)) else {
            return false;
        };
        let current = layer_style_value(layer, "patternOverlay.pattern");
        let exists = layer_style_value(layer, "patternOverlay.exists") == Some(1.0);
        let index = if pattern_key { value.round() } else { 0.0 };
        if pattern_key || !exists {
            let names = pictura_paint::pattern::PATTERN_NAMES.len();
            if !(0.0..names as f64).contains(&index) || (exists && current == Some(index)) {
                return false;
            }
            let Some(id) = ensure_builtin_pattern(doc, index as usize) else {
                return false;
            };
            let Some(layer) = resolve_path_mut(doc, path) else {
                return false;
            };
            set_pattern(layer, &id, index as usize);
            changed = true;
        }
        if pattern_key {
            return changed;
        }
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    set_layer_style_value(layer, key, value) || changed
}

/// Point the layer's Pattern Overlay at pattern `id`, adding the overlay (off,
/// at its defaults) when absent.
fn set_pattern(layer: &mut Layer, id: &str, index: usize) {
    let mut items = lfx2_items(layer);
    let Some(effect) = effect_named("patternOverlay") else {
        return;
    };
    let mut obj = items
        .iter()
        .find(|(k, _)| k.as_slice() == effect.key)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| default_effect(effect));
    if let DescValue::Object { items: fields, .. } = &mut obj {
        set_item(
            fields,
            b"Ptrn",
            object(
                b"Ptrn",
                vec![
                    (
                        b"Nm  ".to_vec(),
                        DescValue::Text(pictura_paint::pattern::PATTERN_NAMES[index].into()),
                    ),
                    (b"Idnt".to_vec(), DescValue::Text(id.into())),
                ],
            ),
        );
    }
    set_item(&mut items, effect.key, obj);
    write_lfx2(layer, items);
}

/// Whether the layer carries any effect, on or off (an `lfx2` or legacy
/// `lrFX` block).
pub fn has_layer_style(layer: &Layer) -> bool {
    layer.extra_block(b"lfx2").is_some() || layer.extra_block(b"lrFX").is_some()
}

/// A copied layer style: the effect blocks plus the Blending Options, which
/// CS6's Copy Layer Style carries with it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerStyle {
    blocks: Vec<LayerBlock>,
    blend: BlendMode,
    opacity: u8,
    fill: u8,
    knockout: Knockout,
    blend_interior: bool,
    blend_clipping: bool,
}

/// Copy `layer`'s style, or `None` when it cannot carry one.
pub fn copy_layer_style(layer: &Layer) -> Option<LayerStyle> {
    styleable(layer).then(|| LayerStyle {
        blocks: layer
            .extra_blocks
            .iter()
            .filter(|b| matches!(&b.key, b"lfx2" | b"lrFX"))
            .cloned()
            .collect(),
        blend: layer.blend,
        opacity: layer.opacity,
        fill: layer.fill,
        knockout: layer.knockout,
        blend_interior: layer.blend_interior,
        blend_clipping: layer.blend_clipping,
    })
}

/// Replace the style on every styleable layer at `paths` with `style`.
/// Returns how many layers changed.
pub fn paste_layer_style(doc: &mut Document, paths: &[&str], style: &LayerStyle) -> usize {
    edit_styles(doc, paths, |layer| {
        let before = layer.clone();
        layer
            .extra_blocks
            .retain(|b| !matches!(&b.key, b"lfx2" | b"lrFX"));
        layer.extra_blocks.extend(style.blocks.iter().cloned());
        layer.blend = style.blend;
        layer.opacity = style.opacity;
        layer.fill = style.fill;
        layer.knockout = style.knockout;
        layer.blend_interior = style.blend_interior;
        layer.blend_clipping = style.blend_clipping;
        *layer != before
    })
}

/// Remove every effect from the layers at `paths`. Returns how many changed.
///
/// ponytail: the Blending Options are left as they are; whether CS6's Clear
/// Layer Style also resets them is unverified.
pub fn clear_layer_style(doc: &mut Document, paths: &[&str]) -> usize {
    edit_styles(doc, paths, |layer| {
        let before = layer.extra_blocks.len();
        layer
            .extra_blocks
            .retain(|b| !matches!(&b.key, b"lfx2" | b"lrFX"));
        layer.extra_blocks.len() != before
    })
}

/// Scale every pixel-sized effect parameter (and the overlay scales) on the
/// layers at `paths` by `percent`, each clamped to its range. Returns how
/// many layers changed.
pub fn scale_layer_effects(doc: &mut Document, paths: &[&str], percent: f64) -> usize {
    if !percent.is_finite() || percent <= 0.0 {
        return 0;
    }
    let factor = percent / 100.0;
    edit_styles(doc, paths, |layer| {
        let mut changed = false;
        for effect in EFFECTS {
            if layer_style_value(layer, &format!("{}.on", effect.name)) != Some(1.0) {
                continue;
            }
            for f in effect.fields {
                // Spread and Choke are percentages stored in pixel units.
                let scaled = match f.kind {
                    Kind::Unit(b"#Pxl", ..) => !matches!(f.name, "spread" | "choke"),
                    Kind::Unit(b"#Prc", ..) => f.name == "scale",
                    _ => false,
                };
                let key = format!("{}.{}", effect.name, f.name);
                if let (true, Some(v)) = (scaled, layer_style_value(layer, &key)) {
                    let next = if f.name == "size" && effect.name == "stroke" {
                        (v * factor).round()
                    } else {
                        v * factor
                    };
                    changed |= set_layer_style_value(layer, &key, next);
                }
            }
        }
        changed
    })
}

/// Show or hide every layer's effects at once (the master `fx` switch), as
/// Layer > Layer Style > Show / Hide All Effects. Returns how many changed.
pub fn set_all_effects_visible(doc: &mut Document, visible: bool) -> usize {
    let paths: Vec<String> = flatten_rows(doc).into_iter().map(|(p, _)| p).collect();
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    edit_styles(doc, &refs, |layer| {
        layer.extra_block(b"lfx2").is_some()
            && set_layer_style_value(layer, "fx.visible", f64::from(u8::from(visible)))
    })
}

/// Whether any layer has effects hidden (`false`) or shown (`true`) by the
/// master switch, which enables Show / Hide All Effects.
pub fn any_effects_visible(doc: &Document, visible: bool) -> bool {
    flatten_rows(doc).into_iter().any(|(path, _)| {
        super::paths::resolve_path(doc, &path).is_some_and(|layer| {
            layer.extra_block(b"lfx2").is_some()
                && layer_style_value(layer, "fx.visible") == Some(f64::from(u8::from(visible)))
        })
    })
}

fn edit_styles(
    doc: &mut Document,
    paths: &[&str],
    mut edit: impl FnMut(&mut Layer) -> bool,
) -> usize {
    let mut changed = 0;
    for path in paths {
        if is_background(doc, path) {
            continue;
        }
        if let Some(layer) = resolve_path_mut(doc, path).filter(|l| styleable(l)) {
            changed += usize::from(edit(layer));
        }
    }
    changed
}

#[cfg(test)]
#[path = "layer_style_tests.rs"]
mod tests;
