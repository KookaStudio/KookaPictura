//! Native-depth compositing: the high-depth adjustment branch and the
//! source-depth composite output. Split from `composite.rs` for the file-size
//! cap.

use pictura_adjust::{AdjustError, Adjustment};
use pictura_core::{BitDepth, ColorMode, Document, Layer, PixelBuffer, Samples};

use crate::composite::{composite_layers, to_u8, Canvas};
use crate::composite_rows::{blend_into_row, for_each_row};

/// The layer's native sample for channel `id` at layer-pixel index `li`, in the
/// unit `f32` domain, when the document is a high-depth Grayscale/RGB read
/// (`source_depth` set, `source_mode` unset) and this layer retains a store
/// matching its rect and that depth. `None` otherwise, so the caller keeps the
/// 8-bit channel path. A converted mode (Lab/CMYK) is excluded because its
/// store holds source-mode planes, not working RGB.
pub(crate) fn native_unit(layer: &Layer, doc: &Document, id: i16, li: usize) -> Option<f32> {
    let depth = doc.source_depth?;
    if doc.source_mode.is_some() {
        return None;
    }
    let store = layer.source_channels.as_ref()?;
    if store.rect != layer.rect || store.depth != depth {
        return None;
    }
    let (_, samples) = store.planes.iter().find(|(cid, _)| *cid == id)?;
    sample_unit(samples, li)
}

fn sample_unit(samples: &Samples, i: usize) -> Option<f32> {
    match samples {
        Samples::U16(v) => v.get(i).map(|&x| x as f32 / 65535.0),
        Samples::F32(v) => v.get(i).map(|&x| x.clamp(0.0, 1.0)),
        Samples::U8(v) => v.get(i).map(|&x| x as f32 / 255.0),
    }
}

/// The layer mask gate in the unit `f32` domain: the native `-2` mask sample
/// when the high-depth document retains one for this layer, else the 8-bit
/// raster byte, combined with the vector-mask coverage. Mirrors `mask_alpha`'s
/// absent/disabled/no-data (`1.0`) and out-of-rect/empty (`default_color/255`)
/// branches through its raster fallback.
pub(crate) fn mask_alpha_unit(doc: &Document, layer: &Layer, x: i32, y: i32) -> f32 {
    // No native `-2` sample (8-bit document, or absent/disabled/out-of-rect
    // mask) keeps the exact 8-bit combine, so the fallback is byte-identical.
    let Some(raster) = native_mask_unit(doc, layer, x, y) else {
        return crate::composite::mask_alpha(layer, crate::mask_refine::canvas(doc), x, y) as f32
            / 255.0;
    };
    let clip = crate::mask_refine::canvas(doc);
    let vector = crate::mask_refine::vector(layer.vector_mask.as_ref(), clip, x, y) as f32 / 255.0;
    raster * vector
}

/// The `-2` mask sample at a canvas pixel in unit space, only for a high-depth
/// document whose layer retains a plane whose length matches the mask rect.
/// Masks are valid in any high-depth mode, so `source_mode` is not gated.
fn native_mask_unit(doc: &Document, layer: &Layer, x: i32, y: i32) -> Option<f32> {
    doc.source_depth?;
    let mask = layer.mask.as_ref()?;
    // ponytail: a refined mask samples through the 8-bit path, so density and
    // feather quantize a high-depth mask to 8 bits.
    if mask.disabled
        || mask.data.is_none()
        || crate::mask_refine::is_refined(mask.density, mask.feather())
    {
        return None;
    }
    let mw = mask.rect.width();
    let mh = mask.rect.height();
    if mw <= 0 || mh <= 0 {
        return None;
    }
    let mx = x - mask.rect.left;
    let my = y - mask.rect.top;
    if mx < 0 || my < 0 || mx >= mw || my >= mh {
        return None;
    }
    let (_, samples) = layer
        .source_channels
        .as_ref()?
        .planes
        .iter()
        .find(|(id, _)| *id == -2)?;
    if samples.len() != mw as usize * mh as usize {
        return None;
    }
    sample_unit(samples, my as usize * mw as usize + mx as usize)
}

