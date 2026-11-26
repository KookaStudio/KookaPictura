//! Document-scope arbitrary rotation (`rotate_document_in`): the angle-form
//! sibling of the exact quarter-turn [`super::rotate_document`].
//!
//! Every layer channel plane, raster mask, document channel, retained native
//! store, and vector-mask subpath is resampled about a document-space pivot,
//! the canvas grows to the rotated content's axis-aligned bounding box, and the
//! document is translated so the box's top-left is the origin. Sampling is the
//! same inverse map and clamp-to-edge rule as `pictura_ops::rotate_in`, over
//! the shared [`pictura_core::Sample`] domain. Bilinear remains the legacy kernel.
//! Every plane is resampled into spillable tiles a strip at a time (#1294); the
//! dense inverse map stays as the tiled one's oracle.

use pictura_core::memory::MemoryError;
use pictura_core::{Document, Plane, PsdRect, Sample, Samples, SourceChannels, TileSample};

use super::layer_ops::kernel::{self, Interpolation};
use super::{for_each_layer, geometry_tiled, materialize_merged_composite, sync_vector_mask_block};

/// The largest accepted magnitude, matching `pictura_ops::rotate_in`.
const MAX_ANGLE: f64 = 359.99;

/// A rotated corner landing exactly on an integer must not round out to the
/// next pixel; same intent as `transform.rs`'s bbox slack.
const BBOX_SLACK: f64 = 1e-9;

/// Whether any layer (recursively) carries a live vector mask. A suppressed
/// rendered-source `vmsk` has no view and is intentionally left untransformed;
/// it is inert source data, not a mask.
fn has_vector_mask(layers: &[pictura_core::Layer]) -> bool {
    layers
        .iter()
        .any(|l| l.vector_mask.is_some() || has_vector_mask(l.children()))
}

/// The document-space similarity about `pivot`: `q = pivot + R(θ)·(p − pivot)`
/// with positive θ clockwise in the y-down screen convention.
#[derive(Clone, Copy)]
struct Rotation {
    pivot_x: f64,
    pivot_y: f64,
    cos: f64,
    sin: f64,
    /// Translation added after rotation so the rotated canvas' min lands on 0.
    ox: f64,
    oy: f64,
    interpolation: Interpolation,
}

impl Rotation {
    /// Map a source document point to the new document frame.
    fn forward(&self, x: f64, y: f64) -> (f64, f64) {
        let ux = x - self.pivot_x;
        let uy = y - self.pivot_y;
        (
            self.pivot_x + self.cos * ux - self.sin * uy + self.ox,
            self.pivot_y + self.sin * ux + self.cos * uy + self.oy,
        )
    }

    /// Map a new-frame point back to the source document frame.
    fn inverse(&self, qx: f64, qy: f64) -> (f64, f64) {
        let vx = qx - self.ox - self.pivot_x;
        let vy = qy - self.oy - self.pivot_y;
        let rx = self.cos * vx + self.sin * vy;
        let ry = -self.sin * vx + self.cos * vy;
        (self.pivot_x + rx, self.pivot_y + ry)
    }
}

/// Integer bbox of the four rotated corners of `rect` in the new frame.
fn rotated_rect(rect: PsdRect, rot: &Rotation) -> PsdRect {
    let corners = [
        (rect.left as f64, rect.top as f64),
        (rect.right as f64, rect.top as f64),
        (rect.right as f64, rect.bottom as f64),
        (rect.left as f64, rect.bottom as f64),
    ];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in corners {
        let (qx, qy) = rot.forward(x, y);
        min_x = min_x.min(qx);
        min_y = min_y.min(qy);
        max_x = max_x.max(qx);
        max_y = max_y.max(qy);
    }
    PsdRect {
        top: (min_y + BBOX_SLACK).floor() as i32,
        left: (min_x + BBOX_SLACK).floor() as i32,
        bottom: (max_y - BBOX_SLACK).ceil() as i32,
        right: (max_x - BBOX_SLACK).ceil() as i32,
    }
}

