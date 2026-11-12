//! The layer transform's resampler (#1290): one strip of destination rows at a
//! time into a tiled plane, whose dense tiles the evictor can pack or spill
//! while the commit runs. Each block of a strip reads only the source window
//! its samples' kernel taps reach, so neither the source nor the destination is
//! ever held whole. Byte-identical to resampling the whole plane at once: see
//! the change's design (D2) for why the window offset is exact.

use pictura_core::memory::{self, MemoryError};
use pictura_core::{Plane, PsdRect, Sample, TileSample, TILE};
use rayon::prelude::*;

use super::kernel::{self, Interpolation};
use super::transform::PlaneMap;

/// Destination rows one parallel task resamples: its slice of the strip is
/// contiguous, so it is written in place, and its blocks' coordinates stay
/// small.
const TASK_ROWS: usize = 16;

/// The most samples one block's source window may hold before it splits.
#[cfg(not(test))]
const WINDOW_PIXELS: usize = 4 << 20;
/// Small in tests, so every test resample also exercises the block split.
#[cfg(test)]
const WINDOW_PIXELS: usize = 4096;

/// A source plane of `w × h` samples at `rect`, sampled as if surrounded by
/// `pad` samples of zero (the 8-bit path's transparent edge; 0 clamps to the
/// edge instead).
pub(super) struct Source<'a, T> {
    pub(super) plane: &'a Plane<T>,
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) pad: usize,
    pub(super) rect: PsdRect,
}

impl<T> Source<'_, T> {
    fn padded(&self) -> (usize, usize) {
        (self.w + 2 * self.pad, self.h + 2 * self.pad)
    }
}

/// Resample `src` through `map` into the destination rect `dest`; samples that
/// map outside the padded source are zero.
pub(super) fn resample<T, M>(
    src: &Source<'_, T>,
    map: &M,
    dest: (i32, i32, i32, i32),
    interpolation: Interpolation,
) -> Result<Plane<T>, MemoryError>
where
    T: Sample + TileSample,
    M: PlaneMap + Sync,
{
    let (dl, dt, dr, db) = dest;
    let dw = (dr - dl).max(0) as usize;
    let dh = (db - dt).max(0) as usize;
    let resolved = interpolation.resolve((src.w * src.h) as u64, (dw * dh) as u64);
    let mut strip = memory::try_buffer(dw * TILE.min(dh.max(1)), T::default(), "a transform")?;
    let job = pictura_core::job::current();
    Plane::build_tiled_strips(dw, dh, 1, &mut strip, |y0, out| {
        let rows = out.len() / dw.max(1);
        if job.as_ref().is_some_and(|job| job.cancelled()) {
            return Ok(());
        }
        out.par_chunks_mut(dw * TASK_ROWS)
            .enumerate()
            .for_each(|(k, task)| {
                let task_rows = task.len() / dw;
                let top = dt as f64 + (y0 + k * TASK_ROWS) as f64;
                let mut scratch = BlockScratch::default();
                for x0 in (0..dw).step_by(TILE) {
                    let bw = TILE.min(dw - x0);
                    let origin = (dl as f64 + x0 as f64, top);
                    let out = &mut task[x0..];
                    block(
                        src,
                        map,
                        origin,
                        (bw, task_rows),
                        resolved,
                        &mut scratch,
                        out,
                        dw,
                    );
                }
            });
        if let Some(job) = &job {
            job.advance(rows as u64);
        }
        Ok(())
    })
}

/// A task's reusable block buffers.
struct BlockScratch<T> {
    coords: Vec<Option<(f64, f64)>>,
    window: Vec<T>,
}

impl<T> Default for BlockScratch<T> {
    fn default() -> Self {
        Self {
            coords: Vec::new(),
            window: Vec::new(),
        }
    }
}

/// One `bw × rows` block whose top-left pixel's corner is the document point
/// `origin`, written into `out` (the block's top-left sample, rows
/// `out_stride` apart).
#[allow(clippy::too_many_arguments)]
fn block<T, M>(
    src: &Source<'_, T>,
    map: &M,
    origin: (f64, f64),
    (bw, rows): (usize, usize),
    resolved: Interpolation,
    scratch: &mut BlockScratch<T>,
    out: &mut [T],
    out_stride: usize,
) where
    T: Sample + TileSample,
    M: PlaneMap,
{
    let (pw, ph) = src.padded();
    let (pwf, phf) = (pw as f64, ph as f64);
    let coords = &mut scratch.coords;
    coords.clear();
    for oy in 0..rows {
        for ox in 0..bw {
            // The same expression order as the dense resampler, so `qx` and
            // `qy` are the same doubles.
            let qx = origin.0 + ox as f64 + 0.5;
            let qy = origin.1 + oy as f64 + 0.5;
            coords.push(map.inverse(qx, qy).and_then(|(sx, sy)| {
                let lx = sx - src.rect.left as f64 + src.pad as f64;
                let ly = sy - src.rect.top as f64 + src.pad as f64;
                (lx >= 0.0 && ly >= 0.0 && lx < pwf && ly < phf).then_some((lx, ly))
            }));
        }
    }
    let all = Area {
        x: 0,
        y: 0,
        w: bw,
        h: rows,
    };
    let out = Out {
        samples: out,
        stride: out_stride,
    };
    fill(src, coords, bw, all, out, resolved, &mut scratch.window);
}

