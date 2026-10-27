//! Object-based layer effects: the `lfx2` Drop Shadow, Outer Glow, Inner
//! Shadow and Inner Glow.
//!
//! A layer's effects live in the `lfx2` additional-layer-information block as a
//! `DescriptorBlock2`: the top-level `DrSh` object holds the drop shadow, the
//! `OrGl` object holds the outer glow, the `IrSh` object holds the inner shadow,
//! and the `IrGl` object holds the inner glow. This slice decodes and renders
//! all four.
//!
//! ponytail: the effective angle is the stored `lagl`, not the document
//! global-light resource (1037), which this crate does not decode; `uglg` is
//! carried for a later slice. Contour (`TrnS`), noise (`Nose`) and anti-alias
//! (`AntA`) are not applied. A shadow always composites behind the layer
//! content, so a semi-transparent layer shows it through (the `knocks_out =
//! false` look). The matte is a full-canvas `f32` buffer, matching the
//! compositor's ceiling.
//!
//! A glow has no offset: `GlwT` `PrBL` (Precise) is decoded but rendered as
//! `SfBL` (Softer), `Range` (`Inpr`), contour, noise, jitter (`ShdN`),
//! anti-alias and gradient mode (`Grad`) are ignored, the spread is a max-filter
//! dilate of radius `round(spread / 100 * size)` (not the reference's spread-then-
//! blur split), and the exterior is the multiplicative `1 - matte` rather than
//! the exact knock-out equation.
//!
//! An inner shadow composites **above** the layer content (interior-only): it is
//! the offset, choke-eroded and blurred inverted matte multiplied by the content
//! matte. ponytail: `knocks_out` (`layerConceals`) is decoded but inert — the
//! confinement is unconditional; the interior effect blends against the
//! layer-over-backdrop result rather than an isolated content buffer, so
//! `Blend Interior Effects As Group` is not modelled; contour (`TrnS`), noise
//! (`Nose`) and anti-alias (`AntA`) are ignored; and the effective angle is the
//! stored `lagl`, not the global-light resource.
//!
//! An inner glow also composites **above** the content: the choke-eroded content
//! matte is blurred and turned into an interior field. ponytail: `Source =
//! Center` lights the interior far from any edge (the libpsd complement-of-edge
//! approximation), not a bounded radius-`size` blob measured from the centroid;
//! `choke` maps to a min-filter erode of radius `round(choke / 100 · size)`
//! rather than libpsd's blur-then-edge-find (the documented 50 % piecewise
//! behavior is not modelled); `Precise` renders as `Softer`; contour (`TrnS`),
//! noise (`Nose`), the `Range` (`Inpr`) remap and anti-alias (`AntA`) are
//! ignored; and the interior effect blends against the layer-over-backdrop
//! result rather than an isolated content buffer.
//!
//! A stroke (`FrFX`) also composites **above** the content: an exact
//! integer-width band at the content edge, outside, inside or straddling it,
//! filled from the stroke's `fill` — a flat colour, a gradient (over the layer
//! rect when aligned, else the canvas) or a pattern (tiled, anchored to the
//! layer rect when linked, else the canvas origin). ponytail: the aligned
//! gradient clamps at the layer-rect edge and the pattern is anchored to the
//! layer rect (design D4), an approximation; gradient noise/`Dither`/`Ofst` and
//! stop midpoints, and pattern `Angl` rotation, are not applied; contour
//! (`TrnS`), anti-alias (`AntA`) and `overprint` are ignored; the solid colour
//! defaults to black; and the exact inter-effect order among the
//! above-content effects is not modelled.
//!
//! The overlays (`SoFi` Color, `GrFl` Gradient, `patternFill` Pattern) also
//! composite **above** the content: they fill the masked content coverage `M`
//! with a source (a flat colour, the gradient geometry or the document pattern)
//! at alpha `M · source_alpha · opacity/100`. ponytail: the effect object class
//! ids are `SoFi`/`GrFl`/`patternFill`, not the `SoCo`/`PtFl` fill-layer ids;
//! gradient `Ofst`, noise and `Dither` are not modelled; pattern `Angl` rotation
//! is decoded but not applied; the unaligned gradient buffer is canvas-sized.
//!
//! A satin (`ChFX`) composites **above** the content: the blurred content matte
//! is differenced against itself shifted by `distance` along `angle`, and the
//! absolute difference tints the interior band. ponytail: contour (`MpgS`),
//! anti-alias (`AntA`), `uglg` and `showInDialog` are ignored; the effective
//! angle is the stored `lagl`, not the global-light resource; the band is
//! confined to `M` once rather than libpsd's extra knockout multiplication; the
//! build region pads by `dist_reach + blur_support` rather than `size`; the
//! exact inter-effect order among the above-content effects is not modelled.
//!
//! A bevel & emboss (`ebbl`) also composites **above** the content: the content
//! matte `M` is blurred by `size` into a height field, its central-difference
//! normal is lit from `Angle`/`Altitude`, and the signed shading tints the
//! interior. ponytail: only the `Inner` style with the `Smooth` technique
//! renders; the chisel techniques, the `Outer`/`Emboss`/`Pillow`/`Stroke`
//! styles, contour (`MpgS`), gloss contour (`TrnS`), contour range (`Inpr`),
//! anti-alias (`AntA`/`antialiasGloss`), texture, `useShape` and `showInDialog`
//! are decoded/ignored; the effective angle/altitude is the stored `lagl`/`Lald`,
//! not the global-light resource; the height profile is a Gaussian blur of `M`
//! rather than the reference's distance transform; `scale = size · depth/100` and the
//! `dot(N,L) - sin(alt)` flat-offset are ungrounded model choices; the build
//! region pads by the blur supports only; the exact inter-effect and
//! highlight/shadow order are not modelled.
//!
//! The legacy `lrFX` block is decoded by [`legacy`] and exposed through the
//! single [`decode_layer_effects`] resolver: an `lfx2` block is authoritative
//! when present, and the legacy block supplies the mapped set only when `lfx2`
//! is absent, so the two are never combined or double-applied.

