//! The whole-plane resamplers the tiled one replaced (#1290), kept as the
//! oracle its tests compare against byte for byte.

use pictura_core::memory::{self, MemoryError};
use pictura_core::{Plane, PsdRect, Sample};
use rayon::prelude::*;

use super::kernel::{self, Interpolation};
use super::transform::PlaneMap;

const PAD: usize = 2;

/// The whole-plane 8-bit resampler the tiled one replaced (#1290).
pub(super) fn dense_resample_plane<M: PlaneMap + Sync>(
    src: &Plane<u8>,
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Vec<u8>, MemoryError> {
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

/// The whole-plane deep resampler the tiled one replaced (#1290).
pub(super) fn dense_resample_typed<M: PlaneMap, T: Sample>(
    src: &[T],
    w: usize,
    h: usize,
    map: &M,
    rect: PsdRect,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Vec<T>, MemoryError> {
    let (dl, dt, dr, db) = dest;
    let dw = (dr - dl) as usize;
    let dh = (db - dt) as usize;
    let resolved = interpolation.resolve((w * h) as u64, (dw * dh) as u64);
    let mut out = Vec::new();
    out.try_reserve_exact(dw * dh)
        .map_err(|_| MemoryError::Insufficient {
            purpose: "a transform",
            bytes: (dw * dh * std::mem::size_of::<T>()) as u64,
        })?;
    out.resize(dw * dh, T::from_unit(0.0));
    let wf = w as f64;
    let hf = h as f64;
    let job = pictura_core::job::current();
    for oy in 0..dh {
        if let Some(job) = &job {
            if job.cancelled() {
                break;
            }
            job.advance(1);
        }
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
    Ok(out)
}
