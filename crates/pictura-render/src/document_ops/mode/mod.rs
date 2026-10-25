//! `Image > Mode` color-mode and bit-depth conversions (`docs/04-image-ops/
//! image-modes.md` IMG-004, `bit-depth-and-conversion.md` IMG-005,
//! `indexed-color.md` IMG-008). Ported from photorust's `Document::
//! set_color_mode` / `convert_to_indexed` / `set_bit_depth`, re-seated on this
//! engine's model: the working pixels stay 8-bit RGB or Grayscale and the target
//! mode/depth is carried by the `source_mode` / `source_planes` /
//! `source_palette` / `source_depth` stores the PSD writer already re-emits, so
//! a converted document saves in its new mode instead of only being relabelled.
//!
//! ponytail: Duotone and Multichannel (no ink/plate model to author), the Color
//! Table, Bitmap Halftone Screen / Custom Pattern, and Grayscale-from-Bitmap's
//! size ratio are not converted; CMYK/Lab are profile-free (no ICC working
//! space); a conversion into CMYK/Lab of a 16-bit document, or out of one,
//! passes through the 8-bit working planes.

mod flat;
mod planes;

pub use flat::{convert_to_bitmap, convert_to_indexed, indexed_exact_available, BitmapMethod};

use std::borrow::Cow;

use pictura_codec::{ConversionOptions, Profile};
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer};
use pictura_ops::OpsError;

/// The mode the document is in for the user: the recorded source mode when one
/// is carried, else the working mode.
pub fn document_color_mode(doc: &Document) -> ColorMode {
    doc.source_mode.unwrap_or(doc.mode)
}

/// The depth the document is in for the user: 1 for a Bitmap, the retained
/// native depth when a store carries one, else 8.
pub fn document_bit_depth(doc: &Document) -> BitDepth {
    if doc.source_mode == Some(ColorMode::Bitmap) {
        return BitDepth::One;
    }
    if doc.retains_source_depth() {
        doc.source_depth.unwrap_or(BitDepth::Eight)
    } else {
        BitDepth::Eight
    }
}

/// Whether `Image > Mode > target` is available: the IMG-004 conversion matrix
/// and depth limits (Bitmap and Indexed need 8-bit Grayscale / Grayscale-RGB,
/// CMYK and Lab stop at 16-bit, a Bitmap leaves only through Grayscale).
/// False for the current mode and for the modes this engine does not convert.
pub fn can_convert_mode(doc: &Document, target: ColorMode) -> bool {
    if doc.width == 0 || doc.height == 0 {
        return false;
    }
    let from = document_color_mode(doc);
    let depth = document_bit_depth(doc);
    if from == target {
        return false;
    }
    match target {
        ColorMode::Bitmap => {
            from == ColorMode::Grayscale
                && doc.mode == ColorMode::Grayscale
                && depth == BitDepth::Eight
        }
        ColorMode::Grayscale => true,
        ColorMode::Rgb => from != ColorMode::Bitmap,
        ColorMode::Cmyk | ColorMode::Lab => {
            from != ColorMode::Bitmap && matches!(depth, BitDepth::Eight | BitDepth::Sixteen)
        }
        ColorMode::Indexed => {
            matches!(from, ColorMode::Rgb | ColorMode::Grayscale) && depth == BitDepth::Eight
        }
        ColorMode::Duotone | ColorMode::Multichannel => false,
    }
}

/// Whether `Image > Mode > <out> Bits/Channel` is available: Grayscale/RGB
/// take any depth, CMYK/Lab 8 or 16, and Bitmap/Indexed/Duotone/Multichannel
/// none. A 32-bit source is allowed here; it converts through
/// [`crate::convert_depth_exposure_gamma`], not [`convert_bit_depth`].
pub fn can_convert_depth(doc: &Document, out: BitDepth) -> bool {
    if doc.width == 0 || doc.height == 0 || document_bit_depth(doc) == out {
        return false;
    }
    match document_color_mode(doc) {
        ColorMode::Grayscale | ColorMode::Rgb => out != BitDepth::One,
        ColorMode::Cmyk | ColorMode::Lab => matches!(out, BitDepth::Eight | BitDepth::Sixteen),
        _ => false,
    }
}

