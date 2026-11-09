//! `Edit > Fade`: composite a filtered/effected layer back over its input.
//!
//! Fade is the W3C source-over composite of the effect output (`effected`)
//! over the original pixels (`original`) with the requested opacity and blend
//! mode. It never re-runs the effect. See `docs/06-filters/filters-overview.md`.

use pictura_core::blend::blend;
use pictura_core::layer_bounds::grow_layer_to;
use pictura_core::{BlendMode, Channel, Layer, Plane, PsdRect};

/// The Fade compositing parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    /// 0 is the original, 100 is the effect output. Values above 100 clamp.
    pub opacity: u8,
    pub mode: BlendMode,
}

impl Default for Fade {
    fn default() -> Self {
        Self {
            opacity: 100,
            mode: BlendMode::Normal,
        }
    }
}

/// Composite `effected` over `original` with `fade`, returning the layer the
/// document should hold. The result covers the union of both rectangles and
/// keeps the effected layer's metadata and non-color channels.
pub fn fade_layer(original: &Layer, effected: &Layer, fade: &Fade) -> Layer {
    let opacity = fade.opacity.min(100);
    if opacity == 100 && fade.mode == BlendMode::Normal {
        return effected.clone();
    }
    let union = original.rect.union(effected.rect);
    let mut out = effected.clone();
    if out.rect != union && (grow_layer_to(&mut out, union, "Fade").is_err() || out.rect != union) {
        return effected.clone();
    }
    if opacity == 0 {
        return original_at_union(original, out, union);
    }
    composite(original, effected, &mut out, union, opacity, fade.mode);
    out
}

/// The opacity-zero result: the original pixels inside their rectangle and
/// transparency elsewhere, with the effected layer's metadata.
fn original_at_union(original: &Layer, mut out: Layer, union: PsdRect) -> Layer {
    let alpha = union_alpha(original, union);
    let mut source = original.clone();
    if source.rect != union {
        let _ = grow_layer_to(&mut source, union, "Fade");
    }
    let (w, h) = (
        union.width().max(0) as usize,
        union.height().max(0) as usize,
    );
    for channel in &mut out.channels {
        if channel.id >= 0 {
            copy_or_fill(channel, &source, w, h, 0);
        } else if channel.id == -1 {
            match &alpha {
                Some(plane) => channel.data = plane.clone(),
                None => fill(channel, w, h, 255),
            }
        }
    }
    if !out.channels.iter().any(|c| c.id == -1) {
        if let Some(plane) = alpha {
            out.channels.push(Channel {
                id: -1,
                data: plane,
            });
        }
    }
    out
}

/// The original layer's alpha over `union`: its own alpha inside its rectangle,
/// opaque when it has no alpha plane, and transparent outside it. `None` when
/// the layer is already opaque over the whole union.
fn union_alpha(source: &Layer, union: PsdRect) -> Option<Plane<u8>> {
    let source_alpha = source.channels.iter().find(|c| c.id == -1);
    if source.rect == union && source_alpha.is_none() {
        return None;
    }
    let (w, h) = (
        union.width().max(0) as usize,
        union.height().max(0) as usize,
    );
    let stride = source.rect.width().max(0) as usize;
    let alpha = source_alpha.map(|c| c.data.as_slice());
    let mut data = vec![0u8; w * h];
    for y in 0..h {
        let doc_y = union.top + y as i32;
        for x in 0..w {
            let doc_x = union.left + x as i32;
            if let Some(index) = local_index(source.rect, stride, doc_x, doc_y) {
                data[y * w + x] = alpha
                    .and_then(|plane| plane.get(index).copied())
                    .unwrap_or(255);
            }
        }
    }
    Some(Plane::build(data.len(), |out| out.copy_from_slice(&data)))
}

fn copy_or_fill(channel: &mut Channel, source: &Layer, w: usize, h: usize, default: u8) {
    match source.channels.iter().find(|c| c.id == channel.id) {
        Some(src) if src.data.len() == channel.data.len() => {
            channel.data = src.data.clone();
        }
        _ => fill(channel, w, h, default),
    }
}

fn fill(channel: &mut Channel, w: usize, h: usize, value: u8) {
    channel.data = Plane::filled(w, h, 1, value);
}

