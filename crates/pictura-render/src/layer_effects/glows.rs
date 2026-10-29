use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer, PsdRect};

use crate::composite::desc_item;

use super::apply_mask;
use super::planes::{EffectPass, PassWriter};
use super::BuildRegion;
use super::{
    blur_matte, blur_support, bool_or, clamp_finite, clip_rect, content_matte, decode_color,
    dilate_matte, effect_blend_mode, erode_matte, noise_factor, num_clamped, pad_rect, rect_empty,
    spread_dilate, GlowTechnique, MAX_CHOKE, MAX_OPACITY, MAX_SIZE, MAX_SPREAD,
};

/// The typed outer glow decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OuterGlow {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
    /// Percent, `0..=100` (`Ckmt`, the reference's stored key for Spread).
    pub spread: f32,
    /// Pixels, Gaussian radius, `0..=250`.
    pub size: f32,
    pub technique: GlowTechnique,
    /// `Nose`, the grain in percent.
    pub noise: f32,
}

/// The inner-glow source stored in `glwS` (typeID `IGSr`; the legacy `IGsr`
/// spelling is also accepted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowSource {
    /// `SrcE`: the glow starts at the content edge and fades inward.
    Edge,
    /// `SrcC`: the glow is strongest away from the content edge.
    Center,
}

/// The typed inner glow decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InnerGlow {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
    /// Percent, `0..=100` (`Ckmt`, the reference's stored key for Choke, an erode).
    pub choke: f32,
    /// Pixels, Gaussian radius, `0..=250`.
    pub size: f32,
    /// `glwS` (typeID `IGSr`), default Edge.
    pub source: GlowSource,
    pub technique: GlowTechnique,
    /// `Nose`, the grain in percent.
    pub noise: f32,
}

/// Decode a layer's `lfx2` Outer Glow.
///
/// A missing `lfx2`, a missing or wrongly-typed `OrGl`, an unknown data version,
/// a wrong-typed or non-finite value, or a parse error is `None`. Never panics.
pub fn decode_outer_glow(layer: &Layer) -> Option<OuterGlow> {
    let data = &layer.extra_block(b"lfx2")?.data;
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let orgl = desc_item(&obj, b"OrGl")?;
    let DescValue::Object { class_id, .. } = orgl else {
        return None;
    };
    if class_id.as_slice() != b"OrGl" {
        return None;
    }
    let blend_mode = match desc_item(orgl, b"Md  ") {
        None => BlendMode::Screen,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Screen)
        }
        Some(_) => return None,
    };
    let color = match desc_item(orgl, b"Clr ") {
        None => [255, 255, 190],
        Some(value) => decode_color(value)?,
    };
    let technique = match desc_item(orgl, b"GlwT") {
        None => GlowTechnique::Softer,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BETE" => {
            if value.as_slice() == b"PrBL" {
                GlowTechnique::Precise
            } else {
                // `SfBL`, or an unknown value, is Softer.
                GlowTechnique::Softer
            }
        }
        Some(_) => return None,
    };
    Some(OuterGlow {
        enabled: bool_or(orgl, b"enab", false)?,
        present: bool_or(orgl, b"present", false)?,
        blend_mode,
        color,
        opacity: num_clamped(orgl, b"Opct", 75.0, 0.0, MAX_OPACITY)?,
        spread: num_clamped(orgl, b"Ckmt", 0.0, 0.0, MAX_SPREAD)?,
        size: num_clamped(orgl, b"blur", 5.0, 0.0, MAX_SIZE)?,
        noise: num_clamped(orgl, b"Nose", 0.0, 0.0, MAX_OPACITY)?,
        technique,
    })
}

/// Decode a layer's `lfx2` Inner Glow.
///
/// A missing `lfx2`, a missing or wrongly-typed `IrGl`, an unknown data version,
/// a wrong-typed or non-finite value, a wrong `GlwT`/`glwS` typeID, or a parse
/// error is `None`. Never panics.
pub fn decode_inner_glow(layer: &Layer) -> Option<InnerGlow> {
    let data = &layer.extra_block(b"lfx2")?.data;
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let irgl = desc_item(&obj, b"IrGl")?;
    let DescValue::Object { class_id, .. } = irgl else {
        return None;
    };
    if class_id.as_slice() != b"IrGl" {
        return None;
    }
    let blend_mode = match desc_item(irgl, b"Md  ") {
        None => BlendMode::Screen,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Screen)
        }
        Some(_) => return None,
    };
    let color = match desc_item(irgl, b"Clr ") {
        None => [255, 255, 255],
        Some(value) => decode_color(value)?,
    };
    let technique = match desc_item(irgl, b"GlwT") {
        None => GlowTechnique::Softer,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BETE" => {
            if value.as_slice() == b"PrBL" {
                GlowTechnique::Precise
            } else {
                // `SfBL`, or an unknown value, is Softer.
                GlowTechnique::Softer
            }
        }
        Some(_) => return None,
    };
    let source = match desc_item(irgl, b"glwS") {
        None => GlowSource::Edge,
        // `IGSr` is the canonical typeID (psd-tools `Type.InnerGlowSource`,
        // libpsd's assertion); `IGsr` is a legacy/mis-authored spelling the
        // current fixture used and is accepted leniently.
        Some(DescValue::Enum { kind, value })
            if kind.as_slice() == b"IGSr" || kind.as_slice() == b"IGsr" =>
        {
            if value.as_slice() == b"SrcC" {
                GlowSource::Center
            } else {
                // `SrcE`, or an unknown value, is Edge.
                GlowSource::Edge
            }
        }
        Some(_) => return None,
    };
    Some(InnerGlow {
        enabled: bool_or(irgl, b"enab", false)?,
        present: bool_or(irgl, b"present", false)?,
        blend_mode,
        color,
        opacity: num_clamped(irgl, b"Opct", 75.0, 0.0, MAX_OPACITY)?,
        choke: num_clamped(irgl, b"Ckmt", 0.0, 0.0, MAX_CHOKE)?,
        size: num_clamped(irgl, b"blur", 5.0, 0.0, MAX_SIZE)?,
        noise: num_clamped(irgl, b"Nose", 0.0, 0.0, MAX_OPACITY)?,
        source,
        technique,
    })
}

