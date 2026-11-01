//! The object-based Satin layer effect (`ChFX`, ChromeFX).
//!
//! Satin is a directional interior band: the content matte `M` is blurred by
//! `size`, the blurred field `B` is differenced against itself shifted by
//! `distance` along `angle`, and the absolute difference tints the layer inside
//! `M`, above the content.
//!
//! ponytail: the contour (`MpgS`), anti-alias (`AntA`), `uglg` and
//! `showInDialog` keys are ignored; the effective angle is the stored `lagl`,
//! not the document global-light resource (1037); the band is confined to `M`
//! once rather than libpsd's extra knockout multiplication (which squares `M`);
//! the build region pads by `dist_reach + blur_support` rather than libpsd's
//! `size`; `Scale Effects` and the exact inter-effect order are not modelled.

use pictura_adjust::Adjustment;
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer, PsdRect};

use crate::composite::desc_item;

use super::apply_mask;
use super::planes::{EffectPass, PassWriter};
use super::BuildRegion;
use super::{
    blur_matte, bool_or, clamp_finite, clip_rect, content_matte, decode_color, effect_blend_mode,
    num_clamped, num_or, pad_rect, rect_empty, MAX_DISTANCE, MAX_OPACITY, MAX_SIZE,
};

/// The typed satin decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Satin {
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
    /// Pixels, Gaussian radius, `0..=250`.
    pub size: f32,
    pub invert: bool,
}

/// Decode a layer's `lfx2` Satin (`ChFX`).
///
/// A missing `lfx2`, a missing or wrongly-typed `ChFX`, an unknown data
/// version, a wrong-typed or non-finite value, a wrong `Md  ` typeID, a
/// non-`RGBC` `Clr `, or a parse error is `None`. Never panics.
pub fn decode_satin(layer: &Layer) -> Option<Satin> {
    let data = &layer.extra_block(b"lfx2")?.data;
    if data.len() < 8 {
        return None;
    }
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let chfx = desc_item(&obj, b"ChFX")?;
    let DescValue::Object { class_id, .. } = chfx else {
        return None;
    };
    if class_id.as_slice() != b"ChFX" {
        return None;
    }
    let blend_mode = match desc_item(chfx, b"Md  ") {
        None => BlendMode::Multiply,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Multiply)
        }
        Some(_) => return None,
    };
    let color = match desc_item(chfx, b"Clr ") {
        None => [0, 0, 0],
        Some(value) => decode_color(value)?,
    };
    Some(Satin {
        enabled: bool_or(chfx, b"enab", false)?,
        present: bool_or(chfx, b"present", false)?,
        blend_mode,
        color,
        opacity: num_clamped(chfx, b"Opct", 50.0, 0.0, MAX_OPACITY)?,
        angle_deg: num_or(chfx, b"lagl", 19.0)?,
        distance: num_clamped(chfx, b"Dstn", 11.0, 0.0, MAX_DISTANCE)?,
        size: num_clamped(chfx, b"blur", 14.0, 0.0, MAX_SIZE)?,
        invert: bool_or(chfx, b"Invr", false)?,
    })
}

pub(super) fn build_satin(
    layer: &Layer,
    doc: &Document,
    satin: &Satin,
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
    // Clamp again: a hand-built `Satin` may not have gone through decode.
    let distance = clamp_finite(satin.distance, MAX_DISTANCE) as f64;
    let angle = if satin.angle_deg.is_finite() {
        satin.angle_deg as f64
    } else {
        0.0
    };
    let theta = angle.to_radians();
    let ox = (-distance * theta.cos()).round() as i64;
    let oy = (distance * theta.sin()).round() as i64;
    let size = clamp_finite(satin.size, MAX_SIZE) as f64;
    let blur_support = if size > 0.0 {
        (3.0 * pictura_filters::kernel::sigma_from_radius(size)).ceil() as i64
    } else {
        0
    };
    // Content can only be non-zero inside `source`; the shifted samples reach
    // `|dx| + |dy|` and the blur spreads by `blur_support`, so the matte is
    // built over `padded` only.
    let padded = pad_rect(source, ox.abs() + oy.abs() + blur_support, w, h);
    let Some(padded) = build.buffer(padded, ox.abs() + oy.abs() + blur_support) else {
        return Vec::new();
    };
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded, fill);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
    }
    let blurred = blur_matte(&matte, pw, ph, size);
    let sample = |x: i64, y: i64| -> f32 {
        let sx = x - px0 as i64;
        let sy = y - py0 as i64;
        if sx >= 0 && sy >= 0 && sx < pw as i64 && sy < ph as i64 {
            blurred[sy as usize * pw + sx as usize]
        } else {
            0.0
        }
    };
    let color = [
        satin.color[0] as f32 / 255.0,
        satin.color[1] as f32 / 255.0,
        satin.color[2] as f32 / 255.0,
    ];
    let opacity = clamp_finite(satin.opacity, MAX_OPACITY) / 100.0;
    if opacity <= 0.0 {
        return Vec::new();
    }
    let rect = PsdRect {
        top: source.1,
        left: source.0,
        bottom: source.3,
        right: source.2,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = PassWriter::constant(rect, satin.blend_mode, color);
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let band = (sample(x as i64 - ox, y as i64 - oy)
                - sample(x as i64 + ox, y as i64 + oy))
            .abs()
            .clamp(0.0, 1.0);
            let field = if satin.invert { 1.0 - band } else { band };
            let m = matte[(y - py0) as usize * pw + (x - px0) as usize];
            let alpha = m * field * opacity;
            if alpha > 0.0 {
                writer.emit(x, y, color, alpha);
            }
        }
    }
    vec![writer.finish()]
}
