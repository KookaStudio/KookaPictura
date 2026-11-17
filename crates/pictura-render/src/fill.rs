//! Generative fill content: solid (`SoCo`) and gradient (`GdFl`).
//!
//! A fill layer owns no pixels; it is a descriptor that the compositor paints
//! over the layer's rect. This module is the single decoder for both fill keys
//! and the geometry that turns a gradient descriptor into pixels, following
//! psd-tools' `composite/paint.py::draw_gradient_fill`.
//!
//! ponytail: noise gradients (`ClNs`), transparency/opacity stops, stop
//! midpoints (`Mdpn`), non-linear interpolation (`Intr`), and non-RGB colour
//! models are not modelled; a fill that needs them renders as the nearest
//! custom-stop approximation. Raise these one at a time when a fixture shows a
//! real gap.

use pictura_adjust::{GradientFillParams, GradientKind, GradientStop, PatternFillParams};
pub use pictura_codec::layer_content::{
    decode_gradient_fill, decode_pattern_fill, decode_vector_fill, encode_gradient_fill,
    encode_gradient_fill_params, encode_pattern_fill, gradient_params_from_desc,
    pattern_params_from_desc, pattern_params_from_desc_with_link,
};
use pictura_codec::PatternPixels;
use pictura_core::FillContent;
use pictura_core::{Document, Layer, PixelBuffer};

use crate::composite::{blend_into_ctx, Canvas};
use crate::composite_blend::{BlendCtx, BlendOverride};

pub(crate) fn is_pattern_fill(layer: &Layer) -> bool {
    matches!(layer.fill_content(), Some(FillContent::Pattern(_)))
}

/// The layer's decoded fill content: its modeled adjustment block first, else
/// its preserved `vscg` vector fill. Shared by the compositor dispatch and the
/// layer-effects coverage matte, so a shape layer's fill and its effects are
/// gated by the same content.
///
/// The choice is strict: an adjustment block is consulted on its own, so an
/// undecodable key is a no-op and never falls through to the `vscg` fill; the
/// `vscg` block is decoded only when the layer has no adjustment block.
pub fn decode_layer_fill(layer: &Layer) -> Option<FillContent> {
    layer.fill_content().cloned()
}

pub fn gradient_rgba(w: i32, h: i32, params: &GradientFillParams) -> Vec<[u8; 4]> {
    gradient_rgba_region(w, h, (0, 0, w.max(0), h.max(0)), params)
}

/// Generate the same gradient as [`gradient_rgba`] for only the local
/// sub-rectangle `region`, in `w × h` coordinates. The full extent still sets
/// the geometry, so every sample matches the full buffer's slice.
pub fn gradient_rgba_region(
    w: i32,
    h: i32,
    region: (i32, i32, i32, i32),
    params: &GradientFillParams,
) -> Vec<[u8; 4]> {
    let (sx0, sy0, sx1, sy1) = region;
    let sw = (sx1 - sx0).max(0) as usize;
    let sh = (sy1 - sy0).max(0) as usize;
    let mut out = vec![[0u8, 0, 0, 255]; sw * sh];
    if w <= 0 || h <= 0 || sw == 0 || sh == 0 {
        return out;
    }
    let wf = w as f64;
    let hf = h as f64;
    let angle = params.angle_deg as f64;
    let ratio = angle.rem_euclid(90.0);
    let s = (params.scale as f64 / 100.0) * ((90.0 - ratio) / 90.0 * wf + (ratio / 90.0) * hf);
    if !s.is_finite() || s <= 0.0 {
        return out;
    }
    let theta = angle.rem_euclid(360.0).to_radians();
    let (sin_t, cos_t) = (theta.sin(), theta.cos());
    let x0 = -wf / s;
    let x_step = if w > 1 {
        2.0 * wf / s / (wf - 1.0)
    } else {
        0.0
    };
    let y0 = -hf / s;
    let y_step = if h > 1 {
        2.0 * hf / s / (hf - 1.0)
    } else {
        0.0
    };
    for j in sy0..sy1 {
        let y = y0 + j as f64 * y_step;
        for i in sx0..sx1 {
            let x = x0 + i as f64 * x_step;
            let mut z = match params.kind {
                GradientKind::Linear => 0.5 * (cos_t * x - sin_t * y + 1.0),
                GradientKind::Radial => (x * x + y * y).sqrt(),
                GradientKind::Angle => {
                    let a = (180.0 * y.atan2(x) / std::f64::consts::PI) + angle;
                    a.rem_euclid(360.0) / 360.0
                }
                GradientKind::Reflected => (cos_t * x - sin_t * y).abs(),
                GradientKind::Diamond => {
                    (cos_t * x - sin_t * y).abs() + (sin_t * x + cos_t * y).abs()
                }
            };
            z = z.clamp(0.0, 1.0);
            if params.reverse {
                z = 1.0 - z;
            }
            out[(j - sy0) as usize * sw + (i - sx0) as usize] = sample_stops(&params.stops, z);
        }
    }
    out
}