/// Bilinear sample of a `sw×sh` plane at plane-local `(x, y)` (pixel centres
/// at `+0.5`), reading each tap's unit value through `at(px, py)`; outside the
/// source rectangle the sample is the zero of the domain. The dense and the
/// tiled resamplers differ only in `at`, so their output is byte-identical.
fn sample_bilinear<T: Sample>(
    sw: usize,
    sh: usize,
    x: f64,
    y: f64,
    at: impl Fn(usize, usize) -> f64,
) -> T {
    let u = x - 0.5;
    let v = y - 0.5;
    let x0f = u.floor();
    let y0f = v.floor();
    let fx = u - x0f;
    let fy = v - y0f;
    let max_x = sw.saturating_sub(1) as f64;
    let max_y = sh.saturating_sub(1) as f64;
    let x0 = x0f.clamp(0.0, max_x) as usize;
    let y0 = y0f.clamp(0.0, max_y) as usize;
    let x1 = (x0f + 1.0).clamp(0.0, max_x) as usize;
    let y1 = (y0f + 1.0).clamp(0.0, max_y) as usize;
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    T::from_unit(top * (1.0 - fy) + bottom * fy)
}

fn sample_window<T: Sample>(
    source: (usize, usize),
    point: (f64, f64),
    window: &[T],
    width: usize,
    origin: (usize, usize),
    interpolation: Interpolation,
) -> T {
    let (sw, sh) = source;
    let (x, y) = point;
    if x < 0.0 || y < 0.0 || x > sw as f64 || y > sh as f64 {
        return T::from_unit(0.0);
    }
    let (x0, y0) = origin;
    if interpolation == Interpolation::Bilinear {
        return sample_bilinear(sw, sh, x, y, |ix, iy| {
            window[(iy - y0) * width + ix - x0].to_unit()
        });
    }
    kernel::sample(
        window,
        width,
        window.len() / width,
        x - x0 as f64,
        y - y0 as f64,
        interpolation,
    )
}

/// The dense inverse map of one plane (`sw×sh` at source `rect`) into `dest`,
/// kept as [`rotate_tiled`]'s oracle.
#[cfg(test)]
fn resample_samples<T: Sample + Copy>(
    src: &[T],
    sw: usize,
    sh: usize,
    src_rect: PsdRect,
    dest: PsdRect,
    rot: &Rotation,
) -> Vec<T> {
    let dw = dest.width().max(0) as usize;
    let dh = dest.height().max(0) as usize;
    let mut out = Vec::with_capacity(dw * dh);
    for yy in 0..dh {
        for xx in 0..dw {
            let qx = dest.left as f64 + xx as f64 + 0.5;
            let qy = dest.top as f64 + yy as f64 + 0.5;
            let (px, py) = rot.inverse(qx, qy);
            out.push(sample_window(
                (sw, sh),
                (px - src_rect.left as f64, py - src_rect.top as f64),
                src,
                sw,
                (0, 0),
                rot.interpolation,
            ));
        }
    }
    out
}

