//! Running a heal on one pixel layer of a document.

use super::RgbaImage;
use crate::stroke::{layer_at, layer_at_mut, parse_layer_path};
use pictura_core::{layer_pixel_locked, layer_transparency_locked, Document, PsdRect};

#[derive(Debug, PartialEq, Eq)]
pub enum HealError {
    /// The path is not one pixel layer with pixels.
    NoRasterLayer,
    /// The layer's pixels are locked.
    Locked,
}

/// The document rectangle of the pixel layer at `path`, when it is one.
pub fn layer_rect(doc: &Document, path: &str) -> Option<PsdRect> {
    let indices = parse_layer_path(path)?;
    Some(layer_at(doc, &indices)?.rect)
}

/// Run `op` on the pixel layer at `path` (a panel path such as `"0"`). `region`
/// and `coverage` are in document pixels; `op` receives the layer's pixels and
/// the region in window coordinates, and returns what it changed. Only `window`
/// (clipped to the layer) is materialized, so a small heal costs its own box,
/// not the document. A transparency lock keeps every pixel's alpha. Returns the
/// document rectangle changed, or `None` when nothing was.
pub fn heal_layer(
    doc: &mut Document,
    path: &str,
    region: PsdRect,
    coverage: &[f32],
    window: Option<PsdRect>,
    op: impl FnOnce(&mut RgbaImage, PsdRect, &[f32]) -> Option<PsdRect>,
) -> Result<Option<PsdRect>, HealError> {
    let indices = parse_layer_path(path).ok_or(HealError::NoRasterLayer)?;
    let layer = layer_at_mut(doc, &indices).ok_or(HealError::NoRasterLayer)?;
    if layer.is_group || layer.adjustment.is_some() {
        return Err(HealError::NoRasterLayer);
    }
    if layer_pixel_locked(layer) {
        return Err(HealError::Locked);
    }
    let rect = layer.rect;
    let (w, h) = (rect.width(), rect.height());
    let len = (w.max(0) * h.max(0)) as usize;
    let plane = |id: i16| {
        layer
            .channels
            .iter()
            .position(|c| c.id == id && c.data.len() == len)
    };
    let (Some(r), Some(g), Some(b)) = (plane(0), plane(1), plane(2)) else {
        return Err(HealError::NoRasterLayer);
    };
    let alpha = plane(-1);
    let keep_alpha = layer_transparency_locked(layer);
    let Some(window) = window.unwrap_or(rect).intersect(rect) else {
        return Ok(None);
    };
    let (ww, wh) = (window.width() as usize, window.height() as usize);
    let xs = (window.left - rect.left) as usize..(window.right - rect.left) as usize;
    let ys = (window.top - rect.top) as usize..(window.bottom - rect.top) as usize;
    let mut planes = vec![vec![0u8; ww * wh]; if alpha.is_some() { 4 } else { 3 }];
    for (plane, buf) in [r, g, b].into_iter().zip(planes.iter_mut()) {
        layer.channels[plane]
            .data
            .read_rect_into(xs.clone(), ys.clone(), w as usize, buf);
    }
    if let (Some(a), Some(buf)) = (alpha, planes.get_mut(3)) {
        layer.channels[a]
            .data
            .read_rect_into(xs.clone(), ys.clone(), w as usize, buf);
    }
    let mut data = vec![[0u8; 4]; ww * wh];
    for (i, px) in data.iter_mut().enumerate() {
        *px = [
            planes[0][i],
            planes[1][i],
            planes[2][i],
            planes.get(3).map_or(255, |a| a[i]),
        ];
    }
    let mut img = RgbaImage {
        width: ww as i32,
        height: wh as i32,
        data,
    };
    let local = PsdRect {
        top: region.top - window.top,
        left: region.left - window.left,
        bottom: region.bottom - window.top,
        right: region.right - window.left,
    };
    let Some(dirty) = op(&mut img, local, coverage) else {
        return Ok(None);
    };
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: wh as i32,
        right: ww as i32,
    };
    let Some(dirty) = dirty.intersect(canvas) else {
        return Ok(None);
    };
    let xs = dirty.left as usize..dirty.right as usize;
    let ys = dirty.top as usize..dirty.bottom as usize;
    let (stride, rows) = (xs.len(), ys.len());
    let mut buf = vec![0u8; stride * rows];
    for (c, plane) in [r, g, b].into_iter().enumerate() {
        for (dy, y) in ys.clone().enumerate() {
            for (dx, x) in xs.clone().enumerate() {
                buf[dy * stride + dx] = img.data[y * ww + x][c];
            }
        }
        let lx = (window.left - rect.left) as usize + xs.start;
        let ly = (window.top - rect.top) as usize + ys.start;
        layer.channels[plane].data.write_rect_from(
            lx..lx + stride,
            ly..ly + rows,
            w as usize,
            &buf,
        );
    }
    if let (Some(a), false) = (alpha, keep_alpha) {
        for (dy, y) in ys.clone().enumerate() {
            for (dx, x) in xs.clone().enumerate() {
                buf[dy * stride + dx] = img.data[y * ww + x][3];
            }
        }
        let lx = (window.left - rect.left) as usize + xs.start;
        let ly = (window.top - rect.top) as usize + ys.start;
        layer.channels[a]
            .data
            .write_rect_from(lx..lx + stride, ly..ly + rows, w as usize, &buf);
    }
    Ok(Some(PsdRect {
        top: dirty.top + window.top,
        left: dirty.left + window.left,
        bottom: dirty.bottom + window.top,
        right: dirty.right + window.left,
    }))
}
