//! M6-C: destructive filter application to a pixel layer's color channels.
//!
//! Contract: `docs/dev/m6c-filter-integration.md`. The filter runs on channels
//! `0,1,2`, and on the transparency channel (`-1`) unless the layer is
//! transparency-locked. [`apply_filter_region`] bounds the work to a document
//! rectangle for viewport-sized previews.

use pictura_adjust::{Adjustment, ColorBalanceMode};
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, Coverage, Layer, PixelBuffer, PsdRect,
};
use pictura_filters::{Filter, FilterError};
use rayon::prelude::*;

use crate::channel;

#[cfg(feature = "gpu")]
fn apply_kernel_filter(
    filter: &Filter,
    buf: &mut PixelBuffer,
    gpu: bool,
) -> Result<(), FilterError> {
    crate::gpu_filter::apply_filter_active(filter, buf, gpu).map(|_| ())
}

#[cfg(not(feature = "gpu"))]
fn apply_kernel_filter(
    filter: &Filter,
    buf: &mut PixelBuffer,
    _gpu: bool,
) -> Result<(), FilterError> {
    pictura_filters::apply(filter, buf)
}

/// Apply a destructive `filter` to a pixel layer's color channels (0,1,2),
/// gated by an optional document-coordinate coverage `mask` (`None` = full
/// frame). An unlocked layer's transparency is filtered too; a transparency
/// lock preserves `-1` and leaves clear pixels untouched.
///
/// When `gpu_enabled` is set the filter runs through the GPU kernels when the
/// build carries them (`gpu_filter::apply_filter_active`, gated on the `gpu`
/// feature) and falls back to the CPU oracle otherwise.
pub fn apply_filter(
    layer: &mut Layer,
    filter: &Filter,
    mask: Option<&Coverage>,
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
    mask: Option<&Coverage>,
    gpu_enabled: bool,
    region: PsdRect,
) -> Result<(), FilterError> {
    // A clipped Liquify mesh confines itself to the active selection's
    // bounding rectangle, which CS6 processes whole; the coverage mask would
    // narrow that to the selection's shape.
    let mask = match filter {
        Filter::Liquify { mesh } if mesh.clip().is_some() => None,
        _ => mask,
    };
    apply_op_region(
        layer,
        Op::Filter(filter, gpu_enabled),
        mask,
        region,
        None,
        ColorBalanceMode::Rgb,
    )
}

/// Apply a destructive Image > Adjustments `adjustment` to a pixel layer's
/// colour channels, gated by an optional coverage `mask`, over `region`
/// (clamped to the layer rect). Unlike a filter it leaves transparency alone,
/// and a transparency lock keeps clear pixels untouched.
pub fn apply_adjustment_region(
    layer: &mut Layer,
    adjustment: &Adjustment,
    mask: Option<&Coverage>,
    region: PsdRect,
) -> Result<(), FilterError> {
    apply_adjustment_region_in_mode(layer, adjustment, mask, region, None, ColorBalanceMode::Rgb)
}

/// As [`apply_adjustment_region`], with the document's CMYK state so a Channel
/// Mixer mixes ink values, and its visible mode so Color Balance maps into Lab
/// or CMYK. `None` / `Rgb` keep the RGB path.
pub fn apply_adjustment_region_in_mode(
    layer: &mut Layer,
    adjustment: &Adjustment,
    mask: Option<&Coverage>,
    region: PsdRect,
    cmyk: Option<&crate::channel_mixer_cmyk::CmykMix>,
    color_balance: ColorBalanceMode,
) -> Result<(), FilterError> {
    apply_op_region(
        layer,
        Op::Adjustment(adjustment),
        mask,
        region,
        cmyk,
        color_balance,
    )
}

/// Resolve retained storage and document-depth support for a local adjustment.
pub fn adjustment_native_target(
    doc: &pictura_core::Document,
    path: &str,
    adjustment: &Adjustment,
) -> Result<bool, FilterError> {
    if !local_shadows_highlights(adjustment) {
        return Ok(false);
    }
    use pictura_core::BitDepth;
    match crate::document_bit_depth(doc) {
        BitDepth::Eight => Ok(false),
        BitDepth::Sixteen => crate::apply_image_native(doc, path),
        _ => Err(FilterError::Unsupported(
            "Shadows/Highlights supports 8 and 16 bits per channel".into(),
        )),
    }
}

