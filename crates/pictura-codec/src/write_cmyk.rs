//! Profile-aware CMYK composite and layer plane emission.

use pictura_color::ConversionOptions;
use pictura_core::{Document, Layer};

/// The CMYK bytes to emit for a CMYK output's composite color planes: the four
/// retained source planes exactly when their forward RGB conversion still matches
/// the working composite, else the working RGB re-encoded with the exact
/// [`crate::color_mode::rgb_to_cmyk`] right-inverse. The document is never
/// mutated. The retained plane is the 8-bit CMYK read store at depth 8, or the
/// native store narrowed to 8-bit at 16/32; for an RGB/Grayscale document there
/// is none.
///
/// ponytail: the four planes are re-encoded as a unit, so an edited channel also
/// re-encodes its unchanged siblings. Per-channel change tracking would be the
/// upgrade if that matters.
pub(crate) fn cmyk_composite_planes(doc: &Document, depth: u16, plane: usize) -> Vec<u8> {
    let color_channels = doc.composite.channels as usize;
    let current = &doc.composite.data[..color_channels * plane];
    let profile = crate::icc::cmyk_profile_from_resources(doc);
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    let retained: Option<Vec<Vec<u8>>> = (0..4)
        .map(|c| crate::color_mode::composite_retained_8(doc, depth, c))
        .collect();
    if let Some(retained) = retained {
        let retained: Vec<u8> = retained.concat();
        if retained.len() == plane * 4 {
            let forward = profile
                .as_ref()
                .and_then(|p| crate::color_mode::cmyk_to_rgb_with_profile(&retained, p, options))
                .unwrap_or_else(|| crate::color_mode::cmyk_to_rgb(&retained));
            if forward.as_slice() == current {
                return retained;
            }
        }
    }
    profile
        .as_ref()
        .and_then(|p| crate::color_mode::rgb_to_cmyk_with_profile(current, p, options))
        .unwrap_or_else(|| crate::color_mode::rgb_to_cmyk(current))
}

/// The four CMYK planes to emit for a layer's color channels, or empty when the
/// layer does not carry exactly three id-0/1/2 color channels of equal length.
/// The retained source planes are re-emitted when their forward conversion still
/// matches the working RGB; an edited layer is re-encoded through the document
/// CMYK profile, or the exact profile-free inverse when none is attached.
pub(crate) fn cmyk_layer_color_planes(
    layer: &Layer,
    depth: u16,
    profile: Option<&pictura_color::Profile>,
) -> Vec<Vec<u8>> {
    let positions: Vec<usize> = layer
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.id >= 0 && c.id < 3)
        .map(|(i, _)| i)
        .collect();
    if positions.len() != 3 {
        return Vec::new();
    }
    let plane = layer.channels[positions[0]].data.len();
    if positions
        .iter()
        .any(|&i| layer.channels[i].data.len() != plane)
    {
        return Vec::new();
    }
    let mut current = Vec::with_capacity(plane * 3);
    for &i in &positions {
        current.extend_from_slice(&layer.channels[i].data);
    }
    let options = ConversionOptions {
        dither: false,
        ..ConversionOptions::default()
    };
    let retained: Option<Vec<Vec<u8>>> = (0..4)
        .map(|c| crate::color_mode::layer_retained_8(layer, depth, c))
        .collect();
    if let Some(store) = &layer.source_channels {
        if store.working_idents != [0; 3] {
            let ids = [0i16, 1, 2].map(|id| {
                layer
                    .channels
                    .iter()
                    .find(|channel| channel.id == id)
                    .map(|channel| channel.data.ident())
                    .unwrap_or(0)
            });
            if store.working_idents == ids {
                if let Some(retained) = &retained {
                    let flat: Vec<u8> = retained.concat();
                    if flat.len() == plane * 4 {
                        return split_planes(flat, plane);
                    }
                }
            } else {
                return cmyk_from_rgb(&current, plane, profile, options);
            }
        }
    }
    let flat = match retained {
        Some(retained) => {
            let flat: Vec<u8> = retained.concat();
            if flat.len() == plane * 4 {
                #[cfg(test)]
                FORWARD_CONVERSIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let forward = profile
                    .and_then(|p| crate::color_mode::cmyk_to_rgb_with_profile(&flat, p, options))
                    .unwrap_or_else(|| crate::color_mode::cmyk_to_rgb(&flat));
                if forward.as_slice() == current.as_slice() {
                    flat
                } else {
                    profile
                        .and_then(|p| {
                            crate::color_mode::rgb_to_cmyk_with_profile(&current, p, options)
                        })
                        .unwrap_or_else(|| crate::color_mode::rgb_to_cmyk(&current))
                }
            } else {
                profile
                    .and_then(|p| crate::color_mode::rgb_to_cmyk_with_profile(&current, p, options))
                    .unwrap_or_else(|| crate::color_mode::rgb_to_cmyk(&current))
            }
        }
        None => profile
            .and_then(|p| crate::color_mode::rgb_to_cmyk_with_profile(&current, p, options))
            .unwrap_or_else(|| crate::color_mode::rgb_to_cmyk(&current)),
    };
    split_planes(flat, plane)
}

fn split_planes(flat: Vec<u8>, plane: usize) -> Vec<Vec<u8>> {
    (0..4)
        .map(|c| flat[c * plane..(c + 1) * plane].to_vec())
        .collect()
}

fn cmyk_from_rgb(
    current: &[u8],
    plane: usize,
    profile: Option<&pictura_color::Profile>,
    options: ConversionOptions,
) -> Vec<Vec<u8>> {
    let flat = profile
        .and_then(|p| crate::color_mode::rgb_to_cmyk_with_profile(current, p, options))
        .unwrap_or_else(|| crate::color_mode::rgb_to_cmyk(current));
    split_planes(flat, plane)
}

#[cfg(test)]
pub(crate) static FORWARD_CONVERSIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_forward_conversions() {
    FORWARD_CONVERSIONS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn forward_conversions() -> usize {
    FORWARD_CONVERSIONS.load(std::sync::atomic::Ordering::Relaxed)
}
