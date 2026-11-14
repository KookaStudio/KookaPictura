//! The header's colour mode code and an Indexed file's palette.

use pictura_core::ColorMode;

use crate::common::*;
use crate::error::PsdError;

/// The PSD `header.color_mode` code to the engine's [`ColorMode`]. Bitmap,
/// Grayscale, Indexed, RGB, CMYK, Multichannel, Duotone, and Lab are accepted;
/// every other code is unsupported.
pub(super) fn color_mode_from_code(code: u16) -> Result<ColorMode, PsdError> {
    Ok(match code {
        MODE_BITMAP => ColorMode::Bitmap,
        MODE_GRAYSCALE => ColorMode::Grayscale,
        MODE_INDEXED => ColorMode::Indexed,
        MODE_RGB => ColorMode::Rgb,
        MODE_CMYK => ColorMode::Cmyk,
        MODE_MULTICHANNEL => ColorMode::Multichannel,
        MODE_DUOTONE => ColorMode::Duotone,
        MODE_LAB => ColorMode::Lab,
        c => return Err(PsdError::Unsupported(format!("color mode {c}"))),
    })
}

/// The 768-byte Indexed palette (256 red, then green, then blue), or `None` for
/// a mode that has no palette. Any other length is malformed, never a panic.
pub(super) fn palette_from(mode: ColorMode, data: &[u8]) -> Result<Option<[u8; 768]>, PsdError> {
    if mode != ColorMode::Indexed {
        return Ok(None);
    }
    if data.len() != 768 {
        return Err(PsdError::Invalid(format!(
            "indexed palette length {}",
            data.len()
        )));
    }
    let mut palette = [0u8; 768];
    palette.copy_from_slice(data);
    Ok(Some(palette))
}
