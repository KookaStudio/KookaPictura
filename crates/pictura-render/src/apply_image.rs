//! Image > Apply Image (#609): blend a document-sized source plane set into a
//! layer or a layer mask. Unlike an adjustment the kernel needs each pixel's
//! document position and may change alpha, so it has its own write path; the
//! shared preview session and selection handling stay in the app.
//!
//! Calculation runs in the unit `f32` domain at every depth. A 16- or 32-bit
//! layer is blended in its retained store and the 8-bit channels re-narrowed,
//! the same two-copy protocol as the native fill.

use std::sync::Arc;

use pictura_core::nonseparable::lum;
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, BlendMode, Document, Layer, LayerMask,
    Plane, PsdRect, Sample, Samples,
};
use pictura_filters::FilterError;
use rayon::prelude::*;

use crate::blend::blend;
use crate::filter::coverage;
use crate::{channel, composite_native, composite_rgba, resolve_path};

/// The blend slot. Add and Subtract exist only here, never as layer modes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApplyImageMode {
    Layer(BlendMode),
    /// `(A + B) / scale + offset / 255`.
    Add {
        scale: f32,
        offset: f32,
    },
    /// `(A - B) / scale + offset / 255`.
    Subtract {
        scale: f32,
        offset: f32,
    },
}

/// Which plane of a document or layer to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyImageChannel {
    Composite,
    /// A working colour plane: 0 red (or gray), 1 green, 2 blue.
    Color(u8),
    Transparency,
    LayerMask,
    /// An index into `Document::channels`.
    Alpha(usize),
}

/// A document-sized source: planar samples, and which planes feed colour and
/// alpha. A single channel is the same plane on all three colour slots.
#[derive(Clone, Debug)]
pub struct ApplyImagePlanes {
    pub width: usize,
    pub height: usize,
    samples: Samples,
    color: [usize; 3],
    alpha: Option<usize>,
}

impl ApplyImagePlanes {
    /// A one-plane 8-bit source: a selection, a mask, or an alpha channel.
    pub fn from_coverage(width: usize, height: usize, data: Plane<u8>) -> Option<Self> {
        (data.len() == width * height).then_some(Self {
            width,
            height,
            samples: Samples::U8(data),
            color: [0; 3],
            alpha: None,
        })
    }

    #[inline]
    fn value(&self, plane: usize, i: usize) -> f32 {
        let at = plane * self.width * self.height + i;
        match &self.samples {
            Samples::U8(v) => v[at] as f32 / 255.0,
            Samples::U16(v) => v[at] as f32 / 65535.0,
            Samples::F32(v) => v[at],
        }
    }

    #[inline]
    fn rgb(&self, i: usize) -> [f32; 3] {
        self.color.map(|plane| self.value(plane, i))
    }
}

/// Read `channel` of the merged image (`layer` `None`) or one layer, at the
/// document's depth, placed at document size. `None` when the layer, its mask,
/// or the alpha channel does not exist.
pub fn apply_image_planes(
    doc: &Document,
    layer: Option<&str>,
    channel: ApplyImageChannel,
) -> Option<ApplyImagePlanes> {
    let (width, height) = (doc.width as usize, doc.height as usize);
    let source = match layer {
        Some(path) => Some(resolve_path(doc, path)?),
        None => None,
    };
    match channel {
        ApplyImageChannel::LayerMask => {
            let data = mask_plane(source?.mask.as_ref()?, width, height);
            return ApplyImagePlanes::from_coverage(width, height, data.into());
        }
        ApplyImageChannel::Alpha(index) => {
            let plane = doc.channels.get(index)?.data.clone();
            return ApplyImagePlanes::from_coverage(width, height, plane);
        }
        _ => {}
    }
    let samples = match source {
        Some(layer) => layer_samples(doc, layer),
        None => match composite_native(doc) {
            Some(samples) => samples,
            None => {
                let buffer = composite_rgba(doc);
                if buffer.channels != 4 || buffer.width != doc.width || buffer.height != doc.height
                {
                    return None;
                }
                Samples::U8(buffer.data)
            }
        },
    };
    let (color, alpha) = match channel {
        ApplyImageChannel::Color(index) if index < 3 => ([index as usize; 3], Some(3)),
        ApplyImageChannel::Transparency => ([3; 3], None),
        _ => ([0, 1, 2], Some(3)),
    };
    Some(ApplyImagePlanes {
        width,
        height,
        samples,
        color,
        alpha,
    })
}

