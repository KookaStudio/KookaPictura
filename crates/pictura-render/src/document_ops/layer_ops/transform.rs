//! Layer transforms: a similarity map (scale, rotation about the layer centre,
//! and translation) and a projective map (a homography sending the source rect's
//! four corners to a target quad), both resampled with bilinear interpolation.
//!
//! The ops are pure `&mut Document` functions alongside the other
//! `resolve_path`-based layer ops. They mutate the document only on success and
//! do **not** recomposite: like `translate_layer_rect`, the caller owns the
//! composite refresh.

use pictura_core::homography::HOMOGRAPHY_EPSILON;
use pictura_core::memory::{self, MemoryError};
use pictura_core::{
    Channel, Document, Homography, LayerMask, LockFlags, Plane, PsdRect, SourceChannels,
};
use rayon::prelude::*;

use super::crossed_quad;
use super::kernel::{self, Interpolation};
use super::paths::{resolve_path, resolve_path_mut};
use super::shape_layer::{is_shape_layer, map_shape_layer, shape_layer_rect};
use super::transform_native;

pub(super) use super::kernel::bilinear;

use pictura_core::Layer;

/// A similarity transform about the layer's (or mask's) rect centre.
#[derive(Debug, Clone, Copy)]
pub struct LayerTransform {
    pub scale_x: f64,
    pub scale_y: f64,
    pub angle_radians: f64,
    pub dx: f64,
    pub dy: f64,
}

/// Below this absolute scale factor a transform is refused as degenerate.
const SCALE_EPSILON: f64 = 1e-6;
/// Quarter-turn slack: a 90° `cos` is ~6e-17, not exactly 0, so an exact integer
/// corner must not round out to the next pixel. Same intent as the slack in
/// `pictura_ops::rotate_arbitrary`.
const BBOX_SLACK: f64 = 1e-9;

/// Any document-space plane map the shared resample skeleton can drive: a
/// `forward` map used for the destination bounding box and an `inverse` map used
/// to sample the source. Returning `None` marks a point at (or beyond) infinity.
pub(super) trait PlaneMap: Sized {
    fn forward(&self, x: f64, y: f64) -> Option<(f64, f64)>;
    fn inverse(&self, qx: f64, qy: f64) -> Option<(f64, f64)>;
    /// The equivalent map for a plane occupying `rect` (a layer mask): the
    /// similarity map re-centres on the new rect; the projective map is global.
    fn for_rect(&self, rect: PsdRect) -> Self;
}

/// The document-space similarity map of one source rect.
struct Map {
    cx: f64,
    cy: f64,
    sx: f64,
    sy: f64,
    cos: f64,
    sin: f64,
    dx: f64,
    dy: f64,
}

impl Map {
    fn new(rect: PsdRect, t: LayerTransform, cos: f64, sin: f64) -> Self {
        Map {
            cx: (rect.left as f64 + rect.right as f64) / 2.0,
            cy: (rect.top as f64 + rect.bottom as f64) / 2.0,
            sx: t.scale_x,
            sy: t.scale_y,
            cos,
            sin,
            dx: t.dx,
            dy: t.dy,
        }
    }
}

impl PlaneMap for Map {
    /// `p' = c + R(θ)·(S·(p − c)) + (dx, dy)`; positive θ is clockwise in the
    /// y-down screen convention.
    fn forward(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let ux = (x - self.cx) * self.sx;
        let uy = (y - self.cy) * self.sy;
        Some((
            self.cx + self.cos * ux - self.sin * uy + self.dx,
            self.cy + self.sin * ux + self.cos * uy + self.dy,
        ))
    }

    /// `src = c + S⁻¹·R(−θ)·(q − c) − (dx, dy)`.
    fn inverse(&self, qx: f64, qy: f64) -> Option<(f64, f64)> {
        let vx = qx - self.cx - self.dx;
        let vy = qy - self.cy - self.dy;
        let rx = self.cos * vx + self.sin * vy;
        let ry = -self.sin * vx + self.cos * vy;
        Some((self.cx + rx / self.sx, self.cy + ry / self.sy))
    }

    fn for_rect(&self, rect: PsdRect) -> Self {
        Map {
            cx: (rect.left as f64 + rect.right as f64) / 2.0,
            cy: (rect.top as f64 + rect.bottom as f64) / 2.0,
            sx: self.sx,
            sy: self.sy,
            cos: self.cos,
            sin: self.sin,
            dx: self.dx,
            dy: self.dy,
        }
    }
}

/// A document-space projective map. Stores the forward homography (source →
/// destination, used for the bounding box) and its inverse (destination →
/// source, used to resample), both row-major.
#[derive(Clone, Copy)]
pub(super) struct QuadMap {
    h: [f64; 9],
    inv: [f64; 9],
    /// The side of the horizon the source rect's majority lies on (+1 or -1).
    front: f64,
}

