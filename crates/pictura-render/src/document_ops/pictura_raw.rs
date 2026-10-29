//! Pictura Raw document ops: the non-destructive smart-object apply and the
//! destructive raster apply, plus the dialog's preview.
//!
//! `apply_pictura_raw` renders a smart-object layer's embedded source into its
//! `rect`, filters it, writes the result back into the layer color channels (the
//! proxy), and attaches the settings so they round-trip on save.
//! `apply_pictura_raw_destructive` filters a plain raster layer's own color
//! channels in place, creating no smart object and retaining no settings.
//! `preview_pictura_raw` renders a downscaled copy of either for the dialog.
//! ponytail: group/nested smart objects are not recursed into.

use pictura_core::{
    layer_pixel_locked, Channel, ColorMode, Document, Layer, PicturaRawSettings, PixelBuffer,
    PsdRect, SmartObjectKind,
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

/// Whether [`apply_pictura_raw_destructive`] will accept `path`: an RGB or
/// Grayscale document, a raster leaf layer (not a group, adjustment, smart
/// object, or pixel-locked layer), a positive rect, and color channels long
/// enough to filter. Cheap enough for menu-enablement checks and shared with the
/// op so the two never drift.
pub fn can_apply_pictura_raw_destructive(doc: &Document, path: &str) -> bool {
    if !matches!(
        super::document_color_mode(doc),
        ColorMode::Rgb | ColorMode::Grayscale
    ) {
        return false;
    }
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group
        || layer.adjustment.is_some()
        || layer.smart_object.is_some()
        || layer_pixel_locked(layer)
        || layer.rect.width() <= 0
        || layer.rect.height() <= 0
    {
        return false;
    }
    has_color_channels(layer, doc.mode.color_channels() as usize)
}

/// Whether `layer` carries the color planes the destructive op reads, each at
/// least `rect`-sized: channel 0 only for Grayscale, channels 0/1/2 for RGB.
fn has_color_channels(layer: &Layer, color_channels: usize) -> bool {
    let plane = layer.rect.width() as usize * layer.rect.height() as usize;
    let ok = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .is_some_and(|c| c.data.len() >= plane)
    };
    if color_channels <= 1 {
        ok(0)
    } else {
        ok(0) && ok(1) && ok(2)
    }
}

/// Apply the Pictura Raw pipeline destructively to the layer's own pixels at
/// `path`.
///
/// Returns `false` without mutating unless [`can_apply_pictura_raw_destructive`]
/// accepts `path`. On success the layer's color channels hold the filtered pixels
/// over the layer rect, while its alpha, channel set, rect and Background
/// identity are preserved, no smart object is created, and the settings are not
/// retained.
///
/// The engine always edits the 8-bit working planes; a 16/32-bit document
/// re-widens them on save (`codec/write.rs` `native_plane`), so no retained-store
/// bookkeeping is needed here. ponytail: groups/nested layers are not recursed.
pub fn apply_pictura_raw_destructive(
    doc: &mut Document,
    path: &str,
    settings: &PicturaRawSettings,
) -> bool {
    if !can_apply_pictura_raw_destructive(doc, path) {
        return false;
    }
    let color_channels = doc.mode.color_channels() as usize;
    let layer = resolve_path(doc, path).expect("eligibility checked above");
    let Some(src) = raster_source(layer, color_channels) else {
        return false;
    };
    let Ok(filtered) = pictura_adjust::render_pictura_raw(&src, settings) else {
        return false;
    };
    let layer = resolve_path_mut(doc, path).expect("resolved above");
    write_color_channels(layer, &filtered, color_channels);
    true
}

