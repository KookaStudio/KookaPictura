//! The document composite as a pyramid of halved, premultiplied levels.
//!
//! Level 0 is the document's own composite (planar straight alpha); it is
//! supplied by the caller to [`ViewPyramid::rebuild`], [`ViewPyramid::update`],
//! and [`ViewPyramid::crop`] and is never stored, so a 16000² document costs no
//! second full-resolution copy. Levels below 0 are stored premultiplied — the
//! only format averaging transparent edges correctly — and halve until the next
//! long side would fall below [`SMALLEST_SIDE`].
//!
//! A damaged rectangle is repaired level by level from the level above, so an
//! in-place update lands on exactly the pyramid a full rebuild would produce.

use pictura_core::{Plane, PsdRect};
use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

/// Levels stop halving once the next would be smaller than this on its long side.
pub const SMALLEST_SIDE: u32 = 256;

/// The tile grid a damage update is rounded out to, in level-0 pixels. A dirty
/// rectangle grows outward to a whole number of tiles before the level walk, so
/// adjacent dabs of a frame share tiles and the pyramid is repaired on one
/// stable grid. `level0` itself is never touched here: the caller owns it.
pub const TILE: i32 = 64;

/// The most pixels [`ViewPyramid::coarse`] samples into its first filled level.
const COARSE_PIXELS: u64 = 4_000_000;

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

/// One stored pyramid level: interleaved premultiplied RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyramidLevel {
    width: u32,
    height: u32,
    /// Shared with any [`LevelView`] handed out; a write while one is alive
    /// copies the level first so the view keeps the bytes it was made from.
    data: Arc<Vec<u8>>,
}

impl PyramidLevel {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: Arc::new(vec![0u8; width as usize * height as usize * 4]),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Interleaved premultiplied RGBA, row-major.
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// The level's own bytes for writing, copied first when a view shares them.
    fn data_mut(&mut self) -> &mut [u8] {
        Arc::make_mut(&mut self.data).as_mut_slice()
    }
}

/// A borrowed, shared handle to a stored level's premultiplied RGBA bytes.
///
/// The handle keeps the level's storage alive; an update while it exists writes
/// a fresh buffer so the handle's bytes never change under it. Drop it when the
/// display is done with it.
#[derive(Clone)]
pub struct LevelView {
    data: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

impl LevelView {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Interleaved premultiplied RGBA, row-major.
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Identity of the shared allocation, for tests that prove no copy happened.
    pub fn storage_id(&self) -> usize {
        self.data.as_ptr() as usize
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

    /// The first level of a `width`×`height` composite at or under
    /// [`COARSE_PIXELS`], for [`Self::coarse`].
    pub fn coarse_level(width: u32, height: u32) -> usize {
        let (mut w, mut h, mut level) = (width, height, 0);
        while u64::from(w) * u64::from(h) > COARSE_PIXELS {
            (w, h, level) = (half(w), half(h), level + 1);
        }
        level
    }

    /// A stand-in cheap enough for the GUI thread: every level of a rebuild is
    /// allocated, but only `first` and those below it hold pixels. `first` is
    /// point-sampled from level 0, so it aliases until the exact pyramid
    /// replaces it. The caller must not read a level above `first`.
    pub fn coarse(level0: Planes<'_>, first: usize) -> Self {
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
            let level = pyramid.levels.len() + 1;
            if level == first {
                sample_from_planes(&level0, &mut next, level);
            } else if level > first {
                let above = pyramid.levels.last().expect("the level above is stored");
                shrink_into(above, &mut next, rect_of(nw, nh));
            }
            pyramid.levels.push(next);
            lw = nw;
            lh = nh;
        }
        pyramid
    }

