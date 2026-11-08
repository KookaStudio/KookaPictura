//! The pass the Gradient and the Paint Bucket share: every pixel of one layer
//! asks a source for its colour and coverage, and the result is laid down in a
//! Brush mode through the selection.

use crate::fill_uniform::slot;
use crate::replace::grow;
use crate::stroke::{blend_pixel, layer_at, layer_at_mut, parse_layer_path};
use crate::{PaintMode, Rgba};
use pictura_core::mask_ops::TiledMask;
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, ColorMode, Document, PsdRect, Sample,
};
use rayon::prelude::*;

/// The blend of one pixel: the destination's colour and alpha, the source
/// colour, the coverage, and the dissolve roll. `None` when the blend leaves
/// the pixel alone.
pub(crate) type Blend<T> = fn(PaintMode, (T, T, T, T), &Rgba, f32, f32) -> Option<(T, T, T, T)>;

/// What the fill does with the pixels' transparency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alpha {
    /// The source over the backdrop: the backdrop's alpha survives, which is
    /// what a brush-mode blend does.
    Blend,
    /// The destination's alpha survives unchanged, and the pixels the
    /// destination holds empty are left alone rather than filled.
    Keep,
    /// The source colour's alpha becomes the pixel's alpha.
    Replace,
}

/// One row of the target's colour planes: three for an RGB layer, one for a
/// Grayscale layer, and the transparency plane when the layer has one.
pub(crate) enum Row<'a, T> {
    Rgb {
        r: &'a mut [T],
        g: &'a mut [T],
        b: &'a mut [T],
        a: Option<&'a mut [T]>,
    },
    Gray {
        y: &'a mut [T],
        a: Option<&'a mut [T]>,
    },
}

/// Where a fill lays its source, and what it may assume about it.
///
/// `selection` and `mask` are document-sized coverage masks (0–255), tiled on
/// the document grid or flat: the selection confines and scales the fill, the
/// mask is the coverage handed to the source. `solid` says the source answers
/// the same for every pixel given the same coverage (a foreground colour, not
/// a pattern or a ramp), so a tile whose inputs are all uniform is blended
/// once (#1214).
#[derive(Clone, Copy, Default)]
pub(crate) struct Through<'a> {
    pub(crate) selection: Option<&'a TiledMask>,
    pub(crate) mask: Option<&'a TiledMask>,
    pub(crate) solid: bool,
}

/// Fill the pixel layer at `path`. `source` is asked, per document pixel and
/// with the pixel's `mask` coverage (255 without a mask), for the colour to
/// lay and its coverage (0.0–1.0); `None` leaves the pixel alone. A pixel the
/// selection or the mask leaves at 0 is never asked about. Without a
/// selection the whole layer is filled. `alpha` says what the fill does
/// with the pixels' transparency; a transparency lock overrides it and keeps
/// every pixel's alpha. Returns the document rectangle changed, or `None` when
/// nothing changed or the layer cannot be filled: a group, an adjustment,
/// locked pixels, or Clear under a transparency lock.
///
/// Rows run on every core: `source` sees each pixel once, in no fixed order,
/// so it must not depend on the pixels it was asked about before.
///
/// The layer's pixels live in its retained native store when the document has
/// one the compositor reads; the fill writes that copy and keeps the 8-bit
/// channels as its narrowing. Otherwise the 8-bit channels are the pixels.
#[cfg(test)]
pub(crate) fn fill_layer(
    doc: &mut Document,
    path: &str,
    mode: PaintMode,
    selection: Option<&[u8]>,
    alpha: Alpha,
    source: impl Fn(i32, i32) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    // An unknown source's reach is the layer's own rect: this entry does not
    // grow the layer. The boxed entries pass their damage box instead.
    let within = layer_at(doc, &parse_layer_path(path)?).map(|layer| layer.rect);
    let selection = selection.map(|s| TiledMask::from(s.to_vec()));
    let through = Through {
        selection: selection.as_ref(),
        ..Through::default()
    };
    fill_layer_within(doc, path, mode, through, alpha, within, |x, y, _| {
        source(x, y)
    })
}

