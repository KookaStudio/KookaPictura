//! Region and banded compositing. Split from `composite.rs` for the file-size
//! cap; the per-layer dispatch and canvas model stay there.

use pictura_core::{Document, PixelBuffer};

use crate::composite::Canvas;
use crate::composite_knockout::composite_layers;

/// Rows per band of the banded compositor: a band's `f32` canvas stays a few
/// tens of megabytes on a 16k-wide document.
pub(crate) const BAND: u32 = 64;

/// Composite only the document-space region `[x0, x0+rw) × [y0, y0+rh)`.
///
/// Byte-identical to the same slice of [`crate::composite_rgba`] for every
/// stack: per-layer sources are sampled, and effect passes are replayed into
/// the region canvas.
pub fn composite_rgba_region(doc: &Document, x0: u32, y0: u32, rw: u32, rh: u32) -> PixelBuffer {
    if rw == 0 || rh == 0 {
        return PixelBuffer::new(0, 0, 4);
    }
    if banded(doc, rh) {
        return composite_banded(doc, x0, y0, rw, rh);
    }
    let mut canvas = Canvas::new_region(x0 as i32, y0 as i32, rw as usize, rh as usize);
    composite_layers(&mut canvas, doc);
    canvas.into_pixel_buffer()
}

/// Whether `rows` rows of `doc` composite in bands. Cached sources make every
/// stack bandable except one whose effect pass build reads the document pattern
/// library: those passes are intentionally not cached, so a band would rebuild
/// them. Such a stack uses the whole-canvas path instead.
pub(crate) fn banded(doc: &Document, rows: u32) -> bool {
    rows > 2 * BAND && !doc.layers.iter().any(pattern_driven)
}

fn pattern_driven(layer: &pictura_core::Layer) -> bool {
    if !layer.is_group {
        let effects = crate::layer_source::decoded_effects(layer);
        if crate::layer_effects::reads_pattern(layer, &effects) {
            return true;
        }
    }
    layer.children.iter().any(pattern_driven)
}

/// Composite `[x0, x0+rw) × [y0, y0+rh)` a band of rows at a time, the bands in
/// parallel, each written straight into the output planes. Exact because a
/// region of any stack composites exactly (see [`composite_rgba_region`]); it
/// never holds the region's whole `f32` canvas, which on a 267-megapixel
/// document is 4 GB.
pub(crate) fn composite_banded(doc: &Document, x0: u32, y0: u32, rw: u32, rh: u32) -> PixelBuffer {
    use rayon::prelude::*;
    let (w, plane) = (rw as usize, rw as usize * rh as usize);
    let run = BAND as usize * w;
    let data = pictura_core::Plane::build(plane * 4, |out| {
        let (r, rest) = out.split_at_mut(plane);
        let (g, rest) = rest.split_at_mut(plane);
        let (b, a) = rest.split_at_mut(plane);
        r.par_chunks_mut(run)
            .zip(g.par_chunks_mut(run))
            .zip(b.par_chunks_mut(run))
            .zip(a.par_chunks_mut(run))
            .enumerate()
            .for_each(|(band, (((r, g), b), a))| {
                let top = y0 as i32 + (band * BAND as usize) as i32;
                let mut canvas = Canvas::new_region(x0 as i32, top, w, r.len() / w);
                composite_layers(&mut canvas, doc);
                for (i, p) in canvas.px.iter().enumerate() {
                    r[i] = crate::composite::to_u8(p.r);
                    g[i] = crate::composite::to_u8(p.g);
                    b[i] = crate::composite::to_u8(p.b);
                    a[i] = crate::composite::to_u8(p.a);
                }
            });
    });
    PixelBuffer {
        width: rw,
        height: rh,
        channels: 4,
        data,
    }
}