/// Convert `doc` to Grayscale, RGB, CMYK, or Lab (Bitmap and Indexed have their
/// own options: [`convert_to_bitmap`], [`convert_to_indexed`]).
///
/// Grayscale is a Rec. 601 luma of the working RGB (at native depth when a
/// 16/32-bit store is retained); RGB replicates gray or drops the recorded
/// source mode; CMYK and Lab encode each color plane set in the target mode
/// and show the working RGB the encoding decodes to. The document's depth is
/// kept. `Err` without mutating when [`can_convert_mode`] refuses.
pub fn convert_mode(doc: &mut Document, target: ColorMode) -> Result<(), OpsError> {
    convert_mode_with_profiles(
        doc,
        target,
        None,
        ConversionOptions {
            dither: false,
            ..Default::default()
        },
    )
}

/// [`convert_mode`] with the application's working CMYK profile and conversion
/// options. The profile is attached to resource 1039 after a CMYK conversion.
pub fn convert_mode_with_profiles(
    doc: &mut Document,
    target: ColorMode,
    working_cmyk: Option<&Profile>,
    options: ConversionOptions,
) -> Result<(), OpsError> {
    if !can_convert_mode(doc, target) || matches!(target, ColorMode::Bitmap | ColorMode::Indexed) {
        return Err(OpsError::Unsupported(format!(
            "cannot convert {:?} to {target:?}",
            document_color_mode(doc)
        )));
    }
    let depth = document_bit_depth(doc);
    if doc.layers.is_empty() && doc.source_mode == Some(ColorMode::Bitmap) {
        planes::background_from_composite(doc);
    }
    planes::drop_source_state(doc);
    match target {
        ColorMode::Grayscale => planes::reduce_to_gray(doc),
        ColorMode::Rgb => planes::expand_gray(doc),
        _ => {
            planes::expand_gray(doc);
            doc.source_planes = None;
            planes::drop_layer_stores(doc);
            planes::encode_document(doc, target, working_cmyk, options);
            if target == ColorMode::Cmyk {
                if let Some(profile) = working_cmyk {
                    pictura_codec::assign_cmyk_profile(doc, profile);
                }
            }
        }
    }
    if depth != BitDepth::Eight && !doc.retains_source_depth() {
        planes::ensure_stores(doc);
        planes::restore_depth(doc, depth);
        doc.source_depth = Some(depth);
    }
    Ok(())
}

/// Convert `doc` between 8, 16, and 32 bits per channel.
///
/// Up-conversion builds (or widens) the retained stores a save re-emits: 8->16
/// is `v * 257`, 8->32 `v / 255`, 16->32 `v / 65535`. 16->8 narrows with the
/// shared `narrow_to_u8` rule (the high byte) and keeps CMYK/Lab's encoded
/// store at 8-bit. A 32-bit source is refused: it needs the HDR Conversion
/// tone map. `Err` without mutating when [`can_convert_depth`] refuses.
pub fn convert_bit_depth(doc: &mut Document, out: BitDepth) -> Result<(), OpsError> {
    let from = document_bit_depth(doc);
    if !can_convert_depth(doc, out) || from == BitDepth::ThirtyTwo {
        return Err(OpsError::Unsupported(format!(
            "cannot convert {from:?} to {out:?}"
        )));
    }
    if out == BitDepth::Eight {
        if doc.source_mode.is_some() {
            planes::restore_depth(doc, BitDepth::Eight);
        } else {
            doc.source_planes = None;
            planes::drop_layer_stores(doc);
        }
        doc.source_depth = None;
        return Ok(());
    }
    planes::ensure_stores(doc);
    planes::restore_depth(doc, out);
    doc.source_depth = Some(out);
    Ok(())
}

/// `doc` as the PSD writer expects it. The app keeps an RGB composite as RGBA
/// for display, but a document carrying a source mode (CMYK, Lab, Indexed,
/// Bitmap, ...) is written from three color planes, so the display alpha plane
/// is dropped; any other document is borrowed unchanged.
pub fn save_view(doc: &Document) -> Cow<'_, Document> {
    let plane = doc.width as usize * doc.height as usize;
    if doc.source_mode.is_none()
        || doc.mode != ColorMode::Rgb
        || doc.composite.channels != 4
        || doc.composite.data.len() < 3 * plane
    {
        return Cow::Borrowed(doc);
    }
    let mut trimmed = doc.clone();
    trimmed.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 3,
        data: doc.composite.data[..3 * plane].to_vec().into(),
    };
    Cow::Owned(trimmed)
}

#[cfg(test)]
mod tests;
