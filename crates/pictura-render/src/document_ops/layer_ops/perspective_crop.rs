//! Perspective Crop: map a user-marked quadrilateral (something that should be
//! rectangular but was photographed at an angle) onto a rectangle, straightening
//! and cropping in one step. Every layer, mask, and extra channel is warped
//! through the same homography, so the stack stays in register.
//!
//! Ported from photorust's `core/src/perspective.rs` and
//! `Document::perspective_crop` (<https://github.com/perfecto25/photorust>); the
//! homography solve and bilinear sampler are Kooka's own (`transform.rs`).

use pictura_core::{Document, PsdRect};

use super::transform::{bilinear, solve_homography, PlaneMap, QuadMap};
use crate::document_ops::{for_each_layer, recompute};

/// Largest output side, matching the PSD dimension ceiling.
const MAX_SIDE: u32 = 30_000;

/// The output size for `quad` (TL, TR, BR, BL): the longer of each pair of
/// opposite edges, keeping the detail of the side nearest the camera.
pub fn perspective_crop_size(quad: [(f64, f64); 4]) -> (u32, u32) {
    let edge = |a: (f64, f64), b: (f64, f64)| (b.0 - a.0).hypot(b.1 - a.1);
    let width = edge(quad[0], quad[1]).max(edge(quad[3], quad[2]));
    let height = edge(quad[0], quad[3]).max(edge(quad[1], quad[2]));
    let side = |v: f64| {
        if v.is_finite() {
            (v.round() as u32).clamp(1, MAX_SIDE)
        } else {
            1
        }
    };
    (side(width), side(height))
}

/// Why `doc` cannot be perspective-cropped, or `None` when it can.
///
/// ponytail: live type, smart objects, and vector masks would be left out of
/// register by a pixel warp, and retained 16/32-bit samples cannot be warped
/// yet, so these refuse rather than being silently rasterized or dropped.
pub fn perspective_crop_refusal(doc: &Document) -> Option<&'static str> {
    if doc.retains_source_depth() {
        return Some("Perspective Crop is not available for 16/32-bit documents yet.");
    }
    let mut live = false;
    fn visit(layers: &[pictura_core::Layer], live: &mut bool) {
        for layer in layers {
            *live |= layer.is_type()
                || layer.type_tool.is_some()
                || layer.smart_object.is_some()
                || layer.vector_mask.is_some();
            visit(&layer.children, live);
        }
    }
    visit(&doc.layers, &mut live);
    live.then_some(
        "Rasterize type, smart object, and vector-mask layers before a Perspective Crop.",
    )
}

/// Warp `doc` so `quad` (TL, TR, BR, BL, document pixels) becomes the whole
/// canvas, sized by [`perspective_crop_size`]. Pixel layers become canvas-sized;
/// channel-less layers (adjustments, fills) are untouched. Returns false,
/// changing nothing, on a refusal or a degenerate quad.
pub fn perspective_crop(doc: &mut Document, quad: [(f64, f64); 4]) -> bool {
    perspective_crop_sized(doc, quad, 0, 0)
}

/// As [`perspective_crop`], but an explicit `out_w`/`out_h` (a positive pixel
/// count) forces that output dimension; 0 or negative falls back to
/// [`perspective_crop_size`] for that dimension.
pub fn perspective_crop_sized(
    doc: &mut Document,
    quad: [(f64, f64); 4],
    out_w: i32,
    out_h: i32,
) -> bool {
    if perspective_crop_refusal(doc).is_some() {
        return false;
    }
    let (auto_w, auto_h) = perspective_crop_size(quad);
    let side = |v: i32, auto: u32| -> u32 {
        if v > 0 {
            (v as u32).clamp(1, MAX_SIDE)
        } else {
            auto
        }
    };
    let (w, h) = (side(out_w, auto_w), side(out_h, auto_h));
    let (wf, hf) = (w as f64, h as f64);
    let Some(map) = solve_homography(quad, [(0.0, 0.0), (wf, 0.0), (wf, hf), (0.0, hf)]) else {
        return false;
    };
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    };

    for_each_layer(&mut doc.layers, &mut |layer| {
        let rect = layer.rect;
        if layer.is_group || layer.channels.is_empty() || rect.width() <= 0 || rect.height() <= 0 {
            return;
        }
        // ponytail: a layer without alpha (the Background) fills outside the
        // source with white; there is no background swatch to read yet.
        let opaque = !layer.channels.iter().any(|c| c.id == -1);
        for channel in &mut layer.channels {
            let outside = if opaque && channel.id >= 0 { 255 } else { 0 };
            channel.data = warp_plane(&channel.data, rect, &map, w, h, outside).into();
        }
        layer.rect = canvas;
        layer.raw_channels.clear();
        layer.source_channels = None;
        if let Some(mask) = &mut layer.mask {
            if let Some(data) = &mask.data {
                mask.data =
                    Some(warp_plane(data, mask.rect, &map, w, h, mask.default_color).into());
            }
            mask.rect = canvas;
        }
    });

    let old = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    for channel in &mut doc.channels {
        channel.data = warp_plane(&channel.data, old, &map, w, h, 0).into();
    }
    doc.source_planes = None;
    doc.width = w;
    doc.height = h;
    recompute(doc);
    true
}

/// Sample the `rect`-sized plane `src` at every pixel centre of a `w×h` canvas
/// through `map`'s inverse; `outside` fills where the source has no pixel. A
/// plane whose length does not match `rect` becomes all `outside`.
fn warp_plane(src: &[u8], rect: PsdRect, map: &QuadMap, w: u32, h: u32, outside: u8) -> Vec<u8> {
    let (sw, sh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut out = vec![outside; w as usize * h as usize];
    if sw == 0 || sh == 0 || src.len() != sw * sh {
        return out;
    }
    for y in 0..h as usize {
        for x in 0..w as usize {
            let Some((sx, sy)) = map.inverse(x as f64 + 0.5, y as f64 + 0.5) else {
                continue;
            };
            let (lx, ly) = (sx - rect.left as f64, sy - rect.top as f64);
            if lx >= 0.0 && ly >= 0.0 && lx < sw as f64 && ly < sh as f64 {
                out[y * w as usize + x] = bilinear(src, sw, sh, lx, ly);
            }
        }
    }
    out
}
