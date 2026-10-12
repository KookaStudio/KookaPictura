//! M6-C: destructive filter application to a pixel layer's color channels.
//!
//! Contract: `docs/dev/m6c-filter-integration.md`. The filter runs on channels
//! `0,1,2`, and on the transparency channel (`-1`) unless the layer is
//! transparency-locked. [`apply_filter_region`] bounds the work to a document
//! rectangle for viewport-sized previews.

use pictura_adjust::Adjustment;
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, Layer, LayerMask, PixelBuffer, PsdRect,
};
use pictura_filters::{Filter, FilterError, SmartBlurMode};
use rayon::prelude::*;

use crate::channel;

/// Apply a destructive `filter` to a pixel layer's color channels (0,1,2),
/// gated by an optional document-coordinate coverage `mask` (`None` = full
/// frame). An unlocked layer's transparency is filtered too; a transparency
/// lock preserves `-1` and leaves clear pixels untouched.
///
/// When `gpu_enabled` is set the filter runs through
/// [`crate::gpu_filter::apply_filter_active`], which uses the GPU when a kernel
/// exists and falls back to the CPU oracle otherwise.
pub fn apply_filter(
    layer: &mut Layer,
    filter: &Filter,
    mask: Option<&LayerMask>,
    gpu_enabled: bool,
) -> Result<(), FilterError> {
    let region = layer.rect;
    apply_filter_region(layer, filter, mask, gpu_enabled, region)
}

/// As [`apply_filter`], but only over `region` (clamped to the layer rect).
///
/// A neighborhood filter reads up to its support outside `region`, where the
/// source is clamped as if the region were the whole layer. A preview that
/// passes a viewport-sized region expanded by a support-sized apron is correct
/// across the viewport and cheaper than filtering the whole layer.
pub fn apply_filter_region(
    layer: &mut Layer,
    filter: &Filter,
    mask: Option<&LayerMask>,
    gpu_enabled: bool,
    region: PsdRect,
) -> Result<(), FilterError> {
    apply_op_region(layer, Op::Filter(filter, gpu_enabled), mask, region)
}

/// Apply a destructive Image > Adjustments `adjustment` to a pixel layer's
/// colour channels, gated by an optional coverage `mask`, over `region`
/// (clamped to the layer rect). Unlike a filter it leaves transparency alone,
/// and a transparency lock keeps clear pixels untouched.
pub fn apply_adjustment_region(
    layer: &mut Layer,
    adjustment: &Adjustment,
    mask: Option<&LayerMask>,
    region: PsdRect,
) -> Result<(), FilterError> {
    apply_op_region(layer, Op::Adjustment(adjustment), mask, region)
}

