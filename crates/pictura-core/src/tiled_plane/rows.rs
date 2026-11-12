//! Row, rectangle and range copies across a row of tiles.

use std::ops::Range;

use super::{TileSample, TiledPlane, TILE};

impl<T: TileSample> TiledPlane<T> {
    /// Copy global row `y` into `out`, at most `out.len()` samples; `false`
    /// when the row is past the end.
    pub(crate) fn read_row_into(&self, y: usize, out: &mut [T]) -> bool {
        if y >= self.height * self.channels {
            return false;
        }
        let n = out.len().min(self.width);
        self.read_rect_into(0..n, y..y + 1, out);
        true
    }

    /// Write `src` into global row `y` from column 0; samples past the row end
    /// are ignored. Tiles the row would not change are not forked.
    pub(crate) fn write_row_from(&mut self, y: usize, src: &[T]) -> bool {
        if y >= self.height * self.channels {
            return false;
        }
        let n = src.len().min(self.width);
        self.write_rect_from(0..n, y..y + 1, src);
        true
    }

    /// Copy columns `xs` × global rows `ys` into `out` (`xs.len()` per row).
    /// The part outside the plane is left untouched.
    pub(crate) fn read_rect_into(&self, xs: Range<usize>, ys: Range<usize>, out: &mut [T]) {
        let stride = xs.len();
        if stride == 0 {
            return;
        }
        self.each_tile_in(xs.clone(), ys.clone(), |i| {
            let tile = self.tile_ref(i);
            let cx = xs.start.max(tile.x)..xs.end.min(tile.x + tile.w);
            let cy = ys.start.max(tile.y)..ys.end.min(tile.y + tile.h);
            for y in cy {
                let at = (y - ys.start) * stride + (cx.start - xs.start);
                tile.copy_row(
                    y - tile.y,
                    cx.start - tile.x..cx.end - tile.x,
                    &mut out[at..at + cx.len()],
                );
            }
        });
    }

    /// Write `src` (`xs.len()` per row) into columns `xs` × global rows `ys`.
    /// The part outside the plane is skipped, and a tile whose part already
    /// holds `src`'s uniform value is not forked.
    pub(crate) fn write_rect_from(&mut self, xs: Range<usize>, ys: Range<usize>, src: &[T]) {
        let stride = xs.len();
        if stride == 0 {
            return;
        }
        let wanted = self.tiles_in(xs.clone(), ys.clone());
        let gen = crate::fresh_stamp();
        for i in wanted {
            let (tx, ty, tw, th) = self.tile_rect(i);
            let cx = xs.start.max(tx)..xs.end.min(tx + tw);
            let cy = ys.start.max(ty)..ys.end.min(ty + th);
            let at = |y: usize| (y - ys.start) * stride + (cx.start - xs.start);
            let part = cy.clone().map(|y| &src[at(y)..at(y) + cx.len()]);
            if self.writes_nothing(i, part) {
                continue;
            }
            self.fork(i, gen);
            self.with_dense_mut(i, |dst| {
                for y in cy {
                    let d = (y - ty) * tw + (cx.start - tx);
                    dst[d..d + cx.len()].copy_from_slice(&src[at(y)..at(y) + cx.len()]);
                }
            });
        }
    }

    /// Copy the logical samples from index `start` into `out`; the part past
    /// the end is left untouched.
    pub(crate) fn read_span(&self, start: usize, out: &mut [T]) {
        self.for_span(start, out.len(), |y, xs, at| {
            self.read_rect_into(xs, y..y + 1, &mut out[at..]);
        });
    }

    /// Split the logical range `start .. start + len` into row pieces
    /// `(row, columns, offset in the range)`.
    fn for_span(&self, start: usize, len: usize, mut f: impl FnMut(usize, Range<usize>, usize)) {
        let end = (start + len).min(self.len());
        let mut at = start;
        while at < end {
            let (y, x) = (at / self.width, at % self.width);
            let n = (self.width - x).min(end - at);
            f(y, x..x + n, at - start);
            at += n;
        }
    }

    /// A plane whose rows `rows` fills one strip of tiles at a time, so only
    /// `TILE` rows are ever held contiguously.
    pub(crate) fn build<E>(
        width: usize,
        height: usize,
        channels: usize,
        mut rows: impl FnMut(usize, &mut [T]) -> Result<(), E>,
    ) -> Result<Self, E> {
        let mut strip = vec![T::default(); width * TILE.min(height.max(1))];
        Self::build_strips(width, height, channels, &mut strip, |y0, out| {
            for (k, row) in out.chunks_mut(width.max(1)).enumerate() {
                rows(y0 + k, row)?;
            }
            Ok(())
        })
    }

    /// A plane whose strips `fill(first_row, strip)` fills, up to `TILE` global
    /// rows at a time through the caller's `strip` buffer (at least
    /// `width × TILE` samples); each strip is written and settled before the
    /// next, so its uniform tiles hold no allocation.
    pub(crate) fn build_strips<E>(
        width: usize,
        height: usize,
        channels: usize,
        strip: &mut [T],
        mut fill: impl FnMut(usize, &mut [T]) -> Result<(), E>,
    ) -> Result<Self, E> {
        let mut plane = Self::filled(width, height, channels, T::default());
        for c in 0..channels {
            let mut ly = 0;
            while ly < height {
                let h = TILE.min(height - ly);
                let y0 = c * height + ly;
                let out = &mut strip[..width * h];
                out.fill(T::default());
                fill(y0, out)?;
                plane.write_rect_from(0..width, y0..y0 + h, out);
                plane.settle();
                ly += h;
            }
        }
        Ok(plane)
    }
}
