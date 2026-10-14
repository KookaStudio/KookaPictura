//! Pictura Raw document op: bake the filter's Basic controls into a
//! smart-object layer's raster proxy.
//!
//! The layer's embedded source is rendered into its `rect`, filtered, and
//! written back into the layer color channels (the proxy), matching how
//! the reference stores a smart-filtered object. The filter settings are attached
//! so they round-trip on save. ponytail: group/nested smart objects are not
//! recursed into.

use pictura_core::{Channel, Document, Layer, PicturaRawSettings, PixelBuffer, SmartObjectKind};

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
/// rect. For an embedded smart object the source is rendered through the Pictura
/// Raw pipeline; for a plain raster pixel layer its channels are used directly.
/// The layer is never converted, so this is a pure read.
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
    let src = match layer
        .smart_object
        .as_ref()
        .filter(|so| so.kind == SmartObjectKind::Embedded)
    {
        Some(so) => crate::composite::render_smart_source(so, rect, rect)?,
        None => raster_buffer(doc, layer),
    };
    let src = downscale_to(src, max_dim.max(1) as u32);
    pictura_adjust::render_pictura_raw(&src, settings).ok()
}

/// The layer's raster channels as an RGBA buffer (grayscale replicated to RGB),
/// for a not-yet-smart pixel layer. The alpha channel is kept when present so the
/// preview matches the applied result for a layer with transparency.
fn raster_buffer(doc: &Document, layer: &Layer) -> PixelBuffer {
    let w = layer.rect.width() as u32;
    let h = layer.rect.height() as u32;
    let plane = (w as usize) * (h as usize);
    let color = doc.mode.color_channels() as usize;
    let read = |id: i16| -> Option<&[u8]> {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.as_ref())
    };
    let mut data = vec![0u8; plane * 4];
    if color == 1 {
        let g = read(0).unwrap_or(&[]);
        for i in 0..plane {
            let v = g.get(i).copied().unwrap_or(0);
            data[i] = v;
            data[plane + i] = v;
            data[2 * plane + i] = v;
        }
    } else {
        // ponytail: a 4-channel (CMYK) layer takes its first three planes; a
        // real CMYK->RGB preview conversion is out of scope here.
        for c in 0..color.min(3) {
            if let Some(src) = read(c as i16) {
                let n = plane.min(src.len());
                data[c * plane..c * plane + n].copy_from_slice(&src[..n]);
            }
        }
    }
    match read(-1) {
        Some(alpha) => {
            let n = plane.min(alpha.len());
            data[3 * plane..3 * plane + n].copy_from_slice(&alpha[..n]);
        }
        None => data[3 * plane..4 * plane].fill(255),
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 4,
        data: data.into(),
    }
}

/// Nearest-neighbour downscale so the long side is at most `max_dim`; a no-op
/// when the buffer already fits.
fn downscale_to(src: PixelBuffer, max_dim: u32) -> PixelBuffer {
    let (w, h) = (src.width, src.height);
    if w <= max_dim && h <= max_dim {
        return src;
    }
    let scale = (max_dim as f64 / w.max(h) as f64).min(1.0);
    let nw = ((w as f64 * scale).round() as u32).max(1);
    let nh = ((h as f64 * scale).round() as u32).max(1);
    let ch = src.channels as usize;
    let s_plane = (w * h) as usize;
    let mut data = vec![0u8; (nw * nh) as usize * ch];
    for y in 0..nh as usize {
        let sy = (((y as f64 + 0.5) * h as f64 / nh as f64) as usize).min(h as usize - 1);
        for x in 0..nw as usize {
            let sx = (((x as f64 + 0.5) * w as f64 / nw as f64) as usize).min(w as usize - 1);
            let dst = (y * nw as usize + x) * ch;
            for c in 0..ch {
                data[dst + c] = src.data[c * s_plane + sy * w as usize + sx];
            }
        }
    }
    PixelBuffer {
        width: nw,
        height: nh,
        channels: src.channels,
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
}
