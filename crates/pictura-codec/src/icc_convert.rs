//! ICC pixel conversion at the store's depth.
//!
//! Split from `icc.rs` for the file-size cap. This module owns the plane and
//! layer conversion; `icc.rs` owns profile policy and resource rewriting.

use pictura_color::{convert, ConversionOptions, Profile};
use pictura_core::{BitDepth, Document, Layer, LayerStack, Samples};

use super::icc::convert_planes_with_options;

/// Convert a layer's 8-bit RGB channels and, when `native` is set, its
/// retained RGB native store. A native store in a recorded source mode
/// (CMYK/Lab) is not RGB and must not be transformed by an RGB profile
/// conversion; the 8-bit working channels already carry that conversion.
fn convert_layer(
    layer: &mut Layer,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
    native: bool,
) {
    convert_layer_8bit(layer, src, dst, options);
    if native {
        convert_layer_native(layer, src, dst, options);
    }
}

/// Convert a layer's RGB color channels in place, selected by channel id `0..3`
/// rather than by storage order. A layer missing a color channel is unchanged.
fn convert_layer_8bit(layer: &mut Layer, src: &Profile, dst: &Profile, options: ConversionOptions) {
    let mut slots = Vec::with_capacity(3);
    let mut planes = Vec::with_capacity(3);
    for id in 0..3i16 {
        let Some((slot, channel)) = layer.channels.iter().enumerate().find(|(_, c)| c.id == id)
        else {
            return;
        };
        slots.push(slot);
        planes.push(channel.data.to_vec());
    }
    if let Some(converted) = convert_planes_with_options(&planes, src, dst, options) {
        for (slot, plane) in slots.into_iter().zip(converted) {
            layer.channels[slot].data = plane.into();
        }
    }
}

/// Convert `planes` (same length, same `Samples` variant) through lcms2 at the
/// store's depth. Planes are packed in native-endian order, matching the pixel
/// format's expectation; `Samples::to_bytes` is big-endian and is not used here.
fn convert_native(
    planes: &[Samples],
    depth: BitDepth,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) -> Option<Vec<Samples>> {
    let bits = match depth {
        BitDepth::Sixteen => 16u8,
        BitDepth::ThirtyTwo => 32u8,
        _ => return None,
    };
    let channels = planes.len();
    let n = planes.first()?.len();
    if n == 0 || planes.iter().any(|p| p.len() != n) {
        return None;
    }
    let width = u32::try_from(n).ok()?;
    let bytes = bits as usize / 8;
    let mut packed = Vec::with_capacity(n * channels * bytes);
    // lcms2's stock float pipeline is defined on `[0, 1]`. Out-of-gamut
    // samples are converted in range and carried through unchanged, so a save
    // does not silently clamp them.
    // ponytail: an HDR transform needs an unbounded lcms2 pipeline; pass-through
    // is the ceiling until one is wired.
    let mut out_of_range: Option<Vec<f32>> = (bits == 32).then(|| Vec::with_capacity(n * channels));
    for i in 0..n {
        for plane in planes {
            match plane {
                Samples::U16(v) if bits == 16 => {
                    packed.extend_from_slice(&v[i].to_ne_bytes());
                }
                Samples::F32(v) if bits == 32 => {
                    let value = v[i];
                    if let Some(originals) = out_of_range.as_mut() {
                        originals.push(value);
                    }
                    let packed_value = if value.is_finite() && !(0.0..=1.0).contains(&value) {
                        value.clamp(0.0, 1.0)
                    } else {
                        value
                    };
                    packed.extend_from_slice(&packed_value.to_ne_bytes());
                }
                _ => return None,
            }
        }
    }
    let out = convert(
        src,
        dst,
        &packed,
        width,
        1,
        channels as u8,
        bits,
        options.intent,
        options.black_point_compensation,
    )
    .ok()?;
    let mut converted = Vec::with_capacity(channels);
    if bits == 16 {
        let values: Vec<u16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_ne_bytes(*b))
            .collect();
        for ch in 0..channels {
            converted.push(Samples::U16(
                (0..n).map(|i| values[i * channels + ch]).collect(),
            ));
        }
    } else {
        let mut values: Vec<f32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_ne_bytes(*b))
            .collect();
        if let Some(originals) = out_of_range {
            for (slot, original) in values.iter_mut().zip(originals) {
                if !(0.0..=1.0).contains(&original) {
                    *slot = original;
                }
            }
        }
        for ch in 0..channels {
            converted.push(Samples::F32(
                (0..n).map(|i| values[i * channels + ch]).collect(),
            ));
        }
    }
    Some(converted)
}

/// The first three color planes of a flat, native composite store.
fn split_color_planes(samples: &Samples, n: usize) -> Option<Vec<Samples>> {
    if n == 0 || samples.len() < n * 3 {
        return None;
    }
    (0..3)
        .map(|ch| samples.slice(ch * n..(ch + 1) * n))
        .collect()
}