/// What [`apply_op_region`] runs on the working buffer.
#[derive(Clone, Copy)]
enum Op<'a> {
    Filter(&'a Filter, bool),
    Adjustment(&'a Adjustment),
}

/// Whether `filter` leaves an unlocked layer's transparency exactly as found.
///
/// The unlocked-layer alpha pass reruns the kernel over a grey copy of the
/// alpha plane. That is right only for a kernel that moves, spreads, or ranks
/// samples the same way whatever their value (blurs, distortions, rank
/// filters, cell averages): a flat 255 plane comes back 255 and the layer edge
/// moves with the colour. Every other kernel reads absolute values, replaces
/// the planes, or adds onto them (Solarize, Clouds, Lens Flare, Find Edges,
/// Add Noise, the artistic families), so rerunning it would rewrite opacity;
/// those leave alpha bit-identical.
/// ponytail: an allowlist on the call side. The real fix folds alpha into the
/// working buffer so each kernel receives the fourth plane and decides itself.
pub fn filter_preserves_opacity(filter: &Filter) -> bool {
    !matches!(
        filter,
        Filter::GaussianBlur { .. }
            | Filter::BoxBlur { .. }
            | Filter::ShapeBlur { .. }
            | Filter::MotionBlur { .. }
            | Filter::RadialBlur { .. }
            | Filter::Average
            | Filter::Blur
            | Filter::BlurMore
            | Filter::SurfaceBlur { .. }
            | Filter::SmartBlur {
                mode: SmartBlurMode::Normal,
                ..
            }
            | Filter::LensBlur { .. }
            | Filter::FieldBlur { .. }
            | Filter::IrisBlur { .. }
            | Filter::Sharpen
            | Filter::SharpenMore
            | Filter::SharpenEdges
            | Filter::UnsharpMask { .. }
            | Filter::SmartSharpen { .. }
            | Filter::Median { .. }
            | Filter::Despeckle
            | Filter::Maximum { .. }
            | Filter::Minimum { .. }
            | Filter::DustAndScratches { .. }
            | Filter::Offset { .. }
            | Filter::Mosaic { .. }
            | Filter::Crystallize { .. }
            | Filter::Fragment
            | Filter::Twirl { .. }
            | Filter::Pinch { .. }
            | Filter::Spherize { .. }
            | Filter::Ripple { .. }
            | Filter::Wave { .. }
            | Filter::PolarCoordinates { .. }
            | Filter::Shear { .. }
            | Filter::Displace { .. }
            | Filter::Liquify { .. }
            | Filter::ZigZag { .. }
            | Filter::OceanRipple { .. }
            | Filter::Glass { .. }
    )
}

/// Kernels whose row-parallel CPU path beats the GPU round trip, so a layer
/// filter skips the GPU for them. Measured at 8192² on an integrated AMD 780M
/// (#152): uploading and reading back cost about what the CPU takes for the
/// whole kernel (Gaussian radius 4: GPU 0.91 s, CPU 0.46 s; Median radius 10:
/// GPU 11.2 s, CPU 3.4 s). Box, Motion and Surface Blur and Oil Paint stay on
/// the GPU, which still wins there.
/// ponytail: one machine's crossover; re-measure on a discrete GPU.
pub(crate) fn cpu_outruns_gpu(filter: &Filter) -> bool {
    matches!(
        filter,
        Filter::GaussianBlur { .. }
            | Filter::Blur
            | Filter::BlurMore
            | Filter::Sharpen
            | Filter::SharpenMore
            | Filter::UnsharpMask { .. }
            | Filter::HighPass { .. }
            | Filter::Maximum { .. }
            | Filter::Minimum { .. }
            | Filter::Median { .. }
            | Filter::Custom { .. }
    )
}

/// The kernel the alpha pass runs for `filter`. Offset's exposed area takes
/// the Background colour, which CS6's "Set to Background" paints opaque, so
/// the alpha pass fills it with 255 rather than the colour's grey.
/// ponytail: CS6's "Set to Transparent" has no slot in `Filter::Offset` yet.
fn alpha_kernel(filter: &Filter) -> std::borrow::Cow<'_, Filter> {
    match *filter {
        Filter::Offset {
            horizontal,
            vertical,
            wrap,
            ..
        } => std::borrow::Cow::Owned(Filter::Offset {
            horizontal,
            vertical,
            wrap,
            background: [255; 3],
        }),
        // Grain and the specular bloom are colour effects; the alpha pass keeps
        // the depth-driven blur only.
        Filter::LensBlur { .. } => {
            let mut alpha = filter.clone();
            if let Filter::LensBlur {
                specular_brightness,
                noise_amount,
                ..
            } = &mut alpha
            {
                *specular_brightness = 0;
                *noise_amount = 0;
            }
            std::borrow::Cow::Owned(alpha)
        }
        // The bokeh lift and the grain are colour effects; the alpha pass
        // keeps the pin-driven blur only.
        Filter::FieldBlur { .. } | Filter::IrisBlur { .. } => {
            let mut alpha = filter.clone();
            if let Filter::FieldBlur { bokeh, noise, .. } | Filter::IrisBlur { bokeh, noise, .. } =
                &mut alpha
            {
                bokeh.enabled = false;
                noise.enabled = false;
            }
            std::borrow::Cow::Owned(alpha)
        }
        _ => std::borrow::Cow::Borrowed(filter),
    }
}

