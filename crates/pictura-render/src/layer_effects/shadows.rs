use pictura_adjust::Adjustment;
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer, PsdRect};

use crate::composite::desc_item;

use super::apply_mask;
use super::planes::{EffectPass, PassWriter};
use super::BuildRegion;
use super::{
    blur_matte, blur_support, bool_or, clamp_finite, clip_rect, content_matte, decode_color,
    dilate_matte, effect_blend_mode, erode_matte, noise_factor, num_clamped, num_or, pad_rect,
    rect_empty, spread_dilate, MAX_CHOKE, MAX_DISTANCE, MAX_OPACITY, MAX_SIZE, MAX_SPREAD,
};

/// The typed drop shadow decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
    /// The stored local angle in degrees (`lagl`).
    pub angle_deg: f32,
    /// Pixels, `0..=30000`.
    pub distance: f32,
    /// Percent, `0..=100` (`Ckmt`, the reference's stored key for Spread).
    pub spread: f32,
    /// Pixels, Gaussian radius, `0..=250`.
    pub size: f32,
    pub use_global_angle: bool,
    pub knocks_out: bool,
    /// `Nose`, the grain in percent.
    pub noise: f32,
}

/// The typed inner shadow decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InnerShadow {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
    /// The stored local angle in degrees (`lagl`).
    pub angle_deg: f32,
    /// Pixels, `0..=30000`.
    pub distance: f32,
    /// Percent, `0..=100` (`Ckmt`, the reference's stored key for Choke, an erode).
    pub choke: f32,
    /// Pixels, Gaussian radius, `0..=250`.
    pub size: f32,
    pub use_global_angle: bool,
    /// `layerConceals`, decoded for symmetry with `DropShadow`; no render effect.
    pub knocks_out: bool,
    /// `Nose`, the grain in percent.
    pub noise: f32,
}

/// Decode a layer's `lfx2` Drop Shadow.
///
/// A missing `lfx2`, a missing or wrongly-typed `DrSh`, an unknown data version,
/// a wrong-typed or non-finite value, or a parse error is `None`. Never panics.
pub fn decode_drop_shadow(layer: &Layer) -> Option<DropShadow> {
    let data = &layer.extra_block(b"lfx2")?.data;
    // `DescriptorBlock2`: a `u32` version, then the version-16 descriptor block
    // whose own leading `u32` `read_descriptor` consumes.
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let drsh = desc_item(&obj, b"DrSh")?;
    let DescValue::Object { class_id, .. } = drsh else {
        return None;
    };
    if class_id.as_slice() != b"DrSh" {
        return None;
    }
    let blend_mode = match desc_item(drsh, b"Md  ") {
        None => BlendMode::Normal,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Normal)
        }
        Some(_) => return None,
    };
    let color = match desc_item(drsh, b"Clr ") {
        None => [0, 0, 0],
        Some(value) => decode_color(value)?,
    };
    Some(DropShadow {
        enabled: bool_or(drsh, b"enab", false)?,
        present: bool_or(drsh, b"present", false)?,
        blend_mode,
        color,
        opacity: num_clamped(drsh, b"Opct", 75.0, 0.0, MAX_OPACITY)?,
        angle_deg: num_or(drsh, b"lagl", 120.0)?,
        distance: num_clamped(drsh, b"Dstn", 5.0, 0.0, MAX_DISTANCE)?,
        spread: num_clamped(drsh, b"Ckmt", 0.0, 0.0, MAX_SPREAD)?,
        size: num_clamped(drsh, b"blur", 5.0, 0.0, MAX_SIZE)?,
        use_global_angle: bool_or(drsh, b"uglg", true)?,
        knocks_out: bool_or(drsh, b"layerConceals", true)?,
        noise: num_clamped(drsh, b"Nose", 0.0, 0.0, MAX_OPACITY)?,
    })
}

/// Decode a layer's `lfx2` Inner Shadow.
///
/// A missing `lfx2`, a missing or wrongly-typed `IrSh`, an unknown data version,
/// a wrong-typed or non-finite value, or a parse error is `None`. Never panics.
pub fn decode_inner_shadow(layer: &Layer) -> Option<InnerShadow> {
    let data = &layer.extra_block(b"lfx2")?.data;
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let irsh = desc_item(&obj, b"IrSh")?;
    let DescValue::Object { class_id, .. } = irsh else {
        return None;
    };
    if class_id.as_slice() != b"IrSh" {
        return None;
    }
    let blend_mode = match desc_item(irsh, b"Md  ") {
        None => BlendMode::Multiply,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Multiply)
        }
        Some(_) => return None,
    };
    let color = match desc_item(irsh, b"Clr ") {
        None => [0, 0, 0],
        Some(value) => decode_color(value)?,
    };
    Some(InnerShadow {
        enabled: bool_or(irsh, b"enab", false)?,
        present: bool_or(irsh, b"present", false)?,
        blend_mode,
        color,
        opacity: num_clamped(irsh, b"Opct", 75.0, 0.0, MAX_OPACITY)?,
        angle_deg: num_or(irsh, b"lagl", 120.0)?,
        distance: num_clamped(irsh, b"Dstn", 5.0, 0.0, MAX_DISTANCE)?,
        choke: num_clamped(irsh, b"Ckmt", 0.0, 0.0, MAX_CHOKE)?,
        size: num_clamped(irsh, b"blur", 5.0, 0.0, MAX_SIZE)?,
        use_global_angle: bool_or(irsh, b"uglg", true)?,
        knocks_out: bool_or(irsh, b"layerConceals", true)?,
        noise: num_clamped(irsh, b"Nose", 0.0, 0.0, MAX_OPACITY)?,
    })
}

