//! A 2^24-bit RGB cube for `Select > Similar`.
//!
//! The reference colours mark one bit each; the cube is then dilated by the
//! tolerance along each channel, so a query is one bit lookup.

use rayon::prelude::*;

const BINS: usize = 256;
const WORDS_PER_ROW: usize = 4;
const WORDS: usize = BINS * BINS * WORDS_PER_ROW;

pub(crate) struct ColorMatcher {
    bits: Vec<u64>,
}

impl ColorMatcher {
    /// Build the cube from `references` and widen it by `tolerance` on every
    /// channel. With no references the cube stays empty.
    pub(crate) fn new(references: impl IntoIterator<Item = [u8; 3]>, tolerance: u8) -> Self {
        let mut bits = vec![0u64; WORDS];
        let mut any = false;
        for colour in references {
            set(&mut bits, index(colour));
            any = true;
        }
        if any && tolerance > 0 {
            dilate(&mut bits, tolerance as usize);
        }
        Self { bits }
    }

    pub(crate) fn contains(&self, colour: [u8; 3]) -> bool {
        let index = index(colour) as usize;
        self.bits[index >> 6] & (1u64 << (index & 63)) != 0
    }
}

#[inline]
fn index(colour: [u8; 3]) -> u32 {
    ((colour[0] as u32) << 16) | ((colour[1] as u32) << 8) | colour[2] as u32
}

#[inline]
fn set(bits: &mut [u64], index: u32) {
    bits[(index >> 6) as usize] |= 1u64 << (index & 63);
}

/// Widen the set by `t` on each of the three channels.
fn dilate(bits: &mut [u64], t: usize) {
    if t >= BINS - 1 {
        bits.fill(u64::MAX);
        return;
    }
    // Blue: within each 256-bit (r, g) row.
    bits.par_chunks_mut(WORDS_PER_ROW).for_each(|row| {
        let src = [row[0], row[1], row[2], row[3]];
        let mut out = src;
        for shift in 1..=t as u32 {
            or_shift_left(&mut out, &src, shift);
            or_shift_right(&mut out, &src, shift);
        }
        row.copy_from_slice(&out);
    });
    // Green: across the 256 rows of one red slice.
    let src = bits.to_vec();
    bits.par_chunks_mut(WORDS_PER_ROW)
        .enumerate()
        .for_each(|(row, dst)| {
            let red = row / BINS;
            let green = row % BINS;
            let low = green.saturating_sub(t);
            let high = (green + t).min(BINS - 1);
            for other in low..=high {
                let base = (red * BINS + other) * WORDS_PER_ROW;
                for (slot, &value) in dst.iter_mut().zip(&src[base..base + WORDS_PER_ROW]) {
                    *slot |= value;
                }
            }
        });
    // Red: across the 256 slices at one green index.
    let src = bits.to_vec();
    bits.par_chunks_mut(WORDS_PER_ROW)
        .enumerate()
        .for_each(|(row, dst)| {
            let red = row / BINS;
            let green = row % BINS;
            let low = red.saturating_sub(t);
            let high = (red + t).min(BINS - 1);
            for other in low..=high {
                let base = (other * BINS + green) * WORDS_PER_ROW;
                for (slot, &value) in dst.iter_mut().zip(&src[base..base + WORDS_PER_ROW]) {
                    *slot |= value;
                }
            }
        });
}

fn or_shift_left(dst: &mut [u64; WORDS_PER_ROW], src: &[u64; WORDS_PER_ROW], shift: u32) {
    let words = (shift / 64) as usize;
    let bits = shift % 64;
    for i in 0..WORDS_PER_ROW {
        if i < words {
            continue;
        }
        let value = src[i - words];
        dst[i] |= value << bits;
        if bits != 0 && i + 1 < WORDS_PER_ROW {
            dst[i + 1] |= value >> (64 - bits);
        }
    }
}

fn or_shift_right(dst: &mut [u64; WORDS_PER_ROW], src: &[u64; WORDS_PER_ROW], shift: u32) {
    let words = (shift / 64) as usize;
    let bits = shift % 64;
    for i in 0..WORDS_PER_ROW {
        let source = i + words;
        if source >= WORDS_PER_ROW {
            continue;
        }
        let value = src[source];
        dst[i] |= value >> bits;
        if bits != 0 && i > 0 {
            dst[i - 1] |= value << (64 - bits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ColorMatcher;

    fn chebyshev(a: [u8; 3], b: [u8; 3]) -> u8 {
        a.iter()
            .zip(&b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn matcher_matches_brute_force_for_every_tolerance() {
        let mut references = Vec::new();
        for i in 0..48u32 {
            references.push([
                (i * 37 % 256) as u8,
                (i * 71 % 256) as u8,
                (i * 113 % 256) as u8,
            ]);
        }
        let mut candidates = Vec::new();
        for r in (0..256).step_by(16) {
            for g in (0..256).step_by(16) {
                for b in (0..256).step_by(16) {
                    candidates.push([r as u8, g as u8, b as u8]);
                }
            }
        }
        let mut state = 0x9e37_79b9u32;
        for _ in 0..500 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            candidates.push([(state >> 24) as u8, (state >> 16) as u8, (state >> 8) as u8]);
        }
        for tolerance in 0..=255u8 {
            let matcher = ColorMatcher::new(references.iter().copied(), tolerance);
            for colour in &candidates {
                let expected = references
                    .iter()
                    .any(|reference| chebyshev(*reference, *colour) <= tolerance);
                assert_eq!(
                    matcher.contains(*colour),
                    expected,
                    "tolerance {tolerance} colour {colour:?}"
                );
            }
        }
    }
}
