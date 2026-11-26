//! Load Channel Selection: a Channels-panel colour row as a selection, derived
//! the way the row thumbnails are, from the picture streamed band by band
//! (#1274).

use pictura_codec::buffer_to_srgb;
use pictura_core::{ColorMode, Document, PixelBuffer};
use pictura_select::Selection;

/// The colour-channel row layout of a document `mode`: whether a composite row
/// leads the list, and the number of colour channels after it. Mirrors the
/// Channels panel's `channelsForMode`: a single-row mode's one row is the
/// composite, with no separate colour channel.
pub(in crate::cxxqt_object) fn color_rows(mode: ColorMode) -> (bool, usize) {
    match mode {
        ColorMode::Rgb | ColorMode::Lab => (true, 3),
        ColorMode::Cmyk => (true, 4),
        ColorMode::Multichannel => (false, 3),
        ColorMode::Grayscale | ColorMode::Indexed | ColorMode::Bitmap | ColorMode::Duotone => {
            (true, 0)
        }
    }
}

/// Qt's `qGray`: `(r*11 + g*16 + b*5) >> 5`.
pub(in crate::cxxqt_object) fn q_gray(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 11 + g as u32 * 16 + b as u32 * 5) >> 5) as u8
}

/// The naive CMYK ink level `0..=255` for one channel of an RGB pixel: the
/// plate split the Channels panel thumbnails show (`color_models::channel_plane`).
pub(super) fn cmyk_ink(r: u8, g: u8, b: u8, index: usize) -> u8 {
    let (rf, gf, bf) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let k = 1.0 - rf.max(gf).max(bf);
    let inv = if k < 1.0 { 1.0 / (1.0 - k) } else { 0.0 };
    let inks = [
        (1.0 - rf - k) * inv,
        (1.0 - gf - k) * inv,
        (1.0 - bf - k) * inv,
        k,
    ];
    (inks[index].clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The panel thumbnail's grayscale value for colour row `index`
/// (`color_models::channel_plane`): RGB uses its own plane, CMYK and Multichannel their naive
/// ink coverage, everything else the composite luminance.
pub(in crate::cxxqt_object) fn channel_value(
    mode: ColorMode,
    index: usize,
    r: u8,
    g: u8,
    b: u8,
) -> u8 {
    match mode {
        ColorMode::Rgb => [r, g, b][index.min(2)],
        ColorMode::Cmyk => 255 - cmyk_ink(r, g, b, index.min(3)),
        ColorMode::Multichannel => 255 - cmyk_ink(r, g, b, index.min(2)),
        _ => q_gray(r, g, b),
    }
}

/// The selection a Channels-panel colour `row` holds: the composite (row 0) is
/// its luminance over the visible `mask` channels, each colour row its own
/// grayscale plane (the same derivation the row thumbnails use). `None` for a
/// row outside the mode's colour rows or a buffer that does not match the
/// document.
// ponytail: the derived-mode planes (CMYK inks, Lab) are the naive
// conversions the thumbnails already use, not real separations.
pub(super) fn channel_row_selection_from(
    doc: &Document,
    bands: impl FnOnce(&mut dyn FnMut(u32, PixelBuffer)),
    row: i32,
    mask: u8,
) -> Option<Selection> {
    if doc.source.multichannel_plates() {
        return None;
    }
    let row = usize::try_from(row).ok()?;
    let mode = doc.source.mode().unwrap_or(doc.mode);
    let (composite, channels) = color_rows(mode);
    if row >= channels + usize::from(composite) {
        return None;
    }
    let w = doc.width as usize;
    let h = doc.height as usize;
    if w == 0 || h == 0 {
        return None;
    }
    let mut plane = pictura_core::Plane::filled(w, h, 1, 0u8);
    let mut rows = 0usize;
    let mut row_buffer = vec![0u8; w];
    // The picture arrives in full-width bands (#1274); each writes its rows.
    bands(&mut |offset, band| {
        let srgb = buffer_to_srgb(doc, &band);
        let buffer = srgb.as_ref();
        let pixels = buffer.pixel_count();
        let depth = buffer.channels as usize;
        let at = offset as usize;
        if depth == 0
            || buffer.width as usize != w
            || buffer.data.len() != pixels * depth
            || at + buffer.height as usize > h
        {
            return;
        }
        let samples = buffer.data.as_slice();
        for dy in 0..buffer.height as usize {
            for (x, out) in row_buffer.iter_mut().enumerate() {
                *out = channel_row_value(
                    mode,
                    composite,
                    row,
                    mask,
                    samples,
                    pixels,
                    depth,
                    dy * w + x,
                );
            }
            if !plane.write_row_from(at + dy, w, &row_buffer) {
                return;
            }
        }
        rows += buffer.height as usize;
    });
    (rows == h).then(|| Selection {
        width: doc.width,
        height: doc.height,
        data: plane,
    })
}

/// [`channel_row_selection_from`] over one whole buffer.
#[cfg(test)]
pub(super) fn channel_row_selection(
    doc: &Document,
    picture: &PixelBuffer,
    row: i32,
    mask: u8,
) -> Option<Selection> {
    channel_row_selection_from(doc, |f| f(0, picture.clone()), row, mask)
}

/// Pixel `i` of a planar band (`plane` pixels, `depth` planes) on Load Channel
/// Selection's `row`.
#[allow(clippy::too_many_arguments)]
fn channel_row_value(
    mode: ColorMode,
    composite: bool,
    row: usize,
    mask: u8,
    samples: &[u8],
    plane: usize,
    depth: usize,
    i: usize,
) -> u8 {
    let r = samples[i];
    let (g, b) = if depth >= 3 {
        (samples[plane + i], samples[2 * plane + i])
    } else {
        (r, r)
    };
    if composite && row == 0 {
        // The composite row follows the visible channels.
        q_gray(
            if mask & 0x1 != 0 { r } else { 0 },
            if mask & 0x2 != 0 { g } else { 0 },
            if mask & 0x4 != 0 { b } else { 0 },
        )
    } else {
        // A colour row is its own plane regardless of the eyes: CS6 loads a
        // hidden channel too.
        channel_value(mode, if composite { row - 1 } else { row }, r, g, b)
    }
}