/// [`fill_layer`] for a `source` that answers `None` outside `within`
/// (document pixels): an 8-bit layer is visited and written only in the
/// tiles `within` crosses, so a small fill on a large, history-shared layer
/// neither walks nor copies the rest of its planes.
pub(crate) fn fill_layer_within(
    doc: &mut Document,
    path: &str,
    mode: PaintMode,
    through: Through<'_>,
    alpha: Alpha,
    within: Option<PsdRect>,
    source: impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    // The working model is 8-bit; a document the writer accepts always is.
    if doc.depth != BitDepth::Eight {
        return None;
    }
    let canvas = (doc.width as i32, doc.height as i32);
    let n = (canvas.0 * canvas.1) as usize;
    if [through.selection, through.mask]
        .iter()
        .flatten()
        .any(|m| m.len() < n)
    {
        return None;
    }
    // `within` is the box the source may damage. An edit outside a small layer
    // grows it to that box (#1208); a grow the governor refuses leaves the
    // layer alone and the fill is refused.
    let canvas_rect = PsdRect {
        top: 0,
        left: 0,
        bottom: canvas.1,
        right: canvas.0,
    };
    if let Some(damage) = within.and_then(|rect| rect.intersect(canvas_rect)) {
        let target = layer_at_mut(doc, &parse_layer_path(path)?)?;
        let rect = if target.rect.is_empty() {
            damage
        } else {
            target.rect.union(damage)
        };
        if pictura_core::layer_bounds::grow_layer_to(target, rect, "a fill").is_err() {
            return None;
        }
    }
    // Clear has no alpha plane to change on an opaque layer; treating the
    // zeroed RGB as a change would dirty a layer the stroke path leaves alone.
    let target = layer_at(doc, &parse_layer_path(path)?)?;
    if mode == PaintMode::Clear && !target.channels.iter().any(|c| c.id == -1) {
        return None;
    }
    // A Grayscale document's layer carries one colour plane, an RGB one three.
    let gray = doc.mode == ColorMode::Grayscale;
    match crate::fill_native::native_plan(doc, path, gray) {
        crate::fill_native::NativePlan::Store(depth) => {
            return crate::fill_native::fill_native(
                doc, path, depth, mode, through, alpha, gray, source,
            );
        }
        // The store owns some of the colour planes: a fill of the 8-bit
        // channels would reach only the planes the store does not own.
        crate::fill_native::NativePlan::Refused => return None,
        crate::fill_native::NativePlan::Copy => {}
    }
    fill_eight_bit(doc, path, mode, through, alpha, gray, within, source)
}

