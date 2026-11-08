//! Streaming access to a plane: tiles, row copies, rectangles and ranges that
//! never build a contiguous copy of a tiled plane.
//!
//! `width` follows one rule everywhere: a tiled plane uses its own width, and a
//! flat plane uses the argument. A flat plane presents itself as full-width
//! strips of [`TILE`] rows, so a tile walk also serves a plane never tiled.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use rayon::prelude::*;

use super::{Plane, Store, MATERIALIZATIONS};
use crate::tiled_plane::{TileData, TileMut, TileRef, TileSample, TileState, TILE};

/// Run `f` and panic if it materialized a tiled plane.
///
/// The counter is process-wide, so the check is exact only when the test runs
/// alone in its process. Nextest gives every test its own process and sets
/// `NEXTEST`; under the threaded `cargo test` runner, where any other test can
/// bump the counter, `f` runs unchecked.
pub fn assert_no_materialization<R>(f: impl FnOnce() -> R) -> R {
    if std::env::var_os("NEXTEST").is_none() {
        return f();
    }
    let before = MATERIALIZATIONS.load(Ordering::Relaxed);
    let out = f();
    let after = MATERIALIZATIONS.load(Ordering::Relaxed);
    assert_eq!(
        after - before,
        0,
        "a tiled plane was materialized into a contiguous copy"
    );
    out
}

/// The full-width strips of a flat plane meeting rows `ys`, as
/// `(first row, rows)`.
fn strips(len: usize, width: usize, ys: Range<usize>) -> Vec<(usize, usize)> {
    if width == 0 {
        return Vec::new();
    }
    let total = len / width;
    let (y0, y1) = (ys.start.min(total), ys.end.min(total));
    let mut out = Vec::new();
    let mut y = y0 / TILE * TILE;
    while y < y1 {
        let h = TILE.min(total - y);
        out.push((y, h));
        y += h;
    }
    out
}

impl<T: TileSample> Plane<T> {
    /// The number of tiles; `0` while the plane is flat.
    pub fn tile_count(&self) -> usize {
        self.tiled().map_or(0, |t| t.tile_count())
    }

    /// Tile `i`'s generation: a clone keeps it and a write to the tile
    /// replaces it, so two tiles at the same index with the same generation
    /// hold the same samples. `0` while flat.
    pub fn tile_generation(&self, i: usize) -> u64 {
        self.tiled().map_or(0, |t| t.generation(i))
    }

    /// What tile `i` holds; `None` while flat or past the last tile.
    pub fn tile_state(&self, i: usize) -> Option<TileState<T>> {
        self.tiled().and_then(|t| t.state(i))
    }

    /// Tile `i`'s first column, first global row, width and height.
    pub fn tile_rect(&self, i: usize) -> Option<(usize, usize, usize, usize)> {
        self.tiled()
            .filter(|t| i < t.tile_count())
            .map(|t| t.tile_rect(i))
    }

