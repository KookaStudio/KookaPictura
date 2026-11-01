//! The object-based Bevel & Emboss layer effect (`ebbl`).
//!
//! The masked content matte `M`'s signed Euclidean distance to its edge gives a
//! chamfer height rising over `size` pixels (inside the edge for `Inner`,
//! outside it for `Outer`, straddling it for `Emboss` and `Pillow`), rounded by
//! the technique (`Smooth` by a third of the size, `Chisel Soft` a pixel, `Chisel Hard` not at
//! all). A surface normal is derived from its central-difference gradient
//! scaled by `size · depth/100`, and the Lambertian deviation of that normal from flat
//! (`Angle`/`Altitude`) gives a signed highlight (positive) / shadow (negative)
//! shading, softened by `soften`. The style picks where it shows: `Inner` over
//! `M`, `Outer` over `1 - M` (outside the content), `Emboss` over both, and
//! `Pillow` over `M` with the outside (`1 - M`) shading inverted.
//!
//! ponytail: the technique roundings are model choices, not Adobe's; the `Stroke` style
//! (which needs the Stroke effect's band) renders nothing; edge contour (`MpgS`), gloss
//! contour (`TrnS`), contour range (`Inpr`), anti-alias (`AntA`,
//! `antialiasGloss`), texture (`useTexture`, `InvT`, `Algn`, `Scl `, `Ptrn`),
//! `useShape` and `showInDialog` are decoded/ignored; the effective
//! angle/altitude is the stored `lagl`/`Lald`, not the global-light resource
//! (1037); `scale = size · depth/100` and the `dot(N,L) - sin(alt)`
//! flat-offset are ungrounded model choices; the build region pads by the blur
//! supports only; `Scale Effects`, the exact inter-effect order and the
//! highlight/shadow order are not modelled.

use pictura_adjust::Adjustment;
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer, PsdRect};

use crate::composite::desc_item;

use super::apply_mask;
use super::planes::{EffectPass, PassWriter};
use super::BuildRegion;
use super::{
    blur_support, bool_or, clamp_finite, clip_rect, content_matte, decode_color, effect_blend_mode,
    num_clamped, num_or, pad_rect, rect_empty, MAX_ALTITUDE, MAX_DEPTH, MAX_OPACITY, MAX_SIZE,
};

/// Bevel style (`bvlS`, typeID `BESl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevelStyle {
    Inner,
    Outer,
    Emboss,
    Pillow,
    Stroke,
}

/// Bevel technique (`bvlT`, typeID `bvlT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevelTechnique {
    Smooth,
    ChiselHard,
    ChiselSoft,
}

/// Bevel direction (`bvlD`, typeID `BESs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevelDirection {
    Up,
    Down,
}

/// The highlight half of a bevel (`hglM`/`hglC`/`hglO`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelHighlight {
    pub mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
}

/// The shadow half of a bevel (`sdwM`/`sdwC`/`sdwO`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelShadow {
    pub mode: BlendMode,
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
}

/// The typed bevel & emboss decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelEmboss {
    pub enabled: bool,
    pub present: bool,
    pub style: BevelStyle,
    pub technique: BevelTechnique,
    pub direction: BevelDirection,
    /// Percent, `0..=1000` (`srgR`).
    pub depth: f32,
    /// Pixels, Gaussian radius, `0..=250` (`blur`).
    pub size: f32,
    /// Pixels, `0..=250` (`Sftn`).
    pub soften: f32,
    /// The stored local angle in degrees (`lagl`), unclamped.
    pub angle_deg: f32,
    /// Degrees, `0..=90` (`Lald`).
    pub altitude_deg: f32,
    pub use_global_angle: bool,
    pub highlight: BevelHighlight,
    pub shadow: BevelShadow,
}

/// Read an enum key as its value bytes: absent takes `default`, a wrong typeID
/// rejects. An unknown value is the caller's to map (to its default).
fn enum_bytes(obj: &DescValue, key: &[u8], kind: &[u8], default: &[u8]) -> Option<Vec<u8>> {
    match desc_item(obj, key) {
        None => Some(default.to_vec()),
        Some(DescValue::Enum { kind: k, value }) if k.as_slice() == kind => Some(value.clone()),
        Some(_) => None,
    }
}