/// The inverse map of `channels` stacked planes covering `src_rect` into
/// `dest`, built into spillable tiles (#1294). A block reads the box its four
/// corner pixels map back to, padded so rounding at interior pixels stays
/// inside it, and clamped to the source.
fn rotate_tiled<T: TileSample + Sample>(
    src: &Plane<T>,
    src_rect: PsdRect,
    dest: PsdRect,
    channels: usize,
    rot: &Rotation,
) -> Result<Plane<T>, MemoryError> {
    let (sw, sh) = dims(src_rect);
    let (dw, dh) = dims(dest);
    if sw == 0 || sh == 0 {
        return Ok(if dw * dh == 0 {
            Plane::default()
        } else {
            Plane::filled(dw, dh, channels, T::from_unit(0.0))
        });
    }
    let src = geometry_tiled::whole(src, sw * sh * channels, T::default());
    let src = src.as_ref();
    let (left, top) = (src_rect.left as f64, src_rect.top as f64);
    let source = |xx: usize, yy: usize| {
        let qx = dest.left as f64 + xx as f64 + 0.5;
        let qy = dest.top as f64 + yy as f64 + 0.5;
        let (px, py) = rot.inverse(qx, qy);
        (px - left, py - top)
    };
    geometry_tiled::build((dw, dh), channels, |channel, b, out, stride, window| {
        let (mut lo_x, mut lo_y) = (f64::INFINITY, f64::INFINITY);
        let (mut hi_x, mut hi_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (xx, yy) in [
            (b.x, b.y),
            (b.x + b.w - 1, b.y),
            (b.x, b.y + b.h - 1),
            (b.x + b.w - 1, b.y + b.h - 1),
        ] {
            let (x, y) = source(xx, yy);
            (lo_x, hi_x) = (lo_x.min(x), hi_x.max(x));
            (lo_y, hi_y) = (lo_y.min(y), hi_y.max(y));
        }
        // Cubic taps span floor(c − 0.5) − 1 through +2. Keep one
        // extra sample at each side for floating-point roundoff.
        let span = |lo: f64, hi: f64, n: usize| {
            let max = (n - 1) as f64;
            let first = ((lo - 0.5).floor() - 2.0).clamp(0.0, max) as usize;
            let last = ((hi - 0.5).floor() + 3.0).clamp(0.0, max) as usize;
            first..last.max(first) + 1
        };
        let xs = span(lo_x, hi_x, sw);
        let ys = span(lo_y, hi_y, sh);
        let (x0, y0) = (xs.start, ys.start);
        let ww = geometry_tiled::read_window(src, (sw, sh), channel, xs, ys, window);
        for r in 0..b.h {
            for (c, o) in out[r * stride..][..b.w].iter_mut().enumerate() {
                *o = sample_window(
                    (sw, sh),
                    source(b.x + c, b.y + r),
                    window,
                    ww,
                    (x0, y0),
                    rot.interpolation,
                );
            }
        }
    })
}

fn dims(rect: PsdRect) -> (usize, usize) {
    (rect.width().max(0) as usize, rect.height().max(0) as usize)
}

/// Every stacked plane of a native store covering `src_rect` through
/// [`rotate_tiled`]; a trailing partial plane is dropped.
fn rotate_samples(
    samples: &Samples,
    src_rect: PsdRect,
    dest: PsdRect,
    rot: &Rotation,
) -> Result<Samples, MemoryError> {
    let (sw, sh) = dims(src_rect);
    let channels = samples.len() / (sw * sh).max(1);
    Ok(match samples {
        Samples::U8(p) => Samples::U8(rotate_tiled(p, src_rect, dest, channels, rot)?),
        Samples::U16(p) => Samples::U16(rotate_tiled(p, src_rect, dest, channels, rot)?),
        Samples::F32(p) => Samples::F32(rotate_tiled(p, src_rect, dest, channels, rot)?),
    })
}

/// Resample each plane of a native store that covers `layer_rect` (mask `-2`
/// planes cover `mask_rect`).
fn resample_store(
    store: &SourceChannels,
    layer_rect: PsdRect,
    layer_dest: PsdRect,
    mask_rect: Option<PsdRect>,
    mask_dest: Option<PsdRect>,
    rot: &Rotation,
) -> Result<SourceChannels, MemoryError> {
    let mut planes = Vec::with_capacity(store.planes.len());
    for (id, samples) in &store.planes {
        let (src_rect, dest) = if *id == -2 {
            match (mask_rect, mask_dest) {
                (Some(src), Some(dest)) => (src, dest),
                _ => continue,
            }
        } else {
            (layer_rect, layer_dest)
        };
        planes.push((*id, rotate_samples(samples, src_rect, dest, rot)?));
    }
    Ok(SourceChannels::new(store.depth, layer_dest, planes))
}

/// Rotate every subpath vertex about the pivot, in the new frame.
fn rotate_vector_mask(mask: &mut pictura_core::VectorMask, rot: &Rotation) {
    for subpath in &mut mask.subpaths {
        for point in &mut subpath.points {
            let (px, py) = (point[0] as f64 / 256.0, point[1] as f64 / 256.0);
            let (qx, qy) = rot.forward(px, py);
            point[0] = (qx * 256.0).round() as i32;
            point[1] = (qy * 256.0).round() as i32;
        }
    }
    mask.rows = Default::default();
}

/// The rotated canvas frame: the rotation's sin/cos, the translation that puts
/// the pivot at the destination centre (or the bbox at the origin), and the new
/// dimensions.
struct Frame {
    sin: f64,
    cos: f64,
    ox: f64,
    oy: f64,
    new_w: u32,
    new_h: u32,
}

fn frame_geometry(w: f64, h: f64, angle_deg: f64, pivot: (f64, f64)) -> Frame {
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (x, y) in corners {
        let ux = x - pivot.0;
        let uy = y - pivot.1;
        let qx = pivot.0 + cos * ux - sin * uy;
        let qy = pivot.1 + sin * ux + cos * uy;
        min_x = min_x.min(qx);
        min_y = min_y.min(qy);
        max_x = max_x.max(qx);
        max_y = max_y.max(qy);
    }
    let new_w = ((max_x - min_x) - BBOX_SLACK).ceil().max(1.0) as u32;
    let new_h = ((max_y - min_y) - BBOX_SLACK).ceil().max(1.0) as u32;
    // Anchor the pivot at the destination centre when it is the canvas centre
    // (matching `rotate_in`'s grid); otherwise shift the bounding box to origin.
    let (ox, oy) = if pivot == (w / 2.0, h / 2.0) {
        (new_w as f64 / 2.0 - pivot.0, new_h as f64 / 2.0 - pivot.1)
    } else {
        (-min_x, -min_y)
    };
    Frame {
        sin,
        cos,
        ox,
        oy,
        new_w,
        new_h,
    }
}

/// The `(ox, oy)` translation [`rotate_document_in`] applies for a `width`×
/// `height` document, so a caller can project a point (e.g. a crop box) into the
/// rotated frame. `None` for the same invalid inputs `rotate_document_in`
/// refuses; `(0.0, 0.0)` for angle 0.
pub fn rotate_document_in_offset(
    width: u32,
    height: u32,
    angle_deg: f64,
    pivot: (f64, f64),
) -> Option<(f64, f64)> {
    if !angle_deg.is_finite() || !(-MAX_ANGLE..=MAX_ANGLE).contains(&angle_deg) {
        return None;
    }
    if width == 0 || height == 0 {
        return None;
    }
    if angle_deg == 0.0 {
        return Some((0.0, 0.0));
    }
    let frame = frame_geometry(width as f64, height as f64, angle_deg, pivot);
    Some((frame.ox, frame.oy))
}

/// Rotate the whole document about `pivot` by `angle_deg` (positive clockwise),
/// growing the canvas to the rotated content's bounding box. Returns `false`
/// without mutating for a non-finite or out-of-range angle, an empty document,
/// or a zero-area result. An angle of `0.0`, or an exact quarter turn about the
/// document centre, returns `true` (delegating to the exact remap for the
/// latter).
pub fn rotate_document_in(doc: &mut Document, angle_deg: f64, pivot: (f64, f64)) -> bool {
    rotate_document_in_with(doc, angle_deg, pivot, Interpolation::Bilinear)
}

/// Rotate a document using the selected raster kernel. An exact turn about
/// the canvas centre maps indices directly, regardless of interpolation.
pub fn rotate_document_in_with(
    doc: &mut Document,
    angle_deg: f64,
    pivot: (f64, f64),
    interpolation: Interpolation,
) -> bool {
    if !angle_deg.is_finite() || !(-MAX_ANGLE..=MAX_ANGLE).contains(&angle_deg) {
        return false;
    }
    if doc.width == 0 || doc.height == 0 {
        return false;
    }
    if angle_deg == 0.0 {
        return true;
    }
    let w = doc.width as f64;
    let h = doc.height as f64;
    if pivot == (w / 2.0, h / 2.0) {
        let normalized = angle_deg.rem_euclid(360.0);
        let quarter = match normalized {
            a if (a - 90.0).abs() < 1e-9 => Some(1u8),
            a if (a - 180.0).abs() < 1e-9 => Some(2),
            a if (a - 270.0).abs() < 1e-9 => Some(3),
            _ => None,
        };
        if let Some(q) = quarter {
            if !has_vector_mask(&doc.layers) {
                return super::rotate_document(doc, q).is_ok();
            }
            // The exact remap doesn't rotate vector vertices. In the geometry
            // path each pixel centre still maps to one source pixel.
            return rotate_frame(doc, f64::from(q) * 90.0, pivot, Interpolation::Nearest).is_ok();
        }
    }

    if doc.source.multichannel_plates() {
        return false;
    }
    rotate_frame(doc, angle_deg, pivot, interpolation.resolve(1, 1)).is_ok()
}

/// The document's rotation and its new canvas.
fn rotation(width: u32, height: u32, angle_deg: f64, pivot: (f64, f64)) -> (Rotation, u32, u32) {
    let Frame {
        sin,
        cos,
        ox,
        oy,
        new_w,
        new_h,
    } = frame_geometry(width as f64, height as f64, angle_deg, pivot);
    let rot = Rotation {
        pivot_x: pivot.0,
        pivot_y: pivot.1,
        cos,
        sin,
        ox,
        oy,
        interpolation: Interpolation::Bilinear,
    };
    (rot, new_w, new_h)
}

/// Resample every plane into spillable tiles (#1294) on a copy that shares the
/// document's planes, so a refused strip leaves the document unchanged.
fn rotate_frame(
    doc: &mut Document,
    angle_deg: f64,
    pivot: (f64, f64),
    interpolation: Interpolation,
) -> Result<(), MemoryError> {
    let mut next = doc.clone();
    materialize_merged_composite(&mut next);
    let (mut rot, new_w, new_h) = rotation(next.width, next.height, angle_deg, pivot);
    rot.interpolation = interpolation;
    if let Some(job) = pictura_core::job::current() {
        job.begin_stage(rotated_rows(&mut next, &rot, new_h));
    }

    let mut refused = None;
    for_each_layer(&mut next.layers, &mut |layer| {
        if refused.is_none() {
            refused = rotate_layer(layer, &rot, new_w, new_h).err();
        }
    });
    if let Some(err) = refused {
        return Err(err);
    }

    let src = canvas_rect(next.width, next.height);
    let dest = canvas_rect(new_w, new_h);
    for channel in &mut next.channels {
        channel.data = rotate_tiled(&channel.data, src, dest, 1, &rot)?;
    }
    if let Some(mut store) = next.source.planes().clone() {
        // The depth-1 store is a packed plane, not a `width * height` one; it is
        // only re-emitted for an unchanged flat Bitmap, so leave it stale.
        if store.depth != pictura_core::BitDepth::One {
            store.samples = rotate_samples(&store.samples, src, dest, &rot)?;
            store.width = new_w;
            store.height = new_h;
        }

        next.source
            .replace_planes(Some(store))
            .expect("valid source planes");
    }

    next.width = new_w;
    next.height = new_h;
    *doc = next;
    Ok(())
}

fn rotate_layer(
    layer: &mut pictura_core::Layer,
    rot: &Rotation,
    new_w: u32,
    new_h: u32,
) -> Result<(), MemoryError> {
    let old_rect = layer.rect;
    let new_rect = rotated_rect(old_rect, rot);
    let (mask_src, mask_dest) = match &layer.mask {
        Some(mask) => (Some(mask.rect), Some(rotated_rect(mask.rect, rot))),
        None => (None, None),
    };

    for channel in layer.channels_mut() {
        channel.data = rotate_tiled(&channel.data, old_rect, new_rect, 1, rot)?;
    }
    if let Some(store) = layer.source_channels.take() {
        layer.source_channels = Some(resample_store(
            &store, old_rect, new_rect, mask_src, mask_dest, rot,
        )?);
    }
    layer.rect = new_rect;
    // ponytail: an unmodeled raw on-disk stream cannot be resampled, so it
    // drops (matching the exact orientation remap).
    layer.discard_raw_channels();

    if let Some(mask) = &mut layer.mask {
        let dest = rotated_rect(mask.rect, rot);
        if let Some(data) = &mut mask.data {
            *data = rotate_tiled(data, mask.rect, dest, 1, rot)?;
        }
        mask.rect = dest;
    }
    if let Some(vector) = &mut layer.vector_mask {
        rotate_vector_mask(vector, rot);
    }
    // Keep the authoritative `vmsk` block in sync with the rotated view so
    // a save does not re-emit the pre-rotation outline.
    sync_vector_mask_block(layer, new_w, new_h);
    Ok(())
}

fn canvas_rect(width: u32, height: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    }
}

