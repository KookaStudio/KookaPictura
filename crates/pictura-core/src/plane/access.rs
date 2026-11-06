//! Streaming access to a plane: rows, rectangles and bands that never build a
//! contiguous copy of a diverged banded plane.

use std::ops::Range;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{Band, Plane, BAND_ROWS, MATERIALIZATIONS};

/// Run `f` and panic if it materialized a diverged banded plane.
///
/// The counter is process-wide, so a test using this must run alone in its
/// process (nextest does that); under the threaded `cargo test` runner another
/// test can bump it.
pub fn assert_no_materialization<R>(f: impl FnOnce() -> R) -> R {
    let before = MATERIALIZATIONS.load(Ordering::Relaxed);
    let out = f();
    let after = MATERIALIZATIONS.load(Ordering::Relaxed);
    assert_eq!(
        after - before,
        0,
        "a diverged banded plane was materialized into a contiguous copy"
    );
    out
}

impl<T> Plane<T> {
    /// The row width a row access uses: the fractured width for a banded
    /// plane, `width` while the plane is flat.
    fn row_width(&self, width: usize) -> usize {
        self.bands.as_ref().map_or(width, |b| b.width)
    }

    /// Rows `ys`, each `width` samples, read in place; stops at the first row
    /// past the end.
    pub fn rows(&self, ys: Range<usize>, width: usize) -> impl Iterator<Item = &[T]> + '_ {
        // A banded `row` past the end of the last band is an empty slice.
        ys.map_while(move |y| self.row(y, width).filter(|row| !row.is_empty()))
    }

    /// Every band with the row it starts at; a flat plane is one band at row 0.
    pub fn for_each_band(&self, mut f: impl FnMut(usize, &[T])) {
        for (i, band) in self.bands().enumerate() {
            f(i * BAND_ROWS, band);
        }
    }

    /// Copy the logical samples from index `start` into `out`, band by band;
    /// the part of `out` past the plane's end is left untouched. For a buffer
    /// that stacks several channels in one plane, this reads one channel.
    pub fn read_into(&self, start: usize, out: &mut [T])
    where
        T: Clone,
    {
        let mut band_start = 0;
        let mut written = 0;
        for band in self.bands() {
            let band_end = band_start + band.len();
            let at = start + written;
            if written < out.len() && at < band_end && at >= band_start {
                let n = (band_end - at).min(out.len() - written);
                out[written..written + n]
                    .clone_from_slice(&band[at - band_start..at - band_start + n]);
                written += n;
            }
            band_start = band_end;
        }
    }

    /// Copy the samples of columns `xs` × rows `ys` into `out`, `xs.len()` per
    /// row. The part of the rectangle outside the plane is left untouched in
    /// `out`, so an empty or fully outside rectangle copies nothing.
    pub fn read_rect_into(&self, xs: Range<usize>, ys: Range<usize>, width: usize, out: &mut [T])
    where
        T: Clone,
    {
        let stride = xs.len();
        if stride == 0 {
            return;
        }
        let x1 = xs.end.min(self.row_width(width));
        for (dy, y) in ys.enumerate() {
            let Some(row) = self.row(y, width) else {
                break;
            };
            let end = x1.min(row.len());
            if xs.start >= end {
                continue;
            }
            let at = dy * stride;
            out[at..at + end - xs.start].clone_from_slice(&row[xs.start..end]);
        }
    }
}

impl<T: Clone> Plane<T> {
    /// The mutable slices of bands `range`, forking each shared one, so only
    /// those bands diverge and nothing is materialized.
    fn band_slices_mut(&mut self, range: Range<usize>) -> Vec<&mut [T]> {
        for i in range.clone() {
            let _ = self.write_band(i);
        }
        let Plane { base, bands, .. } = self;
        let Some(bands) = bands.as_mut() else {
            return Vec::new();
        };
        let end = range.end.min(bands.bands.len());
        let start = range.start.min(end);
        let selected = &mut bands.bands[start..end];
        let mut rest: &mut [T] = if selected.iter().any(|b| matches!(b, Band::Shared(_))) {
            // write_band leaves a band shared only while the base is unique.
            Arc::get_mut(base).expect("a shared band left by write_band has a unique base")
        } else {
            &mut []
        };
        let mut position = 0;
        let mut out = Vec::with_capacity(selected.len());
        for band in selected {
            match band {
                Band::Owned(data) => out.push(Arc::get_mut(data).expect("a forked band is unique")),
                Band::Shared(r) => {
                    let (_, after) = std::mem::take(&mut rest).split_at_mut(r.start - position);
                    let (slice, tail) = after.split_at_mut(r.end - r.start);
                    out.push(slice);
                    rest = tail;
                    position = r.end;
                }
            }
        }
        out
    }

    /// Rows `ys` mutably; a banded plane forks only the bands they fall in.
    pub fn rows_mut(&mut self, ys: Range<usize>, width: usize) -> impl Iterator<Item = &mut [T]> {
        let w = self.row_width(width);
        let mut rows = Vec::new();
        if w == 0 || ys.is_empty() {
            return rows.into_iter();
        }
        if self.bands.is_none() {
            let total = self.len() / w;
            let (y0, y1) = (ys.start.min(total), ys.end.min(total));
            let flat = &mut self.as_mut_slice()[y0 * w..y1 * w];
            rows.extend(flat.chunks_mut(w));
            return rows.into_iter();
        }
        let first = ys.start / BAND_ROWS;
        let last = ys.end.div_ceil(BAND_ROWS);
        for (b, band) in (first..).zip(self.band_slices_mut(first..last)) {
            for (k, row) in band.chunks_mut(w).enumerate() {
                if ys.contains(&(b * BAND_ROWS + k)) {
                    rows.push(row);
                }
            }
        }
        rows.into_iter()
    }

    /// Every band mutably with the row it starts at, forking each shared band.
    pub fn for_each_band_mut(&mut self, mut f: impl FnMut(usize, &mut [T])) {
        if self.bands.is_none() {
            f(0, self.as_mut_slice());
            return;
        }
        let count = self.band_count();
        for (i, band) in self.band_slices_mut(0..count).into_iter().enumerate() {
            f(i * BAND_ROWS, band);
        }
    }

    /// Write `src` (`xs.len()` per row) into columns `xs` × rows `ys`. The part
    /// of the rectangle outside the plane is skipped.
    pub fn write_rect_from(&mut self, xs: Range<usize>, ys: Range<usize>, width: usize, src: &[T]) {
        let stride = xs.len();
        if stride == 0 || ys.is_empty() {
            return;
        }
        let x1 = xs.end.min(self.row_width(width));
        let y0 = ys.start;
        for (y, row) in (y0..).zip(self.rows_mut(ys, width)) {
            let end = x1.min(row.len());
            if xs.start >= end {
                continue;
            }
            let at = (y - y0) * stride;
            row[xs.start..end].clone_from_slice(&src[at..at + end - xs.start]);
        }
    }
}

#[cfg(test)]
mod tests;