/// Where a block's samples go: `samples[y * stride + x]`.
struct Out<'a, T> {
    samples: &'a mut [T],
    stride: usize,
}

impl<T> Out<'_, T> {
    fn reborrow(&mut self) -> Out<'_, T> {
        Out {
            samples: self.samples,
            stride: self.stride,
        }
    }
}

#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Sample `area` of a block from one source window, splitting the area when
/// its window would exceed [`WINDOW_PIXELS`].
fn fill<T: Sample + TileSample>(
    src: &Source<'_, T>,
    coords: &[Option<(f64, f64)>],
    stride: usize,
    area: Area,
    mut out: Out<'_, T>,
    resolved: Interpolation,
    window: &mut Vec<T>,
) {
    let Some((wx0, wy0, wx1, wy1)) = tap_box(src, coords, stride, area) else {
        return;
    };
    let (ww, wh) = (wx1 - wx0, wy1 - wy0);
    if ww * wh > WINDOW_PIXELS && (area.w > 1 || area.h > 1) {
        let (a, b) = if area.w >= area.h {
            let half = area.w / 2;
            (
                Area { w: half, ..area },
                Area {
                    x: area.x + half,
                    w: area.w - half,
                    ..area
                },
            )
        } else {
            let half = area.h / 2;
            (
                Area { h: half, ..area },
                Area {
                    y: area.y + half,
                    h: area.h - half,
                    ..area
                },
            )
        };
        fill(src, coords, stride, a, out.reborrow(), resolved, window);
        fill(src, coords, stride, b, out, resolved, window);
        return;
    }
    read_window(src, (wx0, wy0, ww, wh), window);
    for y in area.y..area.y + area.h {
        for x in area.x..area.x + area.w {
            if let Some((lx, ly)) = coords[y * stride + x] {
                out.samples[y * out.stride + x] =
                    kernel::sample(window, ww, wh, lx - wx0 as f64, ly - wy0 as f64, resolved);
            }
        }
    }
}

/// The padded-plane box `[x0, x1) × [y0, y1)` every kernel tap of `area`'s
/// samples reads, clipped to the padded plane; `None` when no sample maps.
fn tap_box<T>(
    src: &Source<'_, T>,
    coords: &[Option<(f64, f64)>],
    stride: usize,
    area: Area,
) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for y in area.y..area.y + area.h {
        for x in area.x..area.x + area.w {
            if let Some((lx, ly)) = coords[y * stride + x] {
                // Bicubic reads floor(u) - 1 ..= floor(u) + 2 around u = l - 0.5;
                // bilinear and nearest read a subset.
                let fx = (lx - 0.5).floor() as i64;
                let fy = (ly - 0.5).floor() as i64;
                x0 = x0.min(fx - 1);
                x1 = x1.max(fx + 2);
                y0 = y0.min(fy - 1);
                y1 = y1.max(fy + 2);
            }
        }
    }
    if x1 < x0 {
        return None;
    }
    let (pw, ph) = src.padded();
    let clip = |v: i64, n: usize| v.clamp(0, n as i64 - 1) as usize;
    Some((
        clip(x0, pw),
        clip(y0, ph),
        clip(x1, pw) + 1,
        clip(y1, ph) + 1,
    ))
}

/// Fill `window` with the padded plane's `ww × wh` box at `(x0, y0)`: source
/// samples where the box overlaps the source, zero in the padding.
fn read_window<T: TileSample>(
    src: &Source<'_, T>,
    (x0, y0, ww, wh): (usize, usize, usize, usize),
    window: &mut Vec<T>,
) {
    window.clear();
    window.resize(ww * wh, T::default());
    let pad = src.pad;
    let xs = x0.max(pad)..(x0 + ww).min(pad + src.w);
    if xs.is_empty() {
        return;
    }
    for py in y0.max(pad)..(y0 + wh).min(pad + src.h) {
        let at = (py - y0) * ww + (xs.start - x0);
        src.plane.read_rect_into(
            xs.start - pad..xs.end - pad,
            py - pad..py - pad + 1,
            src.w,
            &mut window[at..at + xs.len()],
        );
    }
}