fn apply_op_region(
    layer: &mut Layer,
    op: Op,
    mask: Option<&LayerMask>,
    region: PsdRect,
) -> Result<(), FilterError> {
    let lrect = layer.rect;
    if lrect.width() <= 0 || lrect.height() <= 0 {
        return Ok(());
    }
    if layer_pixel_locked(layer) {
        return Err(FilterError::Locked);
    }
    let clip = intersect_rect(region, lrect);
    if clip.width() <= 0 || clip.height() <= 0 {
        return Ok(());
    }
    let lw = lrect.width() as usize;
    let full_n = lw * lrect.height() as usize;
    let cw = clip.width() as usize;
    let ch = clip.height() as usize;
    let n = cw * ch;
    let ox = (clip.left - lrect.left) as usize;
    let oy = (clip.top - lrect.top) as usize;

    // Validate the layer's full color planes (data length == layer w*h).
    let c0 = channel(layer, 0)
        .ok_or_else(|| FilterError::InvalidParams("layer is missing color channel 0".into()))?;
    if c0.len() != full_n {
        return Err(FilterError::InvalidParams(format!(
            "channel 0 has {} samples, expected {full_n}",
            c0.len()
        )));
    }
    // A Grayscale layer carries only channel 0; a color layer carries all three.
    // A mix (one of 1/2 present) is malformed and refused, as before.
    let grayscale = match (channel(layer, 1), channel(layer, 2)) {
        (Some(c1), Some(c2)) => {
            for (id, data) in [(1, c1), (2, c2)] {
                if data.len() != full_n {
                    return Err(FilterError::InvalidParams(format!(
                        "channel {id} has {} samples, expected {full_n}",
                        data.len()
                    )));
                }
            }
            false
        }
        (None, None) => true,
        _ => {
            return Err(FilterError::InvalidParams(
                "layer is missing a color channel".into(),
            ));
        }
    };

    // Crop the clip out of the layer into a clip-sized working buffer.
    let sources: [&[u8]; 3] = if grayscale {
        [c0, c0, c0]
    } else {
        [c0, channel(layer, 1).unwrap(), channel(layer, 2).unwrap()]
    };
    let mut buf = PixelBuffer::new(cw as u32, ch as u32, 3);
    buf.data
        .as_mut_slice()
        .par_chunks_mut(cw)
        .enumerate()
        .for_each(|(i, row)| {
            let from = (oy + i % ch) * lw + ox;
            row.copy_from_slice(&sources[i / ch][from..from + cw]);
        });

    // A transparency lock preserves the alpha plane exactly and leaves fully
    // transparent pixels untouched; only opaque pixels take the filter colour.
    // An unlocked layer takes a moving kernel on its transparency too (CS6):
    // run it over a grey copy of the alpha plane so the layer edge smears or
    // moves exactly as the colour does (see `filter_preserves_opacity`).
    // ponytail: the alpha pass reruns the kernel on the CPU; fold it into the
    // working buffer once the kernels accept a fourth plane.
    let alpha_locked = layer_transparency_locked(layer);
    let alpha_original: Option<Vec<u8>> = channel(layer, -1).map(|data| {
        let mut crop = vec![0u8; n];
        crop.par_chunks_mut(cw).enumerate().for_each(|(row, out)| {
            let from = (oy + row) * lw + ox;
            out.copy_from_slice(&data[from..from + cw]);
        });
        crop
    });
    // Without a mask or a transparency lock every pixel takes the filtered
    // value whole, so the pre-filter copy is kept only for the blend.
    let orig = (mask.is_some() || alpha_locked).then(|| buf.clone());
    match op {
        Op::Filter(filter, gpu_enabled) => {
            let gpu = gpu_enabled && !cpu_outruns_gpu(filter);
            crate::gpu_filter::apply_filter_active(filter, &mut buf, gpu)?;
        }
        Op::Adjustment(adjustment) => pictura_adjust::apply(adjustment, &mut buf)
            .map_err(|error| FilterError::InvalidParams(error.to_string()))?,
    }

    let filtered_alpha = match op {
        Op::Filter(filter, _) if !alpha_locked && !filter_preserves_opacity(filter) => {
            alpha_original
                .as_ref()
                // A moving kernel maps a flat opaque plane to itself
                // (`moving_kernels_map_opaque_to_opaque`), so an opaque crop
                // needs no rerun.
                .filter(|alpha| !alpha.par_iter().all(|&a| a == 255))
                .map(|alpha| -> Result<Vec<u8>, FilterError> {
                    let mut gray = PixelBuffer::new(cw as u32, ch as u32, 3);
                    for c in 0..3usize {
                        gray.data[c * n..c * n + n].copy_from_slice(alpha);
                    }
                    pictura_filters::apply(&alpha_kernel(filter), &mut gray)?;
                    Ok(gray.data[0..n].to_vec())
                })
                .transpose()?
        }
        _ => None,
    };

    let at = Placement {
        lw,
        ox,
        oy,
        cw,
        left: clip.left,
        top: clip.top,
    };
    // A Grayscale result is a single plane; a color result has all three.
    let write_planes: &[i16] = if grayscale { &[0] } else { &[0, 1, 2] };
    for &c in write_planes {
        let plane = c as usize * n;
        let blend = orig.as_ref().map(|orig| Blend {
            source: &orig.data[plane..plane + n],
            mask,
            keep_clear: if alpha_locked {
                alpha_original.as_deref()
            } else {
                None
            },
        });
        let out = channel_mut(layer, c).expect("channel presence validated above");
        write_back(out, &buf.data[plane..plane + n], blend, at);
    }

    // Write the filtered transparency (unlocked layers only).
    if let (Some(source), Some(filtered)) = (alpha_original.as_ref(), filtered_alpha.as_ref()) {
        let blend = mask.is_some().then_some(Blend {
            source,
            mask,
            keep_clear: None,
        });
        let out = channel_mut(layer, -1).expect("alpha presence checked above");
        write_back(out, filtered, blend, at);
    }
    Ok(())
}

/// Where a clip-sized result lands in a layer-wide plane: the layer width,
/// the clip's offset and width inside the layer, and its document origin.
#[derive(Clone, Copy)]
struct Placement {
    lw: usize,
    ox: usize,
    oy: usize,
    cw: usize,
    left: i32,
    top: i32,
}

