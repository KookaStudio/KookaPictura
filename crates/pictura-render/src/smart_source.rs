//! Render embedded smart-object sources for compositing.
//!
//! Split from `composite.rs` for the file-size cap.

use pictura_core::{Document, Layer, PixelBuffer, PsdRect, SmartObject};

use crate::composite::{blend_into_ctx, Canvas};
use crate::composite_blend::{BlendCtx, BlendOverride};

/// Render an embedded smart object's source over `region` (document
/// coordinates, within `rect`), scaling with the renderer's integer ratio.
///
/// The returned straight-alpha RGBA buffer has `region`'s dimensions, so a
/// caller that only needs the canvas-clipped part allocates only that much.
/// Source sampling still maps through the full `rect`: a pixel's source is
/// chosen from its offset within `rect`, not within `region`.
///
/// `None` when the object is not embedded, its payload is empty or cannot be
/// decoded, the decoded source is empty, or either rectangle has no positive
/// area. Shared by compositing and by rasterizing a channel-less smart-object
/// layer.
pub(crate) fn render_smart_source(
    so: &SmartObject,
    rect: PsdRect,
    region: PsdRect,
) -> Option<PixelBuffer> {
    let src = crate::layer_source::embedded_source(so)?;
    scale_smart_source(&src, rect, region)
}

/// Place the already-decoded embedded `src` into `region` (document
/// coordinates, within `rect`), scaling with the renderer's integer ratio.
pub(crate) fn scale_smart_source(
    src: &PixelBuffer,
    rect: PsdRect,
    region: PsdRect,
) -> Option<PixelBuffer> {
    let src_w = src.width as usize;
    let src_h = src.height as usize;
    let ch = src.channels as usize;
    if src_w == 0 || src_h == 0 || ch == 0 {
        return None;
    }
    let plane = src_w * src_h;

    let lw = rect.width();
    let lh = rect.height();
    if lw <= 0 || lh <= 0 {
        return None;
    }
    let lw = lw as usize;
    let lh = lh as usize;

    let rw = region.width();
    let rh = region.height();
    if rw <= 0 || rh <= 0 {
        return None;
    }
    let rw = rw as usize;
    let rh = rh as usize;
    let out_plane = rw * rh;
    let mut out = PixelBuffer::new(rw as u32, rh as u32, 4);
    for y in region.top..region.bottom {
        let ly = (y - rect.top) as u64;
        for x in region.left..region.right {
            let lx = (x - rect.left) as u64;
            let sx = lx * src_w as u64 / lw as u64;
            let sy = ly * src_h as u64 / lh as u64;
            let si = sy as usize * src_w + sx as usize;
            if si >= plane {
                continue;
            }
            let (r, g, b) = if ch < 3 {
                let v = src.data[si];
                (v, v, v)
            } else {
                (src.data[si], src.data[plane + si], src.data[2 * plane + si])
            };
            let a = if ch >= 4 {
                src.data[3 * plane + si]
            } else {
                255
            };
            let i = (y - region.top) as usize * rw + (x - region.left) as usize;
            out.data[i] = r;
            out.data[out_plane + i] = g;
            out.data[2 * out_plane + i] = b;
            out.data[3 * out_plane + i] = a;
        }
    }
    Some(out)
}

/// Render a layer from its embedded smart-object source when it has no raster
/// proxy, scaling the decoded source into the layer rect. Returns `true` when
/// it produced pixels, so the caller skips the (empty) channel path.
/// `External`/`Alias`/`Unresolved`, an empty payload, or a decode failure
/// returns `false`; the caller then tries the normal path, which draws nothing
/// for a channel-less layer.
pub(crate) fn composite_smart_source(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    source: &PixelBuffer,
    ctx: &BlendCtx<'_>,
    over: Option<&BlendOverride>,
) -> bool {
    let region = PsdRect {
        top: layer.rect.top.max(canvas.y0()),
        left: layer.rect.left.max(canvas.x0()),
        bottom: layer.rect.bottom.min(canvas.y1()),
        right: layer.rect.right.min(canvas.x1()),
    };
    if region.width() <= 0 || region.height() <= 0 {
        return false;
    }
    let Some(src) = scale_smart_source(source, layer.rect, region) else {
        return false;
    };

    let rw = region.width() as usize;
    let rh = region.height() as usize;
    let plane = rw * rh;
    let clip = canvas.clip();
    for by in 0..rh {
        for bx in 0..rw {
            let i = by * rw + bx;
            blend_into_ctx(
                canvas,
                ctx,
                layer,
                doc,
                clip,
                over,
                region.left as usize + bx,
                region.top as usize + by,
                [
                    src.data[i] as f32 / 255.0,
                    src.data[plane + i] as f32 / 255.0,
                    src.data[2 * plane + i] as f32 / 255.0,
                ],
                src.data[3 * plane + i] as f32 / 255.0,
            );
        }
    }
    true
}