/// Linearly sample `stops` at index `z ∈ [0, 1]` (`z * 4096`), clamping outside
/// the stop range. Stops are strictly increasing in location.
pub(crate) fn sample_stops(stops: &[GradientStop], z: f64) -> [u8; 4] {
    let Some(first) = stops.first() else {
        return [0, 0, 0, 255];
    };
    let last = stops.last().unwrap_or(first);
    let pos = z * 4096.0;
    if stops.len() == 1 || pos <= first.location as f64 {
        return [first.color[0], first.color[1], first.color[2], 255];
    }
    if pos >= last.location as f64 {
        return [last.color[0], last.color[1], last.color[2], 255];
    }
    for pair in stops.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if pos >= a.location as f64 && pos <= b.location as f64 {
            let span = (b.location - a.location) as f64;
            let t = if span <= 0.0 {
                0.0
            } else {
                (pos - a.location as f64) / span
            };
            let lerp = |i: usize| {
                (a.color[i] as f64 + (b.color[i] as f64 - a.color[i] as f64) * t)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            return [lerp(0), lerp(1), lerp(2), 255];
        }
    }
    [last.color[0], last.color[1], last.color[2], 255]
}

/// Composite a solid fill across the layer's rect (clamped to the canvas).
///
/// Unlike a destructive adjustment, every in-rect pixel is source content at
/// the payload's alpha; the layer's mask, opacity, fill, and blend still apply
/// through [`blend_into_ctx`]. Pixels outside the rect are untouched.
pub(crate) fn composite_solid_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    rgba: [u8; 4],
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    // The mask is zero outside the layer's content box, and a zero mask
    // leaves the backdrop alone, so a shape blends only its own box; a pass
    // that ignores the mask fills the whole rect.
    let rect = if over.is_some_and(|over| over.no_mask) {
        layer.rect
    } else {
        crate::layer_effects::fill_content_box(layer).unwrap_or(layer.rect)
    };
    let x0 = rect.left.max(canvas.x0());
    let y0 = rect.top.max(canvas.y0());
    let x1 = rect.right.min(canvas.x1());
    let y1 = rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let cs = [
        rgba[0] as f32 / 255.0,
        rgba[1] as f32 / 255.0,
        rgba[2] as f32 / 255.0,
    ];
    let src_a = rgba[3] as f32 / 255.0;
    let clip = canvas.clip();
    for y in y0..y1 {
        for x in x0..x1 {
            blend_into_ctx(
                canvas, ctx, layer, doc, clip, over, x as usize, y as usize, cs, src_a,
            );
        }
    }
}

