//! Native-depth resampling for the transform family. Dispatches on [`Samples`]
//! so a scaled, rotated, or projectively mapped layer keeps its retained
//! store; the `u8` path delegates to the byte-identical 8-bit kernels.
//!
//! Split from `transform.rs` for the file-size cap.

use pictura_core::{PsdRect, Sample, Samples, SourceChannels};

use super::kernel::{self, Interpolation};
use super::transform::{resample_plane, PlaneMap};

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
pub(super) fn resample_native<M: PlaneMap>(
    samples: &Samples,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Samples {
    match samples {
        Samples::U8(v) => Samples::U8(resample_plane(v, w, h, map, rect, dest, interpolation)),
        Samples::U16(v) => Samples::U16(resample_typed(v, w, h, map, rect, dest, interpolation)),
        Samples::F32(v) => Samples::F32(resample_typed(v, w, h, map, rect, dest, interpolation)),
    }
}

fn resample_typed<M: PlaneMap, T: Sample>(
    src: &[T],
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Vec<T> {
    let (dl, dt, dr, db) = dest;
    let dw = (dr - dl) as usize;
    let dh = (db - dt) as usize;
    let resolved = interpolation.resolve((w * h) as u64, (dw * dh) as u64);
    let mut out = vec![T::from_unit(0.0); dw * dh];
    let wf = w as f64;
    let hf = h as f64;
    for oy in 0..dh {
        for ox in 0..dw {
            let qx = dl as f64 + ox as f64 + 0.5;
            let qy = dt as f64 + oy as f64 + 0.5;
            let Some((sx, sy)) = map.inverse(qx, qy) else {
                continue;
            };
            let lx = sx - rect.left as f64;
            let ly = sy - rect.top as f64;
            if lx < 0.0 || ly < 0.0 || lx >= wf || ly >= hf {
                continue;
            }
            out[oy * dw + ox] = kernel::sample(src, w, h, lx, ly, resolved);
        }
    }
    out
}

/// Resample a layer's retained store through `map`; the `-2` mask plane uses the
/// mask's own map and destination (`mask` is its old rect and new dest).
#[allow(clippy::too_many_arguments)]
pub(super) fn resample_store<M: PlaneMap>(
    store: &SourceChannels,
    layer_w: usize,
    layer_h: usize,
    map: &M,
    layer_rect: PsdRect,
    dest: (i32, i32, i32, i32),
    mask: Option<(PsdRect, (i32, i32, i32, i32))>,
    interpolation: Interpolation,
) -> SourceChannels {
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
                    None => samples.clone(),
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
            (*id, resampled)
        })
        .collect();
    SourceChannels::new(store.depth, rect_of(dest), planes)
}
