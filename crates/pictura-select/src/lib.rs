//! Selection and mask math for Kooka Pictura.
//!
//! M5 scope: an 8-bit selection coverage mask and the boolean/modify operations
//! masked edits depend on. Spec: `docs/08-selection/*.md`.
//!
//! Coverage is `u8` per pixel (0 = outside, 255 = fully inside). Soft masks keep
//! intermediate coverage. Algorithms the reference leaves undocumented (feather kernel,
//! feather radius->sigma, colour distance, structuring element) are inferred and
//! marked below.

use pictura_core::mask_ops::{self, Border};
use pictura_core::{Channel, Layer, LayerMask, PixelBuffer, Plane, PsdRect};

mod color_range;
pub use color_range::{color_range, ColorRangeSelect};
mod contour;
pub use contour::{contour, contour_within};

mod blur;

mod magnetic;
pub use magnetic::EdgeMap;

mod matcher;

mod refine;
pub use refine::{
    coverage_plane, decontaminate, refine, OutputTarget, RefineEdgeSettings, ViewMode,
};

mod translate;

mod bounds;

mod polygon;

mod bounded;
pub mod quick_mask;
mod tiled;
mod wand;
pub use wand::{grow, magic_wand, similar};

#[derive(Debug, thiserror::Error)]
pub enum SelectError {
    #[error("size mismatch: {0}")]
    SizeMismatch(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}

/// A document-sized 8-bit selection coverage mask (0 = outside, 255 = inside),
/// stored as a tiled mask (#1214).
///
/// ponytail: the selection operations, the fills, the wand and Quick Mask read
/// `data` tile by tile; the other consumers (adjustments, clipboard, mask
/// edits, healing, filters) still read it as one slice, a counted
/// materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub width: u32,
    pub height: u32,
    pub data: Plane<u8>,
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
    /// An empty mask: every tile `Empty`, nothing allocated (#1214).
    pub fn none(width: u32, height: u32) -> Self {
        Self::of(
            width,
            height,
            mask_ops::empty_mask(width as usize, height as usize),
        )
    }

    /// Everything selected: every tile `Uniform(255)`, nothing allocated.
    pub fn all(width: u32, height: u32) -> Self {
        Self::of(
            width,
            height,
            Plane::filled(width as usize, height as usize, 1, 255),
        )
    }

