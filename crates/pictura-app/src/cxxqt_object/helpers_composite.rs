use cxx_qt_lib::{QImage, QImageFormat};
use pictura_codec::buffer_to_srgb;
use pictura_core::{
    BlendMode, ColorMode, Document, Layer, LayerMask, PixelBuffer, PsdRect, VectorMask,
};
use pictura_paint::stamp::{layer_surface, sample_scope, surface_from_composite, CloneSampling};
#[cfg(test)]
use pictura_paint::Stroke;
use pictura_render::{Planes, PyramidLevel};
use pictura_select::Selection;
/// The rendered buffer as a 4-plane RGBA frame.
///
/// A 1/2/3-plane render (a layerless document's embedded composite) is expanded
/// with grey replication and opaque alpha exactly as [`buffer_to_image`] does.
pub(super) fn rgba_frame(rendered: &PixelBuffer) -> PixelBuffer {
    if rendered.channels == 4 {
        return rendered.clone();
    }
    let plane = rendered.pixel_count();
    let channels = rendered.channels as usize;
    let mut data = vec![0u8; plane * 4];
    for i in 0..plane {
        let (r, g, b, a) = if channels <= 1 {
            let v = rendered.data[i];
            (v, v, v, 255)
        } else if channels == 2 {
            let v = rendered.data[i];
            (v, v, v, rendered.data[plane + i])
        } else {
            (
                rendered.data[i],
                rendered.data[plane + i],
                rendered.data[2 * plane + i],
                255,
            )
        };
        data[i] = r;
        data[plane + i] = g;
        data[2 * plane + i] = b;
        data[3 * plane + i] = a;
    }
    PixelBuffer {
        width: rendered.width,
        height: rendered.height,
        channels: 4,
        data: data.into(),
    }
}
/// Persist a full-frame rendered composite into `doc.composite`.
///
/// An RGB document takes the rendered 4-plane RGBA frame, so its merged
/// composite is RGBA after any rebuild. A non-RGB mode keeps its existing
/// composite colour-plane count and copies `min(rendered, composite)` planes, so
/// a 1-plane grayscale composite stays 1-plane. A composite whose dimensions no
/// longer match the document is replaced with the rendered frame.
pub(super) fn store_composite(doc: &mut Document, rendered: &PixelBuffer) {
    if rendered.width != doc.width || rendered.height != doc.height {
        return;
    }
    let rgba = rgba_frame(rendered);
    if doc.mode == ColorMode::Rgb
        || doc.composite.width != doc.width
        || doc.composite.height != doc.height
    {
        doc.composite = rgba;
        return;
    }
    let target = doc.composite.channels as usize;
    let plane = (doc.width as usize) * (doc.height as usize);
    let mut data = vec![0u8; plane * target];
    for c in 0..target.min(rgba.channels as usize) {
        data[c * plane..(c + 1) * plane].copy_from_slice(&rgba.data[c * plane..(c + 1) * plane]);
    }
    doc.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: target as u8,
        data: data.into(),
    };
}
/// Copy a region of a 4-plane rendered buffer into `doc.composite` at `(x0, y0)`.
///
/// Writes the composite's own colour-plane count (an RGB composite is 4-plane,
/// a grayscale one 1-plane), mapping region plane `c` to composite plane `c`.
/// Used to keep the cached document composite consistent with the region blit.
/// A channel-count or size mismatch is ignored (the region still reaches the
/// displayed image).
pub(super) fn patch_composite_region(doc: &mut Document, region: &PixelBuffer, x0: i32, y0: i32) {
    if doc.composite.width != doc.width || doc.composite.height != doc.height {
        return;
    }
    patch_buffer_region(&mut doc.composite, region, x0, y0);
}
/// Overwrite `dst` at `(x0, y0)` with a 4-channel planar region buffer, mapping
/// region plane `c` to `dst` plane `c` for `min(4, dst.channels)` planes.
pub(super) fn patch_buffer_region(dst: &mut PixelBuffer, region: &PixelBuffer, x0: i32, y0: i32) {
    if region.channels != 4 {
        return;
    }
    if x0 < 0 || y0 < 0 {
        return;
    }
    let (x0, y0) = (x0 as usize, y0 as usize);
    let rw = region.width as usize;
    let rh = region.height as usize;
    let (fw, fh) = (dst.width as usize, dst.height as usize);
    if x0 + rw > fw || y0 + rh > fh {
        return;
    }
    use rayon::prelude::*;
    let fplane = fw * fh;
    let rplane = rw * rh;
    let planes = (dst.channels as usize).min(4);
    if fplane == 0 || rw == 0 {
        return;
    }
    // A whole-document region is a gigabyte on a large image: rows go to
    // every core.
    for (c, plane) in dst.data.as_mut_slice()[..planes * fplane]
        .chunks_mut(fplane)
        .enumerate()
    {
        plane[y0 * fw..(y0 + rh) * fw]
            .par_chunks_mut(fw)
            .enumerate()
            .for_each(|(ry, row)| {
                let src = c * rplane + ry * rw;
                row[x0..x0 + rw].copy_from_slice(&region.data[src..src + rw]);
            });
    }
}
/// The `w × h` region of planar `buffer` at `(x0, y0)`, every channel kept.
/// The caller clamps the region to the buffer.
pub(super) fn crop_planar(buffer: &PixelBuffer, x0: i32, y0: i32, w: u32, h: u32) -> PixelBuffer {
    let (fw, plane) = (buffer.width as usize, buffer.pixel_count());
    let (x0, y0, w, h) = (x0 as usize, y0 as usize, w as usize, h as usize);
    let mut data = Vec::with_capacity(w * h * buffer.channels as usize);
    for c in 0..buffer.channels as usize {
        for y in y0..y0 + h {
            let at = c * plane + y * fw + x0;
            data.extend_from_slice(&buffer.data[at..at + w]);
        }
    }
    PixelBuffer {
        width: w as u32,
        height: h as u32,
        channels: buffer.channels,
        data: data.into(),
    }
}
/// A full-frame raster mask whose coverage is the selection.
pub(super) fn selection_to_mask(selection: &Selection, doc: &Document) -> LayerMask {
    LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(selection.data.clone().into()),
        ..Default::default()
    }
}
/// A document-sized selection plane cropped to `rect` in mask-local coordinates
/// (row-major over the rectangle); pixels outside the document are unselected.
pub(super) fn crop_selection(data: &[u8], doc_w: u32, doc_h: u32, rect: PsdRect) -> Vec<u8> {
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        let dy = rect.top + y as i32;
        if dy < 0 || dy >= doc_h as i32 {
            continue;
        }
        for x in 0..w {
            let dx = rect.left + x as i32;
            if dx < 0 || dx >= doc_w as i32 {
                continue;
            }
            out[y * w + x] = data[dy as usize * doc_w as usize + dx as usize];
        }
    }
    out
}
/// The selection cropped to `rect` as a full-rect coverage mask, for an edit
/// that runs on a mask-sized document.
pub(super) fn selection_mask_for(selection: &Selection, rect: PsdRect) -> LayerMask {
    let (w, h) = (rect.width().max(0), rect.height().max(0));
    LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        },
        default_color: 0,
        data: Some(crop_selection(&selection.data, selection.width, selection.height, rect).into()),
        ..Default::default()
    }
}
/// The buffer a wand samples: the composited layer stack when present,
/// otherwise the embedded PSD composite.
pub(super) fn current_buffer(doc: &Document, gpu_compute: bool) -> PixelBuffer {
    if doc.layers.is_empty() {
        doc.composite.clone()
    } else {
        pictura_render::composite_active(doc, gpu_compute).0
    }
}
/// Convert the document to a packed RGBA `QImage`.
pub(super) fn document_to_image(doc: &Document, gpu_compute: bool) -> QImage {
    let buffer = current_buffer(doc, gpu_compute);
    buffer_to_image(&buffer_to_srgb(doc, &buffer))
}
/// The full display image for the current state, or `None` without a document.
///
/// While a stroke is active the source is the stroke's working document, so a
/// live (uncommitted) stroke is not lost; otherwise it is the authoritative
/// planar `doc.composite`, which M34 keeps byte-identical to a full composite.
/// The pixels are premultiplied for Qt; the straight-alpha export/thumbnail
/// helpers are unchanged. Test-only now that `PictureView::image` is built on
/// demand from the level-0 frame.
#[cfg(test)]
pub(super) fn rebuild_display(
    doc: &Option<Document>,
    stroke: Option<&Stroke>,
    gpu_compute: bool,
) -> Option<QImage> {
    if let (Some(_), Some(doc)) = (stroke, doc.as_ref()) {
        let rendered = current_buffer(doc, gpu_compute);
        return Some(premultiplied_display_image(&buffer_to_srgb(doc, &rendered)));
    }
    doc.as_ref()
        .map(|doc| premultiplied_display_image(&buffer_to_srgb(doc, &doc.composite)))
}
/// The 4-byte PSD blend key as a `String` (e.g. `"mul "`).
pub(super) fn blend_key(mode: BlendMode) -> String {
    String::from_utf8_lossy(&mode.to_psd_key()).into_owned()
}
/// Convert a pixel layer's planar channels to an RGBA `QImage`, or `None` for an
/// empty rect, a missing/wrong-sized color channel, or an alpha mismatch.
pub(super) fn layer_image(layer: &Layer) -> Option<QImage> {
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    let plane = (width * height) as usize;
    let channel = |id: i16| layer.channels.iter().find(|c| c.id == id).map(|c| &c.data);

    let alpha = channel(-1);
    let buffer = if channel(1).is_none() && channel(2).is_none() {
        let gray = channel(0)?;
        if gray.len() != plane {
            return None;
        }
        let mut data = vec![0u8; plane * 2];
        data[..plane].copy_from_slice(gray);
        match alpha {
            Some(a) if a.len() == plane => data[plane..].copy_from_slice(a),
            Some(_) => return None,
            None => data[plane..].fill(255),
        }
        PixelBuffer {
            width,
            height,
            channels: 2,
            data: data.into(),
        }
    } else {
        let (r, g, b) = (channel(0)?, channel(1)?, channel(2)?);
        if r.len() != plane || g.len() != plane || b.len() != plane {
            return None;
        }
        let mut data = vec![0u8; plane * 4];
        data[..plane].copy_from_slice(r);
        data[plane..2 * plane].copy_from_slice(g);
        data[2 * plane..3 * plane].copy_from_slice(b);
        match alpha {
            Some(a) if a.len() == plane => data[3 * plane..].copy_from_slice(a),
            Some(_) => return None,
            None => data[3 * plane..].fill(255),
        }
        PixelBuffer {
            width,
            height,
            channels: 4,
            data: data.into(),
        }
    };
    Some(buffer_to_image(&buffer))
}
/// Downsample a pixel layer's planar channels straight into a `size`-bounded
/// RGBA `QImage` (long edge `size`, aspect kept), without first materializing a
/// full-resolution RGBA image. Nearest-neighbour: cost is O(target), not
/// O(layer pixels). Returns `None` for the same empty/invalid layers as
/// [`layer_image`].
pub(super) fn layer_thumbnail_image(layer: &Layer, size: u32) -> Option<QImage> {
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    let plane = (width * height) as usize;
    let channel = |id: i16| layer.channels.iter().find(|c| c.id == id).map(|c| &c.data);

    let gray = channel(1).is_none() && channel(2).is_none();
    let c0 = channel(0)?;
    if c0.len() != plane {
        return None;
    }
    let (c1, c2) = if gray {
        (c0, c0)
    } else {
        let (c1, c2) = (channel(1)?, channel(2)?);
        if c1.len() != plane || c2.len() != plane {
            return None;
        }
        (c1, c2)
    };
    let alpha = match channel(-1) {
        Some(a) if a.len() == plane => Some(a),
        Some(_) => return None,
        None => None,
    };

    let (tw, th) = if width >= height {
        (
            size,
            ((height as u64 * size as u64) / width as u64).max(1) as u32,
        )
    } else {
        (
            ((width as u64 * size as u64) / height as u64).max(1) as u32,
            size,
        )
    };

    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let sy = ty as usize * height as usize / th as usize;
        for tx in 0..tw {
            let sx = tx as usize * width as usize / tw as usize;
            let si = sy * width as usize + sx;
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o] = c0[si];
            rgba[o + 1] = c1[si];
            rgba[o + 2] = c2[si];
            rgba[o + 3] = alpha.map_or(255, |a| a[si]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}
/// Sample a pixel layer's RGBA at layer-local `(x, y)`, or `None` when the
/// layer's channels are absent/misshaped or the coordinate is out of bounds.
pub(super) fn layer_pixel(layer: &Layer, x: u32, y: u32) -> Option<[u8; 4]> {
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    if x >= width || y >= height {
        return None;
    }
    let plane = (width * height) as usize;
    let at = (y * width + x) as usize;
    let channel = |id: i16| layer.channels.iter().find(|c| c.id == id).map(|c| &c.data);
    let c0 = channel(0)?;
    if c0.len() != plane {
        return None;
    }
    let (r, g, b) = if channel(1).is_none() && channel(2).is_none() {
        (c0[at], c0[at], c0[at])
    } else {
        let (c1, c2) = (channel(1)?, channel(2)?);
        if c1.len() != plane || c2.len() != plane {
            return None;
        }
        (c0[at], c1[at], c2[at])
    };
    let a = match channel(-1) {
        Some(a) if a.len() == plane => a[at],
        Some(_) => return None,
        None => 255,
    };
    Some([r, g, b, a])
}
/// Panel Options "Entire Document": render a pixel layer inside a transparent
/// `size`×`size` document square, placing its pixels at their document position
/// (`rect.left/top`) scaled by `size / max(doc_width, doc_height)`, so a small
/// layer appears small in its document context.
pub(super) fn layer_thumbnail_positioned(
    layer: &Layer,
    doc_width: u32,
    doc_height: u32,
    size: u32,
) -> Option<QImage> {
    if doc_width == 0 || doc_height == 0 {
        return None;
    }
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let scale = size as f64 / doc_width.max(doc_height) as f64;
    let dest_left = (layer.rect.left as f64 * scale).round() as i64;
    let dest_top = (layer.rect.top as f64 * scale).round() as i64;
    let dest_w = ((width as f64 * scale).round() as i64).max(1);
    let dest_h = ((height as f64 * scale).round() as i64).max(1);
    let (src_w, src_h) = (width as u32, height as u32);

    let mut rgba = vec![0u8; (size as u64 * size as u64 * 4) as usize];
    for py in 0..size as i64 {
        for px in 0..size as i64 {
            if px < dest_left
                || py < dest_top
                || px >= dest_left + dest_w
                || py >= dest_top + dest_h
            {
                continue;
            }
            let sx = ((px - dest_left) as u32 * src_w / dest_w as u32).min(src_w - 1);
            let sy = ((py - dest_top) as u32 * src_h / dest_h as u32).min(src_h - 1);
            let Some([r, g, b, a]) = layer_pixel(layer, sx, sy) else {
                continue;
            };
            let o = ((py as u32 * size + px as u32) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[r, g, b, a]);
        }
    }
    Some(rgba_image(rgba, size as i32, size as i32))
}
/// Downsample a layer mask's grayscale data into a `size`-bounded square (long
/// edge `size`, aspect kept), replicating the value across RGB. Null without
/// mask data.
pub(super) fn mask_thumbnail_image(mask: &LayerMask, size: u32) -> Option<QImage> {
    let data = mask.data.as_ref()?;
    let width = mask.rect.width();
    let height = mask.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    let plane = (width * height) as usize;
    if data.len() != plane {
        return None;
    }
    let (tw, th) = if width >= height {
        (
            size,
            ((height as u64 * size as u64) / width as u64).max(1) as u32,
        )
    } else {
        (
            ((width as u64 * size as u64) / height as u64).max(1) as u32,
            size,
        )
    };

    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let sy = ty as usize * height as usize / th as usize;
        for tx in 0..tw {
            let sx = (tx as usize * width as usize / tw as usize).min(width as usize - 1);
            let v = data[sy * width as usize + sx];
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[v, v, v, 255]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}
/// Downsample a vector mask's coverage into a `size`-bounded square (long edge
/// `size`, aspect kept), replicating the value across RGB with the same
/// white-shows / black-hides convention as [`mask_thumbnail_image`]. An absent,
/// disabled, or all-open mask is fully white (permissive), matching the
/// compositor. `ponytail:` CS6 strokes the path outline; this fills coverage.
pub(super) fn vector_mask_thumbnail_image(
    mask: &VectorMask,
    doc_width: u32,
    doc_height: u32,
    size: u32,
) -> Option<QImage> {
    if doc_width == 0 || doc_height == 0 || size == 0 {
        return None;
    }
    let (tw, th) = if doc_width >= doc_height {
        (
            size,
            ((doc_height as u64 * size as u64) / doc_width as u64).max(1) as u32,
        )
    } else {
        (
            ((doc_width as u64 * size as u64) / doc_height as u64).max(1) as u32,
            size,
        )
    };
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let dy = (ty as u64 * doc_height as u64 / th as u64) as i32;
        for tx in 0..tw {
            let dx = (tx as u64 * doc_width as u64 / tw as u64) as i32;
            let value = if !mask.has_fill() {
                255
            } else {
                let inside = mask.inside(dx, dy);
                let show = if mask.invert { !inside } else { inside };
                if show {
                    255
                } else {
                    0
                }
            };
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}
/// The `0xAARRGGBB` value of a planar buffer pixel, or 0 out of bounds.
///
/// Mirrors [`buffer_to_image`]'s plane rules: 1 plane is opaque grey, 2 is grey
/// plus alpha, 3 is opaque RGB, 4 is RGBA.
pub(super) fn sample_planar_argb(buffer: &PixelBuffer, x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= buffer.width as i32 || y >= buffer.height as i32 {
        return 0;
    }
    let plane = buffer.width as usize * buffer.height as usize;
    let i = y as usize * buffer.width as usize + x as usize;
    let (r, g, b, a) = match buffer.channels {
        0 => return 0,
        1 => {
            let v = buffer.data[i];
            (v, v, v, 255)
        }
        2 => {
            let v = buffer.data[i];
            (v, v, v, buffer.data[plane + i])
        }
        channels if channels >= 4 => (
            buffer.data[i],
            buffer.data[plane + i],
            buffer.data[2 * plane + i],
            buffer.data[3 * plane + i],
        ),
        _ => (
            buffer.data[i],
            buffer.data[plane + i],
            buffer.data[2 * plane + i],
            255,
        ),
    };
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Average the `size`×`size` box centred on `(x, y)` from the eyedropper
/// `scope`: 0 Current Layer, 1 Current & Below, 2 All Layers, 3 All Layers No
/// Adjustments, 4 Current & Below No Adjustments. `0` when nothing is sampled.
pub(super) fn sample_argb_scoped(
    doc: &Document,
    path: &str,
    x: i32,
    y: i32,
    size: i32,
    scope: i32,
) -> u32 {
    let Some(image) = eyedropper_surface(doc, path, scope) else {
        return 0;
    };
    let radius = (size.clamp(1, 101) - 1) / 2;
    let (mut r, mut g, mut b, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
    for sy in (y - radius).max(0)..=(y + radius).min(image.height - 1) {
        for sx in (x - radius).max(0)..=(x + radius).min(image.width - 1) {
            let p = image.data[(sy * image.width + sx) as usize];
            r += p[0] as u32;
            g += p[1] as u32;
            b += p[2] as u32;
            a += p[3] as u32;
            n += 1;
        }
    }
    if n == 0 {
        return 0;
    }
    ((a / n) << 24) | ((r / n) << 16) | ((g / n) << 8) | (b / n)
}

// The sampling surface for one scope; `None` when the scope cannot be read
// (no such layer, or a group/adjustment for Current Layer).
fn eyedropper_surface(
    doc: &Document,
    path: &str,
    scope: i32,
) -> Option<pictura_paint::healing::RgbaImage> {
    match scope {
        0 => layer_surface(doc, path),
        4 => {
            let mut scope_doc = doc.clone();
            truncate_above(&mut scope_doc.layers, &parse_layer_path(path)?);
            strip_adjustments(&mut scope_doc.layers);
            Some(surface_from_composite(&pictura_render::composite_rgba(
                &scope_doc,
            )))
        }
        _ => {
            let sampling = if scope == 1 {
                CloneSampling::CurrentAndBelow
            } else {
                CloneSampling::AllLayers
            };
            let scope_doc = sample_scope(doc, path, sampling, scope == 3)?;
            Some(surface_from_composite(&pictura_render::composite_rgba(
                &scope_doc,
            )))
        }
    }
}

fn parse_layer_path(path: &str) -> Option<Vec<usize>> {
    if path.is_empty() {
        return None;
    }
    path.split('/').map(|s| s.parse::<usize>().ok()).collect()
}

fn truncate_above(layers: &mut Vec<Layer>, path: &[usize]) {
    if let Some((&i, rest)) = path.split_first() {
        layers.truncate(i + 1);
        if let Some(layer) = layers.get_mut(i) {
            truncate_above(&mut layer.children, rest);
        }
    }
}

fn strip_adjustments(layers: &mut Vec<Layer>) {
    layers.retain(|l| l.adjustment.is_none());
    for layer in layers {
        strip_adjustments(&mut layer.children);
    }
}

/// Convert a planar 8-bit buffer (1 = gray, 2 = gray+alpha, 3 = RGB, 4 = RGBA)
/// to interleaved RGBA8888. Gray replicates across RGB; RGB gets opaque alpha.
pub(super) fn buffer_to_rgba_bytes(buffer: &PixelBuffer) -> Vec<u8> {
    let plane = buffer.width as usize * buffer.height as usize;
    let channels = buffer.channels as usize;

    let mut rgba = vec![0u8; plane * 4];
    for i in 0..plane {
        let (r, g, b, a) = if channels <= 1 {
            let v = buffer.data[i];
            (v, v, v, 255)
        } else if channels == 2 {
            let v = buffer.data[i];
            (v, v, v, buffer.data[plane + i])
        } else {
            let a = if channels >= 4 {
                buffer.data[3 * plane + i]
            } else {
                255
            };
            (
                buffer.data[i],
                buffer.data[plane + i],
                buffer.data[2 * plane + i],
                a,
            )
        };
        let o = i * 4;
        rgba[o..o + 4].copy_from_slice(&[r, g, b, a]);
    }
    rgba
}
/// Convert a planar 8-bit buffer to an `RGBA8888` `QImage`.
pub(super) fn buffer_to_image(buffer: &PixelBuffer) -> QImage {
    let width = buffer.width as i32;
    let height = buffer.height as i32;
    let rgba = buffer_to_rgba_bytes(buffer);

    // SAFETY: `rgba` is exactly width*height RGBA8888 bytes, tightly packed.
    unsafe { QImage::from_raw_bytes(rgba, width, height, QImageFormat::Format_RGBA8888) }
}
/// Wrap packed RGBA8888 bytes as a `QImage`.
pub(super) fn rgba_image(rgba: Vec<u8>, width: i32, height: i32) -> QImage {
    // SAFETY: `rgba` is exactly width*height RGBA8888 bytes.
    unsafe { QImage::from_raw_bytes(rgba, width, height, QImageFormat::Format_RGBA8888) }
}
/// Premultiply one straight-alpha sample: `round(c * a / 255)`.
fn premul(c: u8, a: u8) -> u8 {
    ((c as u32 * a as u32 + 127) / 255) as u8
}
/// Convert a planar 8-bit buffer to interleaved premultiplied RGBA8888 for
/// display. Qt then paints it without a per-frame conversion and averages
/// transparent edges correctly. [`buffer_to_rgba_bytes`] stays straight alpha
/// for the export encoder.
pub(super) fn display_rgba_bytes(buffer: &PixelBuffer) -> Vec<u8> {
    use rayon::prelude::*;
    let plane = buffer.pixel_count();
    let channels = buffer.channels as usize;
    let mut rgba = vec![0u8; plane * 4];
    // A full-document frame is hundreds of megabytes on a large image; spread
    // it over every core in runs of pixels.
    const RUN: usize = 1 << 16;
    rgba.par_chunks_mut(RUN * 4)
        .enumerate()
        .for_each(|(run, out)| {
            for (k, px) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let i = run * RUN + k;
                let (r, g, b, a) = if channels <= 1 {
                    let v = buffer.data[i];
                    (v, v, v, 255)
                } else if channels == 2 {
                    let v = buffer.data[i];
                    (v, v, v, buffer.data[plane + i])
                } else {
                    let a = if channels >= 4 {
                        buffer.data[3 * plane + i]
                    } else {
                        255
                    };
                    (
                        buffer.data[i],
                        buffer.data[plane + i],
                        buffer.data[2 * plane + i],
                        a,
                    )
                };
                px.copy_from_slice(&[premul(r, a), premul(g, a), premul(b, a), a]);
            }
        });
    rgba
}
/// Convert a planar 8-bit buffer to a premultiplied `RGBA8888` `QImage`.
pub(super) fn premultiplied_display_image(buffer: &PixelBuffer) -> QImage {
    let width = buffer.width as i32;
    let height = buffer.height as i32;
    let rgba = display_rgba_bytes(buffer);
    // SAFETY: `rgba` is exactly width*height premultiplied RGBA8888 bytes.
    unsafe {
        QImage::from_raw_bytes(
            rgba,
            width,
            height,
            QImageFormat::Format_RGBA8888_Premultiplied,
        )
    }
}
/// Wrap an already-premultiplied interleaved RGBA pyramid level as a `QImage`.
pub(super) fn level_display_image(level: &PyramidLevel) -> QImage {
    // SAFETY: level data is exactly width*height premultiplied RGBA8888 bytes.
    unsafe {
        QImage::from_raw_bytes(
            level.data().to_vec(),
            level.width() as i32,
            level.height() as i32,
            QImageFormat::Format_RGBA8888_Premultiplied,
        )
    }
}
/// A borrowed 4-plane view of `buffer`, or `None` unless it is exactly RGBA.
pub(super) fn planes_of(buffer: &PixelBuffer) -> Option<Planes<'_>> {
    if buffer.channels != 4 {
        return None;
    }
    let plane = buffer.pixel_count();
    if buffer.data.len() < plane * 4 {
        return None;
    }
    Some(Planes {
        width: buffer.width,
        height: buffer.height,
        r: &buffer.data[..plane],
        g: &buffer.data[plane..2 * plane],
        b: &buffer.data[2 * plane..3 * plane],
        a: &buffer.data[3 * plane..4 * plane],
    })
}
/// Expand an owned rendered buffer to a 4-plane RGBA frame, taking it over
/// without a copy when it is already RGBA.
pub(super) fn into_rgba_frame(rendered: PixelBuffer) -> PixelBuffer {
    if rendered.channels == 4 {
        rendered
    } else {
        rgba_frame(&rendered)
    }
}
/// The 4-plane straight **sRGB** level-0 frame for `source`: the color-managed
/// composite the display image, pyramid, and canvas crops all share.
pub(super) fn level0_buffer(source: &Document) -> PixelBuffer {
    level0_from_buffer(source, &source.composite)
}

/// The level-0 frame for an already-rendered `buffer` of `source`.
///
/// Level 0 is patched on its own (mid-stroke it runs ahead of the document's
/// composite), so it never shares the buffer's plane: a shared plane would be
/// copied whole by whichever of the two is written first.
pub(super) fn level0_from_buffer(source: &Document, buffer: &PixelBuffer) -> PixelBuffer {
    let mut frame = into_rgba_frame(buffer_to_srgb(source, buffer).into_owned());
    if frame.data.shares(&buffer.data) {
        frame.data = crate::history::copy_plane(&frame.data);
    }
    frame
}

/// The level-0 frame for `source`, compositing its layers first. Used when
/// `source`'s cached `composite` is stale — a live stroke's working document.
pub(super) fn level0_composited(source: &Document, gpu_compute: bool) -> PixelBuffer {
    level0_from_buffer(source, &current_buffer(source, gpu_compute))
}
/// Deterministic gradient so the window always has something to show.
pub(super) fn test_image() -> QImage {
    let (width, height) = (512i32, 512i32);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let o = ((y * width + x) * 4) as usize;
            rgba[o] = (x * 255 / width) as u8;
            rgba[o + 1] = (y * 255 / height) as u8;
            rgba[o + 2] = ((x ^ y) & 0xff) as u8;
            rgba[o + 3] = 255;
        }
    }

    rgba_image(rgba, width, height)
}
