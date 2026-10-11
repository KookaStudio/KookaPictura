//! Selection and mask math for Kooka Pictura.
//!
//! M5 scope: an 8-bit selection coverage mask and the boolean/modify operations
//! masked edits depend on. Spec: `docs/08-selection/*.md`.
//!
//! Coverage is `u8` per pixel (0 = outside, 255 = fully inside). Soft masks keep
//! intermediate coverage. Algorithms the reference leaves undocumented (feather kernel,
//! feather radius->sigma, colour distance, structuring element) are inferred and
//! marked below.

use std::collections::{HashSet, VecDeque};

use pictura_core::{Channel, Layer, PixelBuffer};

mod color_range;
pub use color_range::{color_range, ColorRangeSelect};
mod contour;
pub use contour::contour;

mod magnetic;
pub use magnetic::EdgeMap;

mod refine;
pub use refine::{
    coverage_plane, decontaminate, refine, OutputTarget, RefineEdgeSettings, ViewMode,
};

mod translate;

#[derive(Debug, thiserror::Error)]
pub enum SelectError {
    #[error("size mismatch: {0}")]
    SizeMismatch(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}

/// A document-sized 8-bit selection coverage mask (0 = outside, 255 = inside).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// How a new selection combines with the existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectOp {
    Replace,
    Add,
    Subtract,
    Intersect,
}

/// Combine mode for rasterized shapes (`"new"` replaces, the rest are boolean).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineMode {
    New,
    Add,
    Subtract,
    Intersect,
}

impl From<CombineMode> for SelectOp {
    fn from(mode: CombineMode) -> Self {
        match mode {
            CombineMode::New => SelectOp::Replace,
            CombineMode::Add => SelectOp::Add,
            CombineMode::Subtract => SelectOp::Subtract,
            CombineMode::Intersect => SelectOp::Intersect,
        }
    }
}

const FEATHER_MAX: f64 = 250.0;
const MORPH_MAX: u32 = 100;
const BORDER_MAX: u32 = 200;
const SMOOTH_MAX: u32 = 100;