/// The output rows [`rotate_frame`] writes, which the tiled resampler reports
/// as a job's progress.
fn rotated_rows(doc: &mut Document, rot: &Rotation, new_h: u32) -> u64 {
    let area = (doc.width as usize * doc.height as usize).max(1);
    let store = doc
        .source
        .planes()
        .as_ref()
        .filter(|s| s.depth != pictura_core::BitDepth::One)
        .map_or(0, |s| s.samples.len() / area);
    let mut rows = (doc.channels.len() + store) as u64 * u64::from(new_h);
    let mut count = |rect: PsdRect, planes: usize| {
        let (w, h) = dims(rect);
        if w * h > 0 {
            rows += planes as u64 * dims(rotated_rect(rect, rot)).1 as u64;
        }
    };
    for_each_layer(&mut doc.layers, &mut |layer| {
        count(layer.rect, layer.channels().len());
        if let Some(mask) = layer.mask.as_ref().filter(|m| m.data.is_some()) {
            count(mask.rect, 1);
        }
        for (id, samples) in layer.source_channels.iter().flat_map(|s| &s.planes) {
            let rect = match (&layer.mask, *id) {
                (Some(mask), -2) => mask.rect,
                (None, -2) => continue,
                _ => layer.rect,
            };
            count(rect, samples.len() / (dims(rect).0 * dims(rect).1).max(1));
        }
    });
    rows
}