/// How [`write_back`] mixes a filtered plane into the layer: each pixel moves
/// from `source` toward the filtered value by the `mask` coverage, and pixels
/// clear in `keep_clear` (a transparency-locked alpha crop) keep their value.
#[derive(Clone, Copy)]
struct Blend<'a> {
    source: &'a [u8],
    mask: Option<&'a LayerMask>,
    keep_clear: Option<&'a [u8]>,
}

/// Write the clip-sized `filtered` plane into the layer-wide `out`, row-parallel.
/// Without a `blend` every pixel takes the filtered value, which is what the
/// blend yields at full coverage.
fn write_back(out: &mut [u8], filtered: &[u8], blend: Option<Blend>, at: Placement) {
    let Placement {
        lw,
        ox,
        oy,
        cw,
        left,
        top,
    } = at;
    let rows = filtered.len() / cw;
    out[oy * lw..(oy + rows) * lw]
        .par_chunks_mut(lw)
        .enumerate()
        .for_each(|(ly, row)| {
            let dst = &mut row[ox..ox + cw];
            let src = &filtered[ly * cw..(ly + 1) * cw];
            let Some(blend) = blend else {
                dst.copy_from_slice(src);
                return;
            };
            for lx in 0..cw {
                let i = ly * cw + lx;
                if blend.keep_clear.is_some_and(|a| a[i] == 0) {
                    continue;
                }
                let cov = coverage(blend.mask, left + lx as i32, top + ly as i32);
                if cov == 0 {
                    continue;
                }
                let o = blend.source[i] as f64;
                let f = src[lx] as f64;
                dst[lx] = (o + (f - o) * cov as f64 / 255.0).round().clamp(0.0, 255.0) as u8;
            }
        });
}

fn intersect_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.max(b.top),
        left: a.left.max(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.min(b.right),
    }
}

/// A conservative document-pixel margin a preview must keep around a viewport
/// so a neighborhood filter's result is exact across the visible area.
pub fn preview_apron(filter: &Filter) -> i32 {
    let radius = match filter {
        Filter::GaussianBlur { radius }
        | Filter::UnsharpMask { radius, .. }
        | Filter::HighPass { radius } => radius.ceil(),
        // Smart Sharpen iterates a halo reaching 2.4·radius (3σ at
        // σ = 0.8·radius); later iterations reach further but weigh less.
        Filter::SmartSharpen { radius, .. } => (3.0 * radius).ceil(),
        Filter::HdrToning(p) => (3.0 * p.radius).ceil(),
        // One more pixel for the edge test against the 4-neighbours.
        Filter::SmartBlur { radius, .. } => radius.ceil() + 1.0,
        Filter::BoxBlur { radius }
        | Filter::ShapeBlur { radius, .. }
        | Filter::SurfaceBlur { radius, .. }
        | Filter::LensBlur { radius, .. }
        | Filter::Median { radius }
        | Filter::Maximum { radius }
        | Filter::Minimum { radius } => f64::from(*radius),
        Filter::MotionBlur { distance, .. } => f64::from(*distance),
        Filter::FieldBlur { pins, .. } => pins.iter().map(|pin| pin.blur).fold(0.0, f64::max),
        Filter::IrisBlur { pins, .. } => pins.iter().map(|pin| pin.blur).fold(0.0, f64::max),
        Filter::RadialBlur { amount, .. } => *amount,
        Filter::Offset {
            horizontal,
            vertical,
            ..
        } => f64::from(horizontal.abs().max(vertical.abs())),
        // Distort, render, texture, and cell filters can scatter or displace
        // widely; keep a generous margin.
        _ => 128.0,
    };
    (radius.ceil() as i32).clamp(8, 512)
}

fn channel_mut(layer: &mut Layer, id: i16) -> Option<&mut [u8]> {
    layer
        .channels
        .iter_mut()
        .find(|c| c.id == id)
        .map(|c| c.data.as_mut_slice())
}

/// Mask coverage at a document pixel. `255` when there is no mask or the mask
/// is disabled; outside the mask rect (or with no decoded data) the mask's
/// `default_color` applies.
pub(crate) fn coverage(mask: Option<&LayerMask>, x: i32, y: i32) -> u8 {
    let Some(mask) = mask else {
        return 255;
    };
    if mask.disabled {
        return 255;
    }
    let (mw, mh) = (mask.rect.width(), mask.rect.height());
    let (mx, my) = (x - mask.rect.left, y - mask.rect.top);
    if mw <= 0 || mh <= 0 || mx < 0 || my < 0 || mx >= mw || my >= mh {
        return mask.default_color;
    }
    let Some(data) = &mask.data else {
        return mask.default_color;
    };
    data.get(my as usize * mw as usize + mx as usize)
        .copied()
        .unwrap_or(mask.default_color)
}

#[cfg(test)]
mod tests;