use pictura_adjust::Adjustment;
use pictura_codec::DescValue;
use pictura_core::mask_ops::{self, Border};
use pictura_core::{BlendMode, Document, Layer, PixelBuffer, PsdRect};

use crate::composite::{channel, Canvas};

mod bevel;
mod glows;
mod legacy;
mod overlays;
mod planes;
mod satin;
mod shadows;
mod strokes;

pub(crate) use legacy::{decode_legacy_effects, LegacyEffects};
pub(crate) use planes::EffectPass;

pub use bevel::{
    decode_bevel_emboss, BevelDirection, BevelEmboss, BevelHighlight, BevelShadow, BevelStyle,
    BevelTechnique,
};
pub use glows::{decode_inner_glow, decode_outer_glow, GlowSource, InnerGlow, OuterGlow};
pub use overlays::{
    decode_color_overlay, decode_gradient_overlay, decode_pattern_overlay, ColorOverlay,
    GradientOverlay, PatternOverlay,
};
pub use satin::{decode_satin, Satin};
pub use shadows::{decode_drop_shadow, decode_inner_shadow, DropShadow, InnerShadow};
pub use strokes::{decode_stroke, Stroke, StrokeFill, StrokePosition};

/// Documented Drop Shadow parameter caps (`docs/05-layers/layer-styles.md`).
const MAX_OPACITY: f32 = 100.0;
const MAX_DISTANCE: f32 = 30_000.0;
const MAX_SPREAD: f32 = 100.0;
const MAX_CHOKE: f32 = 100.0;
const MAX_SIZE: f32 = 250.0;
/// Documented Bevel & Emboss caps (`docs/05-layers/layer-styles.md`).
const MAX_DEPTH: f32 = 1000.0;
const MAX_ALTITUDE: f32 = 90.0;

/// The blur technique stored in `GlwT` (typeID `BETE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowTechnique {
    /// `SfBL`: the ordinary Gaussian blur.
    Softer,
    /// `PrBL`: the reference's distance-measure technique, rendered as `Softer`
    /// (a stated ceiling).
    Precise,
}

/// Read a numeric key as `f32`. A value that is non-finite as an `f64`, or
/// finite as an `f64` but overflows to infinity as an `f32`, is rejected.
fn num_or(obj: &DescValue, key: &[u8], default: f32) -> Option<f32> {
    match crate::composite::desc_item(obj, key) {
        None => Some(default),
        Some(DescValue::UnitFloat { value, .. }) if value.is_finite() => finite_f32(*value),
        Some(DescValue::Double(v)) if v.is_finite() => finite_f32(*v),
        Some(_) => None,
    }
}

fn finite_f32(value: f64) -> Option<f32> {
    let v = value as f32;
    v.is_finite().then_some(v)
}

/// Read a numeric key and clamp it to its documented range. Out-of-range but
/// finite values are clamped; non-finite values reject the whole effect.
fn num_clamped(obj: &DescValue, key: &[u8], default: f32, min: f32, max: f32) -> Option<f32> {
    num_or(obj, key, default).map(|v| v.clamp(min, max))
}

