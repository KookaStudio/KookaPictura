//! A bounded-scale luma/alpha view of one layer for Auto-Align's matcher.
//!
//! Matching runs on a work grid no larger than [`MAX_SIDE`], so a PSB-sized
//! document costs a predictable amount of memory. Coordinates are layer-local
//! work pixels; [`WorkImage::to_document`] maps them back.

use pictura_core::{Document, Layer};
use pictura_filters::LensMap;

use super::super::paths::resolve_path;

/// The longest side of a work image.
pub(super) const MAX_SIDE: f64 = 640.0;

pub(super) struct WorkImage {
    pub width: usize,
    pub height: usize,
    /// Document pixels per work pixel.
    pub scale: f64,
    /// Document-space top-left of work pixel `(0, 0)`.
    pub origin: (f64, f64),
    pub luma: Vec<f32>,
    pub alpha: Vec<bool>,
}

impl WorkImage {
    /// Build the work image for `path`, sampling through `lens` when the run
    /// asked for lens correction. `None` for a missing layer with no area.
    pub fn build(doc: &Document, path: &str, lens: Option<&LensMap>) -> Option<Self> {
        let layer = resolve_path(doc, path)?;
        Self::from_layer(layer, lens)
    }

    pub fn from_layer(layer: &Layer, lens: Option<&LensMap>) -> Option<Self> {
        let w = layer.rect.width().max(0);
        let h = layer.rect.height().max(0);
        if w == 0 || h == 0 {
            return None;
        }
        let scale = (f64::from(w.max(h)) / MAX_SIDE).max(1.0);
        let width = (f64::from(w) / scale).ceil().max(1.0) as usize;
        let height = (f64::from(h) / scale).ceil().max(1.0) as usize;
        let channels: [Option<&[u8]>; 3] = [0, 1, 2].map(|id| {
            layer
                .channels
                .iter()
                .find(|c| c.id == id && c.data.len() == (w as usize * h as usize))
                .map(|c| c.data.as_slice())
        });
        let alpha = layer
            .channels
            .iter()
            .find(|c| c.id == -1 && c.data.len() == (w as usize * h as usize))
            .map(|c| c.data.as_slice());
        let color = channels[0].or(channels[1]).or(channels[2])?;
        let cw = w as usize;
        let sample = |x: f64, y: f64| -> Option<(f32, bool)> {
            let [sx, sy] = match lens {
                Some(map) => map.source([x, y])?,
                None => [x, y],
            };
            let ix = sx.floor();
            let iy = sy.floor();
            if ix < 0.0 || iy < 0.0 || ix >= w as f64 || iy >= h as f64 {
                return None;
            }
            let at = iy as usize * cw + ix as usize;
            let a = alpha.is_none_or(|plane| plane[at] > 0);
            let l = if channels[1].is_some() && channels[2].is_some() {
                let r = f32::from(channels[0].unwrap_or(color)[at]);
                let g = f32::from(channels[1].unwrap_or(color)[at]);
                let b = f32::from(channels[2].unwrap_or(color)[at]);
                (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
            } else {
                f32::from(color[at]) / 255.0
            };
            Some((l, a))
        };
        let mut luma = vec![0.0f32; width * height];
        let mut mask = vec![false; width * height];
        for y in 0..height {
            for x in 0..width {
                let p = ((x as f64 + 0.5) * scale, (y as f64 + 0.5) * scale);
                if let Some((l, a)) = sample(p.0, p.1) {
                    luma[y * width + x] = l;
                    mask[y * width + x] = a;
                }
            }
        }
        Some(Self {
            width,
            height,
            scale,
            origin: (f64::from(layer.rect.left), f64::from(layer.rect.top)),
            luma,
            alpha: mask,
        })
    }

    /// Work coordinates to document coordinates.
    pub fn to_document(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.origin.0 + (x + 0.5) * self.scale,
            self.origin.1 + (y + 0.5) * self.scale,
        )
    }

    /// Document coordinates to work coordinates.
    pub fn to_work(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.origin.0) / self.scale - 0.5,
            (y - self.origin.1) / self.scale - 0.5,
        )
    }

    pub fn at(&self, x: i32, y: i32) -> Option<(f32, bool)> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let i = y as usize * self.width + x as usize;
        Some((self.luma[i], self.alpha[i]))
    }

    /// Bilinear luma at work coordinates, with alpha from the nearest pixel.
    pub fn sample(&self, x: f64, y: f64) -> Option<(f32, bool)> {
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
        let p = |dx: i32, dy: i32| self.at(x0 as i32 + dx, y0 as i32 + dy);
        let (p00, p10, p01, p11) = (p(0, 0)?, p(1, 0)?, p(0, 1)?, p(1, 1)?);
        if !(p00.1 && p10.1 && p01.1 && p11.1) {
            return None;
        }
        let top = p00.0 * (1.0 - fx) + p10.0 * fx;
        let bottom = p01.0 * (1.0 - fx) + p11.0 * fx;
        Some((top * (1.0 - fy) + bottom * fy, true))
    }
}
