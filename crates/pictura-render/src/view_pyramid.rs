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

use pictura_core::PsdRect;
use std::sync::Arc;

/// Levels stop halving once the next would be smaller than this on its long side.
pub const SMALLEST_SIDE: u32 = 256;

/// The tile grid a damage update is rounded out to, in level-0 pixels. A dirty
/// rectangle grows outward to a whole number of tiles before the level walk, so
/// adjacent dabs of a frame share tiles and the pyramid is repaired on one
/// stable grid. `level0` itself is never touched here: the caller owns it.
pub const TILE: i32 = 64;

/// A borrowed planar straight-alpha level-0 view, one slice per channel.
#[derive(Clone, Copy)]
pub struct Planes<'a> {
    pub width: u32,
    pub height: u32,
    pub r: &'a [u8],
    pub g: &'a [u8],
    pub b: &'a [u8],
    pub a: &'a [u8],
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
            for ox in 0..rw as i32 {
                let sx = rect.left + ox;
                if sx < 0 || sx >= lw_i {
                    continue;
                }
                let o = (oy as usize * rw as usize + ox as usize) * 4;
                let px = if level == 0 {
                    premultiplied_plane_pixel(&level0, sx as usize, sy as usize)
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

fn premultiplied_plane_pixel(l0: &Planes<'_>, x: usize, y: usize) -> [u8; 4] {
    let i = y * l0.width as usize + x;
    let a = l0.a[i];
    [
        premul(l0.r[i], a) as u8,
        premul(l0.g[i], a) as u8,
        premul(l0.b[i], a) as u8,
        a,
    ]
}

fn stored_pixel(level: &PyramidLevel, x: usize, y: usize) -> [u8; 4] {
    let o = (y * level.width as usize + x) * 4;
    let data = level.data();
    [data[o], data[o + 1], data[o + 2], data[o + 3]]
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
    each_row(dst, rect, |y, row| {
        for x in rect.left..rect.right {
            let (mut sr, mut sg, mut sb, mut sa, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for dy in 0..2 {
                let sy = 2 * y + dy;
                if sy >= lh {
                    continue;
                }
                for dx in 0..2 {
                    let sx = 2 * x + dx;
                    if sx >= lw {
                        continue;
                    }
                    let i = sy as usize * lw as usize + sx as usize;
                    let a = level0.a[i];
                    sr += premul(level0.r[i], a);
                    sg += premul(level0.g[i], a);
                    sb += premul(level0.b[i], a);
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
            Planes {
                width: self.width,
                height: self.height,
                r: &self.r,
                g: &self.g,
                b: &self.b,
                a: &self.a,
            }
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
                        let px = premultiplied_plane_pixel(&p.planes(), sx, sy);
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
                let want = premultiplied_plane_pixel(&p.planes(), 10 + x, 20 + y);
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
        let corner = premultiplied_plane_pixel(&p.planes(), 0, 0);
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
        let p = Planes {
            width: w,
            height: h,
            r: &r,
            g: &[0u8; 600 * 600],
            b: &[0u8; 600 * 600],
            a: &a,
        };
        let pyramid = ViewPyramid::rebuild(p);
        let out = pyramid.crop(p, 1, rect(10, 10, 1, 1));
        let o = 0;
        let alpha = out.data()[o + 3];
        assert!((alpha as i32 - 128).abs() <= 1, "alpha {alpha}");
        let unpremul_red = (out.data()[o] as u32 * 255 + alpha as u32 / 2) / alpha as u32;
        assert_eq!(unpremul_red, 255);
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
        let planes = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
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
        let planes = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
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
        let planes = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
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
        let original = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
        let mut pyramid = ViewPyramid::rebuild(original);
        let view = pyramid.level_view(1).expect("a stored level");
        let held = view.data().to_vec();
        let held_id = view.storage_id();

        change_tile(&mut r, &mut g, &mut b, &mut a, 900);
        let changed = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
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
        let original = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
        let mut pyramid = ViewPyramid::rebuild(original);
        let id = pyramid.level_view(1).expect("a stored level").storage_id();

        change_tile(&mut r, &mut g, &mut b, &mut a, 900);
        let changed = Planes {
            width: 900,
            height: 600,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
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
        let planes = Planes {
            width: 300,
            height: 200,
            r: &r,
            g: &g,
            b: &b,
            a: &a,
        };
        let pyramid = ViewPyramid::rebuild(planes);
        assert_eq!(pyramid.level_count(), 1, "a small composite has one level");
        assert!(pyramid.level_view(0).is_none());
        assert!(pyramid.level_view(1).is_none());
    }
}