pub(super) fn build_drop_shadow(
    layer: &Layer,
    doc: &Document,
    shadow: &DropShadow,
    build: BuildRegion,
    fill: Option<&Adjustment>,
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
    // Clamp again: a hand-built `DropShadow` may not have gone through decode.
    let distance = clamp_finite(shadow.distance, MAX_DISTANCE) as f64;
    let angle = if shadow.angle_deg.is_finite() {
        shadow.angle_deg as f64
    } else {
        0.0
    };
    let theta = angle.to_radians();
    let dx = -distance * theta.cos();
    let dy = distance * theta.sin();
    let spread = clamp_finite(shadow.spread, MAX_SPREAD) as f64;
    let size = clamp_finite(shadow.size, MAX_SIZE) as f64;
    let dilate_radius = spread_dilate(spread, size);
    let blur_pad = blur_support(size);
    // Content can only be non-zero inside `source`; dilate and blur spread it by
    // at most `reach`, so the matte is built and processed over `padded` only.
    let ox = dx.round() as i64;
    let oy = dy.round() as i64;
    let padded = pad_rect(source, dilate_radius + blur_pad, w, h);
    let Some(padded) = build.buffer(padded, dilate_radius + blur_pad + ox.abs() + oy.abs()) else {
        return Vec::new();
    };
    let (px0, py0, px1, py1) = padded;
    // The shadow at document `(x, y)` is the matte at `(x - dx, y - dy)`; only
    // the shifted region can be non-zero.
    let cx0 = (px0 as i64 + ox).max(0);
    let cy0 = (py0 as i64 + oy).max(0);
    let cx1 = (px1 as i64 + ox).min(w as i64);
    let cy1 = (py1 as i64 + oy).min(h as i64);
    if cx1 <= cx0 || cy1 <= cy0 {
        return Vec::new();
    }
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded, fill);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
    }
    let dilated = if dilate_radius > 0 {
        dilate_matte(&matte, pw, ph, dilate_radius)
    } else {
        matte
    };
    let blurred = blur_matte(&dilated, pw, ph, size);
    let color = [
        shadow.color[0] as f32 / 255.0,
        shadow.color[1] as f32 / 255.0,
        shadow.color[2] as f32 / 255.0,
    ];
    let opacity = clamp_finite(shadow.opacity, MAX_OPACITY) / 100.0;
    let rect = PsdRect {
        top: cy0 as i32,
        left: cx0 as i32,
        bottom: cy1 as i32,
        right: cx1 as i32,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = PassWriter::constant(rect, shadow.blend_mode, color);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let sx = (x as i64 - ox - px0 as i64) as usize;
            let sy = (y as i64 - oy - py0 as i64) as usize;
            let alpha = blurred[sy * pw + sx] * opacity * noise_factor(shadow.noise, x, y);
            if alpha > 0.0 {
                writer.emit(x, y, color, alpha);
            }
        }
    }
    vec![writer.finish()]
}

pub(super) fn build_inner_shadow(
    layer: &Layer,
    doc: &Document,
    shadow: &InnerShadow,
    build: BuildRegion,
    fill: Option<&Adjustment>,
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
    // Clamp again: a hand-built `InnerShadow` may not have gone through decode.
    let distance = clamp_finite(shadow.distance, MAX_DISTANCE) as f64;
    let angle = if shadow.angle_deg.is_finite() {
        shadow.angle_deg as f64
    } else {
        0.0
    };
    let theta = angle.to_radians();
    let dx = -distance * theta.cos();
    let dy = distance * theta.sin();
    let choke = clamp_finite(shadow.choke, MAX_CHOKE) as f64;
    let size = clamp_finite(shadow.size, MAX_SIZE) as f64;
    let erode_radius = (choke / 100.0 * size).round() as i64;
    let blur_pad = blur_support(size);
    let ox = dx.round() as i64;
    let oy = dy.round() as i64;
    // Content can only be non-zero inside `source`; erode and blur spread the
    // inverted matte by at most `reach`, so it is built over `padded` only.
    let padded = pad_rect(source, erode_radius + blur_pad, w, h);
    let Some(padded) = build.buffer(padded, erode_radius + blur_pad + ox.abs() + oy.abs()) else {
        return Vec::new();
    };
    if rect_empty(padded) {
        return Vec::new();
    }
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded, fill);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
    }
    let inverted: Vec<f32> = matte.iter().map(|&m| 1.0 - m).collect();
    let eroded = if erode_radius > 0 {
        erode_matte(&inverted, pw, ph, erode_radius)
    } else {
        inverted
    };
    let blurred = blur_matte(&eroded, pw, ph, size);
    let color = [
        shadow.color[0] as f32 / 255.0,
        shadow.color[1] as f32 / 255.0,
        shadow.color[2] as f32 / 255.0,
    ];
    let opacity = clamp_finite(shadow.opacity, MAX_OPACITY) / 100.0;
    let rect = PsdRect {
        top: source.1,
        left: source.0,
        bottom: source.3,
        right: source.2,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = PassWriter::constant(rect, shadow.blend_mode, color);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let sx = x as i64 - ox - px0 as i64;
            let sy = y as i64 - oy - py0 as i64;
            let b = if sx >= 0 && sy >= 0 && sx < pw as i64 && sy < ph as i64 {
                blurred[sy as usize * pw + sx as usize]
            } else {
                // Beyond the built region there is no content, so `1 - M = 1`.
                1.0
            };
            let m = matte[(y - py0) as usize * pw + (x - px0) as usize];
            let alpha = m * b * opacity * noise_factor(shadow.noise, x, y);
            if alpha > 0.0 {
                writer.emit(x, y, color, alpha);
            }
        }
    }
    vec![writer.finish()]
}
