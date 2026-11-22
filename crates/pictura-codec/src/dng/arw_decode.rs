//! Sony ARW sample coding (#1470): the ARW2 block compression with its tone
//! curve, and the cipher of the SR2 private block. Built from public
//! descriptions of the format (ExifTool's Sony tag documentation for the
//! cipher and the tone-curve tag) and observation of real files, checked by
//! decoding the same scene shot compressed and uncompressed; no dcraw, LibRaw
//! or rawspeed code.

/// Photosites one pair of ARW2 blocks covers.
pub(super) const ARW2_GROUP: usize = 32;

/// One 16-byte ARW2 block: sixteen same-colour 11-bit codes. Read as a
/// 128-bit little-endian number from bit 0, it holds the maximum (11 bits),
/// the minimum (11 bits), the positions of the maximum and the minimum (4 bits
/// each), then a 7-bit delta for each other position in order. A delta is
/// scaled by the smallest shift `s` (at most 4) for which `128 << s` exceeds
/// the block's range, added to the minimum and capped at 2047.
fn decode_block(block: &[u8; 16]) -> [u16; 16] {
    let v = u128::from_le_bytes(*block);
    let bits = |at: u32, n: u32| ((v >> at) as u32) & ((1 << n) - 1);
    let (max, min) = (bits(0, 11), bits(11, 11));
    let (imax, imin) = (bits(22, 4) as usize, bits(26, 4) as usize);
    let range = max.saturating_sub(min);
    let mut shift = 0;
    while shift < 4 && (0x80 << shift) <= range {
        shift += 1;
    }
    let mut out = [0u16; 16];
    let mut at = 30;
    for (i, o) in out.iter_mut().enumerate() {
        *o = if i == imax {
            max
        } else if i == imin {
            min
        } else {
            // With both positions equal, the fifteenth delta has no bits left.
            let d = if at + 7 <= 128 { bits(at, 7) } else { 0 };
            at += 7;
            ((d << shift) + min).min(0x7FF)
        } as u16;
    }
    out
}

/// Decodes one ARW2 row of `dst.len()` codes (a multiple of 32) from `src`,
/// one byte per photosite: each 32-photosite group is a block of its even
/// columns followed by a block of its odd ones.
pub(super) fn decode_arw2_row(src: &[u8], dst: &mut [u16]) {
    let (groups, _) = src.as_chunks::<ARW2_GROUP>();
    let (outs, _) = dst.as_chunks_mut::<ARW2_GROUP>();
    for (group, out) in groups.iter().zip(outs) {
        let (blocks, _) = group.as_chunks::<16>();
        for (phase, block) in blocks.iter().enumerate() {
            for (k, code) in decode_block(block).into_iter().enumerate() {
                out[2 * k + phase] = code;
            }
        }
    }
}

/// The ARW2 tone curve (tag `0x7010`) as a 2048-entry table from 11-bit
/// codes to linear values on the 14-bit scale. Over half-code steps the curve
/// rises with slope 1, 2, 4, 8 and then 16 between knees at 0, each tag value
/// divided by 4, and 4095; a code reads the curve at twice its value.
pub(super) fn tone_curve(knees: [u16; 4]) -> Vec<u16> {
    let mut k = [0usize, 0, 0, 0, 0, 4095];
    for (dst, v) in k[1..5].iter_mut().zip(knees) {
        *dst = (v as usize >> 2) & 0xFFF;
    }
    let mut curve: Vec<u32> = (0..=4096).collect();
    for i in 0..5 {
        for j in k[i] + 1..=k[i + 1] {
            curve[j] = curve[j - 1] + (1 << i);
        }
    }
    (0..2048)
        .map(|code| curve[2 * code].min(65535) as u16)
        .collect()
}

/// Deciphers Sony's SR2 private block in place. The key stream is 32-bit
/// words: four seeded from `key` by the step `k × 48828125 + 1`, the fourth
/// folded with the first and third, then a lagged-XOR feedback register of
/// 127 words; each word is XORed onto the block read as big-endian words.
/// Bytes past the last whole word are left as they are.
pub(super) fn decipher_sr2(data: &mut [u8], mut key: u32) {
    let mut pad = [0u32; 128];
    for word in &mut pad[..4] {
        key = key.wrapping_mul(48_828_125).wrapping_add(1);
        *word = key;
    }
    pad[3] = (pad[3] << 1) | ((pad[0] ^ pad[2]) >> 31);
    for p in 4..127 {
        pad[p] = ((pad[p - 4] ^ pad[p - 2]) << 1) | ((pad[p - 3] ^ pad[p - 1]) >> 31);
    }
    for (p, word) in (127usize..).zip(data.as_chunks_mut::<4>().0) {
        pad[p & 127] = pad[(p + 1) & 127] ^ pad[(p + 65) & 127];
        *word = (u32::from_be_bytes(*word) ^ pad[p & 127]).to_be_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decipher_is_its_own_inverse() {
        let plain: Vec<u8> = (0..203u32).map(|i| (i * 37 % 251) as u8).collect();
        let mut data = plain.clone();
        decipher_sr2(&mut data, 0x4433_2211);
        assert_ne!(data[..200], plain[..200]);
        assert_eq!(data[200..], plain[200..]);
        decipher_sr2(&mut data, 0x4433_2211);
        assert_eq!(data, plain);
    }

    #[test]
    fn tone_curve_follows_its_knees() {
        let curve = tone_curve([8000, 10400, 12900, 14100]);
        assert_eq!(curve.len(), 2048);
        assert_eq!(curve[..4], [0, 2, 4, 6]);
        assert_eq!(curve[1000], 2000);
        // Slope 2 per half-code from 2000 to 2600.
        assert_eq!(curve[1300], 3200);
        assert!(curve.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn block_with_flat_range_keeps_its_minimum() {
        let mut v: u128 = 300 | (300 << 11) | (3 << 22) | (3 << 26);
        v |= 5 << 30;
        let codes = decode_block(&v.to_le_bytes());
        assert_eq!(codes[0], 305);
        assert_eq!(codes[3], 300);
        assert_eq!(codes[15], 300);
    }
}
