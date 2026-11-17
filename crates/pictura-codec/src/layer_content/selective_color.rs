//! Selective Color (`selc`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring `curves.rs` and `color_balance.rs`. The layout is the
//! libpsd/ag-psd/psd-tools three-way agreement: a `u16` version (must be 1), a
//! `u16` correction method (`0` relative, nonzero absolute), then ten plates of
//! four big-endian `i16` (cyan, magenta, yellow, black). Plate 0 is reserved and
//! skipped; plates 1..9 are the ranges reds, yellows, greens, cyans, blues,
//! magentas, whites, neutrals, blacks.
//!
//! ponytail: the reserved plate is ignored on read and written as zeroes; give
//! it a model only if a file is found that uses it.

use pictura_core::{Adjustment, SelectiveColorMethod, SelectiveColorParams, SelectiveRange};
use pictura_core::AdjustmentData;

use super::basic::{be_i16, be_u16};

/// The header (version + method) plus ten plates.
const LEN: usize = 4 + 10 * 8;

/// `selc`: a payload shorter than 84 bytes, a version other than 1, or a range
/// correction outside `-100..=100` is `None`, never a panic. The reserved plate
/// is skipped without validation and every byte after offset 84 is ignored.
pub fn decode_selective_color(d: &[u8]) -> Option<Adjustment> {
    if d.len() < LEN || be_u16(d, 0)? != 1 {
        return None;
    }
    let method = if be_u16(d, 2)? == 0 {
        SelectiveColorMethod::Relative
    } else {
        SelectiveColorMethod::Absolute
    };
    let mut ranges = [SelectiveRange::default(); 9];
    for (index, range) in ranges.iter_mut().enumerate() {
        let at = 12 + index * 8;
        let values = [
            be_i16(d, at)?,
            be_i16(d, at + 2)?,
            be_i16(d, at + 4)?,
            be_i16(d, at + 6)?,
        ];
        if values.iter().any(|v| !(-100..=100).contains(v)) {
            return None;
        }
        *range = SelectiveRange {
            c: values[0],
            m: values[1],
            y: values[2],
            k: values[3],
        };
    }
    Some(Adjustment::SelectiveColor(SelectiveColorParams {
        method,
        ranges,
    }))
}

/// `selc`: version 1, the method (`0` relative, `1` absolute), an 8-byte zero
/// reserved plate, and the nine range plates in order, each as big-endian
/// `i16`. Every correction is clamped to `-100..=100`, so the output always
/// decodes.
pub fn encode_selective_color(
    method: SelectiveColorMethod,
    ranges: &[SelectiveRange; 9],
) -> AdjustmentData {
    let mut data = Vec::with_capacity(LEN);
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&method_code(method).to_be_bytes());
    data.extend_from_slice(&[0u8; 8]);
    for range in ranges {
        for value in [range.c, range.m, range.y, range.k] {
            data.extend_from_slice(&value.clamp(-100, 100).to_be_bytes());
        }
    }
    AdjustmentData {
        key: *b"selc",
        data: data.into(),
    }
}

fn method_code(method: SelectiveColorMethod) -> u16 {
    match method {
        SelectiveColorMethod::Relative => 0,
        SelectiveColorMethod::Absolute => 1,
    }
}