fn bool_or(obj: &DescValue, key: &[u8], default: bool) -> Option<bool> {
    match crate::composite::desc_item(obj, key) {
        None => Some(default),
        Some(DescValue::Bool(b)) => Some(*b),
        Some(_) => None,
    }
}

fn decode_color(value: &DescValue) -> Option<[u8; 3]> {
    let DescValue::Object { class_id, .. } = value else {
        return None;
    };
    if class_id.as_slice() != b"RGBC" {
        return None;
    }
    let component = |key: &[u8]| match crate::composite::desc_item(value, key) {
        Some(DescValue::Double(v)) if v.is_finite() => Some(v.round().clamp(0.0, 255.0) as u8),
        Some(DescValue::UnitFloat { value, .. }) if value.is_finite() => {
            Some(value.round().clamp(0.0, 255.0) as u8)
        }
        _ => None,
    };
    Some([
        component(b"Rd  ")?,
        component(b"Grn ")?,
        component(b"Bl  ")?,
    ])
}

/// Clone a `GrFl`/`GdFl` object, injecting the overlay defaults the shared
/// helper requires: an absent `Angl` becomes `0` and an absent `Type` becomes
/// the `GrdT`/`Lnr ` Linear enum. A present but malformed key is left as-is so
/// the helper still rejects it. This is overlay/stroke-layer only; the strict
/// `GdFl` contract keeps requiring both keys.
pub(crate) fn with_gradient_defaults(obj: &DescValue) -> DescValue {
    let DescValue::Object {
        name,
        class_id,
        items,
    } = obj
    else {
        return obj.clone();
    };
    let mut items = items.clone();
    if crate::composite::desc_item(obj, b"Angl").is_none() {
        items.push((b"Angl".to_vec(), DescValue::Double(0.0)));
    }
    if crate::composite::desc_item(obj, b"Type").is_none() {
        items.push((
            b"Type".to_vec(),
            DescValue::Enum {
                kind: b"GrdT".to_vec(),
                value: b"Lnr ".to_vec(),
            },
        ));
    }
    DescValue::Object {
        name: name.clone(),
        class_id: class_id.clone(),
        items,
    }
}

/// Map an `lfx2` effect blend-mode value (typeID `BlnM`) to a [`BlendMode`],
/// falling back to the effect's `default` for an unknown or empty value.
///
/// The `BlnM` descriptor vocabulary is **capitalized** (`Nrml`/`Mltp`/`Scrn`)
/// and is *not* the layer blend key (`norm`/`mul `/`scrn`), so
/// [`BlendMode::from_psd_key`] must not be used for effect modes. The extended
/// codes below are accepted best-effort (ag-psd / libpsd write them), as are
/// the string ids Photoshop writes for the newer modes (`linearBurn`,
/// `blendSubtraction`, …).
pub(crate) fn effect_blend_mode(value: &[u8], default: BlendMode) -> BlendMode {
    match value {
        b"Nrml" => BlendMode::Normal,
        b"Dslv" => BlendMode::Dissolve,
        b"Drkn" => BlendMode::Darken,
        b"Mltp" => BlendMode::Multiply,
        b"CBrn" => BlendMode::ColorBurn,
        b"Lghn" => BlendMode::Lighten,
        b"Scrn" => BlendMode::Screen,
        b"CDdg" => BlendMode::ColorDodge,
        b"Ovrl" => BlendMode::Overlay,
        b"SftL" => BlendMode::SoftLight,
        b"HrdL" => BlendMode::HardLight,
        b"Dfrn" => BlendMode::Difference,
        b"Xclu" => BlendMode::Exclusion,
        b"H   " => BlendMode::Hue,
        b"Strt" => BlendMode::Saturation,
        b"Clr " => BlendMode::Color,
        b"Lmns" => BlendMode::Luminosity,
        b"Sbtr" => BlendMode::Subtract,
        b"vLit" => BlendMode::VividLight,
        b"lLit" => BlendMode::LinearLight,
        b"pLit" => BlendMode::PinLight,
        b"hMix" => BlendMode::HardMix,
        b"lbrn" => BlendMode::LinearBurn,
        b"lddg" => BlendMode::LinearDodge,
        b"fsub" => BlendMode::Subtract,
        b"fdiv" => BlendMode::Divide,
        b"dkCl" => BlendMode::DarkerColor,
        b"lgCl" => BlendMode::LighterColor,
        b"linearBurn" => BlendMode::LinearBurn,
        b"linearDodge" => BlendMode::LinearDodge,
        b"darkerColor" => BlendMode::DarkerColor,
        b"lighterColor" => BlendMode::LighterColor,
        b"vividLight" => BlendMode::VividLight,
        b"linearLight" => BlendMode::LinearLight,
        b"pinLight" => BlendMode::PinLight,
        b"hardMix" => BlendMode::HardMix,
        b"blendSubtraction" => BlendMode::Subtract,
        b"blendDivide" => BlendMode::Divide,
        _ => default,
    }
}

