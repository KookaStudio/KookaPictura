//! Native-depth resampling for the transform family. Dispatches on [`Samples`]
//! so a scaled, rotated, or projectively mapped layer keeps its retained
//! store; the `u8` path delegates to the byte-identical 8-bit kernels.
//!
//! Split from `transform.rs` for the file-size cap.

use pictura_core::memory::MemoryError;
use pictura_core::{Plane, PsdRect, Sample, Samples, SourceChannels, TileSample};

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

/// Resample one native plane through `map` into `dest`.
pub(super) fn resample_native<M: PlaneMap + Sync>(
    samples: &Samples,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Samples, MemoryError> {
    Ok(match samples {
        Samples::U8(v) => Samples::U8(resample_plane(v, w, h, map, rect, dest, interpolation)?),
        Samples::U16(v) => Samples::U16(resample_typed(v, w, h, map, rect, dest, interpolation)?),
        Samples::F32(v) => Samples::F32(resample_typed(v, w, h, map, rect, dest, interpolation)?),
    })
}

/// A deep plane through the tiled resampler, clamped to its edge (no padding).
fn resample_typed<M: PlaneMap + Sync, T: Sample + TileSample>(
    src: &Plane<T>,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Plane<T>, MemoryError> {
    let source = Source {
        plane: src,
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
                        resample_native(samples, mw, mh, &mmap, mrect, mdest, interpolation)
                    }
                    None => Ok(samples.clone()),
                }
            } else {
                resample_native(
                    samples,
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
