//! Parallel builders for a stroke's layer source surface.

use crate::healing::RgbaImage;
use pictura_core::{Layer, Plane};
use rayon::prelude::*;

/// The R, G, B and alpha planes of `layer`, resolved once for a whole surface.
pub(crate) type LayerPlanes<'a> = [Option<&'a Plane<u8>>; 4];

pub(crate) fn layer_planes(layer: &Layer) -> LayerPlanes<'_> {
    super::plane_index(layer).map(|k| k.and_then(|k| layer.channels.get(k)).map(|c| &c.data))
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
        let r = planes[0].and_then(|p| p.row(y, w)).unwrap_or(&[]);
        let g = planes[1].and_then(|p| p.row(y, w)).unwrap_or(&[]);
        let b = planes[2].and_then(|p| p.row(y, w)).unwrap_or(&[]);
        let a = planes[3].and_then(|p| p.row(y, w)).unwrap_or(&[]);
        for (x, px) in row.iter_mut().enumerate() {
            *px = [
                r.get(x).copied().unwrap_or(0),
                g.get(x).copied().unwrap_or(0),
                b.get(x).copied().unwrap_or(0),
                a.get(x).copied().unwrap_or(255),
            ];
        }
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