/// The ten typed effects resolved for one layer, from whichever encoding the
/// layer carries.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct LayerEffects {
    pub drop_shadow: Option<DropShadow>,
    pub outer_glow: Option<OuterGlow>,
    pub inner_shadow: Option<InnerShadow>,
    pub inner_glow: Option<InnerGlow>,
    pub bevel: Option<BevelEmboss>,
    pub satin: Option<Satin>,
    pub stroke: Option<Stroke>,
    pub color_overlay: Option<ColorOverlay>,
    pub gradient_overlay: Option<GradientOverlay>,
    pub pattern_overlay: Option<PatternOverlay>,
}

impl LayerEffects {
    /// The shipped `lfx2` decoders, one per effect.
    fn from_lfx2(layer: &Layer) -> Self {
        Self {
            drop_shadow: decode_drop_shadow(layer),
            outer_glow: decode_outer_glow(layer),
            inner_shadow: decode_inner_shadow(layer),
            inner_glow: decode_inner_glow(layer),
            bevel: decode_bevel_emboss(layer),
            satin: decode_satin(layer),
            stroke: decode_stroke(layer),
            color_overlay: decode_color_overlay(layer),
            gradient_overlay: decode_gradient_overlay(layer),
            pattern_overlay: decode_pattern_overlay(layer),
        }
    }

    /// The legacy `lrFX` mapping; the effects with no legacy record stay absent.
    fn from_legacy(legacy: LegacyEffects) -> Self {
        Self {
            drop_shadow: legacy.drop_shadow,
            outer_glow: legacy.outer_glow,
            inner_shadow: legacy.inner_shadow,
            inner_glow: legacy.inner_glow,
            bevel: legacy.bevel,
            satin: None,
            stroke: None,
            color_overlay: legacy.color_overlay,
            gradient_overlay: None,
            pattern_overlay: None,
        }
    }
}

/// Resolve a layer's effects once. A present `lfx2` block is authoritative and
/// ignores the legacy block entirely; otherwise the mapped legacy `lrFX` set is
/// used. Both are never combined. An `lfx2` whose `masterFXSwitch` is off
/// (Hide All Effects) draws nothing.
pub(crate) fn decode_layer_effects(layer: &Layer) -> LayerEffects {
    if let Some(block) = layer.extra_block(b"lfx2") {
        if !master_switch_on(&block.data) {
            return LayerEffects::default();
        }
        LayerEffects::from_lfx2(layer)
    } else {
        decode_legacy_effects(layer)
            .map(LayerEffects::from_legacy)
            .unwrap_or_default()
    }
}

/// The `lfx2` master switch; absent or unreadable counts as on.
fn master_switch_on(data: &[u8]) -> bool {
    let Some(obj) = data
        .get(4..)
        .and_then(|body| pictura_codec::read_descriptor(body).ok())
    else {
        return true;
    };
    !matches!(
        crate::composite::desc_item(&obj, b"masterFXSwitch"),
        Some(DescValue::Bool(false))
    )
}