fn mask_plane(mask: &LayerMask, width: usize, height: usize) -> Vec<u8> {
    let (mw, mh) = (
        mask.rect.width().max(0) as usize,
        mask.rect.height().max(0) as usize,
    );
    let data = mask.data.as_ref().filter(|data| data.len() == mw * mh);
    let mut out = vec![mask.default_color; width * height];
    let Some(data) = data else {
        return out;
    };
    for (y, row) in out.chunks_mut(width.max(1)).enumerate() {
        let my = y as i32 - mask.rect.top;
        if my < 0 || my as usize >= mh {
            continue;
        }
        for (x, value) in row.iter_mut().enumerate() {
            let mx = x as i32 - mask.rect.left;
            if mx >= 0 && (mx as usize) < mw {
                *value = data[my as usize * mw + mx as usize];
            }
        }
    }
    out
}

/// One layer plane as the compositor would read it.
enum Read<'a> {
    Native(&'a Samples),
    Bytes(&'a [u8]),
    Opaque,
}

impl Read<'_> {
    #[inline]
    fn raw(&self, i: usize) -> f64 {
        match self {
            Read::Native(Samples::U8(v)) => v[i] as f64 / 255.0,
            Read::Native(Samples::U16(v)) => v[i] as f64 / 65535.0,
            Read::Native(Samples::F32(v)) => v[i] as f64,
            Read::Bytes(v) => v[i] as f64 / 255.0,
            Read::Opaque => 1.0,
        }
    }
}

fn layer_samples(doc: &Document, layer: &Layer) -> Samples {
    match doc
        .authoritative_plane(layer, 0)
        .and(doc.source_depth)
        .unwrap_or(BitDepth::Eight)
    {
        BitDepth::Sixteen => Samples::U16(place::<u16>(doc, layer).into()),
        BitDepth::ThirtyTwo => Samples::F32(place::<f32>(doc, layer).into()),
        _ => Samples::U8(place::<u8>(doc, layer).into()),
    }
}

/// The layer's RGBA planes at document size, transparent outside its rect.
fn place<T: Sample + Default + Send + Sync>(doc: &Document, layer: &Layer) -> Vec<T> {
    let (width, height) = (doc.width as usize, doc.height as usize);
    let rect = layer.rect;
    let lw = rect.width().max(0) as usize;
    let lh = rect.height().max(0) as usize;
    let read = |id: i16| -> Read {
        if let Some(samples) = doc.authoritative_plane(layer, id) {
            return Read::Native(samples);
        }
        match channel(layer, id).filter(|data| data.len() == lw * lh) {
            Some(data) => Read::Bytes(data),
            None if id == -1 => Read::Opaque,
            None => doc
                .authoritative_plane(layer, 0)
                .map(Read::Native)
                .or_else(|| channel(layer, 0).map(Read::Bytes))
                .unwrap_or(Read::Opaque),
        }
    };
    let planes = [read(0), read(1), read(2), read(-1)];
    let mut out = vec![T::default(); width * height * 4];
    if width == 0 || height == 0 {
        return out;
    }
    out.par_chunks_mut(width)
        .enumerate()
        .for_each(|(row, line)| {
            let plane = &planes[row / height];
            let ly = (row % height) as i32 - rect.top;
            if ly < 0 || ly as usize >= lh {
                return;
            }
            for (x, value) in line.iter_mut().enumerate() {
                let lx = x as i32 - rect.left;
                if lx >= 0 && (lx as usize) < lw {
                    *value = T::from_unit(plane.raw(ly as usize * lw + lx as usize));
                }
            }
        });
    out
}

/// Everything the dialog chose. Planes are shared, so a preview copies nothing.
#[derive(Clone, Debug)]
pub struct ApplyImageParams {
    pub source: Arc<ApplyImagePlanes>,
    pub invert: bool,
    pub mode: ApplyImageMode,
    /// `0..=1`.
    pub opacity: f32,
    pub preserve_transparency: bool,
    pub mask: Option<Arc<ApplyImagePlanes>>,
    pub mask_invert: bool,
}

