//! A reduced copy of part of a document for interactive filter previews.
//!
//! The Filter Gallery shows its stack on a [`FilterProxy`]: the visible region
//! of the document, box-averaged down to about the pane's device pixels, so a
//! preview costs what the pane shows rather than what the document holds. The
//! proxy is never committed; OK filters the full-resolution layer.
//!
//! ponytail: only pixel planes and raster masks are reduced. Live type, vector
//! masks, smart-object sources and layer-effect sizes still render at document
//! scale inside the proxy, so a preview over such layers is approximate beyond
//! the filter itself; scale them too if a gallery preview over one matters.

use pictura_core::{Document, Layer, LayerMask, PixelBuffer, PsdRect};
use pictura_filters::{Filter, FilterError};
use rayon::prelude::*;

use crate::document_ops::for_each_layer;

pub struct FilterProxy {
    doc: Document,
    selection: Option<LayerMask>,
}

impl FilterProxy {
    /// The part of `doc` inside `region` (clamped to the canvas), scaled by
    /// `scale` (clamped to `(0, 1]`), with `selection`, a canvas-sized mask,
    /// reduced alongside. `None` for an empty region.
    pub fn new(
        doc: &Document,
        selection: Option<&LayerMask>,
        region: PsdRect,
        scale: f64,
    ) -> Option<Self> {
        let region = intersect(
            region,
            PsdRect {
                top: 0,
                left: 0,
                bottom: doc.height as i32,
                right: doc.width as i32,
            },
        )?;
        let scale = if scale.is_finite() && scale > 0.0 {
            scale.min(1.0)
        } else {
            1.0
        };
        let map = Map { region, scale };
        let full = map.rect(region);
        let mut proxy = doc.clone();
        proxy.width = full.width() as u32;
        proxy.height = full.height() as u32;
        proxy.source_planes = None;
        proxy.channels.clear();
        for_each_layer(&mut proxy.layers, &mut |layer| reduce_layer(layer, &map));
        // A layerless document composites from its flat image.
        proxy.composite = reduce_flat(&doc.composite, &map, full);
        let selection = selection.map(|mask| reduce_mask(mask, &map));
        Some(Self {
            doc: proxy,
            selection,
        })
    }

    pub fn width(&self) -> u32 {
        self.doc.width
    }

    pub fn height(&self) -> u32 {
        self.doc.height
    }

    /// The proxy composited with `filters` applied in order to the layer at
    /// panel path `layer`, gated by the reduced selection. An empty stack
    /// composites the proxy unfiltered.
    pub fn render(
        &self,
        layer: &str,
        filters: &[Filter],
        gpu_enabled: bool,
    ) -> Result<PixelBuffer, FilterError> {
        let mut doc = self.doc.clone();
        if !filters.is_empty() {
            let target = crate::resolve_path_mut(&mut doc, layer)
                .ok_or_else(|| FilterError::Unsupported("no such layer".to_string()))?;
            for filter in filters {
                crate::apply_filter(target, filter, self.selection.as_ref(), gpu_enabled)?;
            }
        }
        Ok(crate::composite_rgba(&doc))
    }
}

/// Document coordinates inside `region` to proxy coordinates.
struct Map {
    region: PsdRect,
    scale: f64,
}

impl Map {
    fn x(&self, v: i32) -> i32 {
        (f64::from(v - self.region.left) * self.scale).round() as i32
    }

    fn y(&self, v: i32) -> i32 {
        (f64::from(v - self.region.top) * self.scale).round() as i32
    }

    /// The proxy rect of a document rect already inside the region, never
    /// thinner than one pixel.
    fn rect(&self, r: PsdRect) -> PsdRect {
        let (left, top) = (self.x(r.left), self.y(r.top));
        PsdRect {
            top,
            left,
            bottom: self.y(r.bottom).max(top + 1),
            right: self.x(r.right).max(left + 1),
        }
    }

