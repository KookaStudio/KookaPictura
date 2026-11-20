//! Native-depth resampling for the transform family. Dispatches on [`Samples`]
//! so a scaled, rotated, or projectively mapped layer keeps its retained
//! store; the `u8` path delegates to the byte-identical 8-bit kernels.
//!
//! Split from `transform.rs` for the file-size cap.

use pictura_core::memory::MemoryError;
use pictura_core::{Document, Layer, Plane, PsdRect, Sample, Samples, SourceChannels, TileSample};

use super::kernel::Interpolation;
use super::transform::{resample_plane, PlaneMap};
use super::transform_tiled::{self, Source};

/// The `(left, top, right, bottom)` tuple of a rect.
pub(super) fn dest_tuple(rect: PsdRect) -> (i32, i32, i32, i32) {
    (rect.left, rect.top, rect.right, rect.bottom)
}

/// The rect of a destination tuple.
fn rect_of(dest: (i32, i32, i32, i32)) -> PsdRect {
    PsdRect {
        top: dest.1,
        left: dest.0,
        bottom: dest.3,
        right: dest.2,
    }
}

/// The layer's retained store, with every plane that no longer narrows to the
/// 8-bit plane it shadows replaced by that plane widened to the store's depth.
/// A brush or eraser stroke edits only the 8-bit planes, and `write_layer`
/// narrows the resampled store back over them, so a stale plane would restore
/// the pre-stroke pixels (#1446). A converted source mode's store holds
/// source-mode planes that are never narrowed back, so it is kept as is.
pub(super) fn current_store(doc: &Document, layer: &Layer) -> Option<SourceChannels> {
    let mut store = layer.source_channels.clone()?;
    if doc.source.mode().is_some() {
        return Some(store);
    }
    let size = |r: PsdRect| (r.width().max(0) as usize, r.height().max(0) as usize);
    for (id, samples) in &mut store.planes {
        let current = if *id == -2 {
            let mask = layer.mask.as_ref();
            mask.and_then(|m| m.data.as_ref().map(|d| (d, size(m.rect))))
        } else {
            let channel = layer.channels().iter().find(|c| c.id == *id);
            channel.map(|c| (&c.data, size(layer.rect)))
        };
        let Some((plane, (w, h))) = current else {
            continue;
        };
        if plane.len() != w * h || samples.len() != plane.len() {
            continue;
        }
        let mut now = vec![0u8; plane.len()];
        plane.read_rect_into(0..w, 0..h, w, &mut now);
        if samples.narrow_to_u8() != now {
            *samples = Samples::widen_from_u8(&now, samples.depth());
        }
    }
    Some(store)
}

/// Resample one native plane through `map` into `dest`; a colour plane's
/// taps are weighted by `alpha` when it has the same sample type.
#[allow(clippy::too_many_arguments)]
pub(super) fn resample_native<M: PlaneMap + Sync>(
    samples: &Samples,
    alpha: Option<&Samples>,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Samples, MemoryError> {
    let geometry = (w, h, rect, dest, interpolation);
    Ok(match samples {
        Samples::U8(v) => {
            let a = match alpha {
                Some(Samples::U8(a)) => Some(a),
                _ => None,
            };
            Samples::U8(resample_plane(v, a, w, h, map, rect, dest, interpolation)?)
        }
        Samples::U16(v) => {
            let a = match alpha {
                Some(Samples::U16(a)) => Some(a),
                _ => None,
            };
            Samples::U16(resample_typed(v, a, map, geometry)?)
        }
        Samples::F32(v) => {
            let a = match alpha {
                Some(Samples::F32(a)) => Some(a),
                _ => None,
            };
            Samples::F32(resample_typed(v, a, map, geometry)?)
        }
    })
}

/// A deep plane through the tiled resampler, clamped to its edge (no padding).
fn resample_typed<M: PlaneMap + Sync, T: Sample + TileSample>(
    src: &Plane<T>,
    alpha: Option<&Plane<T>>,
    map: &M,
    (w, h, rect, dest, interpolation): (usize, usize, PsdRect, (i32, i32, i32, i32), Interpolation),
) -> Result<Plane<T>, MemoryError> {
    let source = Source {
        plane: src,
        alpha,
        w,
        h,
        pad: 0,
        rect,
    };
    transform_tiled::resample(&source, map, dest, interpolation)
}

/// Resample a layer's retained store through `map`; the `-2` mask plane uses the
/// mask's own map and destination (`mask` is its old rect and new dest).
#[allow(clippy::too_many_arguments)]
pub(super) fn resample_store<M: PlaneMap + Sync>(
    store: &SourceChannels,
    layer_w: usize,
    layer_h: usize,
    map: &M,
    layer_rect: PsdRect,
    dest: (i32, i32, i32, i32),
    mask: Option<(PsdRect, (i32, i32, i32, i32))>,
    interpolation: Interpolation,
) -> Result<SourceChannels, MemoryError> {
    let alpha = store
        .planes
        .iter()
        .find(|(id, _)| *id == -1)
        .map(|(_, samples)| samples);
    let planes = store
        .planes
        .iter()
        .map(|(id, samples)| {
            let resampled = if *id == -2 {
                match mask {
                    Some((mrect, mdest)) => {
                        let mmap = map.for_rect(mrect);
                        let mw = mrect.width().max(0) as usize;
                        let mh = mrect.height().max(0) as usize;
                        resample_native(samples, None, mw, mh, &mmap, mrect, mdest, interpolation)
                    }
                    None => Ok(samples.clone()),
                }
            } else {
                resample_native(
                    samples,
                    alpha.filter(|_| *id >= 0),
                    layer_w,
                    layer_h,
                    map,
                    layer_rect,
                    dest,
                    interpolation,
                )
            };
            Ok((*id, resampled?))
        })
        .collect::<Result<_, MemoryError>>()?;
    Ok(SourceChannels::new(store.depth, rect_of(dest), planes))
}