impl QuadMap {
    /// The inverse map's homogeneous divisor at `(x, y)`: its sign is the side
    /// of the horizon the destination point samples.
    #[cfg(test)]
    pub(super) fn inverse_w(&self, x: f64, y: f64) -> f64 {
        self.inv[6] * x + self.inv[7] * y + self.inv[8]
    }

    #[cfg(test)]
    pub(super) fn front(&self) -> f64 {
        self.front
    }
}

impl PlaneMap for QuadMap {
    fn forward(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        apply_homography(&self.h, x, y)
    }

    fn inverse(&self, qx: f64, qy: f64) -> Option<(f64, f64)> {
        let w = self.inv[6] * qx + self.inv[7] * qy + self.inv[8];
        if !w.is_finite() || w.abs() < HOMOGRAPHY_EPSILON || w * self.front <= 0.0 {
            return None;
        }
        let px = (self.inv[0] * qx + self.inv[1] * qy + self.inv[2]) / w;
        let py = (self.inv[3] * qx + self.inv[4] * qy + self.inv[5]) / w;
        (px.is_finite() && py.is_finite()).then_some((px, py))
    }

    fn for_rect(&self, _rect: PsdRect) -> Self {
        *self
    }
}

/// The side of the horizon holding the majority of `src`'s four forward
/// homogeneous divisors; the centre's side on a tie.
fn front_side(h: &[f64; 9], src: &[(f64, f64); 4]) -> f64 {
    let w = |(x, y): (f64, f64)| h[6] * x + h[7] * y + h[8];
    let ahead = src.iter().filter(|p| w(**p) > 0.0).count();
    match ahead {
        3 | 4 => 1.0,
        0 | 1 => -1.0,
        _ => {
            let cx = src.iter().map(|p| p.0).sum::<f64>() / 4.0;
            let cy = src.iter().map(|p| p.1).sum::<f64>() / 4.0;
            if w((cx, cy)) >= 0.0 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

/// Apply a row-major homography in homogeneous coordinates, dividing by the
/// third component; `None` when that divisor is non-finite or ~0.
fn apply_homography(m: &[f64; 9], x: f64, y: f64) -> Option<(f64, f64)> {
    Homography(*m).apply(x, y)
}

/// The four source-rect corners in TL, TR, BR, BL order.
fn source_corners(rect: PsdRect) -> [(f64, f64); 4] {
    [
        (rect.left as f64, rect.top as f64),
        (rect.right as f64, rect.top as f64),
        (rect.right as f64, rect.bottom as f64),
        (rect.left as f64, rect.bottom as f64),
    ]
}

/// The homography sending `src[0..4]` to `dst[0..4]`, or `None` for a
/// non-finite corner or a singular/near-singular map.
pub(super) fn solve_homography(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<QuadMap> {
    let h = Homography::from_quads(src, dst)?;
    let inv = h.inverse()?;
    Some(QuadMap {
        h: h.0,
        inv: inv.0,
        front: front_side(&h.0, &src),
    })
}

/// Largest result area the op will allocate for one plane, in pixels. A
/// similarity transform of a real layer is orders of magnitude smaller; this
/// only stops a pathological scale from attempting a multi-gigabyte allocation.
// ponytail: fixed 1 Gpx ceiling; thread the codec's allocation budget through
// here if transforms of near-PSB-limit layers ever matter.
const MAX_RESULT_PIXELS: u64 = 1 << 30;

/// Integer bounding box of four or more document-space points, or `None` when
/// the result is empty, out of `i32` range, or implausibly large.
pub(super) fn integer_bbox(points: &[(f64, f64)]) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &(px, py) in points {
        if !px.is_finite() || !py.is_finite() {
            return None;
        }
        min_x = min_x.min(px);
        min_y = min_y.min(py);
        max_x = max_x.max(px);
        max_y = max_y.max(py);
    }
    let left = (min_x + BBOX_SLACK).floor();
    let top = (min_y + BBOX_SLACK).floor();
    let right = (max_x - BBOX_SLACK).ceil();
    let bottom = (max_y - BBOX_SLACK).ceil();
    if !left.is_finite() || !top.is_finite() || !right.is_finite() || !bottom.is_finite() {
        return None;
    }
    if left < i32::MIN as f64
        || right > i32::MAX as f64
        || top < i32::MIN as f64
        || bottom > i32::MAX as f64
    {
        return None;
    }
    let (left, top, right, bottom) = (left as i32, top as i32, right as i32, bottom as i32);
    let width = (right as i64) - (left as i64);
    let height = (bottom as i64) - (top as i64);
    if width <= 0 || height <= 0 || (width as u64) * (height as u64) > MAX_RESULT_PIXELS {
        return None;
    }
    Some((left, top, right, bottom))
}

/// Integer bounding box of the four transformed corners, or `None` when the
/// result is empty, out of `i32` range, or implausibly large.
fn bounding_box<M: PlaneMap>(map: &M, rect: PsdRect) -> Option<(i32, i32, i32, i32)> {
    let mut points = [(0.0, 0.0); 4];
    for (out, (x, y)) in points.iter_mut().zip(source_corners(rect)) {
        *out = map.forward(x, y)?;
    }
    integer_bbox(&points)
}

/// Resample one `w×h` plane into the destination rect; out-of-source → 0. Its
/// buffers are allocated fallibly, so a refusal is an error, never an abort.
///
/// ponytail: the refusal is recorded on the calling thread, so a deferred
/// commit's worker reports a generic failure, not the memory message, until
/// the out-of-core commit replaces this path (#1290).
pub(super) fn resample_plane<M: PlaneMap + Sync>(
    src: &Plane<u8>,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Vec<u8>, MemoryError> {
    // A layer's opaque pixels can run right up to its rect edge. Clamping the
    // kernel there keeps the alpha at 255, so a rotated filled layer gets a
    // hard, stair-stepped edge. Pad with transparent pixels instead: the taps
    // that fall outside the source read 0 and the edge anti-aliases.
    const PAD: usize = 2;
    let (pw, ph) = (w + PAD * 2, h + PAD * 2);
    let mut padded = memory::try_plane(pw * ph, 0, "a transform")?;
    for y in 0..h {
        let dst = (y + PAD) * pw + PAD;
        src.read_rect_into(0..w, y..y + 1, w, &mut padded[dst..dst + w]);
    }
    let (dl, dt, dr, db) = dest;
    let dw = (dr - dl) as usize;
    let dh = (db - dt) as usize;
    let resolved = interpolation.resolve((w * h) as u64, (dw * dh) as u64);
    let mut out = memory::try_plane(dw * dh, 0, "a transform")?;
    let (pwf, phf) = (pw as f64, ph as f64);
    let job = pictura_core::job::current();
    out.par_chunks_mut(dw).enumerate().for_each(|(oy, row)| {
        if job.as_ref().is_some_and(|job| job.cancelled()) {
            return;
        }
        if let Some(job) = &job {
            job.advance(1);
        }
        for (ox, pixel) in row.iter_mut().enumerate() {
            let qx = dl as f64 + ox as f64 + 0.5;
            let qy = dt as f64 + oy as f64 + 0.5;
            let Some((sx, sy)) = map.inverse(qx, qy) else {
                continue;
            };
            let lx = sx - rect.left as f64 + PAD as f64;
            let ly = sy - rect.top as f64 + PAD as f64;
            if lx < 0.0 || ly < 0.0 || lx >= pwf || ly >= phf {
                continue;
            }
            *pixel = kernel::sample(&padded, pw, ph, lx, ly, resolved);
        }
    });
    Ok(out)
}

/// Whether `t`'s parameters are finite and non-degenerate.
fn params_ok(t: &LayerTransform) -> bool {
    t.scale_x.is_finite()
        && t.scale_y.is_finite()
        && t.angle_radians.is_finite()
        && t.dx.is_finite()
        && t.dy.is_finite()
        && t.scale_x.abs() >= SCALE_EPSILON
        && t.scale_y.abs() >= SCALE_EPSILON
}

/// Whether `layer` carries a materializable channel-less embedded smart object.
fn materialized_channels(doc: &Document, layer: &Layer) -> Option<Vec<Channel>> {
    let so = layer.smart_object.as_ref()?;
    let buf = crate::render_smart_source(so, layer.rect, layer.rect)?;
    if buf.width != layer.rect.width() as u32 || buf.height != layer.rect.height() as u32 {
        return None;
    }
    let plane = buf.width as usize * buf.height as usize;
    if buf.channels < 4 || buf.data.len() != plane * 4 {
        return None;
    }
    let mut channels: Vec<Channel> = (0..doc.mode.color_channels() as usize)
        .map(|c| Channel {
            id: c as i16,
            data: buf.data[c * plane..(c + 1) * plane].to_vec().into(),
        })
        .collect();
    channels.push(Channel {
        id: -1,
        data: buf.data[3 * plane..4 * plane].to_vec().into(),
    });
    Some(channels)
}

/// Resample `mask` by the same warp as the layer. A mask without plane data still
/// has its `rect` transformed (and keeps `data` `None`) so the rect follows the
/// layer; a similarity map re-centres on the mask's own rect, a projective map
/// is global. `Ok(None)` keeps the old mask; `Err` refuses the transform.
fn transform_mask<M: PlaneMap + Sync>(
    mask: &LayerMask,
    layer_map: &M,
    interpolation: Interpolation,
) -> Result<Option<LayerMask>, MemoryError> {
    let rect = mask.rect;
    let map = layer_map.for_rect(rect);
    let Some(dest) = bounding_box(&map, rect) else {
        return Ok(None);
    };
    let data = match mask.data.as_ref() {
        Some(data) => {
            let w = rect.width();
            let h = rect.height();
            if w <= 0 || h <= 0 || data.len() != (w as usize) * (h as usize) {
                return Ok(None);
            }
            Some(resample_plane(
                data,
                w as usize,
                h as usize,
                &map,
                rect,
                dest,
                interpolation,
            )?)
        }
        None => None,
    };
    Ok(Some(LayerMask {
        rect: PsdRect {
            top: dest.1,
            left: dest.0,
            bottom: dest.3,
            right: dest.2,
        },
        data: data.map(Into::into),
        ..mask.clone()
    }))
}

/// `mask` moved by whole pixels: what [`transform_mask`] returns for a pure
/// integer translation, without resampling.
fn shift_mask(mask: &LayerMask, dx: i32, dy: i32) -> Option<LayerMask> {
    let rect = mask.rect;
    let (w, h) = (rect.width(), rect.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    if mask
        .data
        .as_ref()
        .is_some_and(|data| data.len() != (w as usize) * (h as usize))
    {
        return None;
    }
    let dest = integer_bbox(&[
        (
            f64::from(rect.left) + f64::from(dx),
            f64::from(rect.top) + f64::from(dy),
        ),
        (
            f64::from(rect.right) + f64::from(dx),
            f64::from(rect.bottom) + f64::from(dy),
        ),
    ])?;
    Some(LayerMask {
        rect: PsdRect {
            top: dest.1,
            left: dest.0,
            bottom: dest.3,
            right: dest.2,
        },
        ..mask.clone()
    })
}

/// A validated transform target with its materialized source planes.
pub(super) struct Prepared {
    pub(super) rect: PsdRect,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) source_channels: Vec<Channel>,
    pub(super) native: Option<SourceChannels>,
    pub(super) mask: Option<LayerMask>,
    pub(super) materialize: bool,
    pub(super) old_uuid: String,
}

/// Resolve `path` and apply the shared refusal rules, materializing a
/// channel-less embedded smart object. `None` refuses without mutation.
pub(super) fn prepare_layer(doc: &Document, path: &str) -> Option<Prepared> {
    let layer = resolve_path(doc, path)?;
    if layer.is_group
        || layer.adjustment.is_some()
        || layer.background
        || layer.lock.contains(LockFlags::POSITION)
    {
        return None;
    }
    let rect = layer.rect;
    let (w, h) = (rect.width(), rect.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let plane = (w as usize) * (h as usize);
    // A channel-bearing layer (e.g. a placed image) keeps its smart_object
    // payload and preserved blocks: the original source survives as the proxy's
    // cache and Replace re-renders from a new source. Only a channel-less target
    // is consumed below.
    let materialize = !layer.channels.iter().any(|c| c.id == 0);
    let source_channels = if materialize {
        materialized_channels(doc, layer)?
    } else {
        if layer.channels.iter().any(|c| c.data.len() != plane) {
            return None;
        }
        layer.channels.clone()
    };
    let old_uuid = layer
        .smart_object
        .as_ref()
        .map(|so| so.uuid.clone())
        .unwrap_or_default();
    Some(Prepared {
        rect,
        w,
        h,
        source_channels,
        native: layer.source_channels.clone(),
        mask: layer.mask.clone(),
        materialize,
        old_uuid,
    })
}

/// The resampled result of one layer transform, before it is written back.
pub(super) struct LayerOutput {
    pub(super) channels: Vec<Channel>,
    pub(super) mask: Option<LayerMask>,
    pub(super) native: Option<SourceChannels>,
    pub(super) dest: (i32, i32, i32, i32),
}

/// Write a resampled channel set and rect back to the layer at `path`, applying
/// the raw-channel rule and consuming a materialized smart object.
pub(super) fn write_layer(
    doc: &mut Document,
    path: &str,
    prepared: &Prepared,
    mut out: LayerOutput,
    pure_translate: bool,
) -> bool {
    // Keep the 8-bit channel equal to the resampled native plane's narrowing so
    // the writer's `narrow(retained) == current` gate re-emits native samples.
    // A converted source mode stores source-mode planes, not working RGB, so is
    // excluded here.
    if doc.source_mode.is_none() && !pure_translate {
        if let Some(store) = &out.native {
            for (id, samples) in &store.planes {
                let narrowed = samples.narrow_to_u8();
                if *id == -2 {
                    if let Some(mask) = out.mask.as_mut() {
                        if mask.data.is_some() {
                            mask.data = Some(narrowed.into());
                        }
                    }
                } else if let Some(channel) = out.channels.iter_mut().find(|c| c.id == *id) {
                    channel.data = narrowed.into();
                }
            }
        }
    }
    let layer = resolve_path_mut(doc, path).expect("resolved by prepare_layer");
    layer.channels = out.channels;
    layer.rect = PsdRect {
        top: out.dest.1,
        left: out.dest.0,
        bottom: out.dest.3,
        right: out.dest.2,
    };
    // A raw channel stream carries no position, so a pure integer translation
    // (Free Transform's move handle) keeps it and only re-anchors the store rect.
    if pure_translate {
        if let Some(store) = layer.source_channels.as_mut() {
            store.rect = layer.rect;
        }
    } else {
        // ponytail: an unmodeled raw on-disk stream cannot be resampled, so a
        // scale/rotate/fractional move, projective map, or mesh warp drops it.
        layer.raw_channels.clear();
        layer.source_channels = out.native;
    }
    if let Some(mask) = out.mask {
        layer.mask = Some(mask);
    }
    if prepared.materialize {
        layer
            .extra_blocks
            .retain(|block| !matches!(&block.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd"));
        layer.smart_object = None;
        if !prepared.old_uuid.is_empty() {
            if let Some(cleaned) = pictura_codec::remove_linked_source(
                &doc.layer_section_extra,
                &prepared.old_uuid,
                doc.is_psb,
            ) {
                doc.layer_section_extra = cleaned.into();
            }
        }
    }
    true
}

/// Shared refusal / materialization / resample / write path for both ops.
///
/// `build_map` validates the warp for the layer rect and returns its plane map
/// (`None` refuses). `pure_translate` selects whether an unmodeled raw channel
/// stream survives (a similarity integer move) or is dropped. Mutates `doc` only
/// after every refusal has passed, so a refused call is bit-identical.
fn apply_layer_map<M, F>(
    doc: &mut Document,
    path: &str,
    build_map: F,
    pure_translate: bool,
    interpolation: Interpolation,
) -> bool
where
    M: PlaneMap + Sync,
    F: Fn(PsdRect) -> Option<M>,
{
    transform_layer_with_map(doc, path, build_map, pure_translate, interpolation)
}

/// Apply any [`PlaneMap`] to the layer at `path`. Sibling layer ops that need a
/// map the public transform constructors cannot express (Auto-Align's
/// cylindrical and spherical projections) reuse this path so masks, native
/// planes, and raw-channel rules stay identical.
pub(super) fn transform_layer_with_map<M, F>(
    doc: &mut Document,
    path: &str,
    build_map: F,
    pure_translate: bool,
    interpolation: Interpolation,
) -> bool
where
    M: PlaneMap + Sync,
    F: Fn(PsdRect) -> Option<M>,
{
    let Some(mut prepared) = prepare_layer(doc, path) else {
        return false;
    };
    let Some(map) = build_map(prepared.rect) else {
        return false;
    };
    let Some(dest) = bounding_box(&map, prepared.rect) else {
        return false;
    };
    // At whole-pixel offsets every kernel but the B-spline returns its source
    // sample, so the planes move instead of being resampled (#1290).
    let exact = interpolation.resolve(1, 1) != Interpolation::BicubicSmoother;
    if pure_translate && exact {
        return move_layer(doc, path, &mut prepared, dest);
    }
    if let Some(job) = pictura_core::job::current() {
        job.begin_stage(resample_rows(&prepared, dest));
    }
    let resampled: Result<Vec<Channel>, MemoryError> = prepared
        .source_channels
        .iter()
        .map(|c| {
            let data = resample_plane(
                &c.data,
                prepared.w as usize,
                prepared.h as usize,
                &map,
                prepared.rect,
                dest,
                interpolation,
            )?;
            Ok(Channel {
                id: c.id,
                data: data.into(),
            })
        })
        .collect();
    let Ok(new_channels) = resampled else {
        return false;
    };
    let new_mask = match prepared.mask.as_ref() {
        Some(m) => match transform_mask(m, &map, interpolation) {
            Ok(mask) => mask,
            Err(_) => return false,
        },
        None => None,
    };
    let native = if pure_translate {
        None
    } else {
        let resampled = prepared.native.as_ref().map(|store| {
            let mask_info = prepared
                .mask
                .as_ref()
                .zip(new_mask.as_ref())
                .map(|(m, nm)| (m.rect, transform_native::dest_tuple(nm.rect)));
            transform_native::resample_store(
                store,
                prepared.w as usize,
                prepared.h as usize,
                &map,
                prepared.rect,
                dest,
                mask_info,
                interpolation,
            )
        });
        match resampled.transpose() {
            Ok(native) => native,
            Err(_) => return false,
        }
    };
    if job_cancelled() {
        return false;
    }
    let out = LayerOutput {
        channels: new_channels,
        mask: new_mask,
        native,
        dest,
    };
    write_layer(doc, path, &prepared, out, pure_translate)
}

/// A pure integer move: the planes keep their tiles under the rect shifted to
/// `dest`, which is what an exact resample would produce. The job stage is one
/// unit, so a multi-layer job's stages still add up.
fn move_layer(
    doc: &mut Document,
    path: &str,
    prepared: &mut Prepared,
    dest: (i32, i32, i32, i32),
) -> bool {
    if let Some(job) = pictura_core::job::current() {
        job.begin_stage(1);
        job.advance(1);
    }
    if job_cancelled() {
        return false;
    }
    let (dx, dy) = (dest.0 - prepared.rect.left, dest.1 - prepared.rect.top);
    let mask = prepared.mask.as_ref().and_then(|m| shift_mask(m, dx, dy));
    let out = LayerOutput {
        channels: std::mem::take(&mut prepared.source_channels),
        mask,
        native: None,
        dest,
    };
    write_layer(doc, path, prepared, out, true)
}

/// Whether the installed job was cancelled: a cancelled resample is partial
/// and must refuse before it writes.
pub(super) fn job_cancelled() -> bool {
    pictura_core::job::current().is_some_and(|job| job.cancelled())
}

/// The output rows a layer resample visits, for its job stage: every channel
/// and retained plane at the destination height, the mask counted the same.
pub(super) fn resample_rows(prepared: &Prepared, dest: (i32, i32, i32, i32)) -> u64 {
    let planes = prepared.source_channels.len()
        + usize::from(prepared.mask.as_ref().is_some_and(|m| m.data.is_some()))
        + prepared.native.as_ref().map_or(0, |n| n.planes.len());
    (dest.3 - dest.1).max(0) as u64 * planes as u64
}

/// A shape layer's outline box: its transforms map the path about this box,
/// not about the canvas-sized rect of its fill. `None` for another layer.
pub(super) fn shape_rect(doc: &Document, path: &str) -> Option<PsdRect> {
    let layer = resolve_path(doc, path).filter(|l| is_shape_layer(l))?;
    shape_layer_rect(layer, doc.width, doc.height)
}

/// Apply a similarity transform to the layer at `path`.
///
/// On success every channel plane (and the mask, when present) is resampled into
/// the transformed bounding rect and `layer.rect` is updated. The op refuses —
/// leaving `doc` bit-identical — for a missing path, a group, an adjustment
/// layer, a Background layer, a position-locked layer, a zero-area source, any
/// non-finite parameter, a scale below [`SCALE_EPSILON`], a zero-area result, or
/// a channel-less target that does not materialize from its embedded source.
///
/// A channel-less embedded smart object is materialized from its payload and, on
/// success, consumed: the smart object, its preserved `SoLd`/`plLd` blocks, and
/// its linked record are dropped, so no stale untransformed source survives.
///
/// # Ceiling
///
/// Bilinear resampling (use [`transform_layer_with`] to choose the method);
/// skew and perspective go through [`transform_layer_quad`]. A type layer is
/// re-set as type instead (`type_layer::transform_type_layer`), staying sharp
/// and editable; a shape layer maps its outline about the outline's box
/// (`map_shape_layer`), staying vector.
pub fn transform_layer(doc: &mut Document, path: &str, transform: LayerTransform) -> bool {
    transform_layer_with(doc, path, transform, Interpolation::Bilinear)
}

/// Apply a similarity transform to the layer at `path` with an explicit
/// resample `interpolation`. Same contract as [`transform_layer`].
pub fn transform_layer_with(
    doc: &mut Document,
    path: &str,
    transform: LayerTransform,
    interpolation: Interpolation,
) -> bool {
    if !params_ok(&transform) {
        return false;
    }
    let (sin, cos) = transform.angle_radians.sin_cos();
    if let Some(rect) = shape_rect(doc, path) {
        let map = Map::new(rect, transform, cos, sin);
        let pure = transform.scale_x == 1.0 && transform.scale_y == 1.0 && sin == 0.0 && cos == 1.0;
        let translate = pure.then_some((transform.dx, transform.dy));
        return map_shape_layer(doc, path, translate, |x, y| map.forward(x, y));
    }
    if let Some(rect) = resolve_path(doc, path)
        .filter(|l| l.is_type())
        .map(|l| l.rect)
    {
        let (cx, cy) = (
            f64::from(rect.left + rect.right) / 2.0,
            f64::from(rect.top + rect.bottom) / 2.0,
        );
        let (xx, xy) = (cos * transform.scale_x, sin * transform.scale_x);
        let (yx, yy) = (-sin * transform.scale_y, cos * transform.scale_y);
        let affine = [
            xx,
            xy,
            yx,
            yy,
            cx + transform.dx - (xx * cx + yx * cy),
            cy + transform.dy - (xy * cx + yy * cy),
        ];
        if let Some(done) = crate::type_layer::transform_type_layer(doc, path, affine) {
            return done;
        }
    }
    let pure_translate = transform.scale_x == 1.0
        && transform.scale_y == 1.0
        && transform.angle_radians == 0.0
        && transform.dx.fract() == 0.0
        && transform.dy.fract() == 0.0;
    apply_layer_map(
        doc,
        path,
        |rect| Some(Map::new(rect, transform, cos, sin)),
        pure_translate,
        interpolation,
    )
}

/// Apply a projective transform to the layer at `path`.
///
/// `quad[i]` is the document-space target of source corner `i` in TL, TR, BR, BL
/// order. The homography sending the source rect's corners to `quad` is solved by
/// an 8×8 linear system and inverted to sample each channel plane and the mask.
/// A self-intersecting (bowtie) `quad` instead fills its own outline, the two
/// straight-edged triangles between its crossing edges, as Photoshop's
/// Perspective does ([`crossed_quad`]).
/// Refusals (missing path, group, adjustment, Background, position-locked,
/// zero-area source, no materializable channel-less payload, empty/oversized
/// result) match [`transform_layer`], plus a non-finite corner, a singular or
/// near-singular map, or a zero-area destination. A projective map is never a
/// pure integer translation, so an unmodeled raw channel stream is dropped.
pub fn transform_layer_quad(doc: &mut Document, path: &str, quad: [(f64, f64); 4]) -> bool {
    transform_layer_quad_with(doc, path, quad, Interpolation::Bilinear)
}

/// Apply a projective transform with an explicit resample `interpolation`.
/// Same contract as [`transform_layer_quad`].
pub fn transform_layer_quad_with(
    doc: &mut Document,
    path: &str,
    quad: [(f64, f64); 4],
    interpolation: Interpolation,
) -> bool {
    if let Some(rect) = shape_rect(doc, path) {
        let Some(map) = solve_homography(source_corners(rect), quad) else {
            return false;
        };
        return map_shape_layer(doc, path, None, |x, y| map.forward(x, y));
    }
    if let Some(rect) = resolve_path(doc, path)
        .filter(|l| l.is_type())
        .map(|l| l.rect)
    {
        if let Some(affine) = affine_quad(rect, quad) {
            if let Some(done) = crate::type_layer::transform_type_layer(doc, path, affine) {
                return done;
            }
        }
    }
    if crossed_quad::quad_is_crossed(&quad) {
        return apply_layer_map(
            doc,
            path,
            |rect| crossed_quad::CrossedMap::new(rect, quad),
            false,
            interpolation,
        );
    }
    apply_layer_map(
        doc,
        path,
        |rect| solve_homography(source_corners(rect), quad),
        false,
        interpolation,
    )
}

/// The affine map (`xx, xy, yx, yy, tx, ty`) sending `rect`'s TL, TR, BL
/// corners to `quad`'s, when BR lands where that map puts it (Skew); `None` for
/// a projective quad (Distort, Perspective).
fn affine_quad(rect: PsdRect, quad: [(f64, f64); 4]) -> Option<[f64; 6]> {
    let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let [tl, tr, br, bl] = quad;
    let (xx, xy) = ((tr.0 - tl.0) / w, (tr.1 - tl.1) / w);
    let (yx, yy) = ((bl.0 - tl.0) / h, (bl.1 - tl.1) / h);
    let expected = (tr.0 + bl.0 - tl.0, tr.1 + bl.1 - tl.1);
    if (expected.0 - br.0).abs() > 1e-3 || (expected.1 - br.1).abs() > 1e-3 {
        return None;
    }
    let (l, t) = (f64::from(rect.left), f64::from(rect.top));
    Some([
        xx,
        xy,
        yx,
        yy,
        tl.0 - xx * l - yx * t,
        tl.1 - xy * l - yy * t,
    ])
}

/// Test hook: the forward-mapped source-rect corners of `quad`.
#[cfg(test)]
pub(crate) fn quad_corner_targets(rect: PsdRect, quad: [(f64, f64); 4]) -> Option<[(f64, f64); 4]> {
    let map = solve_homography(source_corners(rect), quad)?;
    let mut out = [(0.0, 0.0); 4];
    for (i, (x, y)) in source_corners(rect).into_iter().enumerate() {
        out[i] = map.forward(x, y)?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::Document;

    fn rotate(doc: &mut Document) -> bool {
        transform_layer_with(
            doc,
            "0",
            LayerTransform {
                scale_x: 1.25,
                scale_y: 0.8,
                angle_radians: 17f64.to_radians(),
                dx: 3.0,
                dy: -2.0,
            },
            Interpolation::Bicubic,
        )
    }

    fn seeded(w: u32, h: u32) -> Document {
        let rgba: Vec<u8> = (0..w * h * 4).map(|i| (i * 37 % 251) as u8).collect();
        Document::from_rgba("seeded", w, h, &rgba)
    }

    #[test]
    fn transform_progress_rises_to_completion() {
        use pictura_core::job::{self, JobProgress};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let mut doc = seeded(257, 131);
        let job = JobProgress::new();
        let stop = Arc::new(AtomicBool::new(false));
        let sampler = {
            let (job, stop) = (job.clone(), stop.clone());
            std::thread::spawn(move || {
                let mut seen = vec![job.permille()];
                while !stop.load(Ordering::Relaxed) {
                    seen.push(job.permille());
                }
                seen
            })
        };
        assert!(job::scope(&job, || rotate(&mut doc)));
        // Every declared row was visited: the stage is full but not finished.
        assert_eq!(job.permille(), 999);
        assert_eq!(job::run(&job, || ()), Ok(()));
        stop.store(true, Ordering::Relaxed);
        let mut seen = sampler.join().expect("sampler");
        seen.push(job.permille());
        assert!(seen.windows(2).all(|w| w[0] <= w[1]), "progress fell");
        assert_eq!(*seen.last().unwrap(), 1000);
    }

    #[test]
    fn a_cancelled_transform_leaves_the_document() {
        use pictura_core::job::{self, Cancelled, JobProgress};
        let original = seeded(257, 131);
        let mut doc = original.clone();
        let job = JobProgress::new();
        job.cancel();
        assert_eq!(job::run(&job, || rotate(&mut doc)), Err(Cancelled));
        assert_eq!(job.permille(), 0, "no row was resampled");
        let (before, after) = (
            resolve_path(&original, "0").expect("layer"),
            resolve_path(&doc, "0").expect("layer"),
        );
        assert_eq!(before.rect, after.rect);
        for (a, b) in before.channels.iter().zip(&after.channels) {
            assert!(a.data == b.data, "channel {}", a.id);
        }
    }

    #[test]
    fn a_rotated_edge_is_antialiased() {
        let side = 64u32;
        let mut rgba = vec![0u8; (side * side * 4) as usize];
        for y in 16..48u32 {
            for x in 16..48u32 {
                let i = ((y * side + x) * 4) as usize;
                rgba[i + 3] = 255;
            }
        }
        let mut doc = Document::from_rgba("square", side, side, &rgba);
        assert!(transform_layer_with(
            &mut doc,
            "0",
            LayerTransform {
                scale_x: 1.0,
                scale_y: 1.0,
                angle_radians: 30f64.to_radians(),
                dx: 0.0,
                dy: 0.0,
            },
            Interpolation::Bicubic,
        ));
        let layer = resolve_path(&doc, "0").expect("layer");
        let alpha = layer.channels.iter().find(|c| c.id == -1).expect("alpha");
        let fractional = (0..alpha.data.len())
            .filter_map(|i| alpha.data.sample(i))
            .filter(|a| *a != 0 && *a != 255)
            .count();
        assert!(fractional > 0, "the rotated edge is not anti-aliased");
    }

    #[test]
    fn a_quad_rotated_edge_is_antialiased() {
        let side = 64u32;
        let mut rgba = vec![0u8; (side * side * 4) as usize];
        for y in 16..48u32 {
            for x in 16..48u32 {
                let i = ((y * side + x) * 4) as usize;
                rgba[i + 3] = 255;
            }
        }
        let mut doc = Document::from_rgba("square", side, side, &rgba);
        let rect = resolve_path(&doc, "0").expect("layer").rect;
        let (cx, cy) = (
            f64::from(rect.left + rect.right) / 2.0,
            f64::from(rect.top + rect.bottom) / 2.0,
        );
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let rotate = |x: f64, y: f64| {
            let (dx, dy) = (x - cx, y - cy);
            (cx + cos * dx - sin * dy, cy + sin * dx + cos * dy)
        };
        let quad = source_corners(rect).map(|(x, y)| rotate(x, y));
        assert!(transform_layer_quad_with(
            &mut doc,
            "0",
            quad,
            Interpolation::Bicubic,
        ));
        let layer = resolve_path(&doc, "0").expect("layer");
        let alpha = layer.channels.iter().find(|c| c.id == -1).expect("alpha");
        let fractional = (0..alpha.data.len())
            .filter_map(|i| alpha.data.sample(i))
            .filter(|a| *a != 0 && *a != 255)
            .count();
        assert!(fractional > 0, "the quad-rotated edge is not anti-aliased");
    }
}
