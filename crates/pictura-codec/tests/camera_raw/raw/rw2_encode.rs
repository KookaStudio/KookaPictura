//! Encoders for the RW2 codings the reader takes from rawspeed (#1541). Each
//! writes samples exactly or panics: the samples must be representable, which
//! `tests/camera_raw/raw_rw2_codings.rs` arranges.

const BLOCK: usize = 0x4000;
const SPLIT: usize = 0x1FF8;

/// The inverse of the reader's unswap: stored = logical[BLOCK - split..] then
/// logical[..BLOCK - split], for each whole block.
fn swap_blocks(mut logical: Vec<u8>, split: usize) -> Vec<u8> {
    if split == 0 {
        return logical;
    }
    logical.resize(logical.len().div_ceil(BLOCK) * BLOCK, 0);
    logical
        .as_chunks::<BLOCK>()
        .0
        .iter()
        .flat_map(|b| [&b[BLOCK - split..], &b[..BLOCK - split]].concat())
        .collect()
}

/// RawFormat 5: groups from the low bit, padded with set bits, in swapped
/// blocks.
pub fn blocked(samples: &[u16], bits: u32) -> Vec<u8> {
    let per_group = (128 / bits) as usize;
    let logical: Vec<u8> = samples
        .chunks(per_group)
        .flat_map(|group| {
            let mut value = u128::MAX << (bits as usize * per_group);
            for (k, &v) in group.iter().enumerate() {
                value |= u128::from(v) << (bits as usize * k);
            }
            value.to_le_bytes()
        })
        .collect();
    swap_blocks(logical, SPLIT)
}

/// RawFormat 3 and 4's bit writer: the reader's addressing, run forwards.
struct PredictiveWriter {
    buf: Vec<u8>,
    vbits: u32,
}

impl PredictiveWriter {
    fn put(&mut self, value: u32, n: u32) {
        assert!(value < 1 << n, "{value} in {n} bits");
        self.vbits = self.vbits.wrapping_sub(n) & 0x1ffff;
        let byte = (self.vbits >> 3) as usize ^ 0x3ff0;
        let word = value << (self.vbits & 7);
        self.buf[byte] |= word as u8;
        self.buf[byte + 1] |= (word >> 8) as u8;
    }
}

/// The fields that make the reader output `t` for the photosite at `p`,
/// updating its state; `None` when `sh` cannot.
fn predictive_pixel(
    sh: i32,
    pred: &mut i32,
    nonz: &mut i32,
    p: usize,
    t: i32,
) -> Option<Vec<(u32, u32)>> {
    if *nonz == 0 {
        return if t >= 16 {
            (*nonz, *pred) = (t >> 4, t);
            Some(vec![((t >> 4) as u32, 8), ((t & 15) as u32, 4)])
        } else if p > 11 {
            *pred = t;
            Some(vec![(0, 8), (t as u32, 4)])
        } else {
            (t == *pred).then(|| vec![(0, 8)])
        };
    }
    (0..=255).find_map(|j: i32| {
        let mut next = *pred;
        if j != 0 {
            next -= 0x80 << sh;
            if next < 0 || sh == 4 {
                next &= (1 << sh) - 1;
            }
            next += j << sh;
        }
        (next == t).then(|| {
            *pred = next;
            vec![(j as u32, 8)]
        })
    })
}

fn predictive_packet(w: &mut PredictiveWriter, values: &[u16]) {
    let (mut pred, mut nonz) = ([0i32; 2], [0i32; 2]);
    let mut p = 0;
    while p < values.len() {
        let run = if p % 3 == 2 { 3 } else { 1 };
        let end = (p + run).min(values.len());
        let shifts: &[(i32, u32)] = if p % 3 == 2 {
            &[(0, 0), (1, 1), (2, 2), (4, 3)]
        } else {
            &[(0, 0)]
        };
        let (code, fields, state) = shifts
            .iter()
            .find_map(|&(sh, code)| {
                let (mut pr, mut nz) = (pred, nonz);
                let mut fields = Vec::new();
                for (q, &t) in values.iter().enumerate().take(end).skip(p) {
                    let c = q & 1;
                    fields.extend(predictive_pixel(
                        sh,
                        &mut pr[c],
                        &mut nz[c],
                        q,
                        i32::from(t),
                    )?);
                }
                Some((code, fields, (pr, nz)))
            })
            .unwrap_or_else(|| panic!("photosites {p}..{end} of {values:?} cannot be coded"));
        if p % 3 == 2 {
            w.put(code, 2);
        }
        for (value, n) in fields {
            w.put(value, n);
        }
        (pred, nonz) = state;
        p = end;
    }
}

/// RawFormat 3 (`split` false: no swap, a short last block) or 4.
pub fn predictive(samples: &[u16], split: bool) -> Vec<u8> {
    let per_block = BLOCK / 16 * 14;
    let mut logical = Vec::new();
    for block in samples.chunks(per_block) {
        let mut w = PredictiveWriter {
            buf: vec![0; BLOCK + 1],
            vbits: 0,
        };
        for packet in block.chunks(14) {
            predictive_packet(&mut w, packet);
        }
        logical.extend(&w.buf[..block.len() / 14 * 16]);
    }
    swap_blocks(logical, if split { SPLIT } else { 0 })
}

