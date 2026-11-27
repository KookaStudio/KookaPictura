//! Content-aware raster scaling. The seam model is an approximation, not an Adobe oracle.

mod pixels;
mod seams;

use pictura_core::{memory, Channel, Document, PsdRect, Samples, SourceChannels};
use std::sync::atomic::AtomicBool;

use super::kernel::Interpolation;
use super::paths::{resolve_path, resolve_path_mut};
use super::transform::{prepare_layer, transform_layer_with, write_layer, LayerOutput, LayerTransform};

#[cfg(test)]
pub(super) use seams::minimum_vertical_seam;

const MAX_SCRATCH: usize = 256 * 1024 * 1024;
const MAX_WORK: usize = 32_000_000;

/// Content-aware contribution and optional source-rect-sized alpha protection.
pub struct ContentAwareOptions<'a> {
    pub amount: u8,
    pub protect: Option<&'a [u8]>,
}

fn dimensions(rect: PsdRect) -> Option<(usize, usize)> {
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    if width <= 0 || height <= 0 || width > i64::from(i32::MAX) || height > i64::from(i32::MAX) {
        return None;
    }
    Some((usize::try_from(width).ok()?, usize::try_from(height).ok()?))
}

fn admitted(source: (usize, usize), target: (usize, usize), planes: usize, amount: u8) -> bool {
    let Some(area) = source.0.max(target.0).checked_mul(source.1.max(target.1)) else { return false; };
    let Some(steps) = source.0.abs_diff(target.0).checked_add(source.1.abs_diff(target.1)) else { return false; };
    if amount != 0 && area.checked_mul(steps).is_none_or(|work| work > MAX_WORK) { return false; }
    let Some(per_pixel) = planes.checked_mul(32).and_then(|n| n.checked_add(32)) else { return false; };
    let Some(bytes) = area.checked_mul(per_pixel) else { return false; };
    bytes <= MAX_SCRATCH && memory::governor().admit(bytes as u64, "content-aware scale")
}