impl Selection {
    pub fn none(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize],
        }
    }

    pub fn all(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![255; width as usize * height as usize],
        }
    }

    /// A `w`×`h` rectangle at `(x, y)`; bounds are half-open and clipped to the
    /// canvas. `w <= 0` or `h <= 0` yields an empty selection.
    pub fn rect(width: u32, height: u32, x: i32, y: i32, w: i32, h: i32) -> Self {
        let mut sel = Selection::none(width, height);
        if w <= 0 || h <= 0 {
            return sel;
        }
        for py in 0..height as i32 {
            if py < y || py >= y.saturating_add(h) {
                continue;
            }
            for px in 0..width as i32 {
                if px >= x && px < x.saturating_add(w) {
                    sel.data[py as usize * width as usize + px as usize] = 255;
                }
            }
        }
        sel
    }

    /// The ellipse inscribed in the `w`×`h` rectangle at `(x, y)`; a pixel is
    /// inside when its centre is. Clipped to the canvas. `w <= 0` or `h <= 0`
    /// yields an empty selection.
    pub fn ellipse(width: u32, height: u32, x: i32, y: i32, w: i32, h: i32) -> Self {
        let mut sel = Selection::none(width, height);
        if w <= 0 || h <= 0 {
            return sel;
        }
        let rx = w as f64 / 2.0;
        let ry = h as f64 / 2.0;
        let cx = x as f64 + rx;
        let cy = y as f64 + ry;
        for py in 0..height {
            let dy = (py as f64 + 0.5 - cy) / ry;
            for px in 0..width {
                let dx = (px as f64 + 0.5 - cx) / rx;
                if dx * dx + dy * dy <= 1.0 {
                    sel.data[py as usize * width as usize + px as usize] = 255;
                }
            }
        }
        sel
    }

    /// Even-odd scanline fill of `points`, sampled at pixel centres and clipped
    /// to the canvas. Fewer than three points yields an empty selection.
    pub fn polygon(width: u32, height: u32, points: &[(i32, i32)]) -> Self {
        let mut sel = Selection::none(width, height);
        if points.len() < 3 {
            return sel;
        }
        let n = points.len();
        for py in 0..height {
            let cy = py as f64 + 0.5;
            for px in 0..width {
                let cx = px as f64 + 0.5;
                let mut inside = false;
                let mut j = n - 1;
                for i in 0..n {
                    let (xi, yi) = (points[i].0 as f64, points[i].1 as f64);
                    let (xj, yj) = (points[j].0 as f64, points[j].1 as f64);
                    if (yi > cy) != (yj > cy) {
                        let cross = xi + (cy - yi) / (yj - yi) * (xj - xi);
                        if cx < cross {
                            inside = !inside;
                        }
                    }
                    j = i;
                }
                if inside {
                    sel.data[py as usize * width as usize + px as usize] = 255;
                }
            }
        }
        sel
    }

    /// Export this selection as a document-level channel (8-bit grayscale
    /// coverage, one byte per pixel).
    pub fn to_channel(&self, id: i16) -> Channel {
        Channel {
            id,
            data: self.data.clone().into(),
        }
    }

    /// Interpret a document-level channel as a selection. `width`/`height` are
    /// the document dimensions the channel must match.
    pub fn from_channel(ch: &Channel, width: u32, height: u32) -> Result<Selection, SelectError> {
        let expected = width as usize * height as usize;
        if ch.data.len() != expected {
            return Err(SelectError::SizeMismatch(format!(
                "channel {} has {} bytes, expected {expected} for {width}x{height}",
                ch.id,
                ch.data.len()
            )));
        }
        Ok(Selection {
            width,
            height,
            data: ch.data.to_vec(),
        })
    }

    /// Combine `other` into `self` with `op` (same dimensions required).
    pub fn combine(&mut self, other: &Selection, op: SelectOp) -> Result<(), SelectError> {
        if self.width != other.width || self.height != other.height {
            return Err(SelectError::SizeMismatch(format!(
                "{}x{} vs {}x{}",
                self.width, self.height, other.width, other.height
            )));
        }
        if self.data.len() != other.data.len() {
            return Err(SelectError::SizeMismatch(format!(
                "coverage length {} vs {}",
                self.data.len(),
                other.data.len()
            )));
        }
        match op {
            SelectOp::Replace => self.data.copy_from_slice(&other.data),
            SelectOp::Add => {
                for (a, b) in self.data.iter_mut().zip(&other.data) {
                    *a = (*a).max(*b);
                }
            }
            SelectOp::Subtract => {
                for (a, b) in self.data.iter_mut().zip(&other.data) {
                    *a = a.saturating_sub(*b);
                }
            }
            SelectOp::Intersect => {
                for (a, b) in self.data.iter_mut().zip(&other.data) {
                    *a = (((*a as u16) * (*b as u16) + 127) / 255) as u8;
                }
            }
        }
        Ok(())
    }

    /// Combine `other` into `self` with `mode`.
    ///
    /// Same-size masks are merged in place; a mismatched `other` can only
    /// replace (`New`), since the boolean ops have no shared canvas.
    pub fn combine_with(&mut self, other: &Selection, mode: CombineMode) {
        if self.width == other.width
            && self.height == other.height
            && self.data.len() == other.data.len()
        {
            let _ = self.combine(other, mode.into());
        } else if matches!(mode, CombineMode::New) {
            *self = other.clone();
        }
    }

    pub fn invert(&self) -> Selection {
        Selection {
            width: self.width,
            height: self.height,
            data: self.data.iter().map(|v| 255 - v).collect(),
        }
    }

    /// Blur the coverage with a Gaussian. radius 0 = identity.
    // ponytail: sigma = radius/2, separable f64 convolution with clamped edges.
    // Swap in a tiled/FFT blur only if full-document feathering shows up in profiles.
    pub fn feather(&self, radius: f64) -> Selection {
        if !radius.is_finite() || radius <= 0.0 || self.data.is_empty() {
            return self.clone();
        }
        let sigma = radius.clamp(0.0, FEATHER_MAX) / 2.0;
        Selection {
            width: self.width,
            height: self.height,
            data: blur(&self.data, self.width as usize, self.height as usize, sigma),
        }
    }

    /// Dilate the coverage. radius 0 = identity. Clamped to 1..=100.
    pub fn expand(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, MORPH_MAX);
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        self.morph(r, true)
    }

    /// Erode the coverage. radius 0 = identity. Clamped to 1..=100.
    pub fn contract(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, MORPH_MAX);
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        self.morph(r, false)
    }

    /// Band centred on the current edge (roughly half in / half out).
    pub fn border(&self, width: u32) -> Selection {
        let w = width.min(BORDER_MAX);
        if w == 0 {
            return Selection::none(self.width, self.height);
        }
        let outside = w.div_ceil(2);
        let inside = w / 2;
        let dilated = self.expand(outside);
        let eroded = self.contract(inside);
        Selection {
            width: self.width,
            height: self.height,
            data: dilated
                .data
                .iter()
                .zip(&eroded.data)
                .map(|(a, b)| a.abs_diff(*b))
                .collect(),
        }
    }

    /// Median/majority smoothing over a square window. radius 0 = identity.
    pub fn smooth(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, SMOOTH_MAX) as usize;
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        Selection {
            width: self.width,
            height: self.height,
            data: median(&self.data, self.width as usize, self.height as usize, r),
        }
    }

    fn morph(&self, radius: u32, dilate: bool) -> Selection {
        let (w, h) = (self.width as usize, self.height as usize);
        let r = radius as isize;
        let pick = |a: u8, b: u8| if dilate { a.max(b) } else { a.min(b) };
        let mut tmp = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = self.data[y * w + x];
                for i in -r..=r {
                    let xx = (x as isize + i).clamp(0, w as isize - 1) as usize;
                    acc = pick(acc, self.data[y * w + xx]);
                }
                tmp[y * w + x] = acc;
            }
        }
        let mut out = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = tmp[y * w + x];
                for i in -r..=r {
                    let yy = (y as isize + i).clamp(0, h as isize - 1) as usize;
                    acc = pick(acc, tmp[yy * w + x]);
                }
                out[y * w + x] = acc;
            }
        }
        Selection {
            width: self.width,
            height: self.height,
            data: out,
        }
    }
}