/// Apply in authoritative native storage when the document owns the planes.
pub fn apply_adjustment_region_with_native(
    layer: &mut Layer,
    adjustment: &Adjustment,
    mask: Option<&Coverage>,
    region: PsdRect,
    cmyk: Option<&crate::channel_mixer_cmyk::CmykMix>,
    mode: ColorBalanceMode,
    native: bool,
) -> Result<(), FilterError> {
    if native && local_shadows_highlights(adjustment) {
        return crate::shadow_highlight::apply_native_region(
            layer, adjustment, mask, region, mode,
        );
    }
    apply_adjustment_region_in_mode(layer, adjustment, mask, region, cmyk, mode)
}

fn local_shadows_highlights(adjustment: &Adjustment) -> bool {
    matches!(adjustment, Adjustment::ShadowsHighlights(params) if params.local.is_some())
}

/// What [`apply_op_region`] runs on the working buffer.
#[derive(Clone, Copy)]
enum Op<'a> {
    Filter(&'a Filter, bool),
    Adjustment(&'a Adjustment),
}

fn apply_op_region(
    layer: &mut Layer,
    op: Op,
    mask: Option<&Coverage>,
    region: PsdRect,
    cmyk: Option<&crate::channel_mixer_cmyk::CmykMix>,
    color_balance: ColorBalanceMode,
) -> Result<(), FilterError> {
    let lrect = layer.rect;
    // A lock outranks the empty-rect no-op: a locked layer must refuse the
    // filter, not report a successful no-op (#1208).
    if layer_pixel_locked(layer) {
        return Err(FilterError::Locked);
    }
    if lrect.width() <= 0 || lrect.height() <= 0 {
        return Ok(());
    }
    let Some(clip) = region.intersect(lrect) else {
        return Ok(());
    };
    let local = matches!(op, Op::Adjustment(adjustment) if local_shadows_highlights(adjustment));
    let source = if local { lrect } else { clip };
    let sw = source.width() as usize;
    let sh = source.height() as usize;
    let sx = (source.left - lrect.left) as usize;
    let sy = (source.top - lrect.top) as usize;
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

    // Crop the clip out of the layer into a clip-sized working buffer. The
    // layer's transparency is folded in as a fourth plane whenever the op can
    // touch it (a filter on an unlocked layer that has alpha), so each kernel
    // receives the alpha plane and decides it: a moving kernel filters it with
    // the colour, a value kernel leaves it bit-identical. There is no second
    // pass and no call-site allowlist — the plane counts and write-backs below
    // are the whole protocol.
    let alpha_locked = layer_transparency_locked(layer);
    let alpha_original: Option<Vec<u8>> = channel(layer, -1).map(|data| {
        let mut crop = vec![0u8; n];
        crop.par_chunks_mut(cw).enumerate().for_each(|(row, out)| {
            let from = (oy + row) * lw + ox;
            out.copy_from_slice(&data[from..from + cw]);
        });
        crop
    });
    let fold_alpha = matches!(op, Op::Filter(..)) && !alpha_locked && alpha_original.is_some();
    let sources: [&[u8]; 3] = if grayscale {
        [c0, c0, c0]
    } else {
        [c0, channel(layer, 1).unwrap(), channel(layer, 2).unwrap()]
    };
    let mut buf = PixelBuffer::new(sw as u32, sh as u32, if fold_alpha { 4 } else { 3 });
    buf.data
        .as_mut_slice()
        .par_chunks_mut(sw)
        .enumerate()
        .for_each(|(i, row)| {
            let r = i % sh;
            let from = (sy + r) * lw + sx;
            if i / sh < 3 {
                row.copy_from_slice(&sources[i / sh][from..from + sw]);
            } else {
                row.copy_from_slice(
                    &alpha_original.as_ref().expect("folded")[r * cw..(r + 1) * cw],
                );
            }
        });

    // A transparency lock preserves the alpha plane exactly and leaves fully
    // transparent pixels untouched; only opaque pixels take the filter colour.
    // Without a mask or a transparency lock every pixel takes the filtered
    // value whole, so the pre-filter copy is kept only for the blend.
    let crop_to_clip = |buffer: PixelBuffer| {
        if source == clip {
            return buffer;
        }
        let left = (clip.left - source.left) as usize;
        let top = (clip.top - source.top) as usize;
        let mut cropped = PixelBuffer::new(cw as u32, ch as u32, buffer.channels);
        for (i, row) in cropped.data.as_mut_slice().chunks_exact_mut(cw).enumerate() {
            let from = (i / ch) * sw * sh + (top + i % ch) * sw + left;
            row.copy_from_slice(&buffer.data[from..from + cw]);
        }
        cropped
    };
    let orig = (mask.is_some() || alpha_locked).then(|| crop_to_clip(buf.clone()));
    // Auto's histograms count only selected pixels. The coverage plane is in
    // clip coordinates; any nonzero coverage selects the pixel.
    let coverage_plane = if !local && matches!(op, Op::Adjustment(..)) {
        mask.map(|mask| {
            let mut plane = vec![0u8; n];
            plane.par_chunks_mut(cw).enumerate().for_each(|(y, row)| {
                coverage_row(Some(mask), clip.left, clip.top + y as i32, row);
            });
            plane
        })
    } else {
        None
    };
    match op {
        Op::Filter(filter, gpu_enabled) => {
            let gpu = gpu_enabled && !filter.cpu_outruns_gpu();
            apply_kernel_filter(filter, &mut buf, gpu)?;
        }
        Op::Adjustment(adjustment) => {
            let ink = match (cmyk, adjustment) {
                (Some(mix), Adjustment::ChannelMixer(params)) if !params.monochrome => Some(mix),
                _ => None,
            };
            if let (Some(mix), Adjustment::ChannelMixer(params)) = (ink, adjustment) {
                if !crate::channel_mixer_cmyk::apply_planes(&mut buf, params, mix) {
                    return Ok(());
                }
            } else {
                pictura_adjust::apply_with_coverage_in_mode(
                    adjustment,
                    &mut buf,
                    coverage_plane.as_deref(),
                    if local && grayscale { ColorBalanceMode::Grayscale } else { color_balance },
                )
                .map_err(|error| FilterError::InvalidParams(error.to_string()))?
            }
        }
    }

    let buf = crop_to_clip(buf);
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

    // Write the transparency the kernel left in the fourth plane (unlocked
    // layers that had alpha; a value kernel leaves the plane bit-identical, so
    // this is a no-op write for those).
    if let (Some(source), true) = (alpha_original.as_ref(), fold_alpha) {
        let blend = mask.is_some().then_some(Blend {
            source,
            mask,
            keep_clear: None,
        });
        let out = channel_mut(layer, -1).expect("alpha presence checked above");
        write_back(out, &buf.data[3 * n..4 * n], blend, at);
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
    mask: Option<&'a Coverage>,
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
            let mut covs = vec![0u8; cw];
            coverage_row(blend.mask, left, top + ly as i32, &mut covs);
            for lx in 0..cw {
                let i = ly * cw + lx;
                if blend.keep_clear.is_some_and(|a| a[i] == 0) {
                    continue;
                }
                let cov = covs[lx];
                if cov == 0 {
                    continue;
                }
                let o = blend.source[i] as f64;
                let f = src[lx] as f64;
                dst[lx] = (o + (f - o) * cov as f64 / 255.0).round().clamp(0.0, 255.0) as u8;
            }
        });
}

fn channel_mut(layer: &mut Layer, id: i16) -> Option<&mut [u8]> {
    layer
        .channels_mut()
        .iter_mut()
        .find(|c| c.id == id)
        .map(|c| c.data.as_mut_slice())
}

/// Coverage of an optional editing constraint at a document pixel.
pub(crate) fn coverage(mask: Option<&Coverage>, x: i32, y: i32) -> u8 {
    mask.map_or(255, |mask| mask.sample(x, y))
}

pub(crate) fn coverage_row(mask: Option<&Coverage>, left: i32, y: i32, out: &mut [u8]) {
    match mask {
        Some(mask) => mask.read_row(left, y, out),
        None => out.fill(255),
    }
}

#[cfg(test)]
use pictura_core::LayerMask;

#[cfg(test)]
mod tests;
