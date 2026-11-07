//! Channel Mixer (`mixr`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring `color_balance.rs`. The layout is ag-psd's, not psd-tools'
//! (whose `ChannelMixer` reads only the red row): a `u16` version (1), a `u16`
//! monochrome flag, then 10-byte channel rows. A row is four big-endian `i16`
//! sources and one big-endian `i16` constant; ag-psd calls the fourth source
//! two reserved bytes, so an RGB row uses three sources, the reserved pair, and
//! the constant, while a CMYK row uses all four sources (the fourth is K).
//!
//! A non-monochrome block carries four rows (red/green/blue/gray in RGB,
//! C/M/Y/K in CMYK); a monochrome block carries one gray row followed by 30
//! zero bytes. The CMYK interpretation is inferred: no independent decoder
//! models it, so it is covered by round-trip and hand-built bytes only.

use pictura_adjust::{Adjustment, ChannelMixerParams};
use pictura_core::AdjustmentData;

use crate::composite::{be_i16, be_u16};

fn read_row(d: &[u8], at: usize) -> Option<([f64; 4], f64)> {
    let mut sources = [0.0f64; 4];
    for (i, source) in sources.iter_mut().enumerate() {
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
    Some((sources, constant as f64))
}

/// `mixr`: version must be `1`, the channels must fit, and every source
/// percentage and constant must be in `-200..=200`; otherwise `None`. Trailing
/// bytes are ignored.
///
/// Both views are filled from the same bytes: the RGB rows use the first three
/// sources of rows 0..3, and `ink` uses all four sources of every row. The
/// monochrome branch reads only the gray row, as the op's monochrome path does.
pub(crate) fn decode_channel_mixer(d: &[u8]) -> Option<Adjustment> {
    if be_i16(d, 0)? != 1 {
        return None;
    }
    let monochrome = be_u16(d, 2)? != 0;
    if monochrome {
        let (gray, gray_constant) = read_row(d, 4)?;
        return Some(Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [gray[0], gray[1], gray[2]],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [gray_constant, 0.0, 0.0],
            ..Default::default()
        }));
    }
    let mut rows = [[0.0f64; 4]; 4];
    let mut constants = [0.0f64; 4];
    for (index, (row, constant)) in rows.iter_mut().zip(constants.iter_mut()).enumerate() {
        (*row, *constant) = read_row(d, 4 + index * 10)?;
    }
    Some(Adjustment::ChannelMixer(ChannelMixerParams {
        monochrome: false,
        red: [rows[0][0], rows[0][1], rows[0][2]],
        green: [rows[1][0], rows[1][1], rows[1][2]],
        blue: [rows[2][0], rows[2][1], rows[2][2]],
        constant: [constants[0], constants[1], constants[2]],
        ink: rows,
        ink_constant: constants,
    }))
}

fn write_channel(data: &mut Vec<u8>, rgb: [f64; 3], constant: f64) {
    for value in rgb {
        data.extend_from_slice(&clamp_i16(value).to_be_bytes());
    }
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&clamp_i16(constant).to_be_bytes());
}

fn write_ink_row(data: &mut Vec<u8>, sources: [f64; 4], constant: f64) {
    for value in sources {
        data.extend_from_slice(&clamp_i16(value).to_be_bytes());
    }
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

/// The CMYK form: four rows of four sources and a constant, so the fourth
/// source occupies the pair an RGB row leaves reserved. Every value is clamped
/// to `-200..=200`, so the output always decodes.
pub fn encode_channel_mixer_ink(ink: [[f64; 4]; 4], ink_constant: [f64; 4]) -> AdjustmentData {
    let mut data = Vec::with_capacity(44);
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());
    for (row, constant) in ink.into_iter().zip(ink_constant) {
        write_ink_row(&mut data, row, constant);
    }
    AdjustmentData {
        key: *b"mixr",
        data: data.into(),
    }
}