    fn of(width: u32, height: u32, data: Plane<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    fn size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    /// A `w`×`h` rectangle at `(x, y)`; bounds are half-open and clipped to the
    /// canvas. `w <= 0` or `h <= 0` yields an empty selection.
    pub fn rect(width: u32, height: u32, x: i32, y: i32, w: i32, h: i32) -> Self {
        let mut sel = Selection::none(width, height);
        if w <= 0 || h <= 0 {
            return sel;
        }
        let clip = |lo: i32, len: i32, max: u32| {
            let max = max as i64;
            let (lo, hi) = (lo as i64, lo as i64 + len as i64);
            (lo.clamp(0, max) as usize, hi.clamp(0, max) as usize)
        };
        let (x0, x1) = clip(x, w, width);
        let (y0, y1) = clip(y, h, height);
        if x0 < x1 {
            sel.data = tiled::rasterize(width as usize, height as usize, y0..y1, |_, runs| {
                runs.push((x0, x1))
            })
            .0;
        }
        sel
    }

    /// The ellipse inscribed in the `w`×`h` rectangle at `(x, y)`; a pixel is
    /// inside when its centre is. Clipped to the canvas. `w <= 0` or `h <= 0`
    /// yields an empty selection.
    pub fn ellipse(width: u32, height: u32, x: i32, y: i32, w: i32, h: i32) -> Self {
        let sel = Selection::none(width, height);
        if w <= 0 || h <= 0 {
            return sel;
        }
        let rx = w as f64 / 2.0;
        let ry = h as f64 / 2.0;
        let cx = x as f64 + rx;
        let cy = y as f64 + ry;
        // No pixel centre outside the bounding box is inside, and each row's
        // inside pixels are one run: estimate its ends in closed form, then
        // settle them with the exact per-pixel test.
        let clip = |lo: i32, len: i32, max: u32| {
            let (lo, hi) = (lo as i64, lo as i64 + len as i64);
            (lo.clamp(0, max as i64), hi.clamp(0, max as i64))
        };
        let (xs, xe) = clip(x, w, width);
        let (ys, ye) = clip(y, h, height);
        let rows = ys as usize..ye as usize;
        let (data, _) = tiled::rasterize(width as usize, height as usize, rows, |py, runs| {
            let py = py as i64;
            let dy = (py as f64 + 0.5 - cy) / ry;
            let inside = |px: i64| {
                let dx = (px as f64 + 0.5 - cx) / rx;
                dx * dx + dy * dy <= 1.0
            };
            let half = rx * (1.0 - dy * dy).max(0.0).sqrt();
            let mut left = ((cx - half - 0.5).ceil() as i64).clamp(xs, xe);
            while left > xs && inside(left - 1) {
                left -= 1;
            }
            while left < xe && !inside(left) {
                left += 1;
            }
            let mut right = ((cx + half - 0.5).floor() as i64 + 1).clamp(left, xe);
            while right < xe && inside(right) {
                right += 1;
            }
            while right > left && !inside(right - 1) {
                right -= 1;
            }
            if left < right {
                runs.push((left as usize, right as usize));
            }
        });
        Self::of(width, height, data)
    }

    /// Even-odd fill of `points`, sampled at pixel centres and clipped to the
    /// canvas. Fewer than three points yields an empty selection.
    pub fn polygon(width: u32, height: u32, points: &[(i32, i32)]) -> Self {
        Self::polygon_bounded(width, height, points).0
    }

    /// [`Selection::polygon`] and the tight bounds of its covered pixels, which
    /// the fill knows for free: a caller can skip the [`Selection::bounds`]
    /// scan of the whole mask.
    pub fn polygon_bounded(
        width: u32,
        height: u32,
        points: &[(i32, i32)],
    ) -> (Self, Option<PsdRect>) {
        let (data, bounds) = polygon::fill(width, height, points);
        (Self::of(width, height, data), bounds)
    }

    /// Export this selection as a document-level channel (8-bit grayscale
    /// coverage, one byte per pixel).
    pub fn to_channel(&self, id: i16) -> Channel {
        Channel {
            id,
            data: self.data.clone(),
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
            data: ch.data.clone(),
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
        let (w, h) = self.size();
        self.data = match op {
            SelectOp::Replace => tiled::tiled_view(&other.data, w, h).into_owned(),
            SelectOp::Add => tiled::zip_tiles(&self.data, &other.data, w, h, u8::max),
            SelectOp::Subtract => {
                tiled::zip_tiles(&self.data, &other.data, w, h, u8::saturating_sub)
            }
            SelectOp::Intersect => tiled::zip_tiles(&self.data, &other.data, w, h, |a, b| {
                (((a as u16) * (b as u16) + 127) / 255) as u8
            }),
        };
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
        let (w, h) = self.size();
        Self::of(
            self.width,
            self.height,
            tiled::map_tiles(&self.data, w, h, |v| 255 - v),
        )
    }

    /// Blur the coverage with a Gaussian. radius 0 = identity.
    ///
    /// `sigma = radius / 2`, separable with clamped edges. A wide kernel
    /// (`radius >= 4`) switches to the variance-matched box stack in [`blur`],
    /// so a large feather costs `O(pixels)` rather than `O(pixels * radius)`.
    pub fn feather(&self, radius: f64) -> Selection {
        if !radius.is_finite() || radius <= 0.0 || self.data.is_empty() {
            return self.clone();
        }
        let sigma = radius.clamp(0.0, FEATHER_MAX) / 2.0;
        let (w, h) = self.size();
        let pad = blur::reach(sigma) + 1;
        let data = mask_ops::map_bands(&self.data, w, h, pad, |s, w, h| blur::blur(s, w, h, sigma));
        Self::of(self.width, self.height, data)
    }

    /// A morphology or median kernel reading at most `r` pixels away, run in
    /// bands of tiles.
    fn banded(&self, r: usize, f: impl Fn(&[u8], usize, usize) -> Vec<u8> + Sync) -> Selection {
        let (w, h) = self.size();
        Self::of(
            self.width,
            self.height,
            mask_ops::map_bands(&self.data, w, h, r + 1, f),
        )
    }

    /// Dilate the coverage. radius 0 = identity. Clamped to 1..=100.
    pub fn expand(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, MORPH_MAX) as usize;
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        self.banded(r, |s, w, h| mask_ops::dilate(s, w, h, r, Border::Zero(0u8)))
    }

    /// Erode the coverage. radius 0 = identity. Clamped to 1..=100.
    pub fn contract(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, MORPH_MAX) as usize;
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        self.banded(r, |s, w, h| mask_ops::erode(s, w, h, r, Border::Zero(0u8)))
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
        let (w, h) = self.size();
        Self::of(
            self.width,
            self.height,
            tiled::zip_tiles(&dilated.data, &eroded.data, w, h, u8::abs_diff),
        )
    }