/// Fill the layer's 8-bit channels tile by tile, only in the tiles `within`
/// crosses when it is given; tiles the fill leaves holding one value collapse
/// to `Uniform` afterwards.
#[allow(clippy::too_many_arguments)]
fn fill_eight_bit(
    doc: &mut Document,
    path: &str,
    mode: PaintMode,
    through: Through<'_>,
    alpha: Alpha,
    gray: bool,
    within: Option<PsdRect>,
    source: impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    let canvas = (doc.width as i32, doc.height as i32);
    let layer = layer_at_mut(doc, &parse_layer_path(path)?)?;
    if layer.is_group || layer.adjustment.is_some() || layer_pixel_locked(layer) {
        return None;
    }
    let keep_alpha = layer_transparency_locked(layer);
    if keep_alpha && mode == PaintMode::Clear {
        return None;
    }
    // A transparency lock is the user's instruction about the layer's
    // transparency, so it outranks the caller's policy.
    let policy = if keep_alpha { Alpha::Keep } else { alpha };
    let rect = layer.rect;
    let w = rect.width().max(0);
    let n = (w * rect.height().max(0)) as usize;
    let canvas_rect = PsdRect {
        top: 0,
        left: 0,
        bottom: canvas.1,
        right: canvas.0,
    };
    let region = match within {
        Some(within) => within
            .intersect(rect)
            .and_then(|r| r.intersect(canvas_rect))?,
        None => rect.intersect(canvas_rect)?,
    };
    let pass = Pass {
        canvas,
        mode,
        through,
        alpha: policy,
        gray,
        within: region,
    };
    if let Some(changed) = crate::fill_uniform::fill_tiled_layer(layer, n, &pass, &source) {
        return changed;
    }
    // The R, G, B and alpha planes, each a whole layer long; a missing colour
    // plane refuses the fill, a missing alpha is an opaque layer.
    let mut planes: [Option<&mut [u8]>; 4] = [None, None, None, None];
    for c in layer.channels.iter_mut() {
        let Some(k) = slot(c.id) else {
            continue;
        };
        if c.data.len() == n && planes[k].is_none() {
            planes[k] = Some(c.data.as_mut_slice());
        }
    }
    let [r, g, b, a] = planes;
    let (r, g, b) = if gray {
        let r = r?;
        (r, None, None)
    } else {
        let (Some(r), Some(g), Some(b)) = (r, g, b) else {
            return None;
        };
        (r, Some(g), Some(b))
    };
    let pass = Pass {
        within: canvas_rect,
        ..pass
    };
    fill_planes(r, g, b, a, rect, &pass, &source, blend_pixel)
}

/// The last blend a row ran: its inputs (destination, colour, strength bits)
/// and its answer, reused while a uniform run repeats them.
pub(crate) type LastBlend<T> = Option<(((T, T, T, T), Rgba, u32), Option<(T, T, T, T)>)>;

/// What one fill pass lays down, shared by every tile and plane it visits.
#[derive(Clone, Copy)]
pub(crate) struct Pass<'a> {
    pub(crate) canvas: (i32, i32),
    pub(crate) mode: PaintMode,
    pub(crate) through: Through<'a>,
    pub(crate) alpha: Alpha,
    pub(crate) gray: bool,
    /// The document box the pass is limited to.
    pub(crate) within: PsdRect,
}

impl Pass<'_> {
    /// The new samples of one pixel of `dst` (colour and alpha) at selection
    /// coverage `selected` (non-zero) and source answer `(color, cover)`, or
    /// `None` when the pixel is left alone. `has_alpha` is whether the target
    /// has a transparency plane.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn blend_one<T: Sample + PartialEq>(
        &self,
        (x, y): (i32, i32),
        dst: (T, T, T, T),
        has_alpha: bool,
        selected: u8,
        (color, cover): (Rgba, f32),
        blend: Blend<T>,
        last: &mut LastBlend<T>,
    ) -> Option<(T, T, T, T)> {
        let mut strength = cover.clamp(0.0, 1.0) * selected as f32 / 255.0;
        // A Grayscale layer takes the colour's luma.
        let mut color = if self.gray {
            let v = pictura_core::rec601_luma(color.r, color.g, color.b);
            Rgba {
                r: v,
                g: v,
                b: v,
                a: color.a,
            }
        } else {
            color
        };
        if self.mode == PaintMode::Dissolve {
            // A see-through colour dissolves into fewer pixels, not fainter ones.
            strength *= color.a as f32 / 255.0;
            color.a = 255;
        }
        if strength <= 0.0 {
            return None;
        }
        let dst_alpha = dst.3;
        if self.alpha == Alpha::Keep && dst_alpha.to_unit() == 0.0 {
            return None;
        }
        // A uniform run feeds the blend the same inputs pixel after pixel, so
        // the last answer is reused; only Dissolve reads the per-pixel noise
        // (#1149).
        let out = if self.mode == PaintMode::Dissolve {
            blend(self.mode, dst, &color, strength, pixel_noise(x, y))
        } else {
            let key = (dst, color, strength.to_bits());
            match *last {
                Some((seen, out)) if seen == key => out,
                _ => {
                    let out = blend(self.mode, dst, &color, strength, 0.0);
                    *last = Some((key, out));
                    out
                }
            }
        };
        let out = out?;
        let out_alpha = match self.alpha {
            // The destination's own alpha, untouched.
            Alpha::Keep => dst_alpha,
            // The source colour's alpha, as it is: the fill's opacity scales
            // the colour, not the ramp's transparency.
            Alpha::Replace => T::from_unit(f64::from(color.a) / 255.0),
            // Whatever the blend decided. A layer without a transparency plane
            // has no alpha to change, so the blend's alpha is dropped and the
            // pixel is unchanged.
            Alpha::Blend if has_alpha => out.3,
            Alpha::Blend => dst_alpha,
        };
        let out = (out.0, out.1, out.2, out_alpha);
        (out != dst).then_some(out)
    }
}