pub(crate) fn effects_reach(layer: &Layer) -> PsdRect {
    let rect = layer.rect;
    let pixels = !layer.channels.is_empty();
    let (l, t, r, b) = match layer.vector_mask.as_ref().filter(|mask| mask.has_fill()) {
        Some(mask) if !mask.invert => {
            let points = mask
                .subpaths
                .iter()
                .filter(|s| s.closed)
                .flat_map(|s| s.points.iter());
            let (l, t, r, b) = points.fold(
                (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
                |(l, t, r, b), p| (l.min(p[0]), t.min(p[1]), r.max(p[0]), b.max(p[1])),
            );
            let mask_box = (
                l.div_euclid(256),
                t.div_euclid(256),
                r / 256 + 1,
                b / 256 + 1,
            );
            if pixels {
                (
                    mask_box.0.max(rect.left),
                    mask_box.1.max(rect.top),
                    mask_box.2.min(rect.right),
                    mask_box.3.min(rect.bottom),
                )
            } else {
                mask_box
            }
        }
        _ => (rect.left, rect.top, rect.right, rect.bottom),
    };
    let effects = crate::layer_source::decoded_effects(layer);
    let mut reach = 0.0f32;
    if let Some(shadow) = effects.drop_shadow.as_ref() {
        reach = reach.max(
            shadow.distance.abs() + spread_pad(shadow.spread as f64, shadow.size as f64) as f32,
        );
    }
    if let Some(glow) = effects.outer_glow.as_ref() {
        reach = reach.max(spread_pad(glow.spread as f64, glow.size as f64) as f32);
    }
    if let Some(stroke) = effects.stroke.as_ref() {
        reach = reach.max(stroke.size as f32);
    }
    if let Some(bevel) = effects.bevel.as_ref() {
        reach = reach.max(bevel::reach(bevel));
    }
    let pad = if reach > 0.0 {
        reach.min(1.0e6).ceil() as i32 + 2
    } else {
        0
    };
    PsdRect {
        top: t - pad,
        left: l - pad,
        bottom: b + pad,
        right: r + pad,
    }
}

/// The document-space box a Layer Style change on `path` can affect, or `None`
/// for a missing path or an empty group. A pixel, smart-object, type, or fill
/// layer is its rect unioned with its effect reach; a group is its descendants'
/// union; an adjustment layer is the document canvas.
pub fn layer_paint_bounds(doc: &Document, path: &str) -> Option<PsdRect> {
    paint_bounds(crate::resolve_path(doc, path)?, doc)
}

fn paint_bounds(layer: &Layer, doc: &Document) -> Option<PsdRect> {
    if layer.is_group {
        return layer
            .children
            .iter()
            .filter_map(|child| paint_bounds(child, doc))
            .reduce(union_rect);
    }
    if is_destructive_adjustment(layer) {
        // A zero-default raster mask confines the adjustment to its rect.
        if let Some(mask) = layer.mask.as_ref() {
            if !mask.disabled
                && mask.data.is_some()
                && mask.default_color == 0
                && mask.rect.width() > 0
                && mask.rect.height() > 0
            {
                return Some(mask.rect);
            }
        }
        return Some(PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height.min(i32::MAX as u32) as i32,
            right: doc.width.min(i32::MAX as u32) as i32,
        });
    }
    let reach = effects_reach(layer);
    let rect = PsdRect {
        left: layer.rect.left.min(reach.left),
        top: layer.rect.top.min(reach.top),
        right: layer.rect.right.max(reach.right),
        bottom: layer.rect.bottom.max(reach.bottom),
    };
    (!rect_empty((rect.left, rect.top, rect.right, rect.bottom))).then_some(rect)
}

fn union_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

/// Replay the already-built below-content pass group.
pub(crate) fn composite_layer_effects(
    canvas: &mut Canvas,
    passes: Option<&crate::layer_source::LayerPasses>,
) {
    if let Some(passes) = passes {
        planes::apply(canvas, &passes.below);
    }
}

/// Replay the already-built above-content pass group.
pub(crate) fn composite_layer_effects_above(
    canvas: &mut Canvas,
    passes: Option<&crate::layer_source::LayerPasses>,
) {
    if let Some(passes) = passes {
        planes::apply(canvas, &passes.above);
    }
}

/// Build every enabled pass group in draw order. Called once per layer per
/// composite by the source cache.
pub(crate) fn build_layer_passes(
    layer: &Layer,
    doc: &Document,
    effects: &LayerEffects,
) -> crate::layer_source::LayerPasses {
    let mut out = crate::layer_source::LayerPasses::default();
    if let Some(shadow) = effects
        .drop_shadow
        .as_ref()
        .filter(|effect| effect.enabled && effect.present)
    {
        out.below
            .extend(shadows::build_drop_shadow(layer, doc, shadow));
    }
    if let Some(glow) = effects
        .outer_glow
        .as_ref()
        .filter(|effect| effect.enabled && effect.present)
    {
        out.below.extend(glows::build_outer_glow(layer, doc, glow));
    }
    let on = |enabled: bool, present: bool| enabled && present;
    if let Some(overlay) = effects
        .pattern_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above
            .extend(overlays::build_pattern_overlay(layer, doc, overlay));
    }
    if let Some(overlay) = effects
        .gradient_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above
            .extend(overlays::build_gradient_overlay(layer, doc, overlay));
    }
    if let Some(overlay) = effects
        .color_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above
            .extend(overlays::build_color_overlay(layer, doc, overlay));
    }
    if let Some(satin) = effects
        .satin
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above.extend(satin::build_satin(layer, doc, satin));
    }
    if let Some(glow) = effects
        .inner_glow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above.extend(glows::build_inner_glow(layer, doc, glow));
    }
    if let Some(shadow) = effects
        .inner_shadow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above
            .extend(shadows::build_inner_shadow(layer, doc, shadow));
    }
    if let Some(stroke) = effects
        .stroke
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above.extend(strokes::build_stroke(layer, doc, stroke));
    }
    if let Some(bevel) = effects
        .bevel
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        out.above
            .extend(bevel::build_bevel_emboss(layer, doc, bevel));
    }
    out
}

