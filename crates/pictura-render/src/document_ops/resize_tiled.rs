//! Image Size's resampler (#1293): one strip of destination rows at a time
//! into a tiled plane, whose dense tiles the evictor can pack or spill while
//! the resize runs. The resample is separable, so each block reads only the
//! source rows its row taps name and the column span its column taps cover.
//! Only the tap indices are moved into that window; the taps and their weights
//! are `pictura_ops::resize`'s, so the output is byte-identical to the dense
//! resamplers (`resample_plane` and `native_store::resize_samples`).

use pictura_core::memory::{self, MemoryError};
use pictura_core::{Plane, Sample, TileSample, TILE};
use pictura_ops::resize::{axis_taps, source_coord, weigh, Taps};
use pictura_ops::Resample;
use rayon::prelude::*;

/// Destination rows one parallel task resamples: its slice of the strip is
/// contiguous, so it is written in place.
const TASK_ROWS: usize = 16;

/// The most samples one block's source window may hold before it splits.
#[cfg(not(test))]
const WINDOW_SAMPLES: usize = 1 << 20;
/// Small in tests, so every test resize also exercises the block split.
#[cfg(test)]
const WINDOW_SAMPLES: usize = 4096;

/// How a plane's samples are weighed and written back.
#[derive(Clone, Copy)]
pub(super) struct Kernel<T> {
    resample: Resample,
    value: fn(T) -> f64,
    finish: fn(f64) -> T,
}

impl Kernel<u8> {
    /// `pictura_ops::resize`'s: the byte as is, rounded and clamped back.
    pub(super) fn bytes(resample: Resample) -> Self {
        Self {
            resample,
            value: |v| v as f64,
            finish: |v| v.round().clamp(0.0, 255.0) as u8,
        }
    }
}

impl<T: Sample> Kernel<T> {
    /// A retained store's: the sample's unit value, as `native_store` weighs it.
    pub(super) fn unit(resample: Resample) -> Self {
        Self {
            resample,
            value: T::to_unit,
            finish: T::from_unit,
        }
    }
}

/// Resample each of the `channels` stacked `sw × sh` planes of `src` to
/// `dw × dh`, into a tiled plane of the same stacking. Unchanged dimensions
/// return the plane as it is.
pub(super) fn resize<T: TileSample>(
    src: &Plane<T>,
    (sw, sh): (usize, usize),
    channels: usize,
    (dw, dh): (usize, usize),
    kernel: Kernel<T>,
) -> Result<Plane<T>, MemoryError> {
    if (sw, sh) == (dw, dh) || sw == 0 || sh == 0 || dw == 0 || dh == 0 || channels == 0 {
        return Ok(src.clone());
    }
    let need = sw * sh * channels;
    let padded;
    let src = if src.len() >= need {
        src
    } else {
        // A malformed short plane reads as zero past its end, as the dense
        // path's zeroed buffer does; only it is copied whole.
        let mut flat = vec![T::default(); need];
        src.read_into(0, &mut flat[..src.len()]);
        padded = Plane::from(flat);
        &padded
    };
    let (xscale, yscale) = (sw as f64 / dw as f64, sh as f64 / dh as f64);
    let cols: Vec<Taps> = (0..dw)
        .map(|x| axis_taps(source_coord(x, xscale), sw, kernel.resample))
        .collect();
    let rows: Vec<Taps> = (0..dh)
        .map(|y| axis_taps(source_coord(y, yscale), sh, kernel.resample))
        .collect();
    let mut strip = memory::try_buffer(dw * TILE.min(dh), T::default(), "an image size")?;
    let job = pictura_core::job::current();
    Plane::build_tiled_strips(dw, dh, channels, &mut strip, |y0, out| {
        if job.as_ref().is_some_and(|job| job.cancelled()) {
            return Ok(());
        }
        // A strip never crosses a channel: each starts at a multiple of 256.
        let (base, top) = ((y0 / dh) * sh, y0 % dh);
        let strip_rows = out.len() / dw;
        out.par_chunks_mut(dw * TASK_ROWS)
            .enumerate()
            .for_each(|(k, task)| {
                let first = top + k * TASK_ROWS;
                let task_rows = &rows[first..first + task.len() / dw];
                let mut scratch = Scratch::default();
                for x0 in (0..dw).step_by(TILE) {
                    let bw = TILE.min(dw - x0);
                    let block = Block {
                        src,
                        base,
                        sw,
                        cols: &cols[x0..x0 + bw],
                        rows: task_rows,
                        kernel,
                    };
                    let all = Area {
                        x: 0,
                        y: 0,
                        w: bw,
                        h: task_rows.len(),
                    };
                    block.fill(all, &mut task[x0..], dw, &mut scratch);
                }
            });
        if let Some(job) = &job {
            job.advance(strip_rows as u64);
        }
        Ok::<(), MemoryError>(())
    })
}