pub(super) fn build_outer_glow(
    layer: &Layer,
    doc: &Document,
    glow: &OuterGlow,
    build: BuildRegion,
) -> Vec<EffectPass> {
    let (w, h) = (
        doc.width.min(i32::MAX as u32) as i32,
        doc.height.min(i32::MAX as u32) as i32,
    );
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let source = clip_rect(layer, w, h);
    if rect_empty(source) {
        return Vec::new();
    }
    // Clamp again: a hand-built `OuterGlow` may not have gone through decode.
    let spread = clamp_finite(glow.spread, MAX_SPREAD) as f64;
    let size = clamp_finite(glow.size, MAX_SIZE) as f64;
    let dilate_radius = spread_dilate(spread, size);
    let blur_pad = blur_support(size);
    // Content can only be non-zero inside `source`; dilate and blur spread it by
    // at most `reach`, so the matte is built and processed over `padded` only.
    let padded = pad_rect(source, dilate_radius + blur_pad, w, h);
    let Some(padded) = build.buffer(padded, dilate_radius + blur_pad) else {
        return Vec::new();
    };
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
    }
    // Exterior mask: the glow is knocked out where the content is opaque.
    let exterior: Vec<f32> = matte.iter().map(|&m| 1.0 - m).collect();
    let dilated = if dilate_radius > 0 {
        dilate_matte(&matte, pw, ph, dilate_radius)
    } else {
        matte
    };
    let blurred = blur_matte(&dilated, pw, ph, size);
    let color = [
        glow.color[0] as f32 / 255.0,
        glow.color[1] as f32 / 255.0,
        glow.color[2] as f32 / 255.0,
    ];
    let opacity = clamp_finite(glow.opacity, MAX_OPACITY) / 100.0;
    let rect = PsdRect {
        top: py0,
        left: px0,
        bottom: py1,
        right: px1,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = PassWriter::constant(rect, glow.blend_mode, color);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let i = (y - py0) as usize * pw + (x - px0) as usize;
            let alpha = blurred[i] * exterior[i] * opacity * noise_factor(glow.noise, x, y);
            if alpha > 0.0 {
                writer.emit(x, y, color, alpha);
            }
        }
    }
    vec![writer.finish()]
}

pub(super) fn build_inner_glow(
    layer: &Layer,
    doc: &Document,
    glow: &InnerGlow,
    build: BuildRegion,
) -> Vec<EffectPass> {
    let (w, h) = (
        doc.width.min(i32::MAX as u32) as i32,
        doc.height.min(i32::MAX as u32) as i32,
    );
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let source = clip_rect(layer, w, h);
    if rect_empty(source) {
        return Vec::new();
    }
    // Clamp again: a hand-built `InnerGlow` may not have gone through decode.
    let choke = clamp_finite(glow.choke, MAX_CHOKE) as f64;
    let size = clamp_finite(glow.size, MAX_SIZE) as f64;
    // `size 0` with `choke 0` under Edge would otherwise tint partial-alpha
    // content through `M · (1 - M)`; the documented no-op returns early.
    if size == 0.0 && choke == 0.0 && matches!(glow.source, GlowSource::Edge) {
        return Vec::new();
    }
    let erode_radius = (choke / 100.0 * size).round() as i64;
    let blur_pad = blur_support(size);
    // Content can only be non-zero inside `source`; erode and blur spread the
    // matte by at most `reach`, so it is built and processed over `padded` only.
    let padded = pad_rect(source, erode_radius + blur_pad, w, h);
    let Some(padded) = build.buffer(padded, erode_radius + blur_pad) else {
        return Vec::new();
    };
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
    }
    let blurred = if erode_radius > 0 {
        let anchor = erode_matte(&matte, pw, ph, erode_radius);
        blur_matte(&anchor, pw, ph, size)
    } else {
        blur_matte(&matte, pw, ph, size)
    };
    let color = [
        glow.color[0] as f32 / 255.0,
        glow.color[1] as f32 / 255.0,
        glow.color[2] as f32 / 255.0,
    ];
    let opacity = clamp_finite(glow.opacity, MAX_OPACITY) / 100.0;
    let rect = PsdRect {
        top: source.1,
        left: source.0,
        bottom: source.3,
        right: source.2,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = PassWriter::constant(rect, glow.blend_mode, color);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let i = (y - py0) as usize * pw + (x - px0) as usize;
            let field = match glow.source {
                GlowSource::Edge => 1.0 - blurred[i],
                GlowSource::Center => blurred[i],
            };
            let alpha = matte[i] * field * opacity * noise_factor(glow.noise, x, y);
            if alpha > 0.0 {
                writer.emit(x, y, color, alpha);
            }
        }
    }
    vec![writer.finish()]
}