/// The layer's own color channels over its rect as a 3-channel RGB buffer, for
/// the destructive apply. A Grayscale layer replicates channel 0 into RGB.
/// `None` when a needed channel is missing or shorter than the layer rect.
fn raster_source(layer: &Layer, color_channels: usize) -> Option<PixelBuffer> {
    let w = layer.rect.width() as usize;
    let h = layer.rect.height() as usize;
    let plane = w.checked_mul(h)?;
    let read = |id: i16| -> Option<&[u8]> {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.as_slice())
    };
    let red = read(0).filter(|d| d.len() >= plane)?;
    let mut data = vec![0u8; plane * 3];
    if color_channels <= 1 {
        data[..plane].copy_from_slice(&red[..plane]);
        data[plane..plane * 2].copy_from_slice(&red[..plane]);
        data[plane * 2..plane * 3].copy_from_slice(&red[..plane]);
    } else {
        let green = read(1).filter(|d| d.len() >= plane)?;
        let blue = read(2).filter(|d| d.len() >= plane)?;
        data[..plane].copy_from_slice(&red[..plane]);
        data[plane..plane * 2].copy_from_slice(&green[..plane]);
        data[plane * 2..plane * 3].copy_from_slice(&blue[..plane]);
    }
    Some(PixelBuffer {
        width: w as u32,
        height: h as u32,
        channels: 3,
        data: data.into(),
    })
}

/// Overwrite the layer's color channels from `src` (a 3-channel buffer over the
/// layer rect), leaving alpha and every other channel untouched. A Grayscale
/// layer takes only plane 0.
fn write_color_channels(layer: &mut Layer, src: &PixelBuffer, color_channels: usize) {
    let plane = src.width as usize * src.height as usize;
    let planes: i16 = if color_channels <= 1 { 1 } else { 3 };
    for id in 0..planes {
        let Some(channel) = layer.channels.iter_mut().find(|c| c.id == id) else {
            continue;
        };
        let n = plane.min(channel.data.len());
        channel.data[..n].copy_from_slice(&src.data[id as usize * plane..id as usize * plane + n]);
    }
}