/// How the edited layer maps onto the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyImageTarget {
    /// Added to a layer coordinate to get a document coordinate: zero for a
    /// layer, the mask's origin for a mask-sized document.
    pub origin: (i32, i32),
    /// Blend one luminosity value, for a layer mask.
    pub gray: bool,
    /// Write the retained store; see [`apply_image_native`].
    pub native: bool,
}

/// Whether the layer at `path` must be blended in its retained store. A store
/// owning only some colour planes is refused: the 8-bit channels would then
/// reach only part of what the compositor reads.
pub fn apply_image_native(doc: &Document, path: &str) -> Result<bool, FilterError> {
    let layer = resolve_path(doc, path)
        .ok_or_else(|| FilterError::InvalidParams("the active layer no longer exists".into()))?;
    let ids: &[i16] = if channel(layer, 1).is_some() {
        &[0, 1, 2]
    } else {
        &[0]
    };
    let owned = ids
        .iter()
        .filter(|&&id| doc.authoritative_plane(layer, id).is_some())
        .count();
    if owned != 0 && owned != ids.len() {
        return Err(FilterError::InvalidParams(
            "the layer's native store is incomplete".into(),
        ));
    }
    Ok(owned != 0)
}

/// A store sample in the unit domain, not clamped, because a 32-bit target
/// keeps values above 1.
trait Store: Sample + Send + Sync {
    const CLAMPED: bool;
    fn raw(self) -> f32;
    fn narrow(self) -> u8;
}

impl Store for u8 {
    const CLAMPED: bool = true;
    fn raw(self) -> f32 {
        self as f32 / 255.0
    }
    fn narrow(self) -> u8 {
        self
    }
}

impl Store for u16 {
    const CLAMPED: bool = true;
    fn raw(self) -> f32 {
        self as f32 / 65535.0
    }
    fn narrow(self) -> u8 {
        (self >> 8) as u8
    }
}

impl Store for f32 {
    const CLAMPED: bool = false;
    fn raw(self) -> f32 {
        self
    }
    fn narrow(self) -> u8 {
        (self * 256.0).trunc().clamp(0.0, 255.0) as u8
    }
}

#[derive(Clone, Copy)]
struct Pass<'a> {
    params: &'a ApplyImageParams,
    selection: Option<&'a LayerMask>,
    target: ApplyImageTarget,
    rect: PsdRect,
    clip: PsdRect,
    keep_alpha: bool,
}

#[inline]
fn blended<const CLAMPED: bool>(mode: ApplyImageMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    let arithmetic = |sign: f32, scale: f32, offset: f32| {
        let mut out = [0.0; 3];
        for c in 0..3 {
            let v = (cb[c] + sign * cs[c]) / scale + offset / 255.0;
            out[c] = if CLAMPED { v.clamp(0.0, 1.0) } else { v };
        }
        out
    };
    match mode {
        ApplyImageMode::Layer(mode) => blend(mode, cb, cs),
        ApplyImageMode::Add { scale, offset } => arithmetic(1.0, scale, offset),
        ApplyImageMode::Subtract { scale, offset } => arithmetic(-1.0, scale, offset),
    }
}

/// The new colour and alpha of one pixel, or `None` to leave it alone.
#[inline]
fn pixel<const CLAMPED: bool>(
    pass: &Pass,
    cb: [f32; 3],
    ab: f32,
    i: usize,
    selected: f32,
) -> Option<([f32; 3], f32)> {
    let p = pass.params;
    let mut cs = p.source.rgb(i);
    if p.invert {
        cs = cs.map(|v| 1.0 - v);
    }
    if pass.target.gray {
        cs = [lum(cs); 3];
    }
    let source_alpha = p.source.alpha.map_or(1.0, |plane| p.source.value(plane, i));
    let mask = p.mask.as_ref().map_or(1.0, |mask| {
        let value = lum(mask.rgb(i));
        if p.mask_invert {
            1.0 - value
        } else {
            value
        }
    });
    let cov = source_alpha * p.opacity * mask * selected;
    if cov <= 0.0 || (pass.keep_alpha && ab <= 0.0) {
        return None;
    }
    let b = blended::<CLAMPED>(p.mode, cb, cs);
    let mut out = [0.0; 3];
    if pass.keep_alpha {
        for c in 0..3 {
            out[c] = cb[c] + (b[c] - cb[c]) * cov;
        }
        return Some((out, ab));
    }
    let ar = ab + cov - ab * cov;
    for c in 0..3 {
        let over = (1.0 - ab) * cs[c] + ab * b[c];
        out[c] = ((1.0 - cov) * ab * cb[c] + cov * over) / ar;
    }
    Some((out, ar))
}