/// Columns `x0..x1` of document row `y` of `mask`.
fn mask_row(mask: &TiledMask, canvas_width: i32, y: i32, (x0, x1): (i32, i32)) -> Vec<u8> {
    let mut out = vec![0; (x1 - x0).max(0) as usize];
    mask.read_rect_into(
        x0 as usize..x1 as usize,
        y as usize..y as usize + 1,
        canvas_width as usize,
        &mut out,
    );
    out
}

/// Lay the source down over the target's planes, in the given brush mode,
/// inside the selection, and report the rectangle changed.
///
/// The planes are the target's own samples, so the same pass fills an 8-bit
/// layer and a layer whose pixels live in a native store; `blend` is the
/// brush-mode blend at the planes' own width. `pass.alpha` says what happens
/// to each pixel's transparency: the blend's own, the destination's (`Keep`),
/// or the source colour's (`Replace`). Each row reads its selection and mask
/// spans once, so a tiled mask is never held whole.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_planes<T: Sample + PartialEq + Send + Sync>(
    r: &mut [T],
    g: Option<&mut [T]>,
    b: Option<&mut [T]>,
    a: Option<&mut [T]>,
    rect: PsdRect,
    pass: &Pass<'_>,
    source: &(impl Fn(i32, i32, u8) -> Option<(Rgba, f32)> + Sync),
    blend: Blend<T>,
) -> Option<PsdRect> {
    let (canvas, within) = (pass.canvas, pass.within);
    let w = rect.width().max(0) as usize;
    let (y0, y1) = (
        rect.top.max(0).max(within.top),
        rect.bottom.min(canvas.1).min(within.bottom),
    );
    let (x0, x1) = (
        rect.left.max(0).max(within.left),
        rect.right.min(canvas.0).min(within.right),
    );
    if y1 <= y0 || x1 <= x0 {
        return None;
    }
    let has_alpha = a.is_some();
    let opaque = T::from_unit(1.0);
    let row = |ly: i32| {
        let at = (ly * w as i32) as usize;
        at..at + w
    };
    // Only the layer rows on the canvas are cut into the row tasks. Every
    // plane is cut on the same span: a layer that hangs off the canvas has
    // its rows before the canvas in the plane, not before the first row.
    let span = row(y0 - rect.top).start..row(y1 - 1 - rect.top).end;
    let alpha_plane = a.map(|a| &mut a[span.clone()]);
    let rows: Vec<(i32, Row<T>)> = match (g, b) {
        (Some(g), Some(b)) => r[span.clone()]
            .chunks_mut(w)
            .zip(g[span.clone()].chunks_mut(w))
            .zip(b[span.clone()].chunks_mut(w))
            .zip(rows_of(alpha_plane, w))
            .enumerate()
            .map(|(k, (((r, g), b), a))| (y0 + k as i32, Row::Rgb { r, g, b, a }))
            .collect(),
        _ => r[span.clone()]
            .chunks_mut(w)
            .zip(rows_of(alpha_plane, w))
            .enumerate()
            .map(|(k, (r, a))| (y0 + k as i32, Row::Gray { y: r, a }))
            .collect(),
    };
    rows.into_par_iter()
        .filter_map(|(y, mut row)| {
            let mut dirty = None;
            let mut last = None;
            // Each row copies the spans it needs; without a mask there is
            // nothing to copy.
            let span = |mask: Option<&TiledMask>| mask.map(|m| mask_row(m, canvas.0, y, (x0, x1)));
            let (selected, covered) = (span(pass.through.selection), span(pass.through.mask));
            for x in x0..x1 {
                let k = (x - x0) as usize;
                let sel = selected.as_ref().map_or(255, |r| r[k]);
                let m = covered.as_ref().map_or(255, |r| r[k]);
                if sel == 0 || m == 0 {
                    continue;
                }
                let Some(answer) = source(x, y, m) else {
                    continue;
                };
                let i = (x - rect.left) as usize;
                let dst = match &row {
                    Row::Rgb { r, g, b, a } => {
                        (r[i], g[i], b[i], a.as_ref().map_or(opaque, |a| a[i]))
                    }
                    Row::Gray { y, a } => (y[i], y[i], y[i], a.as_ref().map_or(opaque, |a| a[i])),
                };
                let Some(out) =
                    pass.blend_one((x, y), dst, has_alpha, sel, answer, blend, &mut last)
                else {
                    continue;
                };
                match &mut row {
                    Row::Rgb { r, g, b, a } => {
                        r[i] = out.0;
                        g[i] = out.1;
                        b[i] = out.2;
                        if let Some(a) = a.as_deref_mut() {
                            a[i] = out.3;
                        }
                    }
                    Row::Gray { y, a } => {
                        y[i] = out.0;
                        if let Some(a) = a.as_deref_mut() {
                            a[i] = out.3;
                        }
                    }
                }
                dirty = Some(grow(dirty, x, y));
            }
            dirty
        })
        .reduce_with(PsdRect::union)
}

