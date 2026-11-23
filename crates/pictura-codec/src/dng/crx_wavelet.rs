//! Canon CRX inverse wavelet (#1474): the reversible 5/3 lifting that
//! rebuilds one tile of one plane line by line from its subbands, over up to
//! three levels, with the overlap a neighbouring tile shares.
//!
//! Ported to Rust from LibRaw 0.22.1, file `src/decoders/crx.cpp`
//! (`crxIdwt53Filter*`, `crxHorizontal53`;
//! <https://github.com/LibRaw/LibRaw>, commit
//! b860248a89d9082b8e0a1e202e516f46af9adb29). Not a clean-room
//! implementation: the owner chose an open, attributed port for this format.
//!
//! Copyright (C) 2018-2019 Alexey Danilchenko
//! Copyright (C) 2019 Alex Tutubalin, LibRaw LLC
//!
//! LibRaw is licensed under the GNU Lesser General Public License, version
//! 2.1, or the Common Development and Distribution License, version 1.0.
//! This port takes the LGPL-2.1 and, as its section 3 permits, is distributed
//! under the GNU General Public License, version 3 or later, like the rest of
//! Kooka Pictura. See `assets/PROVENANCE.md`.

use super::crx_lines::{QSteps, Subband};
use crate::error::PsdError;

pub(super) const RIGHT: u32 = 1;
pub(super) const LEFT: u32 = 2;
pub(super) const BOTTOM: u32 = 4;
pub(super) const TOP: u32 = 8;

/// The even sample between two odd ones: `x - (h + 1) / 2`, rounded down.
fn lo(x: i32, h: i32) -> i32 {
    x.wrapping_sub(h.wrapping_add(1) >> 1)
}

/// The even sample from both odd neighbours: `x - (a + b + 2) / 4`.
fn lo2(x: i32, a: i32, b: i32) -> i32 {
    x.wrapping_sub(a.wrapping_add(b).wrapping_add(2) >> 2)
}

/// The odd sample from its high-pass value and both even neighbours.
fn hi(h: i32, a: i32, b: i32) -> i32 {
    h.wrapping_add(a.wrapping_add(b) >> 1)
}

/// One row of the inverse horizontal lifting: `low` and `high` into `out`'s
/// first `width` values. `tail` writes the last difference at a right-hand
/// neighbour whatever the width's parity, as LibRaw does for one-line bands.
fn horizontal(out: &mut [i32], low: &[i32], high: &[i32], width: usize, flags: u32, tail: bool) {
    if width <= 1 {
        out[0] = low[0];
        return;
    }
    let mut h = 0;
    out[0] = if flags & LEFT != 0 {
        h = 1;
        lo2(low[0], high[0], high[1])
    } else {
        lo(low[0], high[0])
    };
    let mut l = 1;
    let mut o = 0;
    while o + 3 < width {
        let d = lo2(low[l], high[h], high[h + 1]);
        out[o + 1] = hi(high[h], out[o], d);
        out[o + 2] = d;
        l += 1;
        h += 1;
        o += 2;
    }
    if flags & RIGHT != 0 {
        let d = lo2(low[l], high[h], high[h + 1]);
        out[o + 1] = hi(high[h], out[o], d);
        if tail || width & 1 == 1 {
            out[o + 2] = d;
        }
    } else if width & 1 == 1 {
        let d = lo(low[l], high[h]);
        out[o + 1] = hi(high[h], out[o], d);
        out[o + 2] = d;
    } else {
        out[o + 1] = out[o].wrapping_add(high[h]);
    }
}

/// One level of the transform: three rows of low-pass output and a ring of
/// five rows of finished output, `tap` naming the ring's oldest.
pub(super) struct Level {
    lines: [Vec<i32>; 8],
    line: i32,
    ready: i32,
    tap: usize,
    height: i32,
    width: usize,
    /// The ring row of the level below that feeds this level's low band.
    low: usize,
}

impl Level {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            // Two guard values past the width, as for subbands.
            lines: std::array::from_fn(|_| vec![0; width + 2]),
            line: 0,
            ready: 0,
            tap: 0,
            height: height as i32,
            width,
            low: 0,
        }
    }

    fn ring(&self, back: usize) -> usize {
        (self.tap + back) % 5 + 3
    }
}

/// One tile of one plane: its subbands (low band first, then three per
/// level, coarsest first) and its levels.
pub(super) struct TilePlane<'a, 's> {
    pub bands: Vec<Subband<'a>>,
    pub levels: Vec<Level>,
    pub flags: u32,
    pub steps: Option<&'s QSteps>,
}