    /// Median/majority smoothing over a square window. radius 0 = identity.
    pub fn smooth(&self, radius: u32) -> Selection {
        let r = radius.clamp(0, SMOOTH_MAX) as usize;
        if r == 0 || self.data.is_empty() {
            return self.clone();
        }
        self.banded(r, |s, w, h| mask_ops::median(s, w, h, r))
    }
}

fn rgb_at(img: &PixelBuffer, p: usize) -> [u8; 3] {
    let n = img.width as usize * img.height as usize;
    let at = |i: usize| img.data.sample(i).unwrap_or(0);
    if img.channels >= 3 {
        [at(p), at(n + p), at(2 * n + p)]
    } else {
        let g = at(p);
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

/// A selection over a document-sized channel built flat, stored as tiles.
///
/// ponytail: the channel is assembled contiguously before it is tiled; the
/// layer-alpha and mask loads are not on a bounded path (#1214).
fn tiled_selection(channel: &Channel, width: u32, height: u32) -> Selection {
    let mut sel =
        Selection::from_channel(channel, width, height).expect("document-sized selection channel");
    sel.data.tile(width as usize, height as usize);
    sel
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
    tiled_selection(&channel, width, height)
}

/// Build a document-sized selection from a raster `mask`'s coverage. Follows the
/// compositor's mask rule (`pictura_render::composite::mask_value`): the stored
/// coverage inside the mask's rect, `default_color` outside it, and fully
/// selected (`255`) when the mask is disabled or carries no decoded pixels.
/// White (non-zero) means selected, matching the layer-mask convention.
pub fn selection_from_mask(mask: &LayerMask, width: u32, height: u32) -> Selection {
    let mut data = vec![0u8; width as usize * height as usize];
    let full = mask.disabled || mask.data.is_none();
    if !full {
        let mw = mask.rect.width();
        let mh = mask.rect.height();
        let pixels = mask.data.as_deref().unwrap_or(&[]);
        for y in 0..height as i32 {
            let my = y - mask.rect.top;
            for x in 0..width as i32 {
                let mx = x - mask.rect.left;
                let value = if mw > 0 && mh > 0 && mx >= 0 && my >= 0 && mx < mw && my < mh {
                    pixels
                        .get(my as usize * mw as usize + mx as usize)
                        .copied()
                        .unwrap_or(mask.default_color)
                } else {
                    mask.default_color
                };
                data[y as usize * width as usize + x as usize] = value;
            }
        }
    } else {
        data.fill(255);
    }
    let channel = Channel {
        id: -1,
        data: data.into(),
    };
    tiled_selection(&channel, width, height)
}

#[cfg(test)]
mod tests;