    /// Reduce the plane `data` covering `rect` to its part inside the region:
    /// the proxy rect and its samples, or `None` when nothing is inside.
    fn plane(&self, data: &[u8], rect: PsdRect) -> Option<(PsdRect, Vec<u8>)> {
        let (sw, sh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
        if data.len() < sw * sh {
            return None;
        }
        let sub = intersect(rect, self.region)?;
        let out = self.rect(sub);
        let window = Window {
            x: (sub.left - rect.left) as usize,
            y: (sub.top - rect.top) as usize,
            w: sub.width() as usize,
            h: sub.height() as usize,
        };
        let samples = box_reduce(
            data,
            sw,
            window,
            out.width() as usize,
            out.height() as usize,
        );
        Some((out, samples))
    }
}

const EMPTY: PsdRect = PsdRect {
    top: 0,
    left: 0,
    bottom: 0,
    right: 0,
};

fn reduce_flat(flat: &PixelBuffer, map: &Map, full: PsdRect) -> PixelBuffer {
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: flat.height as i32,
        right: flat.width as i32,
    };
    let n = flat.pixel_count();
    let mut out = PixelBuffer::new(full.width() as u32, full.height() as u32, flat.channels);
    let m = out.pixel_count();
    for c in 0..flat.channels as usize {
        if let Some((_, samples)) = flat
            .data
            .get(c * n..(c + 1) * n)
            .and_then(|plane| map.plane(plane, canvas))
        {
            if samples.len() == m {
                out.data[c * m..(c + 1) * m].copy_from_slice(&samples);
            }
        }
    }
    out
}

fn reduce_layer(layer: &mut Layer, map: &Map) {
    let rect = layer.rect;
    let mut reduced = None;
    for channel in &mut layer.channels {
        match map.plane(&channel.data, rect) {
            Some((r, samples)) => {
                channel.data = samples.into();
                reduced = Some(r);
            }
            None => channel.data = Vec::new().into(),
        }
    }
    layer.rect = reduced.unwrap_or(EMPTY);
    if let Some(mask) = &mut layer.mask {
        *mask = reduce_mask(mask, map);
    }
    layer.raw_channels.clear();
    layer.source_channels = None;
}

fn reduce_mask(mask: &LayerMask, map: &Map) -> LayerMask {
    let mut out = mask.clone();
    match &mask.data {
        Some(data) => match map.plane(data, mask.rect) {
            Some((rect, samples)) => {
                out.rect = rect;
                out.data = Some(samples.into());
            }
            None => {
                out.rect = EMPTY;
                out.data = None;
            }
        },
        None => out.rect = intersect(mask.rect, map.region).map_or(EMPTY, |r| map.rect(r)),
    }
    out
}