    /// Whether this was built for a composite of this size.
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
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
                .map_or((0, 0), |l| (l.width, l.height))
        }
    }

    /// Repair `dirty` of level 0 (already changed in the caller's buffer) and
    /// every level below it, leaving each level byte-identical to a rebuild.
    ///
    /// The rectangle is rounded out to the [`TILE`] grid first, so a frame's
    /// dabs land on one stable tile grid; the per-level walk then expands by the
    /// 2× filter footprint (a level-`N` texel reads a 2×2 block at `N-1`).
    pub fn update(&mut self, level0: Planes<'_>, dirty: PsdRect) {
        if self.levels.is_empty() {
            return;
        }
        let mut damaged = clip(tile_align(dirty), rect_of(self.width, self.height));
        let mut rects = Vec::with_capacity(self.levels.len());
        for level in 1..=self.levels.len() {
            let (lw, lh) = self.level_size(level);
            damaged = clip(expand(damaged), rect_of(lw, lh));
            rects.push(damaged);
        }
        for (i, rect) in rects.into_iter().enumerate() {
            if rect.width() <= 0 || rect.height() <= 0 {
                continue;
            }
            if i == 0 {
                shrink_from_planes(&level0, &mut self.levels[0], rect);
            } else {
                let (above, below) = self.levels.split_at_mut(i);
                shrink_into(&above[i - 1], &mut below[0], rect);
            }
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

    /// Overwrite `rect` of a stored `level` with interleaved premultiplied
    /// RGBA, leaving level 0, the levels above it and everything outside `rect`
    /// untouched. `rgba` must cover `rect`; a short or empty one is ignored.
    ///
    /// The large-stroke preview writes one stored level directly: [`Self::update`]
    /// would rebuild that level from level 0 and wipe the preview, and patching
    /// level 0 instead would cost the full-resolution bounding box the preview
    /// exists to avoid.
    pub fn patch_level(&mut self, level: usize, rgba: &[u8], rect: PsdRect) {
        if level == 0 {
            return;
        }
        let Some(dst) = self.levels.get_mut(level - 1) else {
            return;
        };
        let (sw, sh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
        if sw == 0 || sh == 0 || rgba.len() < (sw * sh * 4) as usize {
            return;
        }
        let clipped = clip(rect, rect_of(dst.width, dst.height));
        let (cw, ch) = (
            clipped.width().max(0) as u32,
            clipped.height().max(0) as u32,
        );
        if cw == 0 || ch == 0 {
            return;
        }
        let sx = (clipped.left - rect.left) as u32;
        let sy = (clipped.top - rect.top) as u32;
        let stride = dst.width as usize * 4;
        let data = dst.data_mut();
        for row in 0..ch as usize {
            let src = ((sy as usize + row) * sw as usize + sx as usize) * 4;
            let dst_off = ((clipped.top as usize + row) * stride) + clipped.left as usize * 4;
            let len = cw as usize * 4;
            data[dst_off..dst_off + len].copy_from_slice(&rgba[src..src + len]);
        }
    }

    /// A borrowed, shared view of stored `level`, or `None` for level 0 and
    /// past the last level. Level 0 is planar straight alpha and is never
    /// stored, so it has no interleaved buffer to share.
    pub fn level_view(&self, level: usize) -> Option<LevelView> {
        self.levels
            .get(level.checked_sub(1)?)
            .map(|stored| LevelView {
                data: Arc::clone(&stored.data),
                width: stored.width,
                height: stored.height,
            })
    }
}

fn half(n: u32) -> u32 {
    n.div_ceil(2)
}

fn half_up(n: i32) -> i32 {
    (n + 1).div_euclid(2)
}

fn rect_of(width: u32, height: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    }
}

/// Round a damage rectangle out to whole [`TILE`] cells.
fn tile_align(rect: PsdRect) -> PsdRect {
    PsdRect {
        top: align_down(rect.top),
        left: align_down(rect.left),
        bottom: align_up(rect.bottom),
        right: align_up(rect.right),
    }
}

fn align_down(v: i32) -> i32 {
    v.div_euclid(TILE) * TILE
}

fn align_up(v: i32) -> i32 {
    align_down(v) + if v.rem_euclid(TILE) == 0 { 0 } else { TILE }
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

/// The level-`L` pixels a level-`L+1` rectangle maps to: a pixel stands for a
/// 2×2 block of the level above, so the damage halves outward.
fn expand(rect: PsdRect) -> PsdRect {
    PsdRect {
        top: rect.top.div_euclid(2),
        left: rect.left.div_euclid(2),
        bottom: half_up(rect.bottom),
        right: half_up(rect.right),
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
    let o = (y * level.width as usize + x) * 4;
    let data = level.data();
    [data[o], data[o + 1], data[o + 2], data[o + 3]]
}

/// Fill `dst`, stored `level`, with the level-0 pixel at each texel's corner.
fn sample_from_planes(level0: &Planes<'_>, dst: &mut PyramidLevel, level: usize) {
    let whole = rect_of(dst.width, dst.height);
    each_row(dst, whole, |y, row| {
        let sy = ((y as usize) << level).min(level0.height as usize - 1);
        let Some(rows) = level0.rows(sy) else {
            return;
        };
        for (x, texel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let sx = (x << level).min(level0.width as usize - 1);
            texel.copy_from_slice(&premultiplied_row_pixel(&rows, sx));
        }
    });
}

/// Fill `rect` of `dst` (level 1) from the straight-alpha level 0, each pixel the
/// premultiplied average of its 2×2 block. A short block at an odd edge averages
/// over what is there, so the last row and column are not dimmed.
fn shrink_from_planes(level0: &Planes<'_>, dst: &mut PyramidLevel, rect: PsdRect) {
    let rect = clip(rect, rect_of(dst.width, dst.height));
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
    let stride = dst.width as usize * 4;
    let (top, bottom) = (rect.top as usize, rect.bottom as usize);
    dst.data_mut()[top * stride..bottom * stride]
        .par_chunks_mut(stride)
        .enumerate()
        .for_each(|(k, row)| f(rect.top + k as i32, row));
}

/// Fill `rect` of `dst` from `src`, one level up: each pixel the average of the
/// 2×2 block above it, already premultiplied on both sides.
fn shrink_into(src: &PyramidLevel, dst: &mut PyramidLevel, rect: PsdRect) {
    let rect = clip(rect, rect_of(dst.width, dst.height));
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    let sw = src.width as i32;
    let sh = src.height as i32;
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
                    let s = (sy as usize * src.width as usize + sx as usize) * 4;
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
mod tests {
    use super::*;

    /// A straight-alpha composite with something different in every pixel.
    struct Built {
        r: Vec<u8>,
        g: Vec<u8>,
        b: Vec<u8>,
        a: Vec<u8>,
        width: u32,
        height: u32,
    }

    impl Built {
        fn new(width: u32, height: u32, seed: u32) -> Self {
            let n = width as usize * height as usize;
            let mut r = vec![0u8; n];
            let mut g = vec![0u8; n];
            let mut b = vec![0u8; n];
            let mut a = vec![0u8; n];
            for y in 0..height {
                for x in 0..width {
                    let i = y as usize * width as usize + x as usize;
                    let v = (x * 7 + y * 13 + seed) % 251;
                    r[i] = v as u8;
                    g[i] = (v / 2) as u8;
                    b[i] = (255 - v) as u8;
                    a[i] = (60 + (x * 3 + y) % 196) as u8;
                }
            }
            Self {
                r,
                g,
                b,
                a,
                width,
                height,
            }
        }

        fn planes(&self) -> Planes<'_> {
            Planes::new(self.width, self.height, &self.r, &self.g, &self.b, &self.a)
        }

        fn blit(&mut self, src: &Built, rect: PsdRect) {
            for y in rect.top..rect.bottom {
                for x in rect.left..rect.right {
                    let i = y as usize * self.width as usize + x as usize;
                    let j = y as usize * src.width as usize + x as usize;
                    self.r[i] = src.r[j];
                    self.g[i] = src.g[j];
                    self.b[i] = src.b[j];
                    self.a[i] = src.a[j];
                }
            }
        }
    }

    #[test]
    fn levels_halve_down_to_the_smallest_side() {
        let p = Built::new(1500, 700, 0);
        let pyramid = ViewPyramid::rebuild(p.planes());
        let sizes: Vec<_> = (0..pyramid.level_count())
            .map(|l| pyramid.level_size(l))
            .collect();
        assert_eq!(sizes, vec![(1500, 700), (750, 350), (375, 175)]);
    }

    #[test]
    fn a_511_long_side_still_gets_a_256_level() {
        let p = Built::new(511, 300, 0);
        let pyramid = ViewPyramid::rebuild(p.planes());
        let sizes: Vec<_> = (0..pyramid.level_count())
            .map(|l| pyramid.level_size(l))
            .collect();
        assert_eq!(sizes, vec![(511, 300), (256, 150)]);
    }

    #[test]
    fn coarse_pyramid_matches_the_exact_levels() {
        let p = Built::new(1600, 1200, 5);
        let exact = ViewPyramid::rebuild(p.planes());
        let coarse = ViewPyramid::coarse(p.planes(), 2);
        assert_eq!(coarse.level_count(), exact.level_count());
        for level in 0..exact.level_count() {
            assert_eq!(coarse.level_size(level), exact.level_size(level));
        }
        for (x, y) in [(0, 0), (31, 17), (399, 299)] {
            assert_eq!(
                stored_pixel(&coarse.levels[1], x, y),
                premultiplied_row_pixel(&p.planes().rows(y * 4).expect("row"), x * 4)
            );
        }
        assert!(coarse.levels[0].data().iter().all(|&byte| byte == 0));
    }

    #[test]
    fn the_coarse_level_fits_its_budget() {
        assert_eq!(ViewPyramid::coarse_level(2000, 2000), 0);
        assert_eq!(ViewPyramid::coarse_level(16507, 16196), 4);
    }

    #[test]
    fn a_small_composite_is_its_only_level() {
        let p = Built::new(300, 200, 0);
        assert_eq!(ViewPyramid::rebuild(p.planes()).level_count(), 1);
    }

    #[test]
    fn odd_edges_average_over_what_is_there() {
        let p = Built::new(1031, 517, 3);
        let pyramid = ViewPyramid::rebuild(p.planes());
        let above = &pyramid.levels[0];
        for (x, y) in [(0, 0), (100, 50), (515, 258), (515, 0), (0, 258)] {
            let mut sum = [0u32; 4];
            let mut n = 0u32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let (sx, sy) = (2 * x + dx, 2 * y + dy);
                    if sx < 1031 && sy < 517 {
                        let px = premultiplied_row_pixel(&p.planes().rows(sy).expect("row"), sx);
                        for c in 0..4 {
                            sum[c] += px[c] as u32;
                        }
                        n += 1;
                    }
                }
            }
            let want: Vec<u8> = sum.iter().map(|s| average(*s, n)).collect();
            let got = stored_pixel(above, x, y);
            assert_eq!(got.to_vec(), want, "at {x},{y}");
        }
    }

    #[test]
    fn an_update_leaves_every_level_as_a_rebuild_would() {
        let (w, h) = (1203u32, 811u32);
        let mut level0 = Built::new(w, h, 0);
        let after = Built::new(w, h, 99);

        let mut pyramid = ViewPyramid::rebuild(level0.planes());
        for region in [rect(301, 97, 150, 83), rect(700, 1100, 103, 111)] {
            level0.blit(&after, region);
            pyramid.update(level0.planes(), region);
        }

        let rebuilt = ViewPyramid::rebuild(level0.planes());
        assert_eq!(pyramid.level_count(), rebuilt.level_count());
        for level in 0..rebuilt.levels.len() {
            assert_eq!(
                pyramid.levels[level],
                rebuilt.levels[level],
                "level {} differs",
                level + 1
            );
        }
    }

    #[test]
    fn a_damage_rect_is_rounded_out_to_the_tile_grid() {
        let aligned = tile_align(rect(70, 130, 10, 20));
        assert_eq!(aligned.top, 64);
        assert_eq!(aligned.left, 128);
        assert_eq!(aligned.bottom, 128);
        assert_eq!(aligned.right, 192);
        let already = rect(64, 128, 64, 64);
        assert_eq!(tile_align(already), already, "an aligned rect is fixed");
        let empty = tile_align(rect(0, 0, 0, 0));
        assert_eq!((empty.right, empty.bottom), (0, 0));
    }

    #[test]
    fn a_crop_returns_that_levels_pixels_premultiplied() {
        let p = Built::new(900, 600, 5);
        let pyramid = ViewPyramid::rebuild(p.planes());
        let r = PsdRect {
            top: 20,
            left: 10,
            bottom: 36,
            right: 42,
        };

        let lvl0 = pyramid.crop(p.planes(), 0, r);
        assert_eq!((lvl0.width(), lvl0.height()), (32, 16));
        for y in 0..16usize {
            for x in 0..32usize {
                let want = premultiplied_row_pixel(&p.planes().rows(20 + y).expect("row"), 10 + x);
                let o = (y * 32 + x) * 4;
                assert_eq!(lvl0.data()[o..o + 4], want[..], "at {x},{y}");
            }
        }

        let lvl1 = pyramid.crop(p.planes(), 1, r);
        let stored = &pyramid.levels[0];
        for y in 0..16usize {
            for x in 0..32usize {
                let want = stored_pixel(stored, 10 + x, 20 + y);
                let o = (y * 32 + x) * 4;
                assert_eq!(lvl1.data()[o..o + 4], want[..], "at {x},{y}");
            }
        }
    }

    #[test]
    fn a_crop_past_the_level_is_transparent() {
        let p = Built::new(300, 200, 1);
        let pyramid = ViewPyramid::rebuild(p.planes());
        let out = pyramid.crop(p.planes(), 0, rect(-5, -5, 10, 10));
        assert_eq!((out.width(), out.height()), (10, 10));
        for y in 0..5usize {
            for x in 0..5usize {
                let o = (y * 10 + x) * 4;
                assert_eq!(out.data()[o..o + 4], [0u8, 0, 0, 0][..]);
            }
        }
        // The overlapping 5x5 corner is opaque premultiplied level-0 pixels.
        let corner = premultiplied_row_pixel(&p.planes().rows(0).expect("row"), 0);
        let o = (5 * 10 + 5) * 4;
        assert_eq!(out.data()[o..o + 4], corner[..]);
    }

    #[test]
    fn shrinking_premultiplied_keeps_a_soft_edge_its_colour() {
        let (w, h) = (600u32, 600u32);
        let n = w as usize * h as usize;
        let mut a = vec![255u8; n];
        let mut r = vec![0u8; n];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = y * w as usize + x;
                if x % 2 == 1 {
                    a[i] = 0;
                } else {
                    r[i] = 255;
                }
            }
        }
        let p = Planes::new(w, h, &r, &[0u8; 600 * 600], &[0u8; 600 * 600], &a);
        let pyramid = ViewPyramid::rebuild(p);
        let out = pyramid.crop(p, 1, rect(10, 10, 1, 1));
        let o = 0;
        let alpha = out.data()[o + 3];
        assert!((alpha as i32 - 128).abs() <= 1, "alpha {alpha}");
        let unpremul_red = (out.data()[o] as u32 * 255 + alpha as u32 / 2) / alpha as u32;
        assert_eq!(unpremul_red, 255);
    }

    /// `p`'s four channels as one planar RGBA plane, tiled per channel, with
    /// tile `fork` forked from a held clone so the plane is diverged (a
    /// copy-on-write plane the pyramid must not materialize).
    fn planar_plane(p: &Built, fork: usize) -> Plane<u8> {
        let mut data = Vec::with_capacity((p.width * p.height * 4) as usize);
        data.extend_from_slice(&p.r);
        data.extend_from_slice(&p.g);
        data.extend_from_slice(&p.b);
        data.extend_from_slice(&p.a);
        let mut plane = Plane::from(data);
        plane.tile(p.width as usize, p.height as usize);
        let mut copy = plane.clone();
        let (x, y, _, _) = copy.tile_rect(fork).expect("a tile to fork");
        drop(copy.tiles_mut(x..x + 1, y..y + 1, p.width as usize));
        assert!(!copy.shares(&plane), "the tile was forked");
        copy
    }

    #[test]
    fn a_tiled_level0_builds_the_same_levels() {
        let p = Built::new(600, 400, 3);
        let plane = planar_plane(&p, 1);
        let banded = Planes::planar(p.width, p.height, &plane).expect("planar");

        let exact = ViewPyramid::rebuild(p.planes());
        let from_bands = ViewPyramid::rebuild(banded);
        assert_eq!(from_bands.level_count(), exact.level_count());
        for level in 1..exact.level_count() {
            assert!(
                from_bands.level_view(level).unwrap().data()
                    == exact.level_view(level).unwrap().data(),
                "level {level} differs"
            );
        }

        // The coarse stand-in reads level 0 at its texel corners.
        let coarse = ViewPyramid::coarse(banded, 1);
        let flat = p.planes();
        for (x, y) in [(0, 0), (7, 5), (74, 49)] {
            let want = premultiplied_row_pixel(&flat.rows(y * 2).expect("row"), x * 2);
            assert_eq!(
                stored_pixel(&coarse.levels[0], x, y),
                want,
                "coarse texel {x},{y}"
            );
        }
        assert!(
            !plane.is_mirrored(),
            "a tiled level 0 is never materialized"
        );
    }

    #[test]
    fn a_tiled_level0_updates_and_crops() {
        let (w, h) = (400u32, 300u32);
        let mut flat = Built::new(w, h, 9);
        let changed = Built::new(w, h, 11);
        let mut plane = planar_plane(&flat, 1);

        let mut flat_pyramid = ViewPyramid::rebuild(flat.planes());
        let mut banded_pyramid =
            ViewPyramid::rebuild(Planes::planar(w, h, &plane).expect("planar"));

        let damage = rect(40, 50, 90, 70);
        flat.blit(&changed, damage);
        for channel in 0..4 {
            for y in damage.top..damage.bottom {
                let src = match channel {
                    0 => &flat.r,
                    1 => &flat.g,
                    2 => &flat.b,
                    _ => &flat.a,
                };
                let start = y as usize * w as usize + damage.left as usize;
                let py = channel * h as usize + y as usize;
                plane.write_rect_from(
                    damage.left as usize..damage.right as usize,
                    py..py + 1,
                    w as usize,
                    &src[start..start + damage.width() as usize],
                );
            }
        }

        flat_pyramid.update(flat.planes(), damage);
        banded_pyramid.update(Planes::planar(w, h, &plane).expect("planar"), damage);
        for level in 1..flat_pyramid.level_count() {
            assert!(
                banded_pyramid.level_view(level).unwrap().data()
                    == flat_pyramid.level_view(level).unwrap().data(),
                "level {level} differs after the update"
            );
        }

        let crop = rect(10, 20, 30, 40);
        let banded_crop =
            banded_pyramid.crop(Planes::planar(w, h, &plane).expect("planar"), 0, crop);
        let flat_crop = flat_pyramid.crop(flat.planes(), 0, crop);
        assert!(banded_crop.data() == flat_crop.data(), "the crop differs");
        assert!(
            !plane.is_mirrored(),
            "an update and a crop never materialize level 0"
        );
    }

    fn rect(top: i32, left: i32, width: i32, height: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom: top + height,
            right: left + width,
        }
    }
}