/// Composite a gradient fill across the layer's rect (clamped to the canvas),
/// generating an opaque colour per pixel and blending it through [`blend_into_ctx`].
pub(crate) fn composite_gradient_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    params: &GradientFillParams,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    let w = layer.rect.width();
    let h = layer.rect.height();
    if w <= 0 || h <= 0 {
        return;
    }
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let px = gradient_rgba(w, h, params);
    let row = w as usize;
    let clip = canvas.clip();
    for y in y0..y1 {
        for x in x0..x1 {
            let li = (y - layer.rect.top) as usize * row + (x - layer.rect.left) as usize;
            let c = px[li];
            blend_into_ctx(
                canvas,
                ctx,
                layer,
                doc,
                clip,
                over,
                x as usize,
                y as usize,
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ],
                1.0,
            );
        }
    }
}

/// Fallback when a pattern id is not in the document's decoded pattern library:
/// a missing pattern, or one the decoder skipped (a non-RGB mode, a non-8-bit
/// plane, or a malformed rectangle). It composites an opaque 50 %-grey tile so
/// the layer is visible rather than a no-op. The docs describe a
/// placeholder-or-last-known plus a warning; this uses the grey placeholder and
/// has no warning surface.
/// ponytail: no warning surface in the render crate, and no last-known pattern
/// retained; add them when one exists. Ceiling, not an intended parity claim.
const PATTERN_PLACEHOLDER: [u8; 4] = [128, 128, 128, 255];

/// A tile sampler over a decoded pattern (or the placeholder).
///
/// The tile is `tw × th` pixels; the source is sampled nearest-neighbour, which
/// is exact at scale 100.
///
/// ponytail: scale != 100 uses nearest-neighbour rather than CS6's resample
/// filter; non-RGB pattern modes and 16/32-bit planes never reach here (the
/// codec skips them, so the placeholder renders); `phase`/origin is inferred
/// from the layer-style convention and unverified for `PtFl`.
struct Tile<'a> {
    rgba: &'a [u8],
    pw: i64,
    ph: i64,
    tw: i64,
    th: i64,
    link: bool,
    origin: (i32, i32),
}

impl Tile<'_> {
    /// The pattern pixel for canvas pixel `(cx, cy)` of a rect at
    /// `(rect_left, rect_top)`.
    fn sample(&self, cx: i32, cy: i32, rect_left: i32, rect_top: i32) -> [u8; 4] {
        let sx_base = if self.link { cx - rect_left } else { cx } as i64;
        let sy_base = if self.link { cy - rect_top } else { cy } as i64;
        let sx = (sx_base - self.origin.0 as i64).rem_euclid(self.tw);
        let sy = (sy_base - self.origin.1 as i64).rem_euclid(self.th);
        let px = ((sx as f64 / self.tw as f64) * self.pw as f64).floor() as i64;
        let py = ((sy as f64 / self.th as f64) * self.ph as f64).floor() as i64;
        let px = px.clamp(0, self.pw - 1) as usize;
        let py = py.clamp(0, self.ph - 1) as usize;
        let i = (py * self.pw as usize + px) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

/// Build the tile for `params`, resolving the pattern in the document's decoded
/// pattern set (the placeholder when absent).
fn tile_for<'a>(pattern: Option<&'a PatternPixels>, params: &PatternFillParams) -> Tile<'a> {
    let pattern = pattern.filter(|pattern| {
        if pattern.width == 0 || pattern.height == 0 {
            return false;
        }
        let pixels = (pattern.width as usize).saturating_mul(pattern.height as usize);
        pattern.rgba.len() >= pixels.saturating_mul(4)
    });
    let Some(pattern) = pattern else {
        return Tile {
            rgba: &PATTERN_PLACEHOLDER,
            pw: 1,
            ph: 1,
            tw: 1,
            th: 1,
            link: params.link_with_layer,
            origin: params.origin,
        };
    };
    let scale = if params.scale.is_finite() {
        params.scale as f64
    } else {
        100.0
    };
    let tw = (pattern.width as f64 * scale / 100.0).round().max(1.0) as i64;
    let th = (pattern.height as f64 * scale / 100.0).round().max(1.0) as i64;
    Tile {
        rgba: &pattern.rgba,
        pw: pattern.width as i64,
        ph: pattern.height as i64,
        tw: tw.max(1),
        th: th.max(1),
        link: params.link_with_layer,
        origin: params.origin,
    }
}