fn intersect(a: PsdRect, b: PsdRect) -> Option<PsdRect> {
    let r = PsdRect {
        top: a.top.max(b.top),
        left: a.left.max(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.min(b.right),
    };
    (r.right > r.left && r.bottom > r.top).then_some(r)
}

#[derive(Clone, Copy)]
struct Window {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Average `window` of the `stride`-wide plane `src` down to `dw × dh`: each
/// output sample is the mean of the source cells its footprint covers, so
/// fine detail averages out rather than aliasing. Rows run in parallel.
fn box_reduce(src: &[u8], stride: usize, window: Window, dw: usize, dh: usize) -> Vec<u8> {
    let span = |o: usize, n: usize, d: usize| {
        let a = o * n / d;
        (a, ((o + 1) * n / d).max(a + 1))
    };
    let xs: Vec<(usize, usize)> = (0..dw).map(|x| span(x, window.w, dw)).collect();
    let mut out = vec![0u8; dw * dh];
    out.par_chunks_mut(dw).enumerate().for_each(|(oy, row)| {
        let (y0, y1) = span(oy, window.h, dh);
        for (px, &(x0, x1)) in row.iter_mut().zip(&xs) {
            let mut sum = 0u64;
            for y in y0..y1 {
                let at = (window.y + y) * stride + window.x;
                sum += src[at + x0..at + x1]
                    .iter()
                    .map(|&v| u64::from(v))
                    .sum::<u64>();
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            *px = ((sum + n / 2) / n) as u8;
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }

    #[test]
    fn box_reduce_averages_each_footprint() {
        // 4×2 → 2×1: each output is the mean of a 2×2 block.
        let src = [0, 10, 100, 200, 20, 30, 100, 0];
        let window = Window {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        };
        assert_eq!(box_reduce(&src, 4, window, 2, 1), vec![15, 100]);
        // Scale 1 copies the window.
        let window = Window {
            x: 1,
            y: 0,
            w: 2,
            h: 2,
        };
        assert_eq!(box_reduce(&src, 4, window, 2, 2), vec![10, 100, 30, 100]);
    }

    fn doc_with_layer(w: u32, h: u32, layer_rect: PsdRect) -> Document {
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [255, 255, 255, 255]).collect();
        let mut doc = Document::from_rgba("doc", w, h, &rgba);
        let (lw, lh) = (layer_rect.width() as usize, layer_rect.height() as usize);
        let layer = &mut doc.layers[0];
        layer.rect = layer_rect;
        for channel in &mut layer.channels {
            channel.data = (0..lw * lh)
                .map(|i| if i % lw < lw / 2 { 0 } else { 200 })
                .collect::<Vec<u8>>()
                .into();
        }
        doc
    }

    #[test]
    fn proxy_maps_layers_and_region_to_the_reduced_grid() {
        let doc = doc_with_layer(100, 80, rect(10, 20, 50, 60));
        let proxy = FilterProxy::new(&doc, None, rect(0, 0, 100, 80), 0.5).unwrap();
        assert_eq!((proxy.width(), proxy.height()), (50, 40));
        assert_eq!(proxy.doc.layers[0].rect, rect(5, 10, 25, 30));
        assert_eq!(proxy.doc.layers[0].channels[0].data.len(), 20 * 20);
        // A region crops: only the part of the layer inside it survives, at
        // region-relative coordinates.
        let crop = FilterProxy::new(&doc, None, rect(30, 0, 100, 40), 1.0).unwrap();
        assert_eq!((crop.width(), crop.height()), (70, 40));
        assert_eq!(crop.doc.layers[0].rect, rect(0, 20, 20, 40));
        // A layer outside the region is emptied.
        let none = FilterProxy::new(&doc, None, rect(60, 0, 100, 80), 1.0).unwrap();
        assert_eq!(none.doc.layers[0].rect, EMPTY);
        assert!(FilterProxy::new(&doc, None, rect(200, 0, 300, 80), 1.0).is_none());
    }

    #[test]
    fn full_scale_proxy_renders_like_the_document() {
        let doc = doc_with_layer(40, 30, rect(0, 0, 40, 30));
        let proxy = FilterProxy::new(&doc, None, rect(0, 0, 40, 30), 1.0).unwrap();
        let filter = Filter::GaussianBlur { radius: 2.0 };
        let got = proxy
            .render("0", std::slice::from_ref(&filter), false)
            .unwrap();
        let mut want = doc.clone();
        crate::apply_filter(&mut want.layers[0], &filter, None, false).unwrap();
        assert_eq!(got, crate::composite_rgba(&want));
        // The proxy itself is untouched by a render.
        assert_eq!(
            proxy.render("0", &[], false).unwrap(),
            crate::composite_rgba(&doc)
        );
    }

    #[test]
    fn selection_is_reduced_with_the_document() {
        let doc = doc_with_layer(40, 40, rect(0, 0, 40, 40));
        let selection = LayerMask {
            rect: rect(0, 0, 40, 40),
            data: Some(vec![255u8; 1600].into()),
            ..Default::default()
        };
        let proxy = FilterProxy::new(&doc, Some(&selection), rect(0, 0, 40, 40), 0.25).unwrap();
        let reduced = proxy.selection.as_ref().unwrap();
        assert_eq!(reduced.rect, rect(0, 0, 10, 10));
        assert_eq!(reduced.data.as_ref().unwrap().len(), 100);
    }
}
