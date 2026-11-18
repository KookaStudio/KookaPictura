//! Independent whole-rebuild reference, available only to tests and the
//! non-default `test-oracle` feature. Level 0 is supplied by the caller;
//! reduced levels use their own premultiplication and rounded 2x2 reduction.

use crate::pyramid_level::{PyramidLevel, SMALLEST_SIDE};
use pictura_core::{Plane, PsdRect};
use std::borrow::Cow;
use std::ops::Range;

/// A borrowed planar straight-alpha level-0 view, one channel per row.
///
/// The channels are either four contiguous slices or one planar RGBA
/// copy-on-write plane. The plane is read a row at a time through
/// [`Plane::row`], so a banded or diverged source is never materialized whole.
#[derive(Clone, Copy)]
pub struct Planes<'a> {
    pub width: u32,
    pub height: u32,
    source: Source<'a>,
}

#[derive(Clone, Copy)]
enum Source<'a> {
    Slices {
        r: &'a [u8],
        g: &'a [u8],
        b: &'a [u8],
        a: &'a [u8],
    },
    /// A planar plane whose channel `c` starts `c * height` rows in.
    Planar(&'a Plane<u8>),
}

impl<'a> Planes<'a> {
    /// A view over four contiguous channel slices.
    pub fn new(
        width: u32,
        height: u32,
        r: &'a [u8],
        g: &'a [u8],
        b: &'a [u8],
        a: &'a [u8],
    ) -> Self {
        Self {
            width,
            height,
            source: Source::Slices { r, g, b, a },
        }
    }

    /// A view over a 4-channel planar plane, or `None` unless it holds
    /// `width × height` samples per channel.
    pub fn planar(width: u32, height: u32, plane: &'a Plane<u8>) -> Option<Self> {
        let samples = width as usize * height as usize;
        (samples > 0 && plane.len() >= samples * 4).then_some(Self {
            width,
            height,
            source: Source::Planar(plane),
        })
    }

    /// Channel `channel` (0 = r, 1 = g, 2 = b, 3 = a) of row `y`, or `None`
    /// past the plane.
    pub fn row(&self, channel: usize, y: usize) -> Option<Cow<'a, [u8]>> {
        self.row_span(channel, y, 0..self.width as usize)
    }

    /// Columns `xs` of channel `channel` of row `y`: a tiled level 0 copies
    /// only those columns, so a small update stays small.
    fn row_span(&self, channel: usize, y: usize, xs: Range<usize>) -> Option<Cow<'a, [u8]>> {
        let width = self.width as usize;
        let start = y.checked_mul(width)?;
        match self.source {
            Source::Slices { r, g, b, a } => {
                let plane = match channel {
                    0 => r,
                    1 => g,
                    2 => b,
                    _ => a,
                };
                plane.get(start..start + width)?.get(xs).map(Cow::Borrowed)
            }
            Source::Planar(plane) => plane.row_span(channel * self.height as usize + y, xs, width),
        }
    }

    /// Columns `xs` of one line's four channel rows.
    fn span(&self, y: usize, xs: Range<usize>) -> Option<[Cow<'a, [u8]>; 4]> {
        Some([
            self.row_span(0, y, xs.clone())?,
            self.row_span(1, y, xs.clone())?,
            self.row_span(2, y, xs.clone())?,
            self.row_span(3, y, xs)?,
        ])
    }

    /// One line's four channel rows.
    pub fn rows(&self, y: usize) -> Option<[Cow<'a, [u8]>; 4]> {
        Some([
            self.row(0, y)?,
            self.row(1, y)?,
            self.row(2, y)?,
            self.row(3, y)?,
        ])
    }
}

/// The composite at every stored level below 0. See the module notes.
#[derive(Default)]
pub struct ViewPyramid {
    width: u32,
    height: u32,
    /// `levels[i]` is level `i + 1`.
    levels: Vec<PyramidLevel>,
}

