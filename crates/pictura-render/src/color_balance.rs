//! Color Balance (`blnc`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring how `fill.rs` holds the gradient-fill codec. The layout is
//! the psd-tools `ColorBalance` fixed struct: nine big-endian `i16` shifts in
//! shadows, midtones, highlights order, a `u8` luminosity flag, and padding to a
//! 4-byte boundary.

use pictura_adjust::{Adjustment, ColorBalanceParams};
use pictura_core::AdjustmentData;

use crate::composite::be_i16;

/// `blnc`: a truncated payload or a shift outside `-100..=100` is `None`; every
/// byte after the luminosity flag is ignored.
///
/// ponytail: the reference's per-band preserve-luminosity tri-state is not
/// representable in the fixed struct; it collapses to the single boolean the
/// engine models (`ColorBalanceParams`). The band weighting lives in
/// `pictura-adjust`.
pub(crate) fn decode_color_balance(d: &[u8]) -> Option<Adjustment> {
    let mut bands = [[0.0f64; 3]; 3];
    for (band_index, band) in bands.iter_mut().enumerate() {
        for (component, shift) in band.iter_mut().enumerate() {
            let value = be_i16(d, (band_index * 3 + component) * 2)?;
            if !(-100..=100).contains(&value) {
                return None;
            }
            *shift = value as f64;
        }
    }
    Some(Adjustment::ColorBalance(ColorBalanceParams {
        shadows: bands[0],
        midtones: bands[1],
        highlights: bands[2],
        preserve_luminosity: *d.get(18)? != 0,
    }))
}

/// `blnc`: the nine band shifts (shadows, midtones, highlights order) as
/// big-endian `i16`, the luminosity byte, and one pad byte (20 total). Each
/// shift is clamped to `-100..=100`, so the output always decodes.
pub fn encode_color_balance(
    shadows: [f64; 3],
    midtones: [f64; 3],
    highlights: [f64; 3],
    preserve_luminosity: bool,
) -> AdjustmentData {
    let mut data = Vec::with_capacity(20);
    for band in [shadows, midtones, highlights] {
        for shift in band {
            let value = shift.clamp(-100.0, 100.0).round() as i16;
            data.extend_from_slice(&value.to_be_bytes());
        }
    }
    data.push(u8::from(preserve_luminosity));
    data.push(0);
    AdjustmentData {
        key: *b"blnc",
        data: data.into(),
    }
}