impl TilePlane<'_, '_> {
    fn decode_band(&mut self, band: usize, level: usize) -> Result<(), PsdError> {
        self.bands[band].decode(self.steps.map(|s| (s, level)))
    }

    /// `crxIdwt53FilterGetLine`: the next finished row of `level`.
    pub fn take_line(&mut self, level: usize) -> usize {
        let w = &mut self.levels[level];
        let at = (w.tap as i32 - w.ready + 5).rem_euclid(5) as usize + 3;
        w.ready -= 1;
        at
    }

    pub fn line(&self, level: usize, at: usize) -> &[i32] {
        &self.levels[level].lines[at]
    }

    /// `crxIdwt53FilterDecode`: decodes the subband rows the next
    /// transform step of `level` needs.
    fn decode(&mut self, level: usize) -> Result<(), PsdError> {
        if self.levels[level].ready != 0 {
            return Ok(());
        }
        let band = 3 * level;
        let w = &self.levels[level];
        let tail = w.height - 3 <= w.line && self.flags & BOTTOM == 0;
        if tail && w.height & 1 == 0 {
            return Ok(());
        }
        if level > 0 {
            self.decode(level - 1)?;
        } else {
            self.decode_band(0, 0)?;
        }
        self.decode_band(band + 1, level)?;
        if !tail {
            self.decode_band(band + 2, level)?;
            self.decode_band(band + 3, level)?;
        }
        Ok(())
    }

    /// `crxIdwt53FilterTransform`: the next two (three at an odd bottom)
    /// finished rows of `level`.
    fn transform(&mut self, level: usize) -> Result<(), PsdError> {
        if self.levels[level].ready != 0 {
            return Ok(());
        }
        let flags = self.flags;
        let (line, height) = (self.levels[level].line, self.levels[level].height);
        if line >= height - 3 {
            if flags & BOTTOM != 0 {
                return Ok(());
            }
            if height & 1 == 1 {
                self.feed_low(level)?;
                let (lower, rest) = self.levels.split_at_mut(level);
                let w = &mut rest[0];
                let low: &[i32] = match lower.last() {
                    Some(below) => &below.lines[w.low],
                    None => &self.bands[0].buf,
                };
                let high = &self.bands[3 * level + 1].buf;
                let width = w.width;
                w.lines.swap(1, 2);
                let at = [0, 1, w.ring(0), w.ring(1), w.ring(2)];
                let [l0, l1, h0, h1, h2] = w.lines.get_disjoint_mut(at).expect("distinct rows");
                horizontal(l0, low, high, width, flags, false);
                for i in 0..width {
                    let d = lo(l0[i], l1[i]);
                    h1[i] = hi(l1[i], d, h0[i]);
                    h2[i] = d;
                }
                w.advance(3);
            } else {
                let w = &mut self.levels[level];
                let at = [2, w.ring(0), w.ring(1)];
                let [l2, h0, h1] = w.lines.get_disjoint_mut(at).expect("distinct rows");
                for i in 0..w.width {
                    h1[i] = h0[i].wrapping_add(l2[i]);
                }
                // LibRaw leaves rows 1 and 2 aliased here; neither is read again.
                w.advance(2);
            }
            return Ok(());
        }
        self.feed_low(level)?;
        let (lower, rest) = self.levels.split_at_mut(level);
        let w = &mut rest[0];
        let low: &[i32] = match lower.last() {
            Some(below) => &below.lines[w.low],
            None => &self.bands[0].buf,
        };
        let b = 3 * level;
        let (b1, b2, b3) = (
            &self.bands[b + 1].buf,
            &self.bands[b + 2].buf,
            &self.bands[b + 3].buf,
        );
        let width = w.width;
        w.lines.swap(1, 2);
        let at = [0, 1, 2, w.ring(0), w.ring(1), w.ring(2)];
        let [l0, l1, l2, h0, h1, h2] = w.lines.get_disjoint_mut(at).expect("distinct rows");
        horizontal(l0, low, b1, width, flags, false);
        horizontal(l2, b2, b3, width, flags, false);
        for i in 0..width {
            let d = lo2(l0[i], l2[i], l1[i]);
            h1[i] = hi(l1[i], d, h0[i]);
            h2[i] = d;
        }
        w.advance(if w.line >= w.height - 3 && w.height & 1 == 1 {
            3
        } else {
            2
        });
        Ok(())
    }

    /// Points `level`'s low band at the next finished row of the level below.
    fn feed_low(&mut self, level: usize) -> Result<(), PsdError> {
        if level > 0 {
            if self.levels[level - 1].ready == 0 {
                self.transform(level - 1)?;
            }
            self.levels[level].low = self.take_line(level - 1);
        }
        Ok(())
    }

