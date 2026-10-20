//! `Layer via Copy` / `Layer via Cut` (design D7, requirement "Layer via Copy
//! and Layer via Cut"). The selection arrives as a document-sized
//! [`LayerMask`]; the coverage at each document pixel modulates the copied
//! alpha, so a feathered selection copies (or cuts) partially.

use pictura_core::{Document, Layer, LayerMask};

use super::create::transparent_layer;
use super::paths::{format_segments, parse_path, resolve_path, resolve_path_mut};

/// Copy the pixels covered by `mask` from the layer at `source_path` into a new
/// document-sized pixel layer directly above it, named `"<source> copy"`.
/// Returns the new path, or empty when the path does not resolve, the source is
/// a group, or the mask carries no data.
pub fn layer_via_copy(doc: &mut Document, source_path: &str, mask: &LayerMask) -> String {
    via(doc, source_path, mask, false)
}

/// As [`layer_via_copy`], then clear the covered pixels from the source: the
/// source alpha scales by `(255 - coverage) / 255`. Returns the new path, or
/// empty on the same refusals.
pub fn layer_via_cut(doc: &mut Document, source_path: &str, mask: &LayerMask) -> String {
    via(doc, source_path, mask, true)
}

fn via(doc: &mut Document, source_path: &str, mask: &LayerMask, cut: bool) -> String {
    let Some(segments) = parse_path(source_path) else {
        return String::new();
    };
    let Some(mask_data) = mask.data.as_deref() else {
        return String::new();
    };
    let copy = {
        let Some(source) = resolve_path(doc, source_path) else {
            return String::new();
        };
        if source.is_group {
            return String::new();
        }
        let name = format!("{} copy", source.name);
        copy_source(source, mask, mask_data, doc.width, doc.height, name)
    };
    if cut {
        clear_source(doc, &segments, mask, mask_data);
    }
    let Some((&index, parent)) = segments.split_last() else {
        return String::new();
    };
    let at = index + 1;
    if doc.layers.insert(parent, at, copy).is_err() {
        return String::new();
    }
    let mut path = segments;
    if let Some(last) = path.last_mut() {
        *last = at;
    }
    format_segments(&path)
}

/// A document-sized copy of `source`'s `0/1/2/-1` channels: color is copied
/// where coverage is nonzero and alpha becomes `round(source_alpha * coverage /
/// 255)`. Pixels outside the selection stay fully transparent.
fn copy_source(
    source: &Layer,
    mask: &LayerMask,
    mask_data: &[u8],
    width: u32,
    height: u32,
    name: String,
) -> Layer {
    let mut layer = transparent_layer(width, height, &name);
    let src_w = source.rect.width();
    let src_h = source.rect.height();
    if src_w <= 0 || src_h <= 0 || width == 0 || height == 0 {
        return layer;
    }
    let src_alpha = source.channels.iter().find(|channel| channel.id == -1);
    let w = width as usize;
    for sy in 0..src_h {
        for sx in 0..src_w {
            let x = source.rect.left + sx;
            let y = source.rect.top + sy;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let coverage = coverage_at(mask, mask_data, x, y);
            if coverage == 0 {
                continue;
            }
            let si = sy as usize * src_w as usize + sx as usize;
            let src_a = src_alpha
                .and_then(|channel| channel.data.get(si))
                .copied()
                .unwrap_or(255);
            let out_a = scale(src_a, coverage);
            let di = y as usize * w + x as usize;
            set_channel(&mut layer, -1, di, out_a);
            if out_a == 0 {
                continue;
            }
            for id in [0i16, 1, 2] {
                if let Some(value) = source
                    .channels
                    .iter()
                    .find(|channel| channel.id == id)
                    .and_then(|channel| channel.data.get(si))
                    .copied()
                {
                    set_channel(&mut layer, id, di, value);
                }
            }
        }
    }
    layer
}

/// Scale the source layer's alpha down by coverage, `alpha * (255 - coverage) /
/// 255`; a fully covered pixel becomes transparent.
fn clear_source(doc: &mut Document, segments: &[usize], mask: &LayerMask, mask_data: &[u8]) {
    let path = format_segments(segments);
    let Some(layer) = resolve_path_mut(doc, &path) else {
        return;
    };
    let rect = layer.rect;
    let src_w = rect.width();
    let src_h = rect.height();
    if src_w <= 0 || src_h <= 0 {
        return;
    }
    let Some(alpha) = layer.channels.iter_mut().find(|channel| channel.id == -1) else {
        return;
    };
    for sy in 0..src_h {
        for sx in 0..src_w {
            let x = rect.left + sx;
            let y = rect.top + sy;
            let coverage = coverage_at(mask, mask_data, x, y);
            if coverage == 0 {
                continue;
            }
            let index = sy as usize * src_w as usize + sx as usize;
            let Some(current) = alpha.data.get(index).copied() else {
                continue;
            };
            alpha.data[index] = ((current as u32 * (255 - coverage as u32) + 127) / 255) as u8;
        }
    }
}

/// `round(value * coverage / 255)`.
fn scale(value: u8, coverage: u8) -> u8 {
    ((value as u32 * coverage as u32 + 127) / 255) as u8
}

/// The mask coverage at document pixel `(x, y)`; outside the mask rectangle the
/// mask's default color applies.
fn coverage_at(mask: &LayerMask, data: &[u8], x: i32, y: i32) -> u8 {
    let rx = x - mask.rect.left;
    let ry = y - mask.rect.top;
    let width = mask.rect.width();
    let height = mask.rect.height();
    if rx < 0 || ry < 0 || rx >= width || ry >= height {
        return mask.default_color;
    }
    let index = ry as usize * width as usize + rx as usize;
    data.get(index).copied().unwrap_or(mask.default_color)
}

fn set_channel(layer: &mut Layer, id: i16, index: usize, value: u8) {
    if let Some(channel) = layer.channels.iter_mut().find(|channel| channel.id == id) {
        if index < channel.data.len() {
            channel.data[index] = value;
        }
    }
}