/// Composite a pattern fill across the layer's rect (clamped to the canvas),
/// tiling through [`blend_into_ctx`] so the layer's mask, opacity, fill, and blend
/// apply and pixels outside the rect are untouched.
pub(crate) fn composite_pattern_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    params: &PatternFillParams,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // ponytail: the pattern library is decoded once per pattern-fill layer;
    // thread a shared decode through the composite if re-parsing ever shows up.
    let patterns = pictura_codec::decode_patterns(doc);
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let clip = canvas.clip();
    for y in y0..y1 {
        for x in x0..x1 {
            let c = tile.sample(x, y, layer.rect.left, layer.rect.top);
            blend_into_ctx(
                canvas,
                ctx,
                layer,
                doc,
                clip,
                over,
                x as usize,
                y as usize,
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ],
                c[3] as f32 / 255.0,
            );
        }
    }
}

/// Bake the tiled pattern (or placeholder) for an arbitrary canvas `region`
/// into row-major RGBA. Shares [`Tile`] with the compositor; the caller decodes
/// the pattern library once and passes the anchor the tile is linked to.
pub(crate) fn pattern_tile_region(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    anchor: (i32, i32),
    region: (i32, i32, i32, i32),
) -> Vec<[u8; 4]> {
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let (x0, y0, x1, y1) = region;
    let mut out = Vec::with_capacity(((x1 - x0).max(0) * (y1 - y0).max(0)) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(tile.sample(x, y, anchor.0, anchor.1));
        }
    }
    out
}

/// Bake the tiled pattern (or placeholder) for a rect into row-major RGBA, for
/// fill-content rasterization. A thin wrapper over [`pattern_tile_region`]
/// anchored to the rect's top-left.
pub(crate) fn pattern_tile_rgba(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    rect_left: i32,
    rect_top: i32,
    w: i32,
    h: i32,
) -> Vec<[u8; 4]> {
    pattern_tile_region(
        patterns,
        params,
        (rect_left, rect_top),
        (
            rect_left,
            rect_top,
            rect_left + w.max(0),
            rect_top + h.max(0),
        ),
    )
}

/// A `width`x`height` preview of a pattern fill's layer rect, sampling the tile
/// at the scaled document position. Nearest-neighbour; used by the Layers
/// panel thumbnail so a pattern reads at any thumbnail size.
pub fn pattern_preview_rgba(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    rect: pictura_core::PsdRect,
    width: u32,
    height: u32,
) -> Vec<[u8; 4]> {
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let mut out = Vec::with_capacity((width as usize) * (height as usize));
    let sx = rect.width().max(1) as f64 / width.max(1) as f64;
    let sy = rect.height().max(1) as f64 / height.max(1) as f64;
    for y in 0..height {
        for x in 0..width {
            let px = rect.left + (x as f64 * sx) as i32;
            let py = rect.top + (y as f64 * sy) as i32;
            out.push(tile.sample(px, py, rect.left, rect.top));
        }
    }
    out
}

/// Paint a fill layer's coverage alpha into a region `(x0, y0, x1, y1)` of the
/// canvas (row-major `(x1-x0) × (y1-y0)`), zero outside the layer rect. Returns
/// `false` when `layer` is not a decoded fill, so the caller falls back to the
/// pixel/smart path.
///
/// ponytail: the pattern tile is sampled nearest-neighbour, matching the
/// compositor's [`Tile`].
pub(crate) fn fill_coverage_matte(
    adjustment: &FillContent,
    layer: &Layer,
    doc: &Document,
    region: (i32, i32, i32, i32),
    out: &mut [f32],
) -> bool {
    let (rx0, ry0, rx1, ry1) = region;
    let rw = (rx1 - rx0) as usize;
    let ix0 = layer.rect.left.max(rx0);
    let iy0 = layer.rect.top.max(ry0);
    let ix1 = layer.rect.right.min(rx1);
    let iy1 = layer.rect.bottom.min(ry1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return true;
    }
    let mut put = |x: i32, y: i32, value: f32| {
        out[(y - ry0) as usize * rw + (x - rx0) as usize] = value;
    };
    match adjustment {
        pictura_core::FillContent::Solid(rgba) => {
            let alpha = rgba[3] as f32 / 255.0;
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    put(x, y, alpha);
                }
            }
            true
        }
        pictura_core::FillContent::Gradient(_) => {
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    put(x, y, 1.0);
                }
            }
            true
        }
        pictura_core::FillContent::Pattern(params) => {
            let patterns = pictura_codec::decode_patterns(doc);
            let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
            let tile = tile_for(pattern, params);
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    let c = tile.sample(x, y, layer.rect.left, layer.rect.top);
                    put(x, y, c[3] as f32 / 255.0);
                }
            }
            true
        }
    }
}