fn write_color_planes(samples: &mut Samples, n: usize, planes: &[Samples]) {
    for (ch, plane) in planes.iter().enumerate() {
        match (&mut *samples, plane) {
            (Samples::U16(dst), Samples::U16(src)) => {
                dst[ch * n..(ch + 1) * n].copy_from_slice(src);
            }
            (Samples::F32(dst), Samples::F32(src)) => {
                dst[ch * n..(ch + 1) * n].copy_from_slice(src);
            }
            _ => {}
        }
    }
}

/// Convert the document's retained native composite planes and sync the 8-bit
/// working planes from the narrowed result, so `write_psd` re-emits the native
/// store instead of widening the 8-bit copy. A converted source mode (e.g. an
/// 8-bit Lab read) keeps its source-mode planes untouched.
fn convert_source_planes(
    doc: &mut Document,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) {
    if doc.source_mode.is_some() {
        return;
    }
    let (width, height) = (doc.width, doc.height);
    let Some(store) = doc.source_planes.as_mut() else {
        return;
    };
    if !matches!(store.depth, BitDepth::Sixteen | BitDepth::ThirtyTwo)
        || store.width != width
        || store.height != height
    {
        return;
    }
    let n = width as usize * height as usize;
    let Some(planes) = split_color_planes(&store.samples, n) else {
        return;
    };
    let Some(converted) = convert_native(&planes, store.depth, src, dst, options) else {
        return;
    };
    for (ch, plane) in converted.iter().enumerate() {
        let narrow = plane.narrow_to_u8();
        if let Some(channel) = doc.composite.data.get_mut(ch * n..(ch + 1) * n) {
            if narrow.len() == channel.len() {
                channel.copy_from_slice(&narrow);
            }
        }
    }
    write_color_planes(&mut store.samples, n, &converted);
}

/// Convert a layer's retained native color planes and sync its 8-bit working
/// channels from the narrowed result. A partial store is left unchanged.
fn convert_layer_native(
    layer: &mut Layer,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) {
    let Some(store) = layer.source_channels.as_mut() else {
        return;
    };
    if !matches!(store.depth, BitDepth::Sixteen | BitDepth::ThirtyTwo) {
        return;
    }
    if store.rect != layer.rect {
        return;
    }
    let n = (store.rect.width().max(0) * store.rect.height().max(0)) as usize;
    if n == 0 {
        return;
    }
    let ids = [0i16, 1, 2];
    let mut slots = Vec::with_capacity(3);
    for id in ids {
        let Some((slot, channel)) = layer.channels.iter().enumerate().find(|(_, c)| c.id == id)
        else {
            return;
        };
        if channel.data.len() != n {
            return;
        }
        slots.push(slot);
    }
    let Some(planes) = ids
        .iter()
        .map(|id| {
            store
                .planes
                .iter()
                .find(|(cid, _)| cid == id)
                .map(|(_, samples)| samples.clone())
        })
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    if planes.iter().any(|p| p.len() != n) {
        return;
    }
    let Some(converted) = convert_native(&planes, store.depth, src, dst, options) else {
        return;
    };
    for (id, plane) in ids.iter().zip(&converted) {
        if let Some((_, slot)) = store.planes.iter_mut().find(|(cid, _)| cid == id) {
            *slot = plane.clone();
        }
    }
    for (slot, plane) in slots.into_iter().zip(converted) {
        layer.channels[slot].data = plane.narrow_to_u8().into();
    }
}

/// Convert every layer in `layers` and, recursively, every descendant of a
/// group, so a nested pixel layer is not left in the old profile.
pub(super) fn convert_layers(
    layers: &mut LayerStack,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
    native: bool,
) {
    for layer in layers {
        convert_layer(layer, src, dst, options, native);
        convert_layers(&mut layer.children, src, dst, options, native);
    }
}

/// Convert the document's retained native composite planes after its 8-bit
/// working copy and layers have been converted.
pub(super) fn convert_native_document(
    doc: &mut Document,
    src: &Profile,
    dst: &Profile,
    options: ConversionOptions,
) {
    convert_source_planes(doc, src, dst, options);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_color::Intent;

    #[test]
    fn convert_options_dither_is_8bit_only() {
        let planes = vec![
            Samples::U16(vec![0, 1000, 65535].into()),
            Samples::U16(vec![500, 2000, 40000].into()),
            Samples::U16(vec![900, 3000, 50000].into()),
        ];
        let base = ConversionOptions {
            intent: Intent::RelativeColorimetric,
            black_point_compensation: true,
            dither: false,
        };
        let plain = convert_native(
            &planes,
            BitDepth::Sixteen,
            &Profile::srgb(),
            &Profile::adobe_rgb(),
            base,
        )
        .expect("16-bit transform");
        let dithered = convert_native(
            &planes,
            BitDepth::Sixteen,
            &Profile::srgb(),
            &Profile::adobe_rgb(),
            ConversionOptions {
                dither: true,
                ..base
            },
        )
        .expect("16-bit transform");
        assert_eq!(plain, dithered, "dither is 8-bit only");
    }
}