impl ViewPyramid {
    /// Build every level below 0 from the borrowed level-0 composite.
    pub fn rebuild(level0: Planes<'_>) -> Self {
        let mut pyramid = Self {
            width: level0.width,
            height: level0.height,
            levels: Vec::new(),
        };
        let (mut lw, mut lh) = (level0.width, level0.height);
        loop {
            let (nw, nh) = (half(lw), half(lh));
            if nw.max(nh) < SMALLEST_SIDE || nw <= 1 || nh <= 1 {
                break;
            }
            let mut next = PyramidLevel::new(nw, nh);
            let whole = rect_of(nw, nh);
            match pyramid.levels.last() {
                None => shrink_from_planes(&level0, &mut next, whole),
                Some(above) => shrink_into(above, &mut next, whole),
            }
            pyramid.levels.push(next);
            lw = nw;
            lh = nh;
        }
        pyramid
    }

    /// Level 0 plus the stored levels.
    pub fn level_count(&self) -> usize {
        1 + self.levels.len()
    }

    /// The size of `level`, `(0, 0)` past the last.
    pub fn level_size(&self, level: usize) -> (u32, u32) {
        if level == 0 {
            (self.width, self.height)
        } else {
            self.levels
                .get(level - 1)
                .map_or((0, 0), |l| (l.width(), l.height()))
        }
    }

    /// `rect` of `level`, premultiplied and rectangle-sized. Anything outside
    /// the level comes back transparent.
    pub fn crop(&self, level0: Planes<'_>, level: usize, rect: PsdRect) -> PyramidLevel {
        let (lw, lh) = self.level_size(level);
        let rw = rect.width().max(0) as u32;
        let rh = rect.height().max(0) as u32;
        if rw == 0 || rh == 0 {
            return PyramidLevel::new(0, 0);
        }
        let mut out = PyramidLevel::new(rw, rh);
        let (lw_i, lh_i) = (lw as i32, lh as i32);
        let out_data = out.data_mut();
        for oy in 0..rh as i32 {
            let sy = rect.top + oy;
            if sy < 0 || sy >= lh_i {
                continue;
            }
            let rows = if level == 0 {
                level0.rows(sy as usize)
            } else {
                None
            };
            for ox in 0..rw as i32 {
                let sx = rect.left + ox;
                if sx < 0 || sx >= lw_i {
                    continue;
                }
                let o = (oy as usize * rw as usize + ox as usize) * 4;
                let px = if level == 0 {
                    match &rows {
                        Some(rows) => premultiplied_row_pixel(rows, sx as usize),
                        None => continue,
                    }
                } else {
                    stored_pixel(&self.levels[level - 1], sx as usize, sy as usize)
                };
                out_data[o..o + 4].copy_from_slice(&px);
            }
        }
        out
    }
}

fn half(n: u32) -> u32 {
    n.div_ceil(2)
}

fn rect_of(width: u32, height: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    }
}

fn empty() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
    }
}

fn clip(rect: PsdRect, bounds: PsdRect) -> PsdRect {
    let top = rect.top.max(bounds.top);
    let left = rect.left.max(bounds.left);
    let bottom = rect.bottom.min(bounds.bottom);
    let right = rect.right.min(bounds.right);
    if right <= left || bottom <= top {
        empty()
    } else {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }
}

/// Premultiply one straight-alpha sample: round(c * a / 255).
fn premul(c: u8, a: u8) -> u32 {
    (c as u32 * a as u32 + 127) / 255
}

fn average(sum: u32, n: u32) -> u8 {
    ((sum + n / 2) / n) as u8
}