/// Scale one raster layer into `dest`, leaving the document unchanged on refusal or cancellation.
/// Protection is indexed over the untransformed layer rectangle. The selected alpha channel
/// is read by the caller and never modified. This CPU approximation does not assert Adobe parity.
pub fn content_aware_scale(
    doc: &mut Document,
    path: &str,
    dest: PsdRect,
    options: ContentAwareOptions<'_>,
    cancel: &AtomicBool,
) -> bool {
    let Some(layer) = resolve_path(doc, path) else { return false; };
    if layer.is_group() || layer.has_adjustment_content() || layer.is_type() || layer.is_video_or_3d()
        || layer.smart_object().is_some() || layer.background || layer.mask.is_some()
        || layer.vector_mask.is_some() || layer.lock.bits() != 0
        || (doc.source.mode().is_some() && layer.source_channels.is_none())
        || options.amount > 100 || seams::cancelled(cancel)
    { return false; }
    let Some(source) = dimensions(layer.rect) else { return false; };
    let Some(target) = dimensions(dest) else { return false; };
    if dest.left.checked_sub(layer.rect.left).is_none() || dest.top.checked_sub(layer.rect.top).is_none() { return false; }
    let Some(area) = source.0.checked_mul(source.1) else { return false; };
    if options.protect.is_some_and(|p| p.len() != area) { return false; }
    let native_depth = doc.source.depth().unwrap_or(doc.depth);
    let native_count = layer.source_channels.as_ref().map_or_else(
        || if matches!(native_depth, pictura_core::BitDepth::Sixteen | pictura_core::BitDepth::ThirtyTwo) {
            layer.channels().len()
        } else { 0 },
        |s| s.planes.len(),
    );
    let count = layer.channels().len().saturating_add(native_count);
    if !admitted(source, target, count, options.amount) { return false; }
    if dest == layer.rect { return !seams::cancelled(cancel); }
    let Some(mut prepared) = prepare_layer(doc, path) else { return false; };
    let mut ordinary = doc.clone();
    let synthesize = prepared.native.is_none() && doc.source.mode().is_none()
        && matches!(native_depth, pictura_core::BitDepth::Sixteen | pictura_core::BitDepth::ThirtyTwo);
    if synthesize {
        let mut native = Vec::new();
        if native.try_reserve_exact(prepared.source_channels.len()).is_err() { return false; }
        for channel in &prepared.source_channels {
            let Some(sample) = pixels::widen(channel, native_depth, source.0, source.1) else { return false; };
            native.push((channel.id, sample));
        }
        let store = SourceChannels::new(native_depth, prepared.rect, native);
        let Some(layer) = resolve_path_mut(&mut ordinary, path) else { return false; };
        layer.source_channels = Some(store.clone());
        prepared.native = Some(store);
        if ordinary.source.depth().is_none() {
            let composite = ordinary.source.planes().clone();
            if ordinary.source.set_native(Some(native_depth), composite).is_err() { return false; }
        }
    }
    let src = prepared.rect;
    let transform = LayerTransform {
        scale_x: target.0 as f64 / source.0 as f64,
        scale_y: target.1 as f64 / source.1 as f64,
        angle_radians: 0.0,
        dx: (f64::from(dest.left) + f64::from(dest.right) - f64::from(src.left) - f64::from(src.right)) / 2.0,
        dy: (f64::from(dest.top) + f64::from(dest.bottom) - f64::from(src.top) - f64::from(src.bottom)) / 2.0,
    };
    if !transform_layer_with(&mut ordinary, path, transform, Interpolation::Bilinear) || seams::cancelled(cancel) { return false; }
    let Some(reference) = resolve_path(&ordinary, path) else { return false; };
    if reference.rect != dest { return false; }
    if options.amount == 0 || source == target {
        *doc = ordinary;
        return true;
    }
    let mut planes: Vec<Vec<f32>> = Vec::new();
    if planes.try_reserve_exact(count).is_err() { return false; }
    for channel in &prepared.source_channels {
        let Some(p) = pixels::u8_values(&channel.data, source.0, source.1) else { return false; };
        planes.push(p);
    }
    let native_offset = planes.len();
    if let Some(store) = &prepared.native {
        for (_, samples) in &store.planes {
            let Some(p) = pixels::native_values(samples, source.0, source.1) else { return false; };
            planes.push(p);
        }
    }
    let mut colors = Vec::new();
    if colors.try_reserve_exact(count).is_err() { return false; }
    if let Some(store) = &prepared.native {
        let mode = doc.source.mode().unwrap_or(doc.mode);
        for (index, (id, _)) in store.planes.iter().enumerate() {
            if *id >= 0 && *id < i16::from(mode.color_channels()) { colors.push(native_offset + index); }
        }
    }
    if colors.is_empty() {
        for (index, channel) in prepared.source_channels.iter().enumerate() {
            if channel.id >= 0 && channel.id < i16::from(doc.mode.color_channels().max(1)) { colors.push(index); }
        }
    }
    let mut protection = match options.protect {
        Some(mask) => {
            let Some(mut p) = seams::buffer(area, 0.0f32) else { return false; };
            for (to, from) in p.iter_mut().zip(mask) { *to = f32::from(*from) / 255.0; }
            Some(p)
        }
        None => None,
    };
    let mut width = source.0;
    if seams::resize_width(&mut planes, &mut protection, &colors, &mut width, source.1, target.0, cancel).is_none() { return false; }
    for plane in &mut planes {
        if seams::transpose(plane, width, source.1).is_none() { return false; }
    }
    if let Some(mask) = protection.as_mut() {
        if seams::transpose(mask, width, source.1).is_none() { return false; }
    }
    let mut height = source.1;
    if seams::resize_width(&mut planes, &mut protection, &colors, &mut height, width, target.1, cancel).is_none() { return false; }
    for plane in &mut planes {
        if seams::transpose(plane, height, width).is_none() { return false; }
    }
    if seams::cancelled(cancel) { return false; }
    let amount = f32::from(options.amount) / 100.0;
    let working_alpha_index = prepared.source_channels.iter().position(|c| c.id == -1);
    let working_alpha = match working_alpha_index {
        Some(index) => {
            let Some(reference_alpha) = reference.channels().iter().find(|c| c.id == -1) else { return false; };
            let Some(values) = pixels::u8_values(&reference_alpha.data, target.0, target.1) else { return false; };
            Some((index, values))
        }
        None => None,
    };
    let mut channels: Vec<Channel> = Vec::new();
    if channels.try_reserve_exact(prepared.source_channels.len()).is_err() { return false; }
    for (index, channel) in prepared.source_channels.iter().enumerate() {
        let Some(reference_channel) = reference.channels().iter().find(|c| c.id == channel.id) else { return false; };
        let alpha = working_alpha.as_ref().map(|(i, values)| (planes[*i].as_slice(), values.as_slice()));
        let Some(out) = pixels::channel(channel.id, &planes[index], reference_channel, alpha, target.0, target.1, amount) else { return false; };
        channels.push(out);
    }
    let native = match (&prepared.native, &reference.source_channels) {
        (Some(original), Some(standard)) => {
            let alpha_index = original.planes.iter().position(|(id, _)| *id == -1);
            let standard_alpha = match alpha_index {
                Some(_) => {
                    let Some((_, sample)) = standard.planes.iter().find(|(id, _)| *id == -1) else { return false; };
                    let Some(a) = pixels::native_values(sample, target.0, target.1) else { return false; };
                    Some(a)
                }
                None => None,
            };
            let mut output: Vec<(i16, Samples)> = Vec::new();
            if output.try_reserve_exact(original.planes.len()).is_err() { return false; }
            for (index, (id, _)) in original.planes.iter().enumerate() {
                let Some((_, sample)) = standard.planes.iter().find(|(candidate, _)| candidate == id) else { return false; };
                let alpha = alpha_index.zip(standard_alpha.as_ref())
                    .map(|(i, a)| (planes[native_offset + i].as_slice(), a.as_slice()))
                    .or_else(|| working_alpha.as_ref().map(|(i, a)| (planes[*i].as_slice(), a.as_slice())));
                let Some(result) = pixels::native(&planes[native_offset + index], sample, alpha, *id >= 0, target.0, target.1, amount) else { return false; };
                output.push((*id, result));
            }
            Some(SourceChannels::new(original.depth, dest, output))
        }
        (None, _) => None,
        _ => return false,
    };
    if seams::cancelled(cancel) { return false; }
    let output = LayerOutput { channels, mask: None, native, dest: (dest.left, dest.top, dest.right, dest.bottom) };
    if synthesize {
        let mut staged = doc.clone();
        let composite = staged.source.planes().clone();
        if staged.source.depth().is_none() && staged.source.set_native(Some(native_depth), composite).is_err() { return false; }
        if !write_layer(&mut staged, path, &prepared, output, false) { return false; }
        *doc = staged;
        true
    } else {
        write_layer(doc, path, &prepared, output, false)
    }
}