/// Replace the layer's color channels from `src`, keeping an existing alpha
/// channel or materializing the rendered one.
pub(super) fn write_proxy(layer: &mut Layer, src: &PixelBuffer, color_channels: usize) {
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
    let (pw, ph) = preview_size(
        rect.width() as u32,
        rect.height() as u32,
        max_dim.max(1) as u32,
    );

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
    use pictura_core::{
        AdjustmentData, BitDepth, ColorMode, CrsSettings, LockFlags, PsdRect, Samples, SmartObject,
        SourceChannels,
    };

    fn rgb_settings() -> PicturaRawSettings {
        PicturaRawSettings {
            basic: CrsSettings {
                exposure: Some(1.5),
                ..Default::default()
            },
            ..Default::default()
        }
    }

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
        doc.layers = pictura_core::LayerStack::from_layers(vec![raster_layer(alpha)]);
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
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        let buffer = preview_pictura_raw(&doc, "0", &PicturaRawSettings::default(), 16).unwrap();
        assert_eq!((buffer.width, buffer.height), (16, 8));
    }

    fn destructive_doc(alpha: Option<u8>) -> Document {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![raster_layer(alpha)]);
        doc
    }

    fn channel_of(doc: &Document, id: i16) -> Vec<u8> {
        doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.to_vec())
            .unwrap_or_default()
    }

    #[test]
    fn destructive_apply_changes_pixels_without_a_smart_object() {
        let mut doc = destructive_doc(Some(200));
        let before = channel_of(&doc, 0);
        assert!(apply_pictura_raw_destructive(
            &mut doc,
            "0",
            &rgb_settings()
        ));
        assert_ne!(channel_of(&doc, 0), before, "pixels change");
        assert!(doc.layers[0].smart_object.is_none(), "no smart object");
        assert_eq!(channel_of(&doc, -1), vec![200u8; 4], "alpha preserved");
    }

    #[test]
    fn destructive_apply_refuses_ineligible_targets() {
        let settings = rgb_settings();

        let mut missing = destructive_doc(None);
        assert!(!apply_pictura_raw_destructive(&mut missing, "9", &settings));

        let mut group = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        group.layers = pictura_core::LayerStack::from_layers(vec![Layer {
            name: "g".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            is_group: true,
            ..Default::default()
        }]);
        assert!(!apply_pictura_raw_destructive(&mut group, "0", &settings));

        let mut smart = destructive_doc(None);
        smart.layers[0].smart_object = Some({
            let mut so = SmartObject::default();
            so.kind = SmartObjectKind::Embedded;
            so
        });
        assert!(!apply_pictura_raw_destructive(&mut smart, "0", &settings));

        let mut locked = destructive_doc(None);
        locked.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
        assert!(!apply_pictura_raw_destructive(&mut locked, "0", &settings));

        let mut cmyk = Document::new(2, 2, ColorMode::Cmyk, BitDepth::Eight);
        cmyk.layers = pictura_core::LayerStack::from_layers(vec![raster_layer(None)]);
        assert!(!apply_pictura_raw_destructive(&mut cmyk, "0", &settings));

        let mut adjustment = destructive_doc(None);
        adjustment.layers[0].adjustment = Some(AdjustmentData {
            key: *b"adj?",
            data: Vec::new().into(),
        });
        assert!(!apply_pictura_raw_destructive(
            &mut adjustment,
            "0",
            &settings
        ));

        // The shared eligibility check is false for a channel-less layer and for
        // a layer whose color plane is shorter than its rect, matching the op.
        let mut empty = destructive_doc(None);
        empty.layers[0].channels.clear();
        assert!(!can_apply_pictura_raw_destructive(&empty, "0"));
        assert!(!apply_pictura_raw_destructive(&mut empty, "0", &settings));

        let mut short = destructive_doc(None);
        short.layers[0].channels.retain(|c| c.id != 0);
        assert!(!can_apply_pictura_raw_destructive(&short, "0"));
    }

    #[test]
    fn destructive_apply_keeps_background_identity() {
        let mut doc = destructive_doc(None);
        doc.layers[0].background = true;
        assert!(apply_pictura_raw_destructive(
            &mut doc,
            "0",
            &rgb_settings()
        ));
        assert!(doc.layers[0].background, "still the Background");
        assert!(
            doc.layers[0].channels.iter().all(|c| c.id != -1),
            "no alpha added"
        );
    }

    #[test]
    fn destructive_apply_handles_grayscale() {
        let mut doc = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
            name: "L".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            channels: vec![Channel {
                id: 0,
                data: vec![128u8; 4].into(),
            }],
            ..Default::default()
        }]);
        assert!(apply_pictura_raw_destructive(
            &mut doc,
            "0",
            &rgb_settings()
        ));
        assert_ne!(channel_of(&doc, 0), vec![128u8; 4]);
        assert_eq!(
            doc.layers[0].channels.iter().filter(|c| c.id >= 0).count(),
            1,
            "no color channels added"
        );
    }

    #[test]
    fn destructive_apply_survives_a_16_bit_save() {
        let mut layer = raster_layer(Some(255));
        let rect = layer.rect;
        let native = Samples::widen_from_u8(&[128u8; 4], BitDepth::Sixteen);
        layer.source_channels = Some(SourceChannels::new(
            BitDepth::Sixteen,
            rect,
            (0..3).map(|id| (id as i16, native.clone())).collect(),
        ));
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.source_depth = Some(BitDepth::Sixteen);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

        assert!(apply_pictura_raw_destructive(
            &mut doc,
            "0",
            &rgb_settings()
        ));
        let edited = channel_of(&doc, 0);
        assert_ne!(edited, vec![128u8; 4]);

        let back = pictura_codec::read_psd(&pictura_codec::write_psd(&doc).expect("writes"))
            .expect("re-reads");
        assert_eq!(
            channel_of(&back, 0),
            edited,
            "the save re-widens the edited plane, not the stale native"
        );
    }
}
