//! Gradient Map (`grdm`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, as `color_balance.rs` is. The layout is psd-tools' `GradientMap`
//! struct.

use pictura_adjust::{Adjustment, GradientMapParams, GradientStop, OpacityStop};
use pictura_core::AdjustmentData;

use crate::composite::{be_u16, be_u32};

/// `grdm`: the legacy Gradient Map struct (psd-tools `GradientMap`). Layout:
/// `u16` version (1 or 3), `u8` reverse, `u8` dither, a `4`-byte method when
/// version 3, a unicode name (`u32` UTF-16 char count + data), a `u16` colour
/// stop count, then each stop: `u32` location, `u32` midpoint, `u16` mode, four
/// `u16` colour components, `2` pad bytes; then a `u16` transparency stop count
/// and each: `u32` location, `u32` midpoint, `u16` opacity. The trailing
/// gradient fields are ignored, as are truncated or unordered transparency
/// stops (the map is then opaque).
///
/// ponytail: the first three components are the only colour read, reduced with
/// `>> 8` so `65535` maps to `255`; opacity is read as a percent and clamped,
/// an inferred scale (fully opaque reads alike as 100 or 255); midpoint bias,
/// dither, and non-RGB colour models are not modelled.
pub(crate) fn decode_gradient_map(d: &[u8]) -> Option<Adjustment> {
    let version = be_u16(d, 0)?;
    if version != 1 && version != 3 {
        return None;
    }
    let reverse = *d.get(2)? != 0;
    let _dither = *d.get(3)?;
    let mut at = if version == 3 { 8 } else { 4 };
    let name_chars = be_u32(d, at)? as usize;
    at += 4 + name_chars * 2;
    let count = be_u16(d, at)?;
    at += 2;
    if count < 2 {
        return None;
    }
    let mut stops = Vec::with_capacity(count as usize);
    let mut previous: Option<u16> = None;
    for _ in 0..count {
        let location = be_u32(d, at)?;
        if location > 4096 {
            return None;
        }
        let location = location as u16;
        if previous.is_some_and(|p| location <= p) {
            return None;
        }
        previous = Some(location);
        let color = [
            (be_u16(d, at + 10)? >> 8) as u8,
            (be_u16(d, at + 12)? >> 8) as u8,
            (be_u16(d, at + 14)? >> 8) as u8,
        ];
        stops.push(GradientStop { location, color });
        at += 20;
    }
    let transparency = decode_opacity_stops(d, at).unwrap_or_default();
    Some(Adjustment::GradientMap(GradientMapParams {
        stops,
        reverse,
        transparency,
    }))
}

/// The Dither flag of `grdm` data, or `None` when it is too short. Kept apart
/// from [`decode_gradient_map`] because the adjustment model does not carry it.
pub fn gradient_map_dither(d: &[u8]) -> Option<bool> {
    d.get(3).map(|flag| *flag != 0)
}

fn decode_opacity_stops(d: &[u8], at: usize) -> Option<Vec<OpacityStop>> {
    let count = be_u16(d, at)? as usize;
    let mut stops = Vec::with_capacity(count);
    for i in 0..count {
        let item = at + 2 + i * 10;
        let location = u16::try_from(be_u32(d, item)?)
            .ok()
            .filter(|l| *l <= 4096)?;
        let opacity = be_u16(d, item + 8)?.min(100) as u8;
        if stops
            .last()
            .is_some_and(|s: &OpacityStop| location < s.location)
        {
            return None;
        }
        stops.push(OpacityStop { location, opacity });
    }
    if stops.iter().all(|s| s.opacity == 100) {
        stops.clear();
    }
    Some(stops)
}

/// `grdm`: the version-1 Gradient Map block. Writes the reverse and dither
/// flags, an empty unicode name, the colour stops (8-bit colours scaled to the
/// 16-bit storage scale), the transparency stops (opacity in percent), and
/// psd-tools' trailing defaults, padded to a 4-byte boundary.
pub fn encode_gradient_map(p: &GradientMapParams, dither: bool) -> AdjustmentData {
    let stops = &p.stops;
    let mut data = Vec::new();
    data.extend_from_slice(&1u16.to_be_bytes());
    data.push(u8::from(p.reverse));
    data.push(u8::from(dither));
    data.extend_from_slice(&0u32.to_be_bytes()); // empty unicode name
    data.extend_from_slice(&(stops.len() as u16).to_be_bytes());
    for stop in stops {
        data.extend_from_slice(&(stop.location as u32).to_be_bytes());
        data.extend_from_slice(&50u32.to_be_bytes()); // midpoint
        data.extend_from_slice(&0u16.to_be_bytes()); // mode
        for c in stop.color {
            let v = (c as u16) * 257; // inverse of the decoder's `>> 8`
            data.extend_from_slice(&v.to_be_bytes());
        }
        data.extend_from_slice(&0u16.to_be_bytes()); // alpha
        data.extend_from_slice(&[0, 0]); // stop pad
    }
    data.extend_from_slice(&(p.transparency.len() as u16).to_be_bytes());
    for stop in &p.transparency {
        data.extend_from_slice(&(stop.location as u32).to_be_bytes());
        data.extend_from_slice(&50u32.to_be_bytes()); // midpoint
        data.extend_from_slice(&(stop.opacity as u16).to_be_bytes());
    }
    data.extend_from_slice(&2u16.to_be_bytes()); // expansion
    data.extend_from_slice(&0u16.to_be_bytes()); // interpolation
    data.extend_from_slice(&32u16.to_be_bytes()); // length
    data.extend_from_slice(&0u16.to_be_bytes()); // mode
    data.extend_from_slice(&0u32.to_be_bytes()); // random seed
    data.extend_from_slice(&0u16.to_be_bytes()); // show transparency
    data.extend_from_slice(&0u16.to_be_bytes()); // use vector color
    data.extend_from_slice(&0u32.to_be_bytes()); // roughness
    data.extend_from_slice(&0u16.to_be_bytes()); // color model
    data.extend_from_slice(&[0u8; 16]); // min/max colour (4H each)
    data.extend_from_slice(&[0, 0]); // dummy
    while data.len() % 4 != 0 {
        data.push(0);
    }
    AdjustmentData {
        key: *b"grdm",
        data: data.into(),
    }
}
