//! Pictura Raw document op: bake the filter's Basic controls into a
//! smart-object layer's raster proxy.
//!
//! The layer's embedded source is rendered into its `rect`, filtered, and
//! written back into the layer color channels (the proxy), matching how
//! the reference stores a smart-filtered object. The filter settings are attached
//! so they round-trip on save. ponytail: group/nested smart objects are not
//! recursed into.

use pictura_core::{
    Channel, Document, Layer, PicturaRawSettings, PixelBuffer, PsdRect, SmartObjectKind,
};

use super::layer_ops::{resolve_path, resolve_path_mut};

/// Apply the Pictura Raw Basic controls to the smart-object layer at `path`.
///
/// Returns `false` without mutating for a missing path, a non-smart-object, a
/// group, an adjustment layer, a non-embedded object, an empty/undecodable
/// payload, or an unrenderable rect. On success the layer's color channels hold
/// the filtered source at the layer rect, alpha is preserved, and the settings
/// are attached to the camera-raw smart filter.
pub fn apply_pictura_raw(doc: &mut Document, path: &str, settings: &PicturaRawSettings) -> bool {
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group || layer.adjustment.is_some() {
        return false;
    }
    let Some(so) = layer.smart_object.as_ref() else {
        return false;
    };
    if so.kind != SmartObjectKind::Embedded {
        return false;
    }
    let rect = layer.rect;
    if rect.width() <= 0 || rect.height() <= 0 {
        return false;
    }
    let Some(src) = crate::composite::render_smart_source(so, rect, rect) else {
        return false;
    };
    let Ok(filtered) = pictura_adjust::render_pictura_raw(&src, settings) else {
        return false;
    };
    let color_channels = doc.mode.color_channels() as usize;

    let layer = resolve_path_mut(doc, path).expect("resolved above");
    if pictura_codec::attach_pictura_raw_filter(layer, settings).is_err() {
        return false;
    }
    write_proxy(layer, &filtered, color_channels);
    true
}

/// Replace the layer's color channels from `src`, keeping an existing alpha
/// channel or materializing the rendered one.
fn write_proxy(layer: &mut Layer, src: &PixelBuffer, color_channels: usize) {
    let plane = src.width as usize * src.height as usize;
    let existing_alpha = layer
        .channels
        .iter()
        .find(|c| c.id == -1)
        .map(|c| c.data.to_vec());
    let mut channels: Vec<Channel> = (0..color_channels)
        .map(|c| Channel {
            id: c as i16,
            data: src.data[c * plane..(c + 1) * plane].to_vec().into(),
        })
        .collect();
    let alpha = existing_alpha
        .unwrap_or_else(|| {
            if src.channels >= 4 {
                src.data[3 * plane..4 * plane].to_vec()
            } else {
                vec![255; plane]
            }
        })
        .into();
    channels.push(Channel {
        id: -1,
        data: alpha,
    });
    layer.channels = channels;
}

/// Render a downscaled preview of the layer at `path` with `settings` applied,
/// without mutating the document. `max_dim` caps the long side (clamped to at
/// least 1). Returns `None` for a missing/ineligible target or an unrenderable
/// rect. The source is rendered/scaled straight to the preview size, so the cost
/// does not grow with the full layer. The layer is never converted, so this is a
/// pure read.
/// ponytail: channel bytes are treated as 8-bit; a 16-bit document previews as
/// truncated, matching the apply path's ceiling.
pub fn preview_pictura_raw(
    doc: &Document,
    path: &str,
    settings: &PicturaRawSettings,
    max_dim: i32,
) -> Option<PixelBuffer> {
    let layer = resolve_path(doc, path)?;
    if layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    let rect = layer.rect;
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    let (pw, ph) = preview_size(rect.width() as u32, rect.height() as u32, max_dim.max(1) as u32);

    let src = match layer
        .smart_object
        .as_ref()
        .filter(|so| so.kind == SmartObjectKind::Embedded)
    {
        // Render the payload scaled to the preview size, not the full layer rect.
        Some(so) => {
            let preview = PsdRect {
                top: 0,
                left: 0,
                bottom: ph as i32,
                right: pw as i32,
            };
            crate::composite::render_smart_source(so, preview, preview)?
        }
        None => raster_preview_buffer(doc, layer, pw, ph),
    };
    pictura_adjust::render_pictura_raw(&src, settings).ok()
}