/// Rasterize a fill layer's content into the straight-alpha RGBA buffer the GPU
/// uploads, or `None` when the layer is not a decoded fill.
pub(crate) fn fill_source(
    layer: &Layer,
    doc: &Document,
) -> Option<crate::layer_source::CachedBuffer> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::Arc;

    let adjustment = decode_layer_fill(layer)?;
    // An adjustment layer has an empty rect, so the kind is settled before the
    // no-area case below can pass one off as an empty fill.
    let full_rect = layer.rect;
    let (full_w, full_h) = (full_rect.width(), full_rect.height());
    // A shape's rect is the canvas but its mask is zero outside the mask's
    // box, so only that box is rasterized and uploaded (#1185).
    let rect = crate::layer_effects::fill_content_box(layer)
        .unwrap_or(full_rect)
        .clip_to(doc.width, doc.height)
        .unwrap_or_default();
    let (w, h) = (rect.width(), rect.height());
    // A fill with no on-canvas area is the CPU's no-op; keep it on the GPU as an
    // empty source rather than declining the whole document.
    let (buffer, retain) = if w <= 0 || h <= 0 || full_w <= 0 || full_h <= 0 {
        (PixelBuffer::new(0, 0, 4), false)
    } else {
        match &adjustment {
            pictura_core::FillContent::Solid(rgba) => {
                let n = w as usize * h as usize;
                let data = pictura_core::Plane::build(n * 4, |out| {
                    for (plane, &value) in out.chunks_mut(n).zip(rgba) {
                        plane.fill(value);
                    }
                });
                let buf = PixelBuffer {
                    width: w as u32,
                    height: h as u32,
                    channels: 4,
                    data,
                };
                (buf, true)
            }
            pictura_core::FillContent::Gradient(params) => {
                let px = gradient_rgba_region(
                    full_w,
                    full_h,
                    (
                        rect.left - full_rect.left,
                        rect.top - full_rect.top,
                        rect.right - full_rect.left,
                        rect.bottom - full_rect.top,
                    ),
                    params,
                );
                (rgba_to_planar(&px, w, h), true)
            }
            pictura_core::FillContent::Pattern(params) => {
                let patterns = pictura_codec::decode_patterns(doc);
                let px = pattern_tile_region(
                    &patterns,
                    params,
                    (full_rect.left, full_rect.top),
                    (rect.left, rect.top, rect.right, rect.bottom),
                );
                (rgba_to_planar(&px, w, h), false)
            }
        }
    };
    let mut hasher = DefaultHasher::new();
    layer.content_identity().hash(&mut hasher);
    [
        full_rect.left,
        full_rect.top,
        full_rect.right,
        full_rect.bottom,
    ]
    .hash(&mut hasher);
    [rect.left, rect.top, rect.right, rect.bottom].hash(&mut hasher);
    doc.width.hash(&mut hasher);
    doc.height.hash(&mut hasher);
    doc.mode.hash(&mut hasher);
    if let Some(data) = layer.adjustment() {
        data.key.hash(&mut hasher);
        data.data.hash(&mut hasher);
    }
    if let Some(block) = layer.extra_block(b"vscg") {
        block.data.hash(&mut hasher);
    }
    Some(crate::layer_source::CachedBuffer {
        rect,
        buffer: Arc::new(buffer),
        content: hasher.finish(),
        retain,
    })
}

