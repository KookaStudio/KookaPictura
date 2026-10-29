//! Channel Mixer (`mixr`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring `color_balance.rs`. The layout is ag-psd's, not psd-tools'
//! (whose `ChannelMixer` reads only the red row): a `u16` version (1), a `u16`
//! monochrome flag, the `red`/`green`/`blue` channels when the flag is clear,
//! always a `gray` channel, and each channel three big-endian `i16` source
//! percentages, two reserved bytes, and one big-endian `i16` constant.

use pictura_adjust::{Adjustment, ChannelMixerParams};
use pictura_core::AdjustmentData;

use crate::composite::{be_i16, be_u16};

fn read_channel(d: &[u8], at: usize) -> Option<([f64; 3], f64)> {
    let mut rgb = [0.0f64; 3];
    for (i, source) in rgb.iter_mut().enumerate() {
        let value = be_i16(d, at + i * 2)?;
        if !(-200..=200).contains(&value) {
            return None;
        }
        *source = value as f64;
    }
    let constant = be_i16(d, at + 8)?;
    if !(-200..=200).contains(&constant) {
        return None;
    }
    Some((rgb, constant as f64))
}

/// `mixr`: version must be `1`, the channels must fit, and every source
/// percentage and constant must be in `-200..=200`; otherwise `None`. Trailing
/// bytes are ignored.
///
/// The op's monochrome branch reads only `red` and `constant[0]`, so the
/// unused `green`/`blue` rows carry the identity defaults rather than fabricated
/// data.
pub(crate) fn decode_channel_mixer(d: &[u8]) -> Option<Adjustment> {
    if be_i16(d, 0)? != 1 {
        return None;
    }
    let monochrome = be_u16(d, 2)? != 0;
    let mut at = 4;
    let mut rgb = [[0.0f64; 3]; 3];
    let mut constants = [0.0f64; 3];
    if !monochrome {
        for (row, constant) in rgb.iter_mut().zip(constants.iter_mut()) {
            (*row, *constant) = read_channel(d, at)?;
            at += 10;
        }
    }
    let (gray, gray_constant) = read_channel(d, at)?;
    if monochrome {
        return Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: gray,
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [gray_constant, 0.0, 0.0],
        }));
    }
    Some(Adjustment::ChannelMixer(ChannelMixerParams {
        monochrome: false,
        red: rgb[0],
        green: rgb[1],
        blue: rgb[2],
        constant: constants,
    }))
}

fn write_channel(data: &mut Vec<u8>, rgb: [f64; 3], constant: f64) {
    for value in rgb {
        data.extend_from_slice(&clamp_i16(value).to_be_bytes());
    }
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&clamp_i16(constant).to_be_bytes());
}

fn clamp_i16(value: f64) -> i16 {
    value.clamp(-200.0, 200.0).round() as i16
}

/// `mixr`: version 1, the monochrome flag, `red`/`green`/`blue` followed by a
/// `gray` row derived from `red`/`constant[0]` when not monochrome, or that
/// `gray` row followed by 30 zero bytes when monochrome, so the block is 44
/// bytes in both forms. Every source percentage and constant is clamped to
/// `-200..=200`, so the output always decodes.
pub fn encode_channel_mixer(
    monochrome: bool,
    red: [f64; 3],
    green: [f64; 3],
    blue: [f64; 3],
    constant: [f64; 3],
) -> AdjustmentData {
    let mut data = Vec::with_capacity(44);
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&u16::from(monochrome).to_be_bytes());
    if monochrome {
        write_channel(&mut data, red, constant[0]);
        data.extend_from_slice(&[0; 30]);
    } else {
        write_channel(&mut data, red, constant[0]);
        write_channel(&mut data, green, constant[1]);
        write_channel(&mut data, blue, constant[2]);
        write_channel(&mut data, red, constant[0]);
    }
    AdjustmentData {
        key: *b"mixr",
        data: data.into(),
    }
}
