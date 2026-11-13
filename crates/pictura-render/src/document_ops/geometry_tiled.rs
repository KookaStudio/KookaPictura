//! The strip driver of Canvas Size, the exact turns and flips, and arbitrary
//! Rotate (#1294): each output plane is built one strip of 256 rows at a time
//! into a tiled plane, whose dense tiles the evictor can pack or spill while
//! the command runs. Blocks of a strip run in parallel and read only the source
//! window their kernel names, so no plane is held whole.

use std::borrow::Cow;

use pictura_core::memory::{self, MemoryError};
use pictura_core::{Plane, TileSample, TILE};
use rayon::prelude::*;

use super::native_store::Remap;

/// Destination rows one parallel task builds: its slice of the strip is
/// contiguous, so it is written in place.
const TASK_ROWS: usize = 64;

/// A block of one channel's destination plane: `y` is local to the channel.
#[derive(Clone, Copy, Debug)]
pub(super) struct Block {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
}

/// Build `channels` stacked `dw × dh` planes into a tiled plane.
/// `fill(channel, block, out, stride, scratch)` writes the block's samples to
/// `out[row * stride + col]`; `scratch` is the task's reusable window buffer.
/// Progress advances by each strip's rows; a cancelled job skips the rest.
pub(super) fn build<T: TileSample>(
    (dw, dh): (usize, usize),
    channels: usize,
    fill: impl Fn(usize, Block, &mut [T], usize, &mut Vec<T>) + Sync,
) -> Result<Plane<T>, MemoryError> {
    if dw == 0 || dh == 0 || channels == 0 {
        return Ok(Plane::default());
    }
    let mut strip = memory::try_buffer(dw * TILE.min(dh), T::default(), "a canvas edit")?;
    let job = pictura_core::job::current();
    Plane::build_tiled_strips(dw, dh, channels, &mut strip, |y0, out| {
        if job.as_ref().is_some_and(|job| job.cancelled()) {
            return Ok(());
        }
        // A strip never crosses a channel: each starts at a multiple of 256.
        let (channel, top) = (y0 / dh, y0 % dh);
        let strip_rows = out.len() / dw;
        out.par_chunks_mut(dw * TASK_ROWS)
            .enumerate()
            .for_each(|(k, task)| {
                let (y, h) = (top + k * TASK_ROWS, task.len() / dw);
                let mut scratch = Vec::new();
                for x in (0..dw).step_by(TILE) {
                    let block = Block {
                        x,
                        y,
                        w: TILE.min(dw - x),
                        h,
                    };
                    fill(channel, block, &mut task[x..], dw, &mut scratch);
                }
            });
        if let Some(job) = &job {
            job.advance(strip_rows as u64);
        }
        Ok::<(), MemoryError>(())
    })
}

/// `src` when it holds `need` samples; a malformed short plane is copied once
/// and padded with `pad`, which is what the dense paths read past its end.
pub(super) fn whole<T: TileSample>(src: &Plane<T>, need: usize, pad: T) -> Cow<'_, Plane<T>> {
    if src.len() >= need {
        return Cow::Borrowed(src);
    }
    let mut flat = vec![pad; need];
    src.read_into(0, &mut flat[..src.len()]);
    Cow::Owned(Plane::from(flat))
}

/// Read the `xs × ys` window of channel `channel` of stacked `width × height`
/// planes into `window` (resized to fit), returning its width.
pub(super) fn read_window<T: TileSample>(
    src: &Plane<T>,
    (width, height): (usize, usize),
    channel: usize,
    xs: std::ops::Range<usize>,
    ys: std::ops::Range<usize>,
    window: &mut Vec<T>,
) -> usize {
    let ww = xs.len();
    window.clear();
    window.resize(ww * ys.len(), T::default());
    let base = channel * height;
    src.read_rect_into(xs, base + ys.start..base + ys.end, width, window);
    ww
}

/// Canvas Size's kernel: channel planes of `sw × sh` placed at `(dx, dy)` in
/// `dw × dh`, with `fill` where no source sample lands. The dense
/// `extend_channel` and `extend_samples` read the same way.
pub(super) fn shift<T: TileSample>(
    src: &Plane<T>,
    (sw, sh): (usize, usize),
    channels: usize,
    (dw, dh): (usize, usize),
    (dx, dy): (i64, i64),
    fill: T,
) -> Result<Plane<T>, MemoryError> {
    let src = whole(src, sw * sh * channels, fill);
    let src = src.as_ref();
    // The destination columns a source column lands on.
    let x_lo = dx.clamp(0, dw as i64) as usize;
    let x_hi = (sw as i64 + dx).clamp(0, dw as i64) as usize;
    build((dw, dh), channels, |channel, b, out, stride, _| {
        let lo = x_lo.clamp(b.x, b.x + b.w);
        let hi = x_hi.clamp(lo, b.x + b.w);
        for r in 0..b.h {
            let row = &mut out[r * stride..][..b.w];
            let sy = (b.y + r) as i64 - dy;
            if !(0..sh as i64).contains(&sy) || lo == hi {
                row.fill(fill);
                continue;
            }
            row[..lo - b.x].fill(fill);
            row[hi - b.x..].fill(fill);
            let sx = (lo as i64 - dx) as usize;
            let y = channel * sh + sy as usize;
            src.read_rect_into(
                sx..sx + (hi - lo),
                y..y + 1,
                sw,
                &mut row[lo - b.x..hi - b.x],
            );
        }
    })
}

/// The exact turns' and flips' kernel: every `w × h` channel plane remapped by
/// `kind`. A block's source window is the box of its two corners' sources,
/// since every remap maps a box onto a box.
pub(super) fn orient<T: TileSample>(
    src: &Plane<T>,
    (w, h): (usize, usize),
    channels: usize,
    kind: Remap,
) -> Result<Plane<T>, MemoryError> {
    let src = whole(src, w * h * channels, T::default());
    let src = src.as_ref();
    let to = if kind.swaps_axes() { (h, w) } else { (w, h) };
    build(to, channels, |channel, b, out, stride, window| {
        let (ax, ay) = kind.source(b.x, b.y, w, h);
        let (bx, by) = kind.source(b.x + b.w - 1, b.y + b.h - 1, w, h);
        let (x0, y0) = (ax.min(bx), ay.min(by));
        let xs = x0..ax.max(bx) + 1;
        let ys = y0..ay.max(by) + 1;
        let ww = read_window(src, (w, h), channel, xs, ys, window);
        for r in 0..b.h {
            for (c, o) in out[r * stride..][..b.w].iter_mut().enumerate() {
                let (sx, sy) = kind.source(b.x + c, b.y + r, w, h);
                *o = window[(sy - y0) * ww + sx - x0];
            }
        }
    })
}

#[cfg(test)]
#[path = "geometry_tiled_tests.rs"]
mod tests;