/// Whether the layer has no drawable pass group, so the cache can skip it.
pub(crate) fn no_passes(layer: &Layer, effects: &LayerEffects) -> bool {
    if layer.is_group || is_destructive_adjustment(layer) {
        return true;
    }
    let on = |enabled: bool, present: bool| enabled && present;
    let shadow = effects
        .drop_shadow
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let outer = effects
        .outer_glow
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let inner_shadow = effects
        .inner_shadow
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let inner_glow = effects
        .inner_glow
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let bevel = effects
        .bevel
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let satin = effects
        .satin
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let stroke = effects
        .stroke
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let color = effects
        .color_overlay
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let gradient = effects
        .gradient_overlay
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    let pattern = effects
        .pattern_overlay
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present));
    !(shadow
        || outer
        || inner_shadow
        || inner_glow
        || bevel
        || satin
        || stroke
        || color
        || gradient
        || pattern)
}

/// Whether a pass build reads the document's pattern library, which this
/// change does not key.
pub(crate) fn reads_pattern(layer: &Layer, effects: &LayerEffects) -> bool {
    if matches!(
        crate::fill::decode_layer_fill(layer),
        Some(Adjustment::PatternFill(_))
    ) {
        return true;
    }
    let on = |enabled: bool, present: bool| enabled && present;
    if effects
        .pattern_overlay
        .as_ref()
        .is_some_and(|effect| on(effect.enabled, effect.present))
    {
        return true;
    }
    effects.stroke.as_ref().is_some_and(|stroke| {
        on(stroke.enabled, stroke.present)
            && matches!(&stroke.fill, strokes::StrokeFill::Pattern { .. })
    })
}

/// Test-only: run a hand-built satin (bypassing decode) over a fresh canvas so
/// the composite's own non-finite clamps are exercised.
#[cfg(test)]
pub(crate) fn composite_satin_for_test(
    layer: &Layer,
    doc: &Document,
    satin: &Satin,
) -> PixelBuffer {
    let mut canvas = Canvas::new(doc.width as usize, doc.height as usize);
    let passes = satin::build_satin(layer, doc, satin);
    planes::apply(&mut canvas, &passes);
    canvas.into_pixel_buffer()
}

/// Test-only: run a hand-built bevel (bypassing decode) over a fresh canvas so
/// the composite's own non-finite clamps are exercised.
#[cfg(test)]
pub(crate) fn composite_bevel_emboss_for_test(
    layer: &Layer,
    doc: &Document,
    bevel: &BevelEmboss,
) -> PixelBuffer {
    let mut canvas = Canvas::new(doc.width as usize, doc.height as usize);
    let passes = bevel::build_bevel_emboss(layer, doc, bevel);
    planes::apply(&mut canvas, &passes);
    canvas.into_pixel_buffer()
}

/// A layer whose content is a destructive adjustment (not fill content) has no
/// matte; its effect is deferred.
pub(crate) fn is_destructive_adjustment(layer: &Layer) -> bool {
    match layer.adjustment.as_ref().and_then(crate::decode_adjustment) {
        Some(
            Adjustment::SolidFill(_) | Adjustment::GradientFill(_) | Adjustment::PatternFill(_),
        ) => false,
        Some(_) => true,
        None => false,
    }
}

/// The effect `Noise` grain at canvas `(x, y)`: a factor on the effect's alpha
/// that is 1 at 0 % and breaks the effect into a fixed speckle as it rises.
///
/// ponytail: Adobe's noise model is unpublished; this is a seeded per-pixel
/// dither, `clamp(1 + 2·n·(2r − 1), 0, 1)` for `r` uniform in `[0, 1)`.
fn noise_factor(noise: f32, x: i32, y: i32) -> f32 {
    let n = clamp_finite(noise, 100.0) / 100.0;
    if n <= 0.0 || x < 0 || y < 0 {
        return 1.0;
    }
    let r = crate::composite_rows::dissolve_noise(x as usize, y as usize);
    (1.0 + 2.0 * n * (2.0 * r - 1.0)).clamp(0.0, 1.0)
}