    /// `crxIdwt53FilterInitialize`: primes every level with its first rows.
    pub fn initialize(&mut self) -> Result<(), PsdError> {
        let flags = self.flags;
        for level in 0..self.levels.len() {
            if level > 0 {
                self.levels[level].low = self.take_line(level - 1);
            } else {
                self.decode_band(0, 0)?;
            }
            let b = 3 * level;
            if self.levels[level].height > 1 {
                for band in b + 1..=b + 3 {
                    self.decode_band(band, level)?;
                }
                {
                    let (lower, rest) = self.levels.split_at_mut(level);
                    let w = &mut rest[0];
                    let low: &[i32] = match lower.last() {
                        Some(below) => &below.lines[w.low],
                        None => &self.bands[0].buf,
                    };
                    let (b1, b2, b3) = (
                        &self.bands[b + 1].buf,
                        &self.bands[b + 2].buf,
                        &self.bands[b + 3].buf,
                    );
                    let width = w.width;
                    let at = [0, 1, 2, w.ring(0)];
                    let [l0, l1, l2, h0] = w.lines.get_disjoint_mut(at).expect("distinct rows");
                    horizontal(l0, low, b1, width, flags, false);
                    if flags & TOP != 0 {
                        horizontal(l1, b2, b3, width, flags, false);
                    } else {
                        horizontal(l2, b2, b3, width, flags, false);
                        for i in 0..width {
                            h0[i] = lo(l0[i], l2[i]);
                        }
                    }
                }
                if flags & TOP != 0 {
                    self.decode_band(b + 3, level)?;
                    self.decode_band(b + 2, level)?;
                    let w = &mut self.levels[level];
                    let at = [0, 1, 2, w.ring(0)];
                    let [l0, l1, l2, h0] = w.lines.get_disjoint_mut(at).expect("distinct rows");
                    let (b2, b3) = (&self.bands[b + 2].buf, &self.bands[b + 3].buf);
                    horizontal(l2, b2, b3, w.width, flags, false);
                    for i in 0..w.width {
                        h0[i] = lo2(l0[i], l1[i], l2[i]);
                    }
                }
                self.decode(level)?;
                self.transform(level)?;
            } else {
                self.decode_band(b + 1, level)?;
                let (lower, rest) = self.levels.split_at_mut(level);
                let w = &mut rest[0];
                let low: &[i32] = match lower.last() {
                    Some(below) => &below.lines[w.low],
                    None => &self.bands[0].buf,
                };
                let at = w.ring(0);
                let width = w.width;
                horizontal(
                    &mut w.lines[at],
                    low,
                    &self.bands[b + 1].buf,
                    width,
                    flags,
                    true,
                );
                w.advance(1);
            }
        }
        Ok(())
    }

    /// Decodes and transforms the next finished row of the top level.
    pub fn next_row(&mut self) -> Result<usize, PsdError> {
        let top = self.levels.len() - 1;
        self.decode(top)?;
        self.transform(top)?;
        Ok(self.take_line(top))
    }
}

impl Level {
    fn advance(&mut self, rows: i32) {
        self.ready += rows;
        self.line += rows;
        self.tap = (self.tap + rows as usize) % 5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A forward 5/3 lifting with symmetric edges, the textbook transform the
    /// inverse undoes.
    fn forward(x: &[i32]) -> (Vec<i32>, Vec<i32>) {
        let n = x.len();
        let at = |i: isize| {
            x[if i < 0 {
                -i
            } else if i as usize >= n {
                2 * (n as isize - 1) - i
            } else {
                i
            } as usize]
        };
        let high: Vec<i32> = (0..n / 2)
            .map(|k| {
                let i = 2 * k as isize + 1;
                at(i) - ((at(i - 1) + at(i + 1)) >> 1)
            })
            .collect();
        let h = |k: isize| high[k.clamp(0, high.len() as isize - 1) as usize];
        let low = (0..n.div_ceil(2))
            .map(|k| {
                let k = k as isize;
                x[2 * k as usize] + ((h(k - 1) + h(k) + 2) >> 2)
            })
            .collect();
        (low, high)
    }

    #[test]
    fn horizontal_inverts_the_forward_lifting() {
        for width in 2..12 {
            let x: Vec<i32> = (0..width)
                .map(|i| (i * 37 % 11) as i32 * 100 - 400)
                .collect();
            let (low, high) = forward(&x);
            let mut out = vec![0; width + 2];
            horizontal(
                &mut out,
                &low,
                &[high.clone(), vec![0, 0]].concat(),
                width,
                0,
                false,
            );
            assert_eq!(out[..width], x[..], "width {width}");
        }
    }
}