/// Apply a decoded adjustment to the running backdrop, then gate the result by
/// the layer's mask/opacity/blend (the reference applies the adjustment to the
/// backdrop and blends the adjusted result back).
pub(crate) fn composite_adjustment(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    adjustment: &Adjustment,
) {
    // Fill content is generative: it adds color inside the layer's rect instead
    // of transforming the backdrop, so it takes the normal-content path.
    if let Adjustment::SolidFill(rgba) = adjustment {
        crate::fill::composite_solid_fill(canvas, layer, doc, *rgba);
        return;
    }
    if let Adjustment::GradientFill(params) = adjustment {
        crate::fill::composite_gradient_fill(canvas, layer, doc, params);
        return;
    }
    if let Adjustment::PatternFill(params) = adjustment {
        crate::fill::composite_pattern_fill(canvas, layer, doc, params);
        return;
    }
    let n = canvas.w * canvas.h;
    if n == 0 {
        return;
    }
    // A document read at 16/32 applies the adjustment to the f32 accumulator
    // at native precision. An adjustment outside `apply_native`'s set falls
    // back to the 8-bit path below.
    if doc.source_depth.is_some() {
        match apply_native_rgb(canvas, adjustment) {
            Ok(adjusted) => {
                gate_adjusted(canvas, layer, doc, &adjusted);
                return;
            }
            Err(AdjustError::Unsupported(_)) => {}
            Err(_) => return, // invalid parameters: no-op, matching the 8-bit path
        }
    }
    let mut buf = PixelBuffer::new(canvas.w as u32, canvas.h as u32, 3);
    for (i, p) in canvas.px.iter().enumerate() {
        buf.data[i] = to_u8(p.r);
        buf.data[n + i] = to_u8(p.g);
        buf.data[2 * n + i] = to_u8(p.b);
    }
    if pictura_adjust::apply(adjustment, &mut buf).is_err() {
        return; // invalid/unsupported parameters: no-op, never an error
    }
    let w = canvas.w;
    let ox = canvas.ox;
    let oy = canvas.oy;
    let x0 = canvas.x0();
    let x1 = canvas.x1();
    let y0 = canvas.y0();
    let y1 = canvas.y1();
    let buf = &buf;
    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        let base = (y - oy) as usize * w;
        for x in x0..x1 {
            // Source coverage is the backdrop's own alpha: an adjustment adds no
            // content where the backdrop is transparent.
            let local = (x - ox) as usize;
            let i = base + local;
            let backdrop_alpha = row[local].a;
            if backdrop_alpha <= 0.0 {
                continue;
            }
            let cs = [
                buf.data[i] as f32 / 255.0,
                buf.data[n + i] as f32 / 255.0,
                buf.data[2 * n + i] as f32 / 255.0,
            ];
            blend_into_row(
                row,
                cover.as_deref_mut(),
                ox,
                layer,
                doc,
                x,
                y,
                cs,
                backdrop_alpha,
            );
        }
    });
}

/// Apply `adjustment` to the canvas RGB at native precision through
/// `apply_native` on an `f32` sample store, returning the adjusted color per
/// pixel. `canvas` (the original backdrop) is left untouched so the layer gate
/// still sees it. Alpha is untouched.
fn apply_native_rgb(
    canvas: &Canvas,
    adjustment: &Adjustment,
) -> Result<Vec<[f32; 3]>, AdjustError> {
    let n = canvas.w * canvas.h;
    let mut samples = Vec::with_capacity(n * 3);
    for p in &canvas.px {
        samples.push(p.r);
    }
    for p in &canvas.px {
        samples.push(p.g);
    }
    for p in &canvas.px {
        samples.push(p.b);
    }
    let mut store = Samples::F32(samples);
    pictura_adjust::apply_native(adjustment, &mut store, canvas.w, canvas.h, 3)?;
    let Samples::F32(v) = store else {
        return Err(AdjustError::InvalidParams(
            "native store changed type".into(),
        ));
    };
    Ok((0..n).map(|i| [v[i], v[n + i], v[2 * n + i]]).collect())
}

/// Gate the adjusted color through the layer's mask/opacity/blend, blending it
/// over the original backdrop still held in `canvas.px`.
fn gate_adjusted(canvas: &mut Canvas, layer: &Layer, doc: &Document, adjusted: &[[f32; 3]]) {
    let w = canvas.w;
    let ox = canvas.ox;
    let oy = canvas.oy;
    let x0 = canvas.x0();
    let x1 = canvas.x1();
    let y0 = canvas.y0();
    let y1 = canvas.y1();
    for_each_row(canvas, y0, y1, |y, row, mut cover| {
        let base = (y - oy) as usize * w;
        for x in x0..x1 {
            let local = (x - ox) as usize;
            let i = base + local;
            let backdrop_alpha = row[local].a;
            if backdrop_alpha <= 0.0 {
                continue;
            }
            blend_into_row(
                row,
                cover.as_deref_mut(),
                ox,
                layer,
                doc,
                x,
                y,
                adjusted[i],
                backdrop_alpha,
            );
        }
    });
}