/// A non-negative, finite value no greater than `max` (`0.0` when non-finite).
fn clamp_finite(value: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, max)
    } else {
        0.0
    }
}

/// The layer's content alpha over the canvas region `(x0, y0, x1, y1)`: a pixel
/// layer's `-1` alpha, a fill's payload alpha, or `1.0` inside the rect of an
/// embedded smart source.
fn content_matte(layer: &Layer, doc: &Document, region: (i32, i32, i32, i32)) -> Vec<f32> {
    let (x0, y0, x1, y1) = region;
    let rw = (x1 - x0) as usize;
    let rh = (y1 - y0) as usize;
    let mut matte = vec![0.0f32; rw * rh];
    if crate::fill::fill_coverage_matte(layer, doc, region, &mut matte) {
        return matte;
    }
    // Pixel/smart content exists only inside the layer rect, intersected here.
    let ix0 = layer.rect.left.max(x0);
    let iy0 = layer.rect.top.max(y0);
    let ix1 = layer.rect.right.min(x1);
    let iy1 = layer.rect.bottom.min(y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return matte;
    }
    let at = |x: i32, y: i32| (y - y0) as usize * rw + (x - x0) as usize;
    match channel(layer, -1) {
        Some(alpha) => {
            let lw = layer.rect.width();
            if lw <= 0 {
                return matte;
            }
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    let li = (y - layer.rect.top) as usize * lw as usize
                        + (x - layer.rect.left) as usize;
                    // A missing alpha sample is opaque, matching the content path.
                    matte[at(x, y)] = alpha.get(li).copied().unwrap_or(255) as f32 / 255.0;
                }
            }
        }
        None => {
            if !smart_source_matte(layer, region, &mut matte) {
                // No `-1` channel and no readable source: the content path treats
                // the layer as opaque, and so does the matte.
                for y in iy0..iy1 {
                    for x in ix0..ix1 {
                        matte[at(x, y)] = 1.0;
                    }
                }
            }
        }
    }
    matte
}

/// Fill `matte` from an embedded smart object's rendered alpha over `region`.
/// `false` when the layer has no renderable embedded source, so the caller
/// keeps the opaque-rectangle fallback.
fn smart_source_matte(layer: &Layer, region: (i32, i32, i32, i32), matte: &mut [f32]) -> bool {
    // ponytail: the embedded source is decoded once per effect; cache the
    // rendered alpha if a profile shows it.
    let Some(so) = layer.smart_object.as_ref() else {
        return false;
    };
    let (x0, y0, x1, y1) = region;
    let rw = (x1 - x0) as usize;
    let ix0 = layer.rect.left.max(x0);
    let iy0 = layer.rect.top.max(y0);
    let ix1 = layer.rect.right.min(x1);
    let iy1 = layer.rect.bottom.min(y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return false;
    }
    let sub = PsdRect {
        top: iy0,
        left: ix0,
        bottom: iy1,
        right: ix1,
    };
    let Some(source) = crate::composite::render_smart_source(so, layer.rect, sub) else {
        return false;
    };
    let sw = (ix1 - ix0) as usize;
    let sh = (iy1 - iy0) as usize;
    if source.channels != 4 || source.data.len() < sw * sh * 4 {
        return false;
    }
    let alpha = &source.data[3 * sw * sh..];
    for y in iy0..iy1 {
        for x in ix0..ix1 {
            let si = (y - iy0) as usize * sw + (x - ix0) as usize;
            matte[(y - y0) as usize * rw + (x - x0) as usize] = alpha[si] as f32 / 255.0;
        }
    }
    true
}

/// The layer rect clamped to the canvas as `(x0, y0, x1, y1)`; empty when the
/// rect does not intersect the canvas.
fn clip_rect(layer: &Layer, w: i32, h: i32) -> (i32, i32, i32, i32) {
    (
        layer.rect.left.max(0),
        layer.rect.top.max(0),
        layer.rect.right.min(w),
        layer.rect.bottom.min(h),
    )
}

fn rect_empty(r: (i32, i32, i32, i32)) -> bool {
    r.2 <= r.0 || r.3 <= r.1
}

/// Pad a rect by `pad` on every side and clamp it to the canvas.
fn pad_rect(r: (i32, i32, i32, i32), pad: i64, w: i32, h: i32) -> (i32, i32, i32, i32) {
    let pad = pad as i32;
    (
        (r.0 - pad).max(0),
        (r.1 - pad).max(0),
        (r.2 + pad).min(w),
        (r.3 + pad).min(h),
    )
}