fn rgb_at(img: &PixelBuffer, p: usize) -> [u8; 3] {
    let n = img.width as usize * img.height as usize;
    if img.channels >= 3 {
        [img.data[p], img.data[n + p], img.data[2 * n + p]]
    } else {
        let g = img.data[p];
        [g, g, g]
    }
}

fn chebyshev(a: [u8; 3], b: [u8; 3]) -> u8 {
    a.iter()
        .zip(&b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

fn expected_len(sel: &Selection) -> usize {
    sel.width as usize * sel.height as usize
}

fn check_dims(sel: &Selection, img: &PixelBuffer) -> Result<(), SelectError> {
    if sel.width != img.width || sel.height != img.height {
        return Err(SelectError::SizeMismatch(format!(
            "{}x{} vs image {}x{}",
            sel.width, sel.height, img.width, img.height
        )));
    }
    Ok(())
}

/// Flood/global selection by color tolerance (Magic Wand).
pub fn magic_wand(
    img: &PixelBuffer,
    x: u32,
    y: u32,
    tolerance: u8,
    contiguous: bool,
) -> Result<Selection, SelectError> {
    let (w, h) = (img.width as usize, img.height as usize);
    if w == 0 || h == 0 || x as usize >= w || y as usize >= h {
        return Err(SelectError::InvalidParams(format!(
            "point ({x},{y}) outside {w}x{h}"
        )));
    }
    let mut data = vec![0u8; w * h];
    let reference = rgb_at(img, y as usize * w + x as usize);
    if contiguous {
        let mut seen = vec![false; w * h];
        let mut queue = VecDeque::new();
        let start = y as usize * w + x as usize;
        seen[start] = true;
        data[start] = 255;
        queue.push_back(start);
        while let Some(p) = queue.pop_front() {
            let px = p % w;
            let py = p / w;
            let mut visit = |nx: usize, ny: usize, queue: &mut VecDeque<usize>| {
                let q = ny * w + nx;
                if !seen[q] && chebyshev(rgb_at(img, q), reference) <= tolerance {
                    seen[q] = true;
                    data[q] = 255;
                    queue.push_back(q);
                }
            };
            if px > 0 {
                visit(px - 1, py, &mut queue);
            }
            if px + 1 < w {
                visit(px + 1, py, &mut queue);
            }
            if py > 0 {
                visit(px, py - 1, &mut queue);
            }
            if py + 1 < h {
                visit(px, py + 1, &mut queue);
            }
        }
    } else {
        for (p, slot) in data.iter_mut().enumerate() {
            if chebyshev(rgb_at(img, p), reference) <= tolerance {
                *slot = 255;
            }
        }
    }
    Ok(Selection {
        width: img.width,
        height: img.height,
        data,
    })
}

/// Expand a selection to include adjacent similar-colored pixels.
pub fn grow(sel: &Selection, img: &PixelBuffer, tolerance: u8) -> Result<Selection, SelectError> {
    check_dims(sel, img)?;
    let (w, h) = (sel.width as usize, sel.height as usize);
    let mut out = sel.clone();
    if w == 0 || h == 0 {
        return Ok(out);
    }
    let mut seen: Vec<bool> = sel.data.iter().map(|&v| v > 0).collect();
    let mut queue: VecDeque<usize> = (0..w * h).filter(|&p| seen[p]).collect();
    while let Some(p) = queue.pop_front() {
        let pc = rgb_at(img, p);
        let px = p % w;
        let py = p / w;
        let mut visit = |nx: usize, ny: usize, out: &mut Selection, queue: &mut VecDeque<usize>| {
            let q = ny * w + nx;
            if !seen[q] && chebyshev(rgb_at(img, q), pc) <= tolerance {
                seen[q] = true;
                out.data[q] = 255;
                queue.push_back(q);
            }
        };
        if px > 0 {
            visit(px - 1, py, &mut out, &mut queue);
        }
        if px + 1 < w {
            visit(px + 1, py, &mut out, &mut queue);
        }
        if py > 0 {
            visit(px, py - 1, &mut out, &mut queue);
        }
        if py + 1 < h {
            visit(px, py + 1, &mut out, &mut queue);
        }
    }
    Ok(out)
}

/// Add all similar-colored pixels in the image to the selection.
pub fn similar(
    sel: &Selection,
    img: &PixelBuffer,
    tolerance: u8,
) -> Result<Selection, SelectError> {
    check_dims(sel, img)?;
    let mut out = sel.clone();
    let n = expected_len(sel);
    let mut references: HashSet<[u8; 3]> = HashSet::new();
    for p in 0..n {
        if sel.data[p] > 0 {
            references.insert(rgb_at(img, p));
        }
    }
    if references.is_empty() {
        return Ok(out);
    }
    for p in 0..n {
        if out.data[p] != 0 {
            continue;
        }
        let c = rgb_at(img, p);
        if references.iter().any(|r| chebyshev(c, *r) <= tolerance) {
            out.data[p] = 255;
        }
    }
    Ok(out)
}

pub(crate) fn blur(src: &[u8], w: usize, h: usize, sigma: f64) -> Vec<u8> {
    let radius = (sigma * 3.0).ceil().max(1.0) as isize;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0f64;
    for i in -radius..=radius {
        let v = (-(i * i) as f64 / (2.0 * sigma * sigma)).exp();
        kernel.push(v);
        sum += v;
    }
    for k in &mut kernel {
        *k /= sum;
    }
    let mut tmp = vec![0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f64;
            for (ki, i) in (-radius..=radius).enumerate() {
                let xx = (x as isize + i).clamp(0, w as isize - 1) as usize;
                acc += src[y * w + xx] as f64 * kernel[ki];
            }
            tmp[y * w + x] = acc;
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f64;
            for (ki, i) in (-radius..=radius).enumerate() {
                let yy = (y as isize + i).clamp(0, h as isize - 1) as usize;
                acc += tmp[yy * w + x] * kernel[ki];
            }
            out[y * w + x] = acc.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

pub(crate) fn median(src: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let r = radius as isize;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut hist = [0u32; 256];
            let mut total = 0u32;
            for dy in -r..=r {
                let yy = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                for dx in -r..=r {
                    let xx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                    hist[src[yy * w + xx] as usize] += 1;
                    total += 1;
                }
            }
            let mut acc = 0u32;
            let half = total / 2;
            let mut value = 0u8;
            for (v, count) in hist.iter().enumerate() {
                acc += *count;
                if acc > half {
                    value = v as u8;
                    break;
                }
            }
            out[y * w + x] = value;
        }
    }
    out
}

/// Build a document-sized selection from `layer`'s alpha channel (`-1`),
/// offset by `layer.rect`. A pixel outside the layer's rect has coverage 0; a
/// layer with no alpha channel is fully opaque (`255`) inside its rect. Reuses
/// [`Selection::from_channel`] over a synthesized document-sized channel.
pub fn selection_from_layer_alpha(layer: &Layer, width: u32, height: u32) -> Selection {
    let mut data = vec![0u8; width as usize * height as usize];
    let rect = layer.rect;
    let alpha = layer
        .channels
        .iter()
        .find(|channel| channel.id == -1)
        .map(|channel| channel.data.as_slice());
    let lw = rect.width();
    let lh = rect.height();
    for dy in 0..lh {
        let py = rect.top + dy;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for dx in 0..lw {
            let px = rect.left + dx;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let local = dy as usize * lw as usize + dx as usize;
            let coverage = match alpha {
                Some(bytes) if local < bytes.len() => bytes[local],
                Some(_) => 0,
                None => 255,
            };
            data[py as usize * width as usize + px as usize] = coverage;
        }
    }
    let channel = Channel {
        id: -1,
        data: data.into(),
    };
    Selection::from_channel(&channel, width, height).expect("document-sized selection channel")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel_from(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> Selection {
        let mut s = Selection::none(w, h);
        for y in 0..h {
            for x in 0..w {
                s.data[(y * w + x) as usize] = f(x, y);
            }
        }
        s
    }

    fn rgb_image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> PixelBuffer {
        let n = w as usize * h as usize;
        let mut img = PixelBuffer::new(w, h, 3);
        for y in 0..h {
            for x in 0..w {
                let p = (y * w + x) as usize;
                let c = f(x, y);
                img.data[p] = c[0];
                img.data[n + p] = c[1];
                img.data[2 * n + p] = c[2];
            }
        }
        img
    }

    fn rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Selection {
        sel_from(w, h, |x, y| {
            if x >= x0 && x < x1 && y >= y0 && y < y1 {
                255
            } else {
                0
            }
        })
    }

    #[test]
    fn combine_identities() {
        let a = rect(6, 5, 1, 1, 5, 4);
        let none = Selection::none(6, 5);

        let mut t = a.clone();
        t.combine(&none, SelectOp::Add).unwrap();
        assert_eq!(t, a, "A Add none = A");

        let mut t = a.clone();
        t.combine(&none, SelectOp::Intersect).unwrap();
        assert_eq!(t, none, "A Intersect none = none");

        let mut t = a.clone();
        t.combine(&a, SelectOp::Subtract).unwrap();
        assert_eq!(t, none, "A Subtract A = none");

        let mut t = a.clone();
        t.combine(&a, SelectOp::Intersect).unwrap();
        assert_eq!(t, a, "A Intersect A = A (binary)");

        let mut t = Selection::all(6, 5);
        t.combine(&a, SelectOp::Replace).unwrap();
        assert_eq!(t, a, "Replace = b");
    }

    #[test]
    fn combine_soft_bounds() {
        let a = sel_from(8, 8, |x, _| (x * 17) as u8);
        let b = sel_from(8, 8, |_, y| (y * 23) as u8);

        let mut add = a.clone();
        add.combine(&b, SelectOp::Add).unwrap();
        for p in 0..64 {
            assert_eq!(add.data[p], a.data[p].max(b.data[p]));
            assert!(add.data[p] >= a.data[p] && add.data[p] >= b.data[p]);
        }

        let mut sub = a.clone();
        sub.combine(&b, SelectOp::Subtract).unwrap();
        for p in 0..64 {
            assert_eq!(sub.data[p], a.data[p].saturating_sub(b.data[p]));
            assert!(sub.data[p] <= a.data[p]);
        }

        let mut inter = a.clone();
        inter.combine(&b, SelectOp::Intersect).unwrap();
        for p in 0..64 {
            assert!(inter.data[p] <= a.data[p].min(b.data[p]) + 1);
        }
    }

    #[test]
    fn combine_size_mismatch_errors() {
        let mut a = Selection::none(4, 4);
        let b = Selection::none(5, 4);
        assert!(a.combine(&b, SelectOp::Add).is_err());
    }

    #[test]
    fn rect_selects_half_open_and_clips() {
        let s = Selection::rect(6, 5, 1, 1, 3, 2);
        assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 6);
        assert_eq!(s.data[7], 255);
        assert_eq!(s.data[(2 * 6 + 3) as usize], 255);
        assert_eq!(s.data[10], 0, "right edge is exclusive");
        assert_eq!(s.data[1], 0, "above the rect");
        assert_eq!(s.data[(3 * 6 + 1) as usize], 0, "below the rect");

        assert_eq!(
            Selection::rect(6, 5, 0, 0, 0, 2),
            Selection::none(6, 5),
            "zero width is empty"
        );
        let clipped = Selection::rect(6, 5, -2, -2, 4, 4);
        assert_eq!(clipped.data.iter().filter(|&&v| v > 0).count(), 4);
        assert_eq!(clipped.data[0], 255);
        assert_eq!(clipped.data[7], 255);
        assert_eq!(clipped.data[8], 0);
    }

    #[test]
    fn rect_extreme_coords_do_not_overflow() {
        let s = Selection::rect(4, 4, 1, 1, i32::MAX, i32::MAX);
        assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 9);
    }

    #[test]
    fn ellipse_is_inscribed_and_clipped() {
        let s = Selection::ellipse(4, 4, 0, 0, 4, 4);
        assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 12);
        assert_eq!(s.data[0], 0, "top-left corner is outside");
        assert_eq!(s.data[(3 * 4 + 3) as usize], 0, "bottom-right corner out");
        assert_eq!(s.data[5], 255, "centre is inside");
        assert_eq!(s.data[1], 255, "top edge midpoint in");
        assert_eq!(s.data[(3 * 4 + 2) as usize], 255, "bottom edge midpoint in");

        assert_eq!(Selection::ellipse(4, 4, 0, 0, 0, 4), Selection::none(4, 4));
    }

    #[test]
    fn polygon_even_odd_fill_and_degenerate() {
        let square = Selection::polygon(4, 4, &[(0, 0), (4, 0), (4, 4), (0, 4)]);
        assert_eq!(square.data.iter().filter(|&&v| v > 0).count(), 16);

        let triangle = Selection::polygon(4, 4, &[(0, 0), (4, 0), (0, 4)]);
        assert_eq!(triangle.data.iter().filter(|&&v| v > 0).count(), 6);
        assert_eq!(triangle.data[0], 255);
        assert_eq!(triangle.data[7], 0);
        assert_eq!(triangle.data[(3 * 4 + 3) as usize], 0);

        assert_eq!(
            Selection::polygon(4, 4, &[(0, 0), (1, 1)]),
            Selection::none(4, 4),
            "fewer than three points is empty"
        );

        let over = Selection::polygon(4, 4, &[(-2, -2), (6, -2), (6, 6), (-2, 6)]);
        assert_eq!(over.data.iter().filter(|&&v| v > 0).count(), 16);
    }

    #[test]
    fn combine_with_each_mode() {
        let a = Selection::rect(4, 4, 0, 0, 3, 3);
        let b = Selection::rect(4, 4, 1, 1, 3, 3);
        let count = |s: &Selection| s.data.iter().filter(|&&v| v > 0).count();

        let mut new = a.clone();
        new.combine_with(&b, CombineMode::New);
        assert_eq!(count(&new), 9, "New replaces");
        assert_eq!(new, b);

        let mut add = a.clone();
        add.combine_with(&b, CombineMode::Add);
        assert_eq!(count(&add), 14, "Add is union");

        let mut sub = a.clone();
        sub.combine_with(&b, CombineMode::Subtract);
        assert_eq!(count(&sub), 5, "Subtract removes the overlap");

        let mut inter = a.clone();
        inter.combine_with(&b, CombineMode::Intersect);
        assert_eq!(count(&inter), 4, "Intersect is the overlap");
        assert_eq!(inter.data[5], 255);
        assert_eq!(inter.data[10], 255);
        assert_eq!(inter.data[0], 0);
    }

    #[test]
    fn channel_round_trips_a_selection() {
        let sel = rect(5, 3, 1, 1, 4, 2);
        let ch = sel.to_channel(-4);
        assert_eq!(ch.id, -4);
        assert_eq!(ch.data, sel.data);
        assert_eq!(Selection::from_channel(&ch, 5, 3).unwrap(), sel);
    }

    #[test]
    fn from_channel_wrong_length_errors() {
        let ch = Channel {
            id: 0,
            data: vec![0; 5].into(),
        };
        assert!(Selection::from_channel(&ch, 4, 4).is_err());
    }

    #[test]
    fn layer_alpha_selection_offsets_and_fills() {
        use pictura_core::{Layer, PsdRect};
        let mut layer = Layer {
            rect: PsdRect {
                top: 1,
                left: 2,
                bottom: 3,
                right: 5,
            }, // 3x2
            ..Default::default()
        };
        // No alpha channel -> fully opaque inside the rect, zero outside.
        let at = |x: usize, y: usize| y * 6 + x;
        let sel = selection_from_layer_alpha(&layer, 6, 4);
        assert_eq!(sel.data.iter().filter(|&&v| v == 255).count(), 6);
        assert_eq!(sel.data[at(2, 1)], 255);
        assert_eq!(sel.data[at(2, 0)], 0, "outside the rect");
        assert_eq!(sel.data[at(5, 1)], 0, "right edge exclusive");

        // Alpha channel -> layer-local coverage copied to the document offset.
        layer.channels = vec![Channel {
            id: -1,
            data: vec![0, 64, 255, 128, 0, 255].into(),
        }];
        let sel = selection_from_layer_alpha(&layer, 6, 4);
        assert_eq!(sel.data[at(2, 1)], 0);
        assert_eq!(sel.data[at(3, 1)], 64);
        assert_eq!(sel.data[at(4, 1)], 255);
        assert_eq!(sel.data[at(2, 2)], 128);
        assert_eq!(sel.data[at(3, 2)], 0);
        assert_eq!(sel.data[at(4, 2)], 255);
    }

    #[test]
    fn invert_involution() {
        let a = sel_from(7, 3, |x, y| ((x * 31 + y * 7) % 256) as u8);
        assert_eq!(a.invert().invert(), a);
        assert_eq!(Selection::none(4, 2).invert(), Selection::all(4, 2));
        assert_eq!(Selection::all(4, 2).invert(), Selection::none(4, 2));
    }

    #[test]
    fn feather_zero_is_identity() {
        let a = rect(9, 5, 2, 1, 7, 4);
        assert_eq!(a.feather(0.0), a);
        assert_eq!(a.feather(-3.0), a);
    }

    #[test]
    fn feather_makes_ramp() {
        let a = rect(16, 8, 0, 0, 8, 8);
        let f = a.feather(4.0);
        assert_ne!(f, a);
        let has_partial = f.data.iter().any(|&v| v > 0 && v < 255);
        assert!(has_partial, "hard edge should soften into a ramp");
        for y in 0..8 {
            for x in 1..16 {
                assert!(
                    f.data[(y * 16 + x) as usize] <= f.data[(y * 16 + x - 1) as usize],
                    "coverage must fall monotonically across the edge"
                );
            }
        }
    }

    #[test]
    fn expand_contract_roundtrip() {
        let a = rect(24, 20, 5, 4, 16, 15);
        assert_eq!(a.expand(0), a);
        assert_eq!(a.contract(0), a);
        assert_eq!(a.expand(3).contract(3), a);
    }

    #[test]
    fn border_is_centered_band() {
        let a = rect(24, 24, 8, 8, 16, 16);
        let b = a.border(6);
        assert_eq!(b.data[(12 * 24 + 12) as usize], 0, "interior stays clear");
        assert_eq!(b.data[(8 * 24 + 12) as usize], 255, "edge is selected");
        assert_eq!(b.data[(6 * 24 + 12) as usize], 255, "band extends outside");
        assert_eq!(b.data[(4 * 24 + 12) as usize], 0, "far outside stays clear");
        assert_eq!(b.data[(10 * 24 + 12) as usize], 255, "band extends inside");
        assert_eq!(Selection::none(4, 4).border(5).data.len(), 16);
    }

    #[test]
    fn smooth_removes_isolated_and_keeps_majority() {
        let mut isolated = Selection::none(9, 9);
        isolated.data[(4 * 9 + 4) as usize] = 255;
        let smoothed = isolated.smooth(1);
        assert_eq!(
            smoothed.data[(4 * 9 + 4) as usize],
            0,
            "isolated pixel removed"
        );

        let mut full = Selection::all(11, 11);
        full.data[(5 * 11 + 5) as usize] = 0;
        let filled = full.smooth(1);
        assert_eq!(filled.data[(5 * 11 + 5) as usize], 255, "lone hole filled");
        assert_eq!(filled.data[(3 * 11 + 3) as usize], 255, "interior kept");

        let block = rect(9, 9, 2, 2, 7, 7);
        assert_eq!(block.smooth(1).data[(4 * 9 + 4) as usize], 255);
        assert_eq!(block.smooth(0), block);
    }

    #[test]
    fn magic_wand_contiguous_and_global() {
        let img = rgb_image(8, 8, |x, _| if x < 4 { [255, 0, 0] } else { [0, 0, 255] });
        let red = magic_wand(&img, 1, 1, 10, true).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                let expected = if x < 4 { 255 } else { 0 };
                assert_eq!(red.data[(y * 8 + x) as usize], expected);
            }
        }
        let blue = magic_wand(&img, 6, 6, 10, false).unwrap();
        assert_eq!(blue.data[0], 0);
        assert_eq!(blue.data[(3 * 8 + 6) as usize], 255);
    }

    #[test]
    fn magic_wand_rejects_tolerance_and_bounds() {
        let img = rgb_image(4, 4, |_, _| [100, 100, 100]);
        let picky = magic_wand(&img, 0, 0, 5, true).unwrap();
        assert_eq!(picky, Selection::all(4, 4));

        let img2 = rgb_image(4, 4, |x, _| {
            if x == 0 {
                [100, 100, 100]
            } else {
                [140, 100, 100]
            }
        });
        let strict = magic_wand(&img2, 0, 0, 10, true).unwrap();
        assert_eq!(strict.data[0], 255);
        assert_eq!(strict.data[1], 0, "colour outside tolerance is rejected");

        assert!(magic_wand(&img, 9, 9, 0, true).is_err());
    }

    #[test]
    fn grow_is_contiguous_similar_is_global() {
        let img = rgb_image(12, 4, |x, _| {
            if x < 3 || (8..11).contains(&x) {
                [10, 200, 10]
            } else {
                [200, 10, 10]
            }
        });
        let seed = sel_from(12, 4, |x, y| if x == 1 && y == 2 { 255 } else { 0 });

        let grown = grow(&seed, &img, 20).unwrap();
        assert_eq!(grown.data[0], 255, "grows within first patch");
        assert_eq!(grown.data[3], 0, "stops at different colour");
        assert_eq!(grown.data[9], 0, "does not jump globally");

        let sim = similar(&seed, &img, 20).unwrap();
        assert_eq!(sim.data[9], 255, "similar reaches far patch");
        assert_eq!(sim.data[4], 0, "similar rejects other colour");
    }

    #[test]
    fn grow_similar_size_mismatch_errors() {
        let img = rgb_image(4, 4, |_, _| [0, 0, 0]);
        let sel = Selection::none(5, 4);
        assert!(grow(&sel, &img, 0).is_err());
        assert!(similar(&sel, &img, 0).is_err());
    }

    #[test]
    fn parameters_are_clamped_not_panicking() {
        let a = rect(8, 8, 1, 1, 7, 7);
        assert_eq!(a.expand(10_000).data.len(), 64);
        assert_eq!(a.contract(10_000).data.len(), 64);
        assert_eq!(a.border(10_000).data.len(), 64);
        assert_eq!(a.smooth(10_000).data.len(), 64);
        assert_eq!(a.feather(f64::MAX).data.len(), 64);
        assert_eq!(a.feather(1e9).data.len(), 64);
    }
}