/// The long side of a `max_dim`-capped copy of a `w`x`h` image.
fn preview_size(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    if w <= max_dim && h <= max_dim {
        return (w, h);
    }
    let scale = max_dim as f64 / w.max(h) as f64;
    (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    )
}

/// The layer's raster channels sampled into a `pw`x`ph` RGBA buffer (grayscale
/// replicated to RGB, alpha kept when present), for a not-yet-smart pixel layer.
fn raster_preview_buffer(doc: &Document, layer: &Layer, pw: u32, ph: u32) -> PixelBuffer {
    let w = layer.rect.width().max(0) as usize;
    let h = layer.rect.height().max(0) as usize;
    let color = doc.mode.color_channels() as usize;
    let read = |id: i16| -> Option<&[u8]> {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.as_ref())
    };
    let (red, green, blue) = (read(0), read(1), read(2));
    let alpha = read(-1);
    let plane = pw as usize * ph as usize;
    let mut data = vec![0u8; plane * 4];
    for y in 0..ph as usize {
        let sy = (y * h / ph as usize).min(h.saturating_sub(1));
        for x in 0..pw as usize {
            let sx = (x * w / pw as usize).min(w.saturating_sub(1));
            let si = sy * w + sx;
            let di = y * pw as usize + x;
            let (r, g, b) = if color == 1 {
                let v = red.and_then(|d| d.get(si)).copied().unwrap_or(0);
                (v, v, v)
            } else {
                (
                    red.and_then(|d| d.get(si)).copied().unwrap_or(0),
                    green.and_then(|d| d.get(si)).copied().unwrap_or(0),
                    blue.and_then(|d| d.get(si)).copied().unwrap_or(0),
                )
            };
            data[di] = r;
            data[plane + di] = g;
            data[2 * plane + di] = b;
            data[3 * plane + di] = alpha.and_then(|d| d.get(si)).copied().unwrap_or(255);
        }
    }
    PixelBuffer {
        width: pw,
        height: ph,
        channels: 4,
        data: data.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, PsdRect};

    fn raster_layer(alpha: Option<u8>) -> Layer {
        let mut layer = Layer {
            name: "Layer".to_string(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            ..Default::default()
        };
        for id in 0..3i16 {
            layer.channels.push(Channel {
                id,
                data: vec![128u8; 4].into(),
            });
        }
        if let Some(a) = alpha {
            layer.channels.push(Channel {
                id: -1,
                data: vec![a; 4].into(),
            });
        }
        layer
    }

    fn preview_alpha(alpha: Option<u8>) -> (u8, u8) {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![raster_layer(alpha)];
        let buffer = preview_pictura_raw(&doc, "0", &PicturaRawSettings::default(), 16).unwrap();
        assert_eq!(buffer.channels, 4);
        (buffer.data[0], buffer.data[3 * 4])
    }

    #[test]
    fn raster_preview_keeps_the_layer_alpha() {
        assert_eq!(preview_alpha(Some(64)), (128, 64));
    }

    #[test]
    fn raster_preview_is_opaque_without_an_alpha_channel() {
        assert_eq!(preview_alpha(None), (128, 255));
    }

    #[test]
    fn preview_is_capped_at_max_dim() {
        let mut layer = raster_layer(None);
        layer.rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 40,
            right: 80,
        };
        layer.channels = (0..3)
            .map(|id| Channel {
                id,
                data: vec![10u8; 80 * 40].into(),
            })
            .collect();
        let mut doc = Document::new(80, 40, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![layer];
        let buffer = preview_pictura_raw(&doc, "0", &PicturaRawSettings::default(), 16).unwrap();
        assert_eq!((buffer.width, buffer.height), (16, 8));
    }
}