/// The Spread dilation a drop shadow or glow applies before its blur.
pub(super) fn spread_dilate(spread: f64, size: f64) -> i64 {
    (spread / 100.0 * size).round() as i64
}

/// The pixels a Gaussian blur of `size` can spread past its source.
pub(super) fn blur_support(size: f64) -> i64 {
    if size > 0.0 {
        (3.0 * pictura_filters::kernel::sigma_from_radius(size)).ceil() as i64
    } else {
        0
    }
}

/// The full padding a spread-and-blur matte needs beyond its source.
pub(super) fn spread_pad(spread: f64, size: f64) -> i64 {
    spread_dilate(spread, size) + blur_support(size)
}

/// Grayscale dilation (max filter) of radius `r`, clamp-to-edge.
/// Spread expands the matte before the blur, up to a hard edge at 100 %.
fn dilate_matte(src: &[f32], w: usize, h: usize, r: i64) -> Vec<f32> {
    mask_ops::dilate(src, w, h, r.max(0) as usize, Border::Clamp)
}

/// Grayscale erosion (min filter) of radius `r`, clamp-to-edge.
/// Choke shrinks the inverted matte before the blur, narrowing the shadow.
fn erode_matte(src: &[f32], w: usize, h: usize, r: i64) -> Vec<f32> {
    mask_ops::erode(src, w, h, r.max(0) as usize, Border::Clamp)
}

/// Gaussian-blur the matte with the crate blur, carrying the quantized matte in
/// all three planes of a `PixelBuffer`. `size <= 0` is a no-op.
fn blur_matte(src: &[f32], w: usize, h: usize, size: f64) -> Vec<f32> {
    if !size.is_finite() || size <= 0.0 {
        return src.to_vec();
    }
    let n = w * h;
    let mut buf = PixelBuffer::new(w as u32, h as u32, 3);
    for (i, &v) in src.iter().enumerate() {
        let q = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        buf.data[i] = q;
        buf.data[n + i] = q;
        buf.data[2 * n + i] = q;
    }
    if pictura_filters::blur::gaussian(&mut buf, size).is_err() {
        return src.to_vec();
    }
    buf.data[..n].iter().map(|&b| b as f32 / 255.0).collect()
}

#[cfg(test)]
mod matte_differential {
    use super::{dilate_matte, erode_matte};

    fn axis(
        src: &[f32],
        w: usize,
        h: usize,
        r: isize,
        horizontal: bool,
        want_max: bool,
    ) -> Vec<f32> {
        let mut out = vec![0.0f32; src.len()];
        if horizontal {
            for y in 0..h {
                for x in 0..w {
                    let lo = (x as isize - r).max(0) as usize;
                    let hi = (x as isize + r).min(w as isize - 1) as usize;
                    let mut v = if want_max {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    };
                    for k in lo..=hi {
                        v = if want_max {
                            v.max(src[y * w + k])
                        } else {
                            v.min(src[y * w + k])
                        };
                    }
                    out[y * w + x] = v;
                }
            }
        } else {
            for x in 0..w {
                for y in 0..h {
                    let lo = (y as isize - r).max(0) as usize;
                    let hi = (y as isize + r).min(h as isize - 1) as usize;
                    let mut v = if want_max {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    };
                    for k in lo..=hi {
                        v = if want_max {
                            v.max(src[k * w + x])
                        } else {
                            v.min(src[k * w + x])
                        };
                    }
                    out[y * w + x] = v;
                }
            }
        }
        out
    }

    fn naive(src: &[f32], w: usize, h: usize, r: isize, want_max: bool) -> Vec<f32> {
        let horizontal = axis(src, w, h, r, true, want_max);
        axis(&horizontal, w, h, r, false, want_max)
    }

    #[test]
    fn spread_and_choke_mattes_match_the_pre_migration_scan() {
        let (w, h) = (37usize, 29usize);
        let mut state = 0x5eed_1234u64;
        let mut src = vec![0.0f32; w * h];
        for value in &mut src {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *value = (state % 1000) as f32 / 1000.0;
        }
        for r in [0isize, 1, 3, 10, 50] {
            assert_eq!(
                dilate_matte(&src, w, h, r as i64),
                naive(&src, w, h, r, true),
                "dilate r={r}"
            );
            assert_eq!(
                erode_matte(&src, w, h, r as i64),
                naive(&src, w, h, r, false),
                "erode r={r}"
            );
        }
    }
}