fn premultiplied_row_pixel(rows: &[Cow<'_, [u8]>; 4], x: usize) -> [u8; 4] {
    let a = rows[3][x];
    [
        premul(rows[0][x], a) as u8,
        premul(rows[1][x], a) as u8,
        premul(rows[2][x], a) as u8,
        a,
    ]
}

fn stored_pixel(level: &PyramidLevel, x: usize, y: usize) -> [u8; 4] {
    let o = (y * level.width() as usize + x) * 4;
    let data = level.data();
    [data[o], data[o + 1], data[o + 2], data[o + 3]]
}

/// Fill `rect` of `dst` (level 1) from the straight-alpha level 0, each pixel the
/// premultiplied average of its 2×2 block. A short block at an odd edge averages
/// over what is there, so the last row and column are not dimmed.
fn shrink_from_planes(level0: &Planes<'_>, dst: &mut PyramidLevel, rect: PsdRect) {
    let rect = clip(rect, rect_of(dst.width(), dst.height()));
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    let lw = level0.width as i32;
    let lh = level0.height as i32;
    // Only the level-0 columns under `rect` are read.
    let from = (2 * rect.left).min(lw) as usize;
    let cols = from..(2 * rect.right).min(lw) as usize;
    each_row(dst, rect, |y, row| {
        let above = [
            if 2 * y < lh {
                level0.span((2 * y) as usize, cols.clone())
            } else {
                None
            },
            if 2 * y + 1 < lh {
                level0.span((2 * y + 1) as usize, cols.clone())
            } else {
                None
            },
        ];
        for x in rect.left..rect.right {
            let (mut sr, mut sg, mut sb, mut sa, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for rows in above.iter().flatten() {
                for dx in 0..2 {
                    let sx = 2 * x + dx;
                    if sx >= lw {
                        continue;
                    }
                    let sx = sx as usize - from;
                    let a = rows[3][sx];
                    sr += premul(rows[0][sx], a);
                    sg += premul(rows[1][sx], a);
                    sb += premul(rows[2][sx], a);
                    sa += a as u32;
                    n += 1;
                }
            }
            let o = x as usize * 4;
            row[o] = average(sr, n);
            row[o + 1] = average(sg, n);
            row[o + 2] = average(sb, n);
            row[o + 3] = average(sa, n);
        }
    });
}

/// Run `f` over each destination row of `rect` in parallel, handing it the
/// row's document `y` and the whole row's bytes. Rows are independent: every
/// level texel reads only the level above.
fn each_row(dst: &mut PyramidLevel, rect: PsdRect, f: impl Fn(i32, &mut [u8]) + Sync) {
    use rayon::prelude::*;
    let stride = dst.width() as usize * 4;
    let (top, bottom) = (rect.top as usize, rect.bottom as usize);
    dst.data_mut()[top * stride..bottom * stride]
        .par_chunks_mut(stride)
        .enumerate()
        .for_each(|(k, row)| f(rect.top + k as i32, row));
}

/// Fill `rect` of `dst` from `src`, one level up: each pixel the average of the
/// 2×2 block above it, already premultiplied on both sides.
fn shrink_into(src: &PyramidLevel, dst: &mut PyramidLevel, rect: PsdRect) {
    let rect = clip(rect, rect_of(dst.width(), dst.height()));
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let src_data = src.data();
    each_row(dst, rect, |y, row| {
        for x in rect.left..rect.right {
            let mut sum = [0u32; 4];
            let mut n = 0u32;
            for dy in 0..2 {
                let sy = 2 * y + dy;
                if sy >= sh {
                    continue;
                }
                for dx in 0..2 {
                    let sx = 2 * x + dx;
                    if sx >= sw {
                        continue;
                    }
                    let s = (sy as usize * src.width() as usize + sx as usize) * 4;
                    for (c, slot) in sum.iter_mut().enumerate() {
                        *slot += src_data[s + c] as u32;
                    }
                    n += 1;
                }
            }
            let o = x as usize * 4;
            for (c, value) in sum.iter().enumerate() {
                row[o + c] = average(*value, n);
            }
        }
    });
}

#[cfg(test)]
mod tests;