/// A task's reusable buffers.
#[derive(Default)]
struct Scratch<T> {
    window: Vec<T>,
    lines: Vec<usize>,
    xs: Vec<Taps>,
    ys: Vec<Taps>,
}

/// One block's columns and rows of one channel, whose first source row is
/// `base` in the stacked plane.
struct Block<'a, T> {
    src: &'a Plane<T>,
    base: usize,
    sw: usize,
    cols: &'a [Taps],
    rows: &'a [Taps],
    kernel: Kernel<T>,
}

#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl<T: TileSample> Block<'_, T> {
    /// Resample `area` of the block into `out` (`out[y * stride + x]`) from one
    /// source window, splitting the area when its window would exceed
    /// [`WINDOW_SAMPLES`].
    fn fill(&self, area: Area, out: &mut [T], stride: usize, scratch: &mut Scratch<T>) {
        let cols = &self.cols[area.x..area.x + area.w];
        let rows = &self.rows[area.y..area.y + area.h];
        let (lo, hi) = cols
            .iter()
            .flat_map(|t| &t.idx[..t.len])
            .fold((usize::MAX, 0), |(lo, hi), &i| (lo.min(i), hi.max(i)));
        let Scratch {
            window,
            lines,
            xs,
            ys,
        } = scratch;
        lines.clear();
        lines.extend(rows.iter().flat_map(|t| t.idx[..t.len].iter().copied()));
        lines.sort_unstable();
        lines.dedup();
        let ww = hi - lo + 1;
        if ww * lines.len() > WINDOW_SAMPLES && (area.w > 1 || area.h > 1) {
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
            self.fill(a, out, stride, scratch);
            self.fill(b, out, stride, scratch);
            return;
        }
        window.clear();
        window.resize(ww * lines.len(), T::default());
        for (r, &line) in lines.iter().enumerate() {
            let y = self.base + line;
            self.src
                .read_rect_into(lo..hi + 1, y..y + 1, self.sw, &mut window[r * ww..][..ww]);
        }
        xs.clear();
        xs.extend(cols.iter().map(|t| {
            let mut local = *t;
            local.idx[..t.len].iter_mut().for_each(|i| *i -= lo);
            local
        }));
        ys.clear();
        ys.extend(rows.iter().map(|t| {
            let mut local = *t;
            for i in &mut local.idx[..t.len] {
                *i = lines
                    .binary_search(i)
                    .expect("every row tap is a window line");
            }
            local
        }));
        let Kernel {
            resample,
            value,
            finish,
        } = self.kernel;
        for (dy, yt) in ys.iter().enumerate() {
            let line = &mut out[(area.y + dy) * stride + area.x..][..area.w];
            for (o, xt) in line.iter_mut().zip(xs.iter()) {
                *o = if resample == Resample::Nearest {
                    window[yt.idx[0] * ww + xt.idx[0]]
                } else {
                    finish(weigh(xt, yt, |i, j| value(window[j * ww + i])))
                };
            }
        }
    }
}