/// Decode a layer's `lfx2` Bevel & Emboss (`ebbl`).
///
/// A missing `lfx2`, a missing or wrongly-typed `ebbl`, an unknown data
/// version, a wrong-typed or non-finite value, a wrong enum typeID, a
/// non-`RGBC` colour, or a parse error is `None`. Never panics.
pub fn decode_bevel_emboss(layer: &Layer) -> Option<BevelEmboss> {
    let data = &layer.extra_block(b"lfx2")?.data;
    if data.len() < 8 {
        return None;
    }
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let ebbl = desc_item(&obj, b"ebbl")?;
    let DescValue::Object { class_id, .. } = ebbl else {
        return None;
    };
    if class_id.as_slice() != b"ebbl" {
        return None;
    }
    let style = match enum_bytes(ebbl, b"bvlS", b"BESl", b"InrB")?.as_slice() {
        b"OtrB" => BevelStyle::Outer,
        b"Embs" => BevelStyle::Emboss,
        b"PlEb" => BevelStyle::Pillow,
        b"strokeEmboss" => BevelStyle::Stroke,
        _ => BevelStyle::Inner,
    };
    let technique = match enum_bytes(ebbl, b"bvlT", b"bvlT", b"SfBL")?.as_slice() {
        b"PrBL" => BevelTechnique::ChiselHard,
        b"Slmt" => BevelTechnique::ChiselSoft,
        _ => BevelTechnique::Smooth,
    };
    let direction = match enum_bytes(ebbl, b"bvlD", b"BESs", b"In  ")?.as_slice() {
        b"Out " => BevelDirection::Down,
        _ => BevelDirection::Up,
    };
    let blend = |key: &[u8], default: BlendMode| -> Option<BlendMode> {
        Some(effect_blend_mode(
            &enum_bytes(ebbl, key, b"BlnM", b"")?,
            default,
        ))
    };
    let color = |key: &[u8], default: [u8; 3]| -> Option<[u8; 3]> {
        match desc_item(ebbl, key) {
            None => Some(default),
            Some(value) => decode_color(value),
        }
    };
    Some(BevelEmboss {
        enabled: bool_or(ebbl, b"enab", false)?,
        present: bool_or(ebbl, b"present", false)?,
        style,
        technique,
        direction,
        depth: num_clamped(ebbl, b"srgR", 100.0, 0.0, MAX_DEPTH)?,
        size: num_clamped(ebbl, b"blur", 5.0, 0.0, MAX_SIZE)?,
        soften: num_clamped(ebbl, b"Sftn", 0.0, 0.0, MAX_SIZE)?,
        angle_deg: num_or(ebbl, b"lagl", 120.0)?,
        altitude_deg: num_clamped(ebbl, b"Lald", 30.0, 0.0, MAX_ALTITUDE)?,
        use_global_angle: bool_or(ebbl, b"uglg", true)?,
        highlight: BevelHighlight {
            mode: blend(b"hglM", BlendMode::Screen)?,
            color: color(b"hglC", [255, 255, 255])?,
            opacity: num_clamped(ebbl, b"hglO", 75.0, 0.0, MAX_OPACITY)?,
        },
        shadow: BevelShadow {
            mode: blend(b"sdwM", BlendMode::Multiply)?,
            color: color(b"sdwC", [0, 0, 0])?,
            opacity: num_clamped(ebbl, b"sdwO", 75.0, 0.0, MAX_OPACITY)?,
        },
    })
}

/// How much the technique rounds the chamfer: Smooth rounds it by a third of
/// the size, Chisel Soft just takes the edge off, Chisel Hard keeps it exact.
fn rounding(technique: BevelTechnique, size: f64) -> f64 {
    match technique {
        BevelTechnique::Smooth => (size / 3.0).max(1.5),
        BevelTechnique::ChiselSoft => 1.0,
        BevelTechnique::ChiselHard => 0.0,
    }
}

