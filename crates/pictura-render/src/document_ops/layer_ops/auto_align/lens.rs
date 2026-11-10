//! Bake a Lens Correction pass into a raster layer before Auto-Align's
//! geometric alignment. The same `LensMap`/`LensSource` pair the Filter menu
//! uses keeps distortion, vignette removal, edge handling, and auto-scale
//! identical.

use pictura_core::{Channel, Layer};
use pictura_filters::{LensCorrectionParams, LensMap, LensSource};

/// Correct `layer` in place. Returns false when the layer has no usable
/// channels or the correction is the identity.
pub(super) fn apply(layer: &mut Layer, params: &LensCorrectionParams) -> bool {
    if params.is_identity() {
        return false;
    }
    let w = layer.rect.width().max(0) as usize;
    let h = layer.rect.height().max(0) as usize;
    if w == 0 || h == 0 {
        return false;
    }
    let n = w * h;
    let channel = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id && c.data.len() == n)
            .map(|c| c.data.clone())
    };
    let Some(first) = channel(0) else {
        return false;
    };
    let color = [
        first.clone(),
        channel(1).unwrap_or_else(|| first.clone()),
        channel(2).unwrap_or(first),
    ];
    let alpha = channel(-1);
    let source = LensSource::new(w as u32, h as u32, color, alpha);
    let map = LensMap::new(params, w as u32, h as u32);
    let mut out = vec![0u8; n * 4];
    source.render(Some(&map), [0.0, 0.0], 1.0, (w, h), &mut out);
    let mut planes = [vec![0u8; n], vec![0u8; n], vec![0u8; n]];
    let mut mask = vec![255u8; n];
    for i in 0..n {
        let a = out[i * 4 + 3];
        mask[i] = a;
        if a == 0 {
            continue;
        }
        let scale = 255.0 / f32::from(a);
        for c in 0..3 {
            planes[c][i] = (f32::from(out[i * 4 + c]) * scale)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    let write = |layer: &mut Layer, id: i16, data: Vec<u8>| {
        if let Some(channel) = layer.channels.iter_mut().find(|c| c.id == id) {
            channel.data = data.into();
            true
        } else {
            false
        }
    };
    let mut changed = false;
    for id in [0i16, 1, 2] {
        if layer.channels.iter().any(|c| c.id == id) {
            changed |= write(layer, id, planes[id as usize].clone());
        }
    }
    if !changed {
        return false;
    }
    if layer.channels.iter().any(|c| c.id == -1) {
        write(layer, -1, mask);
    } else if mask.iter().any(|a| *a < 255) {
        layer.channels.push(Channel {
            id: -1,
            data: mask.into(),
        });
    }
    true
}