/// RawFormat 6's field widths in the order the reader uses them.
fn paired_widths(bits: u32) -> Vec<u32> {
    let value = if bits == 14 { 10 } else { 8 };
    let triples = if bits == 14 { 3 } else { 4 };
    let mut widths = vec![bits, bits];
    for _ in 0..triples {
        widths.extend([2, value, value, value]);
    }
    widths
}

/// RawFormat 6: 11 (14-bit) or 14 (12-bit) photosites per 16 bytes.
pub fn paired(samples: &[u16], bits: u32) -> Vec<u8> {
    let per = if bits == 14 { 11 } else { 14 };
    let (base_zero, base_compare) = if bits == 14 {
        (0x200, 0x2000)
    } else {
        (0x80, 0x800)
    };
    let value_bits = if bits == 14 { 10 } else { 8 };
    let widths = paired_widths(bits);
    samples
        .chunks_exact(per)
        .flat_map(|block| {
            let mut fields = Vec::new();
            let mut nonzero = [0u32; 2];
            for (pix, &v) in block.iter().enumerate().take(2) {
                let e = u32::from(v) + 15;
                fields.push(e);
                nonzero[pix] = e;
            }
            for triple in (2..per).step_by(3) {
                let (code, values) = [(0u32, 0u32), (1, 1), (2, 2), (4, 3)]
                    .iter()
                    .find_map(|&(base, code)| {
                        let pixel_base = base_zero << base;
                        let mut nz = nonzero;
                        let mut values = Vec::new();
                        for (pix, &v) in block.iter().enumerate().take(triple + 3).skip(triple) {
                            let e = u32::from(v) + 15;
                            let c = pix % 2;
                            let offset = if pixel_base < base_compare && nz[c] > pixel_base {
                                nz[c] - pixel_base
                            } else {
                                0
                            };
                            let f = e.checked_sub(offset)?;
                            if f % (1 << base) != 0 || f >> base >= 1 << value_bits {
                                return None;
                            }
                            values.push(f >> base);
                            nz[c] = e;
                        }
                        nonzero = nz;
                        Some((code, values))
                    })
                    .unwrap_or_else(|| panic!("triple {triple} of {block:?} cannot be coded"));
                fields.push(code);
                fields.extend(values);
            }
            let skip = if bits == 14 { 4 } else { 0 };
            let mut packed = 0u128;
            let mut at = 128;
            for (&f, &width) in fields.iter().zip(&widths) {
                at -= width;
                packed |= u128::from(f) << at;
            }
            assert_eq!(at, skip, "packet layout");
            packed.to_le_bytes()
        })
        .collect()
}

/// RawFormat 8's code lengths by difference category; the codes are
/// canonical.
pub const CODE_LENGTHS: [u16; 17] = [3, 3, 3, 3, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 13];

/// `(length, code)` for each category, in category order.
pub fn codes() -> Vec<(u16, u16)> {
    let mut order: Vec<usize> = (0..17).collect();
    order.sort_by_key(|&c| (CODE_LENGTHS[c], c));
    let mut out = vec![(0, 0); 17];
    let (mut code, mut len) = (0u32, CODE_LENGTHS[order[0]]);
    for c in order {
        code <<= CODE_LENGTHS[c] - len;
        len = CODE_LENGTHS[c];
        out[c] = (len, code as u16);
        code += 1;
    }
    out
}

/// One RawFormat 8 strip of `width` × `height` from `plane` at `x`, `y`:
/// the bytes and their length in bits.
pub fn strip(
    plane: &[u16],
    plane_width: usize,
    (x, y, width, height): (usize, usize, usize, usize),
    mut pred: [u16; 4],
) -> (Vec<u8>, usize) {
    let codes = codes();
    let mut bits: Vec<bool> = Vec::new();
    let mut push = |value: u32, n: u32| {
        bits.extend((0..n).rev().map(|k| value >> k & 1 == 1));
    };
    let at = |row: usize, col: usize| plane[(y + row) * plane_width + x + col];
    for row in (0..height).step_by(2) {
        for col in (0..width).step_by(2) {
            for dc in 0..2 {
                for dr in 0..2 {
                    let v = at(row + dr, col + dc);
                    let d = i32::from(v) - i32::from(pred[dr * 2 + dc]);
                    let category = 32 - d.unsigned_abs().leading_zeros();
                    let (len, code) = codes[category as usize];
                    push(u32::from(code), u32::from(len));
                    if category > 0 {
                        let stored = if d > 0 { d } else { d + (1 << category) - 1 };
                        push(stored as u32, category);
                    }
                    pred[dr * 2 + dc] = v;
                }
            }
        }
        for dr in 0..2 {
            for dc in 0..2 {
                pred[dr * 2 + dc] = at(row + dr, dc);
            }
        }
    }
    let bytes = bits
        .chunks(8)
        .map(|byte| {
            byte.iter()
                .enumerate()
                .fold(0u8, |a, (k, &b)| a | u8::from(b) << (7 - k))
                .reverse_bits()
        })
        .collect();
    (bytes, bits.len())
}
