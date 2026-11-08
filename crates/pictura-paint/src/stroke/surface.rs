//! Parallel builders for a stroke's layer source surface.

use crate::healing::RgbaImage;
use pictura_core::{Layer, Plane, PsdRect};
use rayon::prelude::*;

/// The R, G, B and alpha planes of `layer`, resolved once for a whole surface.
pub(crate) type LayerPlanes<'a> = [Option<&'a Plane<u8>>; 4];

pub(crate) fn layer_planes(layer: &Layer) -> LayerPlanes<'_> {
    super::plane_index(layer).map(|k| k.and_then(|k| layer.channels.get(k)).map(|c| &c.data))
}

/// Columns `x0..x0 + out.len()` of row `y` of `planes` as straight RGBA.
fn interleave(planes: LayerPlanes<'_>, y: usize, w: usize, x0: usize, out: &mut [[u8; 4]]) {
    let n = out.len();
    let at = |k: usize, missing: u8| {
        let mut span = vec![missing; n];
        if let Some(p) = planes[k] {
            p.read_rect_into(x0..x0 + n, y..y + 1, w, &mut span);
        }
        span
    };
    let (r, g, b, a) = (at(0, 0), at(1, 0), at(2, 0), at(3, 255));
    for (i, px) in out.iter_mut().enumerate() {
        *px = [r[i], g[i], b[i], a[i]];
    }
}

/// Tile edge of a [`LazyPixels`] image.
const LAZY_TILE: usize = 64;

/// A per-dab image filled a tile at a time, for an engine that reads and writes
/// only inside its dab. The image is layer-sized but zero-filled by the
/// allocator, so a stroke's start and its memory follow the area it reaches.
pub(super) struct LazyPixels {
    loaded: Vec<bool>,
    columns: usize,
}

impl LazyPixels {
    /// The empty image for `layer` and the tiles that will fill it.
    pub(super) fn new(layer: &Layer) -> (Self, RgbaImage) {
        let w = layer.rect.width().max(0) as usize;
        let h = layer.rect.height().max(0) as usize;
        let columns = w.div_ceil(LAZY_TILE);
        let lazy = LazyPixels {
            loaded: vec![false; columns * h.div_ceil(LAZY_TILE)],
            columns,
        };
        let image = RgbaImage {
            width: w as i32,
            height: h as i32,
            data: vec![[0u8; 4]; w * h],
        };
        (lazy, image)
    }

    /// Fill every tile of `image` that `rect` meets and no dab has reached, from
    /// `layer`, whose pixels there are still the ones the stroke began on.
    pub(super) fn load(&mut self, layer: &Layer, image: &mut RgbaImage, rect: PsdRect) {
        let w = image.width as usize;
        let first_column = rect.left as usize / LAZY_TILE;
        let end_column = (rect.right as usize).div_ceil(LAZY_TILE).min(self.columns);
        let first_row = rect.top as usize / LAZY_TILE;
        let end_row = (rect.bottom as usize).div_ceil(LAZY_TILE);
        let (columns, planes, loaded) = (self.columns, layer_planes(layer), &self.loaded);
        let end = (end_row * LAZY_TILE * w).min(image.data.len());
        image.data[first_row * LAZY_TILE * w..end]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(row, pixels)| {
                let y = first_row * LAZY_TILE + row;
                for column in first_column..end_column {
                    if !loaded[y / LAZY_TILE * columns + column] {
                        let x0 = column * LAZY_TILE;
                        let x1 = (x0 + LAZY_TILE).min(w);
                        interleave(planes, y, w, x0, &mut pixels[x0..x1]);
                    }
                }
            });
        for row in first_row..end_row {
            self.loaded[row * columns + first_column..row * columns + end_column].fill(true);
        }
    }
}

/// The layer's pixels as straight RGBA; a layer without alpha is opaque.
pub(crate) fn layer_rgba(layer: &Layer) -> RgbaImage {
    let (w, h) = (
        layer.rect.width().max(0) as usize,
        layer.rect.height().max(0) as usize,
    );
    let mut data = vec![[0u8; 4]; w * h];
    if w == 0 || h == 0 {
        return RgbaImage {
            width: w as i32,
            height: h as i32,
            data,
        };
    }
    let planes = layer_planes(layer);
    data.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        interleave(planes, y, w, 0, row);
    });
    RgbaImage {
        width: w as i32,
        height: h as i32,
        data,
    }
}

/// The serial reference `layer_rgba` is held to.
#[cfg(test)]
pub(crate) fn layer_rgba_serial(layer: &Layer) -> RgbaImage {
    let (w, h) = (layer.rect.width().max(0), layer.rect.height().max(0));
    let at = |id: i16, i: usize, missing: u8| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.data.as_slice().get(i))
            .copied()
            .unwrap_or(missing)
    };
    RgbaImage {
        width: w,
        height: h,
        data: (0..(w * h) as usize)
            .map(|i| [at(0, i, 0), at(1, i, 0), at(2, i, 0), at(-1, i, 255)])
            .collect(),
    }
}