/// The clip's rows of one layer-wide plane, or `None` for each row without it.
fn plane_rows<T>(
    plane: Option<&mut [T]>,
    lw: usize,
    oy: usize,
    ch: usize,
) -> Vec<Option<&mut [T]>> {
    match plane {
        Some(plane) => plane.chunks_mut(lw).skip(oy).take(ch).map(Some).collect(),
        None => (0..ch).map(|_| None).collect(),
    }
}

/// Blend the clip of up to four layer-wide planes (colour, then alpha).
fn blend_planes<T: Store>(planes: [Option<&mut [T]>; 4], pass: Pass) {
    let lw = pass.rect.width() as usize;
    let cw = pass.clip.width() as usize;
    let ch = pass.clip.height() as usize;
    let ox = (pass.clip.left - pass.rect.left) as usize;
    let oy = (pass.clip.top - pass.rect.top) as usize;
    let [r, g, b, a] = planes;
    let rows = |plane| plane_rows(plane, lw, oy, ch);
    let work: Vec<_> = rows(r)
        .into_iter()
        .zip(rows(g))
        .zip(rows(b))
        .zip(rows(a))
        .enumerate()
        .map(|(y, (((r, g), b), a))| (y, [r, g, b, a]))
        .collect();
    let source = &pass.params.source;
    let single = pass.target.gray
        || work
            .iter()
            .any(|(_, row)| row[1].is_none() || row[2].is_none());
    work.into_par_iter().for_each(|(y, mut row)| {
        let ly = pass.clip.top + y as i32;
        let dy = ly + pass.target.origin.1;
        if dy < 0 || dy as usize >= source.height {
            return;
        }
        for x in 0..cw {
            let at = ox + x;
            let lx = pass.clip.left + x as i32;
            let dx = lx + pass.target.origin.0;
            if dx < 0 || dx as usize >= source.width {
                continue;
            }
            let selected = match pass.selection {
                Some(_) => coverage(pass.selection, lx, ly) as f32 / 255.0,
                None => 1.0,
            };
            if selected <= 0.0 {
                continue;
            }
            let base = |plane: &Option<&mut [T]>| plane.as_ref().map_or(0.0, |p| p[at].raw());
            let cb = if single {
                [base(&row[0]); 3]
            } else {
                [base(&row[0]), base(&row[1]), base(&row[2])]
            };
            let ab = row[3].as_ref().map_or(1.0, |p| p[at].raw());
            let i = dy as usize * source.width + dx as usize;
            let result = if T::CLAMPED {
                pixel::<true>(&pass, cb, ab, i, selected)
            } else {
                pixel::<false>(&pass, cb, ab, i, selected)
            };
            let Some((color, alpha)) = result else {
                continue;
            };
            for c in 0..3 {
                if let Some(plane) = row[c].as_mut() {
                    plane[at] = T::from_unit(color[if single { 0 } else { c }] as f64);
                }
            }
            if !pass.keep_alpha {
                if let Some(plane) = row[3].as_mut() {
                    plane[at] = T::from_unit(alpha as f64);
                }
            }
        }
    });
}

fn narrow_clip<T: Store>(
    from: &[T],
    to: &mut [u8],
    lw: usize,
    ox: usize,
    oy: usize,
    c: (usize, usize),
) {
    let (cw, ch) = c;
    to.par_chunks_mut(lw)
        .zip(from.par_chunks(lw))
        .skip(oy)
        .take(ch)
        .for_each(|(out, row)| {
            for x in ox..ox + cw {
                out[x] = row[x].narrow();
            }
        });
}