/// What [`rotate_document_in`] about the canvas centre by `angle_deg` writes
/// (#1294, the app's memory gate): every rotated plane's bytes, the widest of
/// them in samples, and the new canvas. An exact quarter turn is an orientation.
pub fn rotated_plane_bytes(doc: &Document, angle_deg: f64) -> (u64, u64, (u32, u32)) {
    let pivot = (doc.width as f64 / 2.0, doc.height as f64 / 2.0);
    let (rot, new_w, new_h) = rotation(doc.width, doc.height, angle_deg, pivot);
    let canvas = (new_w, new_h);
    let (bytes, widest) = super::resize::mapped_plane_bytes(doc, canvas, |rect| {
        let (w, h) = dims(rect);
        (w * h > 0).then(|| {
            let (nw, nh) = dims(rotated_rect(rect, &rot));
            (nw as u32, nh as u32)
        })
    });
    (bytes, widest, canvas)
}

/// The dense inverse map of one plane of a `canvas` document rotated by
/// `angle_deg` about `pivot`: `rotate_tiled`'s oracle.
#[cfg(test)]
pub(super) fn rotate_plane_dense<T: Sample + Copy>(
    src: &[T],
    src_rect: PsdRect,
    dest: PsdRect,
    canvas: (u32, u32),
    angle_deg: f64,
    pivot: (f64, f64),
) -> Vec<T> {
    let (rot, _, _) = rotation(canvas.0, canvas.1, angle_deg, pivot);
    let (sw, sh) = dims(src_rect);
    resample_samples(src, sw, sh, src_rect, dest, &rot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorMode, Layer, LayerMask, LockFlags, Samples,
        SourceChannels,
    };

    fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    fn pixel_layer(name: &str, r: PsdRect, n: usize) -> Layer {
        pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: name.into(),
            rect: r,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            visible: true,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![10u8; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20u8; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30u8; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255u8; n].into(),
                },
            ],
            ..Default::default()
        })
    }

    fn doc() -> Document {
        let mut doc = Document::new(5, 3, ColorMode::Rgb, BitDepth::Eight);
        let mut bg = pixel_layer("Background", rect(0, 0, 3, 5), 15);
        bg.background = true;
        let mask = LayerMask {
            rect: rect(0, 0, 3, 5),
            default_color: 255,
            data: Some(vec![7u8; 15].into()),
            ..Default::default()
        };
        let mut top = pixel_layer("top", rect(0, 0, 3, 5), 15);
        top.mask = Some(mask);
        doc.layers = pictura_core::LayerStack::from_layers(vec![bg, top]);
        doc.composite
            .replace(crate::composite_rgba(&doc))
            .expect("valid composite");
        doc
    }

    #[test]
    fn angle_zero_is_unchanged() {
        let mut d = doc();
        let before = d.clone();
        assert!(rotate_document_in(&mut d, 0.0, (2.5, 1.5)));
        assert_eq!(d, before);
    }

    #[test]
    fn invalid_angle_is_refused_without_mutation() {
        for a in [f64::NAN, f64::INFINITY, 360.0, -359.995] {
            let mut d = doc();
            let before = d.clone();
            assert!(!rotate_document_in(&mut d, a, (2.5, 1.5)), "angle {a}");
            assert_eq!(d, before, "angle {a}");
        }
    }

    #[test]
    fn quarter_turn_about_centre_matches_exact_remap() {
        let mut a = doc();
        let mut b = doc();
        rotate_document_in(&mut a, 90.0, (2.5, 1.5));
        super::super::rotate_document(&mut b, 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn arbitrary_angle_grows_the_canvas_and_keeps_content() {
        let mut d = doc();
        // 45° about the centre: the bounding box grows past the original.
        assert!(rotate_document_in(&mut d, 45.0, (2.5, 1.5)));
        assert!(d.width >= 5 && d.height >= 3);
        d.composite
            .replace(crate::composite_rgba(&d))
            .expect("valid composite");
        assert_eq!(d.composite, crate::composite_rgba(&d));
        for layer in &d.layers {
            let area = (layer.rect.width() * layer.rect.height()) as usize;
            for channel in layer.channels() {
                assert_eq!(channel.data.len(), area, "{} {}", layer.name, channel.id);
            }
            if let Some(mask) = &layer.mask {
                let marea = (mask.rect.width() * mask.rect.height()) as usize;
                assert_eq!(mask.data.as_ref().unwrap().len(), marea);
            }
        }
    }

    #[test]
    fn native_store_is_resampled() {
        let mut d = Document::new(3, 2, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("only", rect(0, 0, 2, 3), 6);
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            layer.rect,
            vec![(0, Samples::U16(vec![100u16; 6].into()))],
        ));
        d.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        d.composite
            .replace(crate::composite_rgba(&d))
            .expect("valid composite");

        assert!(rotate_document_in(&mut d, 30.0, (1.5, 1.0)));
        let store = d.layers[0].source_channels.as_ref().unwrap();
        let area = (d.layers[0].rect.width() * d.layers[0].rect.height()) as usize;
        let (_, samples) = store.planes.iter().find(|(id, _)| *id == 0).unwrap();
        assert_eq!(samples.len(), area);
    }

    #[test]
    fn offset_helper_matches_the_rotated_frame() {
        let (w, h) = (20u32, 10u32);
        let pivot = (10.0, 5.0);
        assert_eq!(
            rotate_document_in_offset(w, h, 0.0, pivot),
            Some((0.0, 0.0))
        );
        assert!(rotate_document_in_offset(w, h, f64::NAN, pivot).is_none());
        assert!(rotate_document_in_offset(w, h, 360.0, pivot).is_none());

        let (ox, oy) = rotate_document_in_offset(w, h, 30.0, pivot).unwrap();
        let geo = frame_geometry(w as f64, h as f64, 30.0, pivot);
        assert!((ox - (geo.new_w as f64 / 2.0 - pivot.0)).abs() < 1e-9);
        assert!((oy - (geo.new_h as f64 / 2.0 - pivot.1)).abs() < 1e-9);
    }

    const INTERPOLATIONS: [Interpolation; 6] = [
        Interpolation::Nearest,
        Interpolation::Bilinear,
        Interpolation::Bicubic,
        Interpolation::BicubicSmoother,
        Interpolation::BicubicSharper,
        Interpolation::BicubicAutomatic,
    ];

    #[test]
    fn tiled_rotation_uses_selected_interpolation() {
        let (w, h) = (310, 83);
        let n = w as usize * h as usize;
        let source = canvas_rect(w, h);
        let pixels: Vec<u8> = (0..n)
            .map(|i| if (i / w as usize + i % w as usize) % 3 == 0 { 255 } else { 0 })
            .collect();
        let mask: Vec<u8> = pixels.iter().map(|&v| 255 - v).collect();
        let native: Vec<u16> = pixels.iter().map(|&v| u16::from(v) * 251).collect();
        let mut layer = pixel_layer("Contrast", source, n);
        layer.channels_mut()[0].data = pixels.clone().into();
        layer.mask = Some(LayerMask {
            rect: source,
            default_color: 0,
            data: Some(mask.clone().into()),
            ..Default::default()
        });
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            source,
            vec![(0, Samples::U16(native.clone().into()))],
        ));
        let mut before = Document::new(w, h, ColorMode::Rgb, BitDepth::Sixteen);
        before.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        before.channels.push(Channel { id: 0, data: mask.clone().into() });

        let pivot = (f64::from(w) / 2.0, f64::from(h) / 2.0);
        let degrees = 21.0;
        let (rotation, new_w, new_h) = rotation(w, h, degrees, pivot);
        assert!(new_w > 256);
        let dest = canvas_rect(new_w, new_h);
        let mut prior_red = Vec::new();
        for interpolation in INTERPOLATIONS {
            let mut actual = before.clone();
            assert!(rotate_document_in_with(&mut actual, degrees, pivot, interpolation));
            assert_eq!((actual.width, actual.height), (new_w, new_h));

            let dense_rotation = Rotation { interpolation: interpolation.resolve(1, 1), ..rotation };
            let layer = &actual.layers[0];
            let wanted_red = resample_samples(&pixels, w as usize, h as usize, source, dest, &dense_rotation);
            let wanted_mask = resample_samples(&mask, w as usize, h as usize, source, dest, &dense_rotation);
            let wanted_native = resample_samples(&native, w as usize, h as usize, source, dest, &dense_rotation);
            assert_eq!(layer.rect, dest);
            assert_eq!(wanted_red[0], 0);
            assert_eq!(wanted_mask[0], 0);
            assert!(layer.channels()[0].data.is_tiled());
            assert_eq!(layer.channels()[0].data, wanted_red, "{interpolation:?} raster");
            assert_eq!(layer.mask.as_ref().unwrap().data.as_ref().unwrap(), &wanted_mask);
            assert_eq!(actual.channels[0].data, wanted_mask, "{interpolation:?} document channel");
            let Samples::U16(native_plane) = &layer.source_channels.as_ref().unwrap().planes[0].1 else {
                panic!("expected 16-bit samples");
            };
            assert_eq!(native_plane, &wanted_native);

            if interpolation == Interpolation::BicubicAutomatic {
                assert_eq!(wanted_red, prior_red[2]);
            } else {
                assert!(!prior_red.contains(&wanted_red), "{interpolation:?} must be distinct");
                prior_red.push(wanted_red);
            }
        }
    }

    #[test]
    fn rotation_sampling_has_analytic_edges_and_seams() {
        let cases = [
            (Interpolation::Nearest, 100u8, 16_000u16),
            (Interpolation::Bilinear, 50, 8_000),
            (Interpolation::Bicubic, 44, 7_000),
            (Interpolation::BicubicSmoother, 52, 8_333),
            (Interpolation::BicubicSharper, 41, 6_500),
        ];
        let seam = [0u8, 0, 100, 200];
        let native = [0u16, 0, 16_000, 32_000];
        for (method, expected, expected_native) in cases {
            // The window spans source columns 254..258; x=256 lies on
            // the destination tile seam between source pixel centres.
            assert_eq!(sample_window((520, 1), (256.0, 0.5), &seam, 4, (254, 0), method), expected);
            assert_eq!(sample_window((520, 1), (256.0, 0.5), &native, 4, (254, 0), method), expected_native);
            let edge = [100u8, 100, 100, 100];
            assert_eq!(sample_window((4, 1), (0.25, 0.5), &edge, 4, (0, 0), method), 100);
            assert_eq!(sample_window((4, 1), (-0.01, 0.5), &edge, 4, (0, 0), method), 0);
            assert_eq!(sample_window((4, 1), (4.01, 0.5), &edge, 4, (0, 0), method), 0);
        }
    }

    #[test]
    fn vector_mask_right_angle_rotation_is_exact() {
        let mut before = doc();
        before.layers[1].channels_mut()[0].data = (0..15).map(|i| (i * 13) as u8)
            .collect::<Vec<_>>().into();
        let mut exact = before.clone();
        super::super::rotate_document(&mut exact, 1).unwrap();
        before.layers[1].vector_mask = Some(pictura_core::VectorMask {
            subpaths: vec![pictura_core::VectorSubpath {
                closed: true,
                points: vec![[0, 0], [1280, 0], [1280, 768]],
                ..Default::default()
            }],
            ..Default::default()
        });
        for interpolation in INTERPOLATIONS {
            let mut actual = before.clone();
            assert!(rotate_document_in_with(&mut actual, 90.0, (2.5, 1.5), interpolation));
            assert_eq!((actual.width, actual.height), (3, 5));
            assert_eq!(actual.layers[1].rect, exact.layers[1].rect);
            assert_eq!(actual.layers[1].channels(), exact.layers[1].channels());
            let vector = actual.layers[1].vector_mask.as_ref().unwrap();
            assert_eq!(vector.subpaths[0].points, [[768, 0], [768, 1280], [0, 1280]]);
        }
    }

    #[test]
    fn vector_mask_vertices_rotate() {
        let mut d = Document::new(10, 10, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("v", rect(0, 0, 10, 10), 100);
        layer.vector_mask = Some(pictura_core::VectorMask {
            subpaths: vec![pictura_core::VectorSubpath {
                closed: true,
                points: vec![[0, 0], [2560, 0], [2560, 2560]],
                ..Default::default()
            }],
            ..Default::default()
        });
        d.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        d.composite
            .replace(crate::composite_rgba(&d))
            .expect("valid composite");

        // 180° about the centre: (0,0) -> (10,10) in 1/256 units.
        assert!(rotate_document_in(&mut d, 180.0, (5.0, 5.0)));
        let pts = &d.layers[0].vector_mask.as_ref().unwrap().subpaths[0].points;
        assert_eq!(pts[0], [2560, 2560]);

        // The authoritative `vmsk` block is re-encoded, so a reload does not
        // resurrect the pre-rotation outline.
        let block = d.layers[0].extra_block(b"vmsk").expect("vmsk synced");
        let decoded = pictura_codec::decode_vector_mask(&block.data, 10, 10).expect("decodes");
        assert_eq!(decoded.subpaths[0].points[0], [2560, 2560]);
    }
}