#[cfg(test)]
mod patch_tests {
    use super::*;

    /// A straight-alpha composite with something different in every pixel.
    fn built(width: u32, height: u32) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let n = width as usize * height as usize;
        let mut r = vec![0u8; n];
        let mut g = vec![0u8; n];
        let mut b = vec![0u8; n];
        let mut a = vec![0u8; n];
        for y in 0..height {
            for x in 0..width {
                let i = y as usize * width as usize + x as usize;
                let v = (x * 7 + y * 13) % 251;
                r[i] = v as u8;
                g[i] = (v / 2) as u8;
                b[i] = (255 - v) as u8;
                a[i] = (60 + (x * 3 + y) % 196) as u8;
            }
        }
        (r, g, b, a)
    }

    #[test]
    fn a_patch_writes_only_the_rect_of_one_level() {
        let (r, g, b, a) = built(900, 600);
        let planes = Planes::new(900, 600, &r, &g, &b, &a);
        let mut pyramid = ViewPyramid::rebuild(planes);
        let level = 1;
        let (lw, lh) = pyramid.level_size(level);
        let before: Vec<u8> = pyramid.levels[level - 1].data().to_vec();

        let rect = PsdRect {
            top: 4,
            left: 6,
            bottom: 12,
            right: 20,
        };
        let w = (rect.right - rect.left) as usize;
        let h = (rect.bottom - rect.top) as usize;
        let patch: Vec<u8> = (0..w * h * 4).map(|i| (i % 251) as u8).collect();

        pyramid.patch_level(level, &patch, rect);

        let after = pyramid.levels[level - 1].data();
        assert_eq!(after.len(), before.len(), "the level keeps its size");
        assert!(lw >= 20 && lh >= 12, "the level must cover the patch");
        for y in 0..lh as usize {
            for x in 0..lw as usize {
                let i = (y * lw as usize + x) * 4;
                let inside = x >= rect.left as usize
                    && x < rect.right as usize
                    && y >= rect.top as usize
                    && y < rect.bottom as usize;
                if inside {
                    let px = (y - rect.top as usize) * w + (x - rect.left as usize);
                    assert_eq!(
                        &after[i..i + 4],
                        &patch[px * 4..px * 4 + 4],
                        "patched at {x},{y}"
                    );
                } else {
                    assert_eq!(&after[i..i + 4], &before[i..i + 4], "untouched at {x},{y}");
                }
            }
        }
        assert_eq!(
            pyramid.crop(planes, level, rect).data(),
            &patch[..],
            "the crop reads back exactly what was patched"
        );
    }

    #[test]
    fn a_patch_of_level_zero_or_past_the_last_is_ignored() {
        let (r, g, b, a) = built(900, 600);
        let planes = Planes::new(900, 600, &r, &g, &b, &a);
        let mut pyramid = ViewPyramid::rebuild(planes);
        let before: Vec<u8> = pyramid.levels[0].data().to_vec();
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        };
        let patch = vec![7u8; 4 * 4 * 4];

        pyramid.patch_level(0, &patch, rect);
        pyramid.patch_level(99, &patch, rect);
        pyramid.patch_level(1, &[], rect);
        let off = PsdRect {
            top: 900,
            left: 900,
            bottom: 904,
            right: 904,
        };
        pyramid.patch_level(1, &patch, off);

        assert_eq!(pyramid.levels[0].data(), before, "nothing was written");
    }

    fn damage_tile() -> PsdRect {
        PsdRect {
            top: 64,
            left: 128,
            bottom: 128,
            right: 192,
        }
    }

    fn change_tile(r: &mut [u8], g: &mut [u8], b: &mut [u8], a: &mut [u8], width: u32) {
        let rect = damage_tile();
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let i = y as usize * width as usize + x as usize;
                r[i] = r[i].wrapping_add(101);
                g[i] = g[i].wrapping_add(53);
                b[i] = b[i].wrapping_add(29);
                a[i] = a[i].max(1);
            }
        }
    }

    #[test]
    fn a_stored_level_view_shares_the_level_bytes() {
        let (r, g, b, a) = built(900, 600);
        let planes = Planes::new(900, 600, &r, &g, &b, &a);
        let pyramid = ViewPyramid::rebuild(planes);
        assert!(pyramid.level_view(0).is_none(), "level 0 is not stored");
        let view = pyramid.level_view(1).expect("a stored level");
        let (lw, lh) = pyramid.level_size(1);
        assert_eq!((view.width(), view.height()), (lw, lh));
        assert_eq!(view.data(), pyramid.levels[0].data());
        let again = pyramid.level_view(1).expect("a second handle");
        assert_eq!(
            view.storage_id(),
            again.storage_id(),
            "handles share the level's storage"
        );
    }

    #[test]
    fn an_update_copies_a_level_a_live_view_holds() {
        let (mut r, mut g, mut b, mut a) = built(900, 600);
        let original = Planes::new(900, 600, &r, &g, &b, &a);
        let mut pyramid = ViewPyramid::rebuild(original);
        let view = pyramid.level_view(1).expect("a stored level");
        let held = view.data().to_vec();
        let held_id = view.storage_id();

        change_tile(&mut r, &mut g, &mut b, &mut a, 900);
        let changed = Planes::new(900, 600, &r, &g, &b, &a);
        pyramid.update(changed, damage_tile());

        assert_eq!(
            view.data(),
            held.as_slice(),
            "the held view keeps the bytes it was made from"
        );
        let fresh = pyramid.level_view(1).expect("the updated level");
        assert_ne!(
            fresh.storage_id(),
            held_id,
            "the update wrote a fresh buffer while a view was alive"
        );
        let reference = ViewPyramid::rebuild(changed);
        assert_eq!(
            fresh.data(),
            reference.level_view(1).expect("reference level").data(),
            "the updated level equals a rebuild"
        );
    }

    #[test]
    fn an_update_without_a_view_writes_in_place() {
        let (mut r, mut g, mut b, mut a) = built(900, 600);
        let original = Planes::new(900, 600, &r, &g, &b, &a);
        let mut pyramid = ViewPyramid::rebuild(original);
        let id = pyramid.level_view(1).expect("a stored level").storage_id();

        change_tile(&mut r, &mut g, &mut b, &mut a, 900);
        let changed = Planes::new(900, 600, &r, &g, &b, &a);
        pyramid.update(changed, damage_tile());
        assert_eq!(
            pyramid
                .level_view(1)
                .expect("the updated level")
                .storage_id(),
            id,
            "an update with no live view writes the level in place"
        );
    }

    #[test]
    fn level_zero_has_no_borrowed_view() {
        let (r, g, b, a) = built(300, 200);
        let planes = Planes::new(300, 200, &r, &g, &b, &a);
        let pyramid = ViewPyramid::rebuild(planes);
        assert_eq!(pyramid.level_count(), 1, "a small composite has one level");
        assert!(pyramid.level_view(0).is_none());
        assert!(pyramid.level_view(1).is_none());
    }
}