    /// Tile `i`, read in place.
    pub fn tile_at(&self, i: usize) -> Option<TileRef<'_, T>> {
        self.tiled()
            .filter(|t| i < t.tile_count())
            .map(|t| t.tile_ref(i))
    }

    /// The indices of the tiles meeting columns `xs` × global rows `ys`, in
    /// row-major order; empty while flat.
    pub fn tiles_in(&self, xs: Range<usize>, ys: Range<usize>) -> Vec<usize> {
        self.tiled().map_or_else(Vec::new, |t| t.tiles_in(xs, ys))
    }

    /// Whether tile `i` is the same tile in `self` and `other` (same geometry,
    /// same generation).
    pub fn shares_tile(&self, i: usize, other: &Self) -> bool {
        match (self.tiled(), other.tiled()) {
            (Some(a), Some(b)) => a.shares_tile(i, b),
            _ => false,
        }
    }

    /// Take `other`'s tile `i`, allocation and generation, so the two share
    /// it. `false`, leaving the plane alone, when the geometries differ.
    pub fn share_tile_from(&mut self, i: usize, other: &Self) -> bool {
        let Some(src) = other.tiled() else {
            return false;
        };
        if self.tiled().is_none_or(|t| !t.same_geometry(src)) {
            return false;
        }
        self.touch();
        match &mut self.store {
            Store::Tiled(t) => t.adopt_tile(i, src),
            Store::Flat(_) => false,
        }
    }

    /// Visit the tiles meeting columns `xs` × rows `ys`, in row-major order.
    /// Each tile is whole: the caller clips it to the rectangle.
    pub fn for_each_tile(
        &self,
        xs: Range<usize>,
        ys: Range<usize>,
        width: usize,
        mut f: impl FnMut(TileRef<'_, T>),
    ) {
        match &self.store {
            Store::Tiled(t) => {
                for i in t.tiles_in(xs, ys) {
                    f(t.tile_ref(i));
                }
            }
            Store::Flat(data) => {
                if xs.start >= xs.end.min(width) {
                    return;
                }
                for (y, h) in strips(data.len(), width, ys) {
                    f(TileRef {
                        x: 0,
                        y,
                        w: width,
                        h,
                        stride: width,
                        data: TileData::Dense(&data[y * width..(y + h) * width]),
                    });
                }
            }
        }
    }

    /// The tiles meeting columns `xs` × rows `ys`, forked and writable, in
    /// row-major order. They are disjoint, so they can be split across
    /// threads. Call [`Self::settle`] when the write is done.
    pub fn tiles_mut(
        &mut self,
        xs: Range<usize>,
        ys: Range<usize>,
        width: usize,
    ) -> Vec<TileMut<'_, T>> {
        self.touch();
        match &mut self.store {
            Store::Tiled(t) => t.tiles_mut(xs, ys),
            Store::Flat(data) => {
                if xs.start >= xs.end.min(width) {
                    return Vec::new();
                }
                let parts = strips(data.len(), width, ys);
                let Some(&(first, _)) = parts.first() else {
                    return Vec::new();
                };
                let mut rest = &mut Arc::make_mut(data)[first * width..];
                let mut out = Vec::with_capacity(parts.len());
                for (y, h) in parts {
                    let (head, tail) = std::mem::take(&mut rest).split_at_mut(h * width);
                    out.push(TileMut {
                        x: 0,
                        y,
                        w: width,
                        h,
                        stride: width,
                        data: head,
                    });
                    rest = tail;
                }
                out
            }
        }
    }

    /// The tiles at `indices` (strictly ascending), forked and writable;
    /// empty while flat or when `indices` is not ascending and in range.
    pub fn tiles_mut_at(&mut self, indices: &[usize]) -> Vec<TileMut<'_, T>> {
        let count = self.tile_count();
        if indices.windows(2).any(|p| p[0] >= p[1]) || indices.last().is_some_and(|&i| i >= count) {
            return Vec::new();
        }
        self.touch();
        match &mut self.store {
            Store::Tiled(t) => t.tiles_mut_at(indices),
            Store::Flat(_) => Vec::new(),
        }
    }

    /// A copy with the same samples that shares no allocation with `self`.
    pub fn detached(&self) -> Self {
        match &self.store {
            Store::Flat(data) => Plane::from(data.to_vec()),
            Store::Tiled(t) => Plane::with_store(Store::Tiled(t.detached())),
        }
    }

    /// Run `f` on every tile meeting the rectangle, in parallel, then settle.
    pub fn for_each_tile_mut(
        &mut self,
        xs: Range<usize>,
        ys: Range<usize>,
        width: usize,
        f: impl Fn(TileMut<'_, T>) + Sync + Send,
    ) {
        self.tiles_mut(xs, ys, width).into_par_iter().for_each(f);
        self.settle();
    }

    /// Collapse the tiles written since the last settle that hold one value
    /// into `Uniform` (or `Empty`). No sample changes, so the stamp, the
    /// ident and every generation are kept. A flat plane has nothing to do.
    pub fn settle(&mut self) {
        if let Store::Tiled(t) = &mut self.store {
            t.settle();
        }
    }

    /// Copy row `y` into `out`, at most `out.len()` samples. `false`, with
    /// `out` untouched, when the row is past the end.
    pub fn read_row_into(&self, y: usize, width: usize, out: &mut [T]) -> bool {
        match &self.store {
            Store::Tiled(t) => t.read_row_into(y, out),
            Store::Flat(data) => {
                let Some(start) = y.checked_mul(width).filter(|s| *s < data.len()) else {
                    return false;
                };
                let n = out.len().min(width).min(data.len() - start);
                out[..n].copy_from_slice(&data[start..start + n]);
                true
            }
        }
    }

    /// Row `y`'s `width` samples: borrowed from a flat plane, copied from a
    /// tiled one (which uses its own width). `None` past the last whole row.
    pub fn row(&self, y: usize, width: usize) -> Option<Cow<'_, [T]>> {
        match &self.store {
            Store::Flat(data) => {
                let start = y.checked_mul(width).filter(|_| width > 0)?;
                data.get(start..start + width).map(Cow::Borrowed)
            }
            Store::Tiled(t) => {
                if y >= t.height * t.channels {
                    return None;
                }
                let mut row = vec![T::default(); t.width];
                t.read_row_into(y, &mut row);
                Some(Cow::Owned(row))
            }
        }
    }

    /// Columns `xs` of row `y`: borrowed from a flat plane, copied from a tiled
    /// one, so a narrow region costs its own width. `None` past the last whole
    /// row or when `xs` runs past the row.
    pub fn row_span(&self, y: usize, xs: Range<usize>, width: usize) -> Option<Cow<'_, [T]>> {
        match &self.store {
            Store::Flat(data) => {
                let start = y.checked_mul(width).filter(|_| xs.end <= width)?;
                data.get(start + xs.start..start + xs.end)
                    .map(Cow::Borrowed)
            }
            Store::Tiled(t) => {
                if y >= t.height * t.channels || xs.end > t.width {
                    return None;
                }
                if let Some(span) = t.span_in_tile(y, xs.clone()) {
                    return Some(Cow::Borrowed(span));
                }
                let mut span = vec![T::default(); xs.len()];
                t.read_rect_into(xs, y..y + 1, &mut span);
                Some(Cow::Owned(span))
            }
        }
    }

    /// Write `src` into row `y` from column 0; samples past the row end are
    /// ignored. A tile the row leaves unchanged is not forked. `false` when
    /// the row is past the end.
    pub fn write_row_from(&mut self, y: usize, width: usize, src: &[T]) -> bool {
        if y >= self.rows(width) {
            return false;
        }
        self.touch();
        match &mut self.store {
            Store::Tiled(t) => t.write_row_from(y, src),
            Store::Flat(data) => {
                let start = y * width;
                let n = src.len().min(width).min(data.len() - start);
                Arc::make_mut(data)[start..start + n].copy_from_slice(&src[..n]);
                true
            }
        }
    }

    /// The number of whole rows: global rows for a tiled plane.
    fn rows(&self, width: usize) -> usize {
        match &self.store {
            Store::Tiled(t) => t.height * t.channels,
            Store::Flat(data) => data.len().checked_div(width).unwrap_or(0),
        }
    }

    /// Copy the logical samples from index `start` into `out`; the part of
    /// `out` past the plane's end is left untouched. For a buffer that stacks
    /// several channels in one plane, this reads one channel.
    pub fn read_into(&self, start: usize, out: &mut [T]) {
        match &self.store {
            Store::Tiled(t) => t.read_span(start, out),
            Store::Flat(data) => {
                let end = (start + out.len()).min(data.len());
                if start < end {
                    out[..end - start].copy_from_slice(&data[start..end]);
                }
            }
        }
    }

    /// Copy the samples of columns `xs` × rows `ys` into `out`, `xs.len()` per
    /// row. The part of the rectangle outside the plane is left untouched in
    /// `out`, so an empty or fully outside rectangle copies nothing.
    pub fn read_rect_into(&self, xs: Range<usize>, ys: Range<usize>, width: usize, out: &mut [T]) {
        match &self.store {
            Store::Tiled(t) => t.read_rect_into(xs, ys, out),
            Store::Flat(data) => {
                let stride = xs.len();
                let x1 = xs.end.min(width);
                if stride == 0 || xs.start >= x1 {
                    return;
                }
                for (dy, y) in ys.enumerate() {
                    let start = y * width;
                    if start + x1 > data.len() {
                        break;
                    }
                    let at = dy * stride;
                    out[at..at + x1 - xs.start]
                        .copy_from_slice(&data[start + xs.start..start + x1]);
                }
            }
        }
    }

    /// Write `src` (`xs.len()` per row) into columns `xs` × rows `ys`. The part
    /// of the rectangle outside the plane is skipped, and a tile the write
    /// leaves unchanged is not forked. Call [`Self::settle`] when the edit is
    /// done.
    pub fn write_rect_from(&mut self, xs: Range<usize>, ys: Range<usize>, width: usize, src: &[T]) {
        let stride = xs.len();
        if stride == 0 || ys.is_empty() {
            return;
        }
        self.touch();
        match &mut self.store {
            Store::Tiled(t) => t.write_rect_from(xs, ys, src),
            Store::Flat(data) => {
                let x1 = xs.end.min(width);
                if xs.start >= x1 {
                    return;
                }
                let total = data.len().checked_div(width).unwrap_or(0);
                let dst = Arc::make_mut(data);
                for (dy, y) in ys.enumerate().take_while(|(_, y)| *y < total) {
                    let (start, at) = (y * width, dy * stride);
                    dst[start + xs.start..start + x1].copy_from_slice(&src[at..at + x1 - xs.start]);
                }
            }
        }
    }

    /// Whether every sample equals `value`. Uniform and empty tiles are not
    /// scanned, and nothing is materialized.
    pub fn all_eq(&self, value: T) -> bool {
        match &self.store {
            Store::Tiled(t) => t.all_eq(value),
            Store::Flat(data) => data.iter().all(|s| *s == value),
        }
    }
}

#[cfg(test)]
mod tests;