fn composite(
    original: &Layer,
    effected: &Layer,
    out: &mut Layer,
    union: PsdRect,
    opacity: u8,
    mode: BlendMode,
) {
    let w = union.width().max(0) as usize;
    let h = union.height().max(0) as usize;
    if w == 0 || h == 0 {
        return;
    }
    let original_colors = color_planes(original);
    let effected_colors = color_planes(effected);
    if original_colors.is_empty() || effected_colors.is_empty() {
        return;
    }
    let out_color_indices: Vec<usize> = out
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0)
        .map(|(i, _)| i)
        .collect();
    if out_color_indices.is_empty() {
        return;
    }
    let source_has_alpha = original.channels.iter().any(|c| c.id == -1)
        || effected.channels.iter().any(|c| c.id == -1);
    if source_has_alpha && !out.channels.iter().any(|c| c.id == -1) {
        out.channels.push(Channel {
            id: -1,
            data: Plane::filled(w, h, 1, 255),
        });
    }
    let opacity = f32::from(opacity) / 100.0;
    let original_alpha = alpha_plane(original);
    let effected_alpha = alpha_plane(effected);
    let original_stride = original.rect.width().max(0) as usize;
    let effected_stride = effected.rect.width().max(0) as usize;
    let out_alpha_index = out.channels.iter().position(|c| c.id == -1);
    // ponytail: the planes are materialized for this pointwise pass. A huge
    // layer pays one contiguous copy per plane; stream the tiles if Fade ever
    // shows up in a profile.
    let mut planes: Vec<&mut [u8]> = out
        .channels
        .iter_mut()
        .map(|channel| channel.data.as_mut_slice())
        .collect();
    for y in 0..h {
        let doc_y = union.top + y as i32;
        for x in 0..w {
            let doc_x = union.left + x as i32;
            let oi = local_index(original.rect, original_stride, doc_x, doc_y);
            let ei = local_index(effected.rect, effected_stride, doc_x, doc_y);
            let cb = color_at(&original_colors, oi);
            let cs = color_at(&effected_colors, ei);
            let ab = alpha_at(original_alpha, oi);
            let ass = alpha_at(effected_alpha, ei);
            let source = ass * opacity;
            let out_alpha = source + ab * (1.0 - source);
            let blended = blend(mode, cb, cs);
            let dst = y * w + x;
            for (slot, &channel_index) in out_color_indices.iter().enumerate() {
                let value = if out_alpha > 0.0 {
                    let (cb_component, cs_component, c) = if slot < 3 {
                        (cb[slot], cs[slot], blended[slot])
                    } else {
                        // Extra color planes are not part of the W3C triplet;
                        // blend them as a repeated component.
                        let cb_component = component_at(&original_colors, slot, oi);
                        let cs_component = component_at(&effected_colors, slot, ei);
                        let c = blend(mode, [cb_component; 3], [cs_component; 3])[0];
                        (cb_component, cs_component, c)
                    };
                    let premultiplied = (1.0 - source) * ab * cb_component
                        + source * ((1.0 - ab) * cs_component + ab * c);
                    premultiplied / out_alpha
                } else {
                    0.0
                };
                planes[channel_index][dst] = to_u8(value);
            }
            if let Some(index) = out_alpha_index {
                planes[index][dst] = to_u8(out_alpha);
            }
        }
    }
}

fn color_planes(layer: &Layer) -> Vec<&[u8]> {
    layer
        .channels
        .iter()
        .filter(|c| c.id >= 0)
        .map(|c| c.data.as_slice())
        .collect()
}

fn alpha_plane(layer: &Layer) -> Option<&[u8]> {
    layer
        .channels
        .iter()
        .find(|c| c.id == -1)
        .map(|c| c.data.as_slice())
}

fn color_at(planes: &[&[u8]], index: Option<usize>) -> [f32; 3] {
    let mut out = [0.0; 3];
    if let Some(i) = index {
        for (slot, plane) in planes.iter().take(3).enumerate() {
            out[slot] = plane.get(i).copied().map_or(0.0, unit);
        }
        if planes.len() == 1 {
            out[1] = out[0];
            out[2] = out[0];
        }
    }
    out
}

fn component_at(planes: &[&[u8]], slot: usize, index: Option<usize>) -> f32 {
    let Some(index) = index else {
        return 0.0;
    };
    planes
        .get(slot)
        .and_then(|plane| plane.get(index))
        .copied()
        .map_or(0.0, unit)
}

fn alpha_at(plane: Option<&[u8]>, index: Option<usize>) -> f32 {
    let Some(index) = index else {
        return 0.0;
    };
    match plane {
        Some(data) => data.get(index).copied().map_or(1.0, unit),
        None => 1.0,
    }
}

fn local_index(rect: PsdRect, stride: usize, x: i32, y: i32) -> Option<usize> {
    if x < rect.left || x >= rect.right || y < rect.top || y >= rect.bottom {
        return None;
    }
    Some((y - rect.top) as usize * stride + (x - rect.left) as usize)
}

fn unit(value: u8) -> f32 {
    f32::from(value) / 255.0
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests;