/// How far a bevel's visible band reaches past the content box: the chamfer,
/// the technique's rounding blur, the soften blur, and two spare pixels.
pub(super) fn reach(bevel: &BevelEmboss) -> f32 {
    let size = clamp_finite(bevel.size, MAX_SIZE) as f64;
    let soften = clamp_finite(bevel.soften, MAX_SIZE) as f64;
    let size = size.max(1.0);
    (size.ceil() as i64 + blur_support(rounding(bevel.technique, size)) + blur_support(soften) + 2)
        as f32
}

pub(super) fn build_bevel_emboss(
    layer: &Layer,
    doc: &Document,
    bevel: &BevelEmboss,
    build: BuildRegion,
    fill: Option<&Adjustment>,
) -> Vec<EffectPass> {
    // Stroke Emboss lights only a Stroke effect's band, which is not modelled.
    if bevel.style == BevelStyle::Stroke {
        return Vec::new();
    }
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
    // Clamp again: a hand-built `BevelEmboss` may not have gone through decode.
    let size = clamp_finite(bevel.size, MAX_SIZE) as f64;
    let soften = clamp_finite(bevel.soften, MAX_SIZE) as f64;
    let depth = clamp_finite(bevel.depth, MAX_DEPTH) as f64;
    let altitude = clamp_finite(bevel.altitude_deg, MAX_ALTITUDE) as f64;
    let angle = if bevel.angle_deg.is_finite() {
        bevel.angle_deg as f64
    } else {
        0.0
    };
    let hi_opacity = clamp_finite(bevel.highlight.opacity, MAX_OPACITY) / 100.0;
    let sh_opacity = clamp_finite(bevel.shadow.opacity, MAX_OPACITY) / 100.0;
    if (hi_opacity <= 0.0 && sh_opacity <= 0.0) || depth <= 0.0 {
        return Vec::new();
    }
    let size = size.max(1.0);
    let round = rounding(bevel.technique, size);
    // The chamfer reaches `size` past the edge (half for the straddling styles);
    // the rounding and soften blurs reach a little further.
    let pad = size.ceil() as i64 + blur_support(round) + blur_support(soften) + 2;
    let padded = pad_rect(source, pad, w, h);
    let Some(padded) = build.buffer(padded, pad) else {
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
    // Height 0..1 along the chamfer, from the signed distance (inside positive).
    let start = match bevel.style {
        BevelStyle::Inner => 0.0,
        BevelStyle::Outer => -size,
        _ => -size / 2.0,
    };
    let mut height: Vec<f32> = signed_distance(&matte, pw, ph)
        .into_iter()
        .map(|d| ((d as f64 - start) / size).clamp(0.0, 1.0) as f32)
        .collect();
    if round > 0.0 {
        height = blur_f32(&height, pw, ph, round);
    }
    let at = |x: i32, y: i32| -> f64 {
        let sx = (x - px0).clamp(0, pw as i32 - 1) as usize;
        let sy = (y - py0).clamp(0, ph as i32 - 1) as usize;
        height[sy * pw + sx] as f64
    };
    let theta = angle.to_radians();
    let phi = altitude.to_radians();
    let (lx, ly, lz) = (phi.cos() * theta.cos(), -phi.cos() * theta.sin(), phi.sin());
    // A full chamfer rises `size · depth/100` over `size` pixels.
    let scale = size * depth / 100.0;
    let down = matches!(bevel.direction, BevelDirection::Down);
    // An Inner Bevel lights only the content, an Outer Bevel only what lies
    // outside it, an Emboss both; a Pillow Emboss lights the outside sunken.
    let lit = |m: f32| -> [(f32, f32); 2] {
        match bevel.style {
            BevelStyle::Inner => [(m, 1.0), (0.0, 1.0)],
            BevelStyle::Outer => [(1.0 - m, 1.0), (0.0, 1.0)],
            BevelStyle::Emboss => [(1.0, 1.0), (0.0, 1.0)],
            _ => [(m, 1.0), (1.0 - m, -1.0)],
        }
    };
    // Inner shades only the content rect; the other styles reach past it.
    let region = if bevel.style == BevelStyle::Inner {
        source
    } else {
        padded
    };
    let Some(region) = build.output_tuple(region) else {
        return Vec::new();
    };
    // The soften blur must read the true neighbours of the requested region,
    // so build the shading over the region grown by the blur support and emit
    // only the requested region.
    let soften_pad = blur_support(soften) as i32;
    let shade = (
        (region.0 - soften_pad).max(px0),
        (region.1 - soften_pad).max(py0),
        (region.2 + soften_pad).min(px1),
        (region.3 + soften_pad).min(py1),
    );
    let sw = (shade.2 - shade.0) as usize;
    let sh = (shade.3 - shade.1) as usize;
    let mut shading = vec![0.0f32; sw * sh];
    for y in shade.1..shade.3 {
        for x in shade.0..shade.2 {
            let gx = (at(x + 1, y) - at(x - 1, y)) / 2.0;
            let gy = (at(x, y + 1) - at(x, y - 1)) / 2.0;
            let (nx, ny) = (-gx * scale, -gy * scale);
            let inv = 1.0 / (nx * nx + ny * ny + 1.0).sqrt();
            let mut s = (nx * lx + ny * ly + lz) * inv - phi.sin();
            if down {
                s = -s;
            }
            shading[(y - shade.1) as usize * sw + (x - shade.0) as usize] = s as f32;
        }
    }
    // The highlight and shadow soften apart, so where they meet they spread
    // into each other instead of cancelling out.
    let (mut lights, mut darks): (Vec<f32>, Vec<f32>) =
        shading.iter().map(|&s| (s.max(0.0), (-s).max(0.0))).unzip();
    if soften > 0.0 {
        lights = blur_f32(&lights, sw, sh, soften);
        darks = blur_f32(&darks, sw, sh, soften);
    }
    let unit = |c: [u8; 3]| c.map(|v| v as f32 / 255.0);
    let shadow_color = unit(bevel.shadow.color);
    let highlight_color = unit(bevel.highlight.color);
    let rect = PsdRect {
        top: region.1,
        left: region.0,
        bottom: region.3,
        right: region.2,
    };
    let two_slots = matches!(bevel.style, BevelStyle::Pillow);
    let mut shadows = vec![PassWriter::constant(rect, bevel.shadow.mode, shadow_color)];
    let mut highlights = vec![PassWriter::constant(
        rect,
        bevel.highlight.mode,
        highlight_color,
    )];
    if two_slots {
        shadows.push(PassWriter::constant(rect, bevel.shadow.mode, shadow_color));
        highlights.push(PassWriter::constant(
            rect,
            bevel.highlight.mode,
            highlight_color,
        ));
    }
    for y in region.1..region.3 {
        for x in region.0..region.2 {
            let m = matte[(y - py0) as usize * pw + (x - px0) as usize];
            let i = (y - shade.1) as usize * sw + (x - shade.0) as usize;
            for (slot, (coverage, sign)) in lit(m).into_iter().enumerate() {
                if coverage <= 0.0 || slot >= shadows.len() {
                    continue;
                }
                // A sunken side swaps which part is the highlight.
                let (light, dark) = if sign > 0.0 {
                    (lights[i], darks[i])
                } else {
                    (darks[i], lights[i])
                };
                let sh_alpha = coverage * dark.clamp(0.0, 1.0) * sh_opacity;
                if sh_alpha > 0.0 {
                    shadows[slot].emit(x, y, shadow_color, sh_alpha);
                }
                let hi_alpha = coverage * light.clamp(0.0, 1.0) * hi_opacity;
                if hi_alpha > 0.0 {
                    highlights[slot].emit(x, y, highlight_color, hi_alpha);
                }
            }
        }
    }
    let mut passes = Vec::with_capacity(shadows.len() * 2);
    for (shadow, highlight) in shadows.into_iter().zip(highlights) {
        passes.push(shadow.finish());
        passes.push(highlight.finish());
    }
    passes
}

/// The signed Euclidean distance of every pixel to the matte's edge (the 0.5
/// level), positive inside. Pixels on the edge take the matte's own fraction,
/// so an anti-aliased outline gives a sub-pixel distance rather than a step.
fn signed_distance(matte: &[f32], w: usize, h: usize) -> Vec<f32> {
    let full: Vec<bool> = matte.iter().map(|&m| m >= 0.999).collect();
    let empty: Vec<bool> = matte.iter().map(|&m| m <= 0.001).collect();
    let to_not_full = distance_transform(&full.iter().map(|&f| !f).collect::<Vec<_>>(), w, h);
    let to_not_empty = distance_transform(&empty.iter().map(|&e| !e).collect::<Vec<_>>(), w, h);
    (0..w * h)
        .map(|i| {
            if full[i] {
                to_not_full[i] - 0.5
            } else if empty[i] {
                0.5 - to_not_empty[i]
            } else {
                matte[i] - 0.5
            }
        })
        .collect()
}

/// The exact Euclidean distance from every pixel to the nearest `seed` pixel
/// (Felzenszwalb–Huttenlocher, two separable passes). No seed is "far".
fn distance_transform(seed: &[bool], w: usize, h: usize) -> Vec<f32> {
    const FAR: f64 = 1.0e12;
    let mut grid: Vec<f64> = seed.iter().map(|&s| if s { 0.0 } else { FAR }).collect();
    let mut line = Vec::new();
    for x in 0..w {
        line.clear();
        line.extend((0..h).map(|y| grid[y * w + x]));
        for (y, v) in squared_distance_1d(&line).into_iter().enumerate() {
            grid[y * w + x] = v;
        }
    }
    for y in 0..h {
        let row = squared_distance_1d(&grid[y * w..(y + 1) * w]);
        grid[y * w..(y + 1) * w].copy_from_slice(&row);
    }
    grid.into_iter().map(|d| d.sqrt() as f32).collect()
}

/// The 1-D squared-distance transform of `f` under the lower envelope of
/// parabolas.
fn squared_distance_1d(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    let mut out = vec![0.0; n];
    if n == 0 {
        return out;
    }
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0usize;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    let cross = |q: usize, p: usize| {
        ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * q as f64 - 2.0 * p as f64)
    };
    for q in 1..n {
        // `z[0]` is −∞, so the scan stops at the first parabola at the latest.
        let mut s = cross(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = cross(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let d = q as f64 - v[k] as f64;
        *slot = d * d + f[v[k]];
    }
    out
}

/// A separable Gaussian blur of an `f32` field with clamped edges; `radius`
/// maps to sigma as the crate blur does.
fn blur_f32(src: &[f32], w: usize, h: usize, radius: f64) -> Vec<f32> {
    let kernel = pictura_filters::kernel::gaussian_kernel(
        pictura_filters::kernel::sigma_from_radius(radius),
    );
    let r = (kernel.len() / 2) as isize;
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0f32; src.len()];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0f64;
                for (i, k) in kernel.iter().enumerate() {
                    let o = i as isize - r;
                    let (sx, sy) = if horizontal {
                        ((x as isize + o).clamp(0, w as isize - 1) as usize, y)
                    } else {
                        (x, (y as isize + o).clamp(0, h as isize - 1) as usize)
                    };
                    acc += k * src[sy * w + sx] as f64;
                }
                out[y * w + x] = acc as f32;
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_transform_matches_brute_force() {
        let (w, h) = (9, 7);
        let seed: Vec<bool> = (0..w * h).map(|i| i == 10 || i == 50 || i == 33).collect();
        let fast = distance_transform(&seed, w, h);
        for y in 0..h {
            for x in 0..w {
                let brute = (0..w * h)
                    .filter(|&i| seed[i])
                    .map(|i| {
                        let (sx, sy) = ((i % w) as f32, (i / w) as f32);
                        ((x as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt()
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!((fast[y * w + x] - brute).abs() < 1e-4, "({x},{y})");
            }
        }
    }

    #[test]
    fn signed_distance_is_positive_inside_and_sub_pixel_on_the_edge() {
        // A 3-pixel-wide bar with an anti-aliased (0.25) right edge.
        let w = 6;
        let matte = [1.0, 1.0, 1.0, 0.25, 0.0, 0.0];
        let d = signed_distance(&matte, w, 1);
        assert_eq!(d[0], 2.5);
        assert_eq!(d[2], 0.5);
        assert_eq!(d[3], -0.25);
        assert_eq!(d[4], -0.5);
        assert_eq!(d[5], -1.5);
    }
}