/// The document's composite at its recorded source depth: `Samples::U16` at 16
/// and `Samples::F32` at 32, in planar RGBA order with straight alpha clamped to
/// `[0, 1]`. `None` for a document with no recorded 16/32-bit source depth.
pub fn composite_native(doc: &Document) -> Option<Samples> {
    let depth = doc.source_depth?;
    if !matches!(depth, BitDepth::Sixteen | BitDepth::ThirtyTwo) {
        return None;
    }
    let mut canvas = Canvas::new(doc.width as usize, doc.height as usize);
    composite_layers(&mut canvas, doc);
    Some(emit_native(&canvas, depth))
}

fn emit_native(canvas: &Canvas, depth: BitDepth) -> Samples {
    let n = canvas.w * canvas.h;
    match depth {
        BitDepth::Sixteen => {
            let mut out = vec![0u16; n * 4];
            for (i, p) in canvas.px.iter().enumerate() {
                out[i] = to_u16(p.r);
                out[n + i] = to_u16(p.g);
                out[2 * n + i] = to_u16(p.b);
                out[3 * n + i] = to_u16(p.a);
            }
            Samples::U16(out)
        }
        _ => {
            let mut out = vec![0f32; n * 4];
            for (i, p) in canvas.px.iter().enumerate() {
                out[i] = p.r.clamp(0.0, 1.0);
                out[n + i] = p.g.clamp(0.0, 1.0);
                out[2 * n + i] = p.b.clamp(0.0, 1.0);
                out[3 * n + i] = p.a.clamp(0.0, 1.0);
            }
            Samples::F32(out)
        }
    }
}

fn to_u16(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 65535.0).round() as u16
}

/// Bytes per sample for a native store; `None` for the 8-bit store, which the
/// refresh gate excludes.
fn sample_stride(samples: &Samples) -> Option<usize> {
    match samples {
        Samples::U16(_) => Some(2),
        Samples::F32(_) => Some(4),
        Samples::U8(_) => None,
    }
}

/// Recompute a dirty high-depth document's composite at its source depth and
/// splice the native color planes into the retained source planes, so a save
/// re-emits native samples instead of widening the 8-bit working composite.
///
/// Returns `false` without mutation for a document outside the gate: no recorded
/// 16/32-bit source depth, a converted source mode (whose retained planes are
/// source-mode, not working RGB), no layers, or no retained source planes. Alpha
/// and extra planes, and `doc.channels`, are left untouched.
pub fn refresh_native_composite(doc: &mut Document) -> bool {
    if !matches!(
        doc.source_depth,
        Some(BitDepth::Sixteen | BitDepth::ThirtyTwo)
    ) || doc.source_mode.is_some()
        || doc.layers.is_empty()
    {
        return false;
    }
    let Some(store) = doc.source_planes.as_ref() else {
        return false;
    };
    let store_depth = store.depth;
    let Some(sample_size) = sample_stride(&store.samples) else {
        return false;
    };
    let mut bytes = store.samples.to_bytes();
    let Some(native) = composite_native(doc) else {
        return false;
    };

    let plane = doc.width as usize * doc.height as usize;
    let color_channels = doc.mode.color_channels() as usize;
    let cut = color_channels * plane * sample_size;
    if bytes.len() < cut {
        return false;
    }
    let native_bytes = native.to_bytes();
    bytes[..cut].copy_from_slice(&native_bytes[..cut]);
    doc.source_planes.as_mut().expect("checked above").samples =
        Samples::from_bytes(&bytes, store_depth);

    let narrowed = native.narrow_to_u8();
    doc.composite = if doc.mode == ColorMode::Rgb {
        PixelBuffer {
            width: doc.width,
            height: doc.height,
            channels: 4,
            data: narrowed.into(),
        }
    } else {
        PixelBuffer {
            width: doc.width,
            height: doc.height,
            channels: color_channels as u8,
            data: narrowed[..plane].to_vec().into(),
        }
    };
    true
}