fn rgba_to_planar(px: &[[u8; 4]], w: i32, h: i32) -> PixelBuffer {
    let n = (w.max(0) as usize) * (h.max(0) as usize);
    let mut buf = PixelBuffer::new(w.max(0) as u32, h.max(0) as u32, 4);
    for (i, c) in px.iter().take(n).enumerate() {
        buf.data[i] = c[0];
        buf.data[n + i] = c[1];
        buf.data[2 * n + i] = c[2];
        buf.data[3 * n + i] = c[3];
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_fill_upload_is_not_retained() {
        let bytes = include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd");
        let doc = pictura_codec::read_psd(bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter()
            .find(|layer| {
                matches!(
                    decode_layer_fill(layer),
                    Some(pictura_core::FillContent::Pattern(_))
                )
            })
            .expect("a pattern fill layer");
        let cached = fill_source(layer, &doc).expect("the fill rasterizes");
        assert!(!cached.retain, "a pattern fill is not retained");
    }

    #[test]
    fn all_fill_fixtures_rasterize() {
        for (name, bytes) in [
            (
                "solid",
                include_bytes!("../../pictura-codec/tests/fixtures/solid_fill.psd").as_slice(),
            ),
            (
                "gradient",
                include_bytes!("../../pictura-codec/tests/fixtures/gradient_fill.psd").as_slice(),
            ),
            (
                "pattern",
                include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd").as_slice(),
            ),
            (
                "vector",
                include_bytes!("../../pictura-codec/tests/fixtures/vector_fill.psd").as_slice(),
            ),
        ] {
            let doc = pictura_codec::read_psd(bytes).expect("fixture parses");
            for layer in doc.layers.iter() {
                if decode_layer_fill(layer).is_some() {
                    assert!(fill_source(layer, &doc).is_some(), "{name}: {}", layer.name);
                }
            }
        }
    }

    #[test]
    fn changing_the_pattern_content_changes_the_raster() {
        let bytes = include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd");
        let mut doc = pictura_codec::read_psd(bytes).expect("fixture parses");
        let mut layer = doc
            .layers
            .iter()
            .find(|layer| {
                matches!(
                    decode_layer_fill(layer),
                    Some(pictura_core::FillContent::Pattern(_))
                )
            })
            .expect("a pattern fill layer")
            .clone();
        let Some(pictura_core::FillContent::Pattern(mut params)) = decode_layer_fill(&layer) else {
            unreachable!()
        };
        let first = fill_source(&layer, &doc).expect("first raster");
        params.pattern_id = "pictura-changed".into();
        pictura_codec::edit_layer(layer.as_mut(), |parts| {
            parts.adjustment = Some(encode_pattern_fill(&params))
        });
        let record = pictura_codec::encode_rgb_pattern(
            "changed",
            "pictura-changed",
            4,
            4,
            &[255u8; 4 * 4 * 4],
        )
        .expect("pattern encodes");
        assert!(pictura_codec::add_document_pattern(
            &mut doc,
            "pictura-changed",
            &record
        ));
        let second = fill_source(&layer, &doc).expect("second raster");
        assert_ne!(first.content, second.content);
        assert_ne!(first.buffer.data.as_ref(), second.buffer.data.as_ref());
    }
}

pub(crate) fn composite_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    fill: &FillContent,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) {
    match fill {
        FillContent::Solid(rgba) => composite_solid_fill(canvas, layer, doc, *rgba, ctx, over),
        FillContent::Gradient(params) => {
            composite_gradient_fill(canvas, layer, doc, params, ctx, over)
        }
        FillContent::Pattern(params) => {
            composite_pattern_fill(canvas, layer, doc, params, ctx, over)
        }
    }
}