/// `plane` cut into rows of `w`, or endless `None`s for an absent plane.
fn rows_of<T>(plane: Option<&mut [T]>, w: usize) -> impl Iterator<Item = Option<&mut [T]>> {
    let mut rows = plane.map(|p| p.chunks_mut(w));
    std::iter::from_fn(move || {
        Some(match rows.as_mut() {
            Some(rows) => Some(rows.next()?),
            None => None,
        })
    })
}

/// A fixed value in `0.0..1.0` for a pixel: the same fill must come out the
/// same every time, so its Dissolve and dither cannot use a generator.
pub(crate) fn pixel_noise(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h & 0xFFFF) as f32 / 65_536.0
}

/// A fixed value in `0.0..1.0` for a pixel, clumped so the stroke grain reads
/// as pencil texture instead of single-pixel static: the fine hash blended
/// with bilinear value noise on 2 px and 3 px grids.
pub(crate) fn grain_noise(x: i32, y: i32) -> f32 {
    let fine = pixel_noise(x, y);
    let mid = value_noise(x, y, 2.0);
    let broad = value_noise(x, y, 3.0);
    // Weighted toward the clumps and mildly expanded to match the 2B Pencil
    // reference's grain (5×5 residual sd ~0.065, neighbour correlation ~0.38);
    // the clamp only bites at the extremes.
    ((0.2 * fine + 0.35 * mid + 0.45 * broad - 0.5) * 1.3 + 0.5).clamp(0.0, 1.0)
}

/// Bilinear value noise on a `scale` px grid, deterministic for a pixel.
fn value_noise(x: i32, y: i32, scale: f32) -> f32 {
    let (fx, fy) = (x as f32 / scale, y as f32 / scale);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let (xi, yi) = (x0 as i32, y0 as i32);
    let (a, b) = (pixel_noise(xi, yi), pixel_noise(xi + 1, yi));
    let (c, d) = (pixel_noise(xi, yi + 1), pixel_noise(xi + 1, yi + 1));
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

#[cfg(test)]
pub(crate) mod tests;