fn apply_store<T: Store>(layer: &mut Layer, pass: Pass) -> Result<(), FilterError> {
    let lw = pass.rect.width() as usize;
    let n = lw * pass.rect.height() as usize;
    let Layer {
        source_channels,
        channels,
        ..
    } = layer;
    let missing = || FilterError::InvalidParams("the layer has no native store".into());
    let store = source_channels.as_mut().ok_or_else(missing)?;
    let depth = store.depth;
    // A store without transparency reads the 8-bit plane, at the store's width.
    let mut widened = match store.planes.iter().any(|(id, _)| *id == -1) {
        true => None,
        false => channels
            .iter()
            .find(|c| c.id == -1)
            .map(|c| Samples::widen_from_u8(c.data.as_slice(), depth)),
    };
    let mut slots: [Option<&mut [T]>; 4] = [None, None, None, None];
    for (id, samples) in store.planes.iter_mut() {
        let slot = match *id {
            0..=2 => *id as usize,
            -1 => 3,
            _ => continue,
        };
        slots[slot] = T::as_slice_mut(samples).filter(|plane| plane.len() == n);
    }
    if let Some(samples) = widened.as_mut() {
        slots[3] = T::as_slice_mut(samples).filter(|plane| plane.len() == n);
    }
    if slots[0].is_none() {
        return Err(missing());
    }
    blend_planes(slots, pass);

    let ox = (pass.clip.left - pass.rect.left) as usize;
    let oy = (pass.clip.top - pass.rect.top) as usize;
    let size = (pass.clip.width() as usize, pass.clip.height() as usize);
    let planes = store.planes.iter_mut().map(|(id, s)| (*id, s));
    for (id, samples) in planes.chain(widened.iter_mut().map(|s| (-1, s))) {
        if !(0..3).contains(&id) && (id != -1 || pass.keep_alpha) {
            continue;
        }
        let out = channels.iter_mut().find(|c| c.id == id);
        if let (Some(out), Some(from)) = (out, T::as_slice_mut(samples)) {
            if out.data.len() == n && from.len() == n {
                narrow_clip(from, out.data.as_mut_slice(), lw, ox, oy, size);
            }
        }
    }
    Ok(())
}

fn apply_channels(layer: &mut Layer, pass: Pass) -> Result<(), FilterError> {
    let n = pass.rect.width() as usize * pass.rect.height() as usize;
    let mut slots: [Option<&mut [u8]>; 4] = [None, None, None, None];
    for channel in layer.channels.iter_mut() {
        let slot = match channel.id {
            0..=2 => channel.id as usize,
            -1 => 3,
            _ => continue,
        };
        if channel.data.len() != n {
            return Err(FilterError::InvalidParams(format!(
                "channel {} has {} samples, expected {n}",
                channel.id,
                channel.data.len()
            )));
        }
        slots[slot] = Some(channel.data.as_mut_slice());
    }
    if slots[0].is_none() {
        return Err(FilterError::InvalidParams(
            "layer is missing color channel 0".into(),
        ));
    }
    blend_planes(slots, pass);
    Ok(())
}

/// Blend `params` into `layer` over `region` (clamped to the layer), confined
/// by `selection`.
///
/// ponytail: the write stops at the layer rect, where Photoshop can grow a
/// smaller layer to the document when Preserve Transparency is off.
pub fn apply_image_region(
    layer: &mut Layer,
    params: &ApplyImageParams,
    selection: Option<&LayerMask>,
    region: PsdRect,
    target: ApplyImageTarget,
) -> Result<(), FilterError> {
    let rect = layer.rect;
    if layer_pixel_locked(layer) {
        return Err(FilterError::Locked);
    }
    let clip = PsdRect {
        top: region.top.max(rect.top),
        left: region.left.max(rect.left),
        bottom: region.bottom.min(rect.bottom),
        right: region.right.min(rect.right),
    };
    if clip.width() <= 0 || clip.height() <= 0 {
        return Ok(());
    }
    let pass = Pass {
        params,
        selection,
        target,
        rect,
        clip,
        keep_alpha: params.preserve_transparency || layer_transparency_locked(layer),
    };
    if !target.native {
        return apply_channels(layer, pass);
    }
    match layer.source_channels.as_ref().map(|store| store.depth) {
        Some(BitDepth::Sixteen) => apply_store::<u16>(layer, pass),
        Some(BitDepth::ThirtyTwo) => apply_store::<f32>(layer, pass),
        _ => Err(FilterError::InvalidParams(
            "the layer has no native store".into(),
        )),
    }
}
