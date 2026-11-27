//! Fujifilm-compressed RAF sensor data (#1620): the 16-byte header, the
//! block-size table, the lossy per-line quantizer bases, and the per-strip
//! decode of X-Trans and Bayer lines, lossless and lossy.
//!
//! Ported to Rust from LibRaw 0.22.1, file `src/decoders/fuji_compressed.cpp`
//! (<https://github.com/LibRaw/LibRaw>, commit
//! b860248a89d9082b8e0a1e202e516f46af9adb29). Not a clean-room
//! implementation: the owner chose an open, attributed port for this format.
//!
//! Copyright (C) 2016-2019 Alexey Danilchenko
//! Adopted to LibRaw by Alex Tutubalin, LibRaw LLC
//!
//! LibRaw is licensed under the GNU Lesser General Public License, version
//! 2.1, or the Common Development and Distribution License, version 1.0.
//! This port takes the LGPL-2.1 and, as its section 3 permits, is distributed
//! under the GNU General Public License, version 3 or later, like the rest of
//! Kooka Pictura. See `assets/PROVENANCE.md`.

use rayon::prelude::*;

use super::CfaPattern;
use crate::error::PsdError;

const SIGNATURE: usize = 0x4953;
pub(super) const HEADER_LEN: usize = 16;
const BLOCK_WIDTH: usize = 0x300;
/// A gradient's count is halved, with its sum, on reaching this.
const MIN_VALUE: i32 = 0x40;

/// Line buffers per strip: five red, eight green, five blue. Lines 0 and 1
/// of each colour hold the previous block's last lines.
const R0: usize = 0;
const R2: usize = R0 + 2;
const R3: usize = R0 + 3;
const R4: usize = R0 + 4;
const G0: usize = 5;
const G2: usize = G0 + 2;
const G3: usize = G0 + 3;
const G4: usize = G0 + 4;
const G5: usize = G0 + 5;
const G6: usize = G0 + 6;
const G7: usize = G0 + 7;
const B0: usize = 13;
const B2: usize = B0 + 2;
const B3: usize = B0 + 3;
const B4: usize = B0 + 4;
const LINES: usize = 18;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("RAF: compressed {what}"))
}

fn be16(b: &[u8], at: usize) -> usize {
    u16::from_be_bytes([b[at], b[at + 1]]) as usize
}

/// The compressed-data header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Header {
    pub lossless: bool,
    pub xtrans: bool,
    pub bits: u32,
    pub width: usize,
    pub height: usize,
    blocks: usize,
    lines: usize,
}

/// The header at the start of the sensor data; `None` when the data does
/// not start with its signature.
pub(super) fn header(data: &[u8]) -> Option<Result<Header, PsdError>> {
    let h = data.get(..HEADER_LEN)?;
    (be16(h, 0) == SIGNATURE).then(|| parse_header(h))
}

fn parse_header(h: &[u8]) -> Result<Header, PsdError> {
    let (lossless, raw_type, bits) = (h[2], h[3], h[4]);
    let height = be16(h, 5);
    let rounded = be16(h, 7);
    let width = be16(h, 9);
    let block = be16(h, 11);
    let blocks = h[13] as usize;
    let lines = be16(h, 14);
    let valid = lossless <= 1
        && (6..=0x4002).contains(&height)
        && height.is_multiple_of(6)
        && (0x300..=0x4200).contains(&width)
        && width.is_multiple_of(24)
        && block == BLOCK_WIDTH
        && rounded <= 0x4200
        && rounded >= block
        && rounded.is_multiple_of(block)
        && rounded - width < block
        && (1..=0x10).contains(&blocks)
        && blocks == rounded / block
        && (1..=0xAAB).contains(&lines)
        && lines == height / 6
        && matches!(bits, 12 | 14 | 16)
        && matches!(raw_type, 0 | 16);
    if !valid {
        return Err(invalid("header"));
    }
    Ok(Header {
        lossless: lossless == 1,
        xtrans: raw_type == 16,
        bits: bits.into(),
        width,
        height,
        blocks,
        lines,
    })
}

fn log2ceil(val: i32) -> u32 {
    if val <= 0 {
        0
    } else {
        32 - (val - 1).leading_zeros()
    }
}

/// A quantizer: the gradient lookup over `-max..=max`, its step base and
/// the code range.
struct QTable {
    table: Vec<i8>,
    q_base: i32,
    max_grad: i32,
    grad_mult: i32,
    total_values: i32,
    raw_bits: u32,
}

/// `setup_qlut`.
fn lookup(qp: [i32; 5]) -> Vec<i8> {
    (-qp[4]..=qp[4])
        .map(|v| match v {
            v if v <= -qp[3] => -4,
            v if v <= -qp[2] => -3,
            v if v <= -qp[1] => -2,
            v if v < -qp[0] => -1,
            v if v <= qp[0] => 0,
            v if v < qp[1] => 1,
            v if v < qp[2] => 2,
            v if v < qp[3] => 3,
            _ => 4,
        })
        .collect()
}

/// `init_main_qtable`.
fn main_qtable(q_base: i32, max_value: i32) -> QTable {
    let limit = max_value + 1;
    let mut qp = [
        q_base,
        3 * q_base + 0x12,
        5 * q_base + 0x43,
        7 * q_base + 0x114,
        max_value,
    ];
    if qp[1] >= limit || qp[1] < q_base + 1 {
        qp[1] = q_base + 1;
    }
    if qp[2] < qp[1] || qp[2] >= limit {
        qp[2] = qp[1];
    }
    if qp[3] < qp[2] || qp[3] >= limit {
        qp[3] = qp[2];
    }
    let total_values = (qp[4] + 2 * q_base) / (2 * q_base + 1) + 1;
    QTable {
        table: lookup(qp),
        q_base,
        max_grad: 0,
        grad_mult: 9,
        total_values,
        raw_bits: log2ceil(total_values),
    }
}

/// The three fixed lossy quantizers of `init_fuji_compr`, for small
/// gradients.
fn lossy_qtables(max_value: i32) -> [QTable; 3] {
    let thresholds = [
        [0x12, 0x43, 0x114],
        [0x15, 0x48, 0x11B],
        [0x18, 0x4D, 0x122],
    ];
    std::array::from_fn(|k| {
        let q_base = k as i32;
        let t = thresholds[k];
        let q1 = if max_value >= t[0] { t[0] } else { q_base + 1 };
        let q2 = if max_value >= t[1] { t[1] } else { q1 };
        let q3 = if max_value >= t[2] { t[2] } else { q2 };
        let total_values = (max_value + 2 * q_base) / (2 * q_base + 1) + 1;
        QTable {
            table: lookup([q_base, q1, q2, q3, max_value]),
            q_base,
            max_grad: 5 + q_base,
            grad_mult: 3,
            total_values,
            raw_bits: log2ceil(total_values),
        }
    })
}

fn max_diff(q: &QTable) -> i32 {
    ((q.total_values + 0x20) >> 6).max(2)
}

#[derive(Debug, Clone, Copy)]
struct Grad {
    sum: i32,
    count: i32,
}

#[derive(Clone, Copy)]
struct Grads {
    main: [Grad; 41],
    lossy: [[Grad; 5]; 3],
}

impl Grads {
    const ZERO: Self = Self {
        main: [Grad { sum: 0, count: 0 }; 41],
        lossy: [[Grad { sum: 0, count: 0 }; 5]; 3],
    };
}

/// What does not change between strips.
struct Params {
    max_value: i32,
    max_bits: i32,
    line_width: usize,
    lossy: Option<[QTable; 3]>,
    xtrans: bool,
}

/// MSB-first bits of one block. Past the block's bytes one zero byte reads;
/// moving beyond it fails, as LibRaw's buffer refill does.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            end: (data.len() + 1) * 8,
        }
    }

    fn peek(&self) -> u32 {
        let at = self.pos >> 3;
        let word = match self.data.get(at..at + 8) {
            Some(b) => u64::from_be_bytes(b.try_into().unwrap()),
            None => (0..8).fold(0u64, |w, i| {
                w << 8 | u64::from(self.data.get(at + i).copied().unwrap_or(0))
            }),
        };
        ((word << (self.pos & 7)) >> 32) as u32
    }

    fn skip(&mut self, n: usize) -> Result<(), PsdError> {
        self.pos += n;
        if self.pos >= self.end {
            return Err(PsdError::Truncated);
        }
        Ok(())
    }

    /// `fuji_zerobits`: zeros before the next one bit, which is consumed.
    fn zeros(&mut self) -> Result<i32, PsdError> {
        let mut count = 0i32;
        loop {
            let lz = self.peek().leading_zeros();
            if lz < 32 {
                self.skip(lz as usize + 1)?;
                return Ok(count + lz as i32);
            }
            count = count.saturating_add(32);
            self.skip(32)?;
        }
    }

    /// `fuji_read_code`.
    fn read(&mut self, n: u32) -> Result<i32, PsdError> {
        if n == 0 {
            return Ok(0);
        }
        let v = self.peek() >> (32 - n);
        self.skip(n as usize)?;
        Ok(v as i32)
    }
}

/// `bitDiff`.
fn bit_diff(sum: i32, count: i32) -> u32 {
    let mut n = 0;
    if count < sum {
        while n <= 14 {
            n += 1;
            if count << n >= sum {
                break;
            }
        }
    }
    n
}

/// How a pass treats its even positions on one line.
#[derive(Clone, Copy)]
enum Even {
    Sample,
    Interpolate,
    /// Interpolated at multiples of four, sampled elsewhere.
    SampleOff4,
    /// Interpolated two past multiples of four, sampled elsewhere.
    InterpolateAt2,
}

/// One of a block's six passes: two lines, how each takes its even
/// positions, and the gradient set it uses.
struct Pass {
    lines: [usize; 2],
    even: [Even; 2],
    grads: usize,
}

use Even::{Interpolate, InterpolateAt2, Sample, SampleOff4};

/// `xtrans_decode_block`.
const XTRANS_PASSES: [Pass; 6] = [
    Pass {
        lines: [R2, G2],
        even: [Interpolate, Sample],
        grads: 0,
    },
    Pass {
        lines: [G3, B2],
        even: [Sample, Interpolate],
        grads: 1,
    },
    Pass {
        lines: [R3, G4],
        even: [SampleOff4, Interpolate],
        grads: 2,
    },
    Pass {
        lines: [G5, B3],
        even: [Sample, InterpolateAt2],
        grads: 0,
    },
    Pass {
        lines: [R4, G6],
        even: [InterpolateAt2, Sample],
        grads: 1,
    },
    Pass {
        lines: [G7, B4],
        even: [Interpolate, SampleOff4],
        grads: 2,
    },
];

/// `fuji_bayer_decode_block`.
const BAYER_PASSES: [Pass; 6] = [
    Pass {
        lines: [R2, G2],
        even: [Sample; 2],
        grads: 0,
    },
    Pass {
        lines: [G3, B2],
        even: [Sample; 2],
        grads: 1,
    },
    Pass {
        lines: [R3, G4],
        even: [Sample; 2],
        grads: 2,
    },
    Pass {
        lines: [G5, B3],
        even: [Sample; 2],
        grads: 0,
    },
    Pass {
        lines: [R4, G6],
        even: [Sample; 2],
        grads: 1,
    },
    Pass {
        lines: [G7, B4],
        even: [Sample; 2],
        grads: 2,
    },
];

/// The first line of the colour group `line` belongs to, and the group's
/// last line.
fn group(line: usize) -> (usize, usize) {
    match line {
        R0..G0 => (R2, R4),
        G0..B0 => (G2, G7),
        _ => (B2, B4),
    }
}

/// One vertical strip of the plane, decoded six rows at a time.
struct Strip<'a> {
    bits: Bits<'a>,
    width: usize,
    buf: Vec<u16>,
    main: QTable,
    even: [Grads; 3],
    odd: [Grads; 3],
    q_bases: Option<&'a [u8]>,
    out: Vec<u16>,
}

impl<'a> Strip<'a> {
    fn new(p: &Params, data: &'a [u8], width: usize, q_bases: Option<&'a [u8]>) -> Self {
        let mut strip = Self {
            bits: Bits::new(data),
            width,
            buf: vec![0; LINES * (p.line_width + 2)],
            main: main_qtable(0, p.max_value),
            even: [Grads::ZERO; 3],
            odd: [Grads::ZERO; 3],
            q_bases,
            out: vec![0; 6 * width],
        };
        match &p.lossy {
            None => strip.reset_main_grads(),
            Some(lossy) => {
                for (k, q) in lossy.iter().enumerate() {
                    let start = Grad {
                        sum: max_diff(q),
                        count: 1,
                    };
                    for g in strip.even.iter_mut().chain(strip.odd.iter_mut()) {
                        g.lossy[k] = [start; 5];
                    }
                }
            }
        }
        strip
    }

    /// `init_main_grads`.
    fn reset_main_grads(&mut self) {
        let start = Grad {
            sum: max_diff(&self.main),
            count: 1,
        };
        for g in self.even.iter_mut().chain(self.odd.iter_mut()) {
            g.main = [start; 41];
        }
    }

    /// Index of position `pos` (from -1) of buffer line `line`.
    fn at(p: &Params, line: usize, pos: usize) -> usize {
        line * (p.line_width + 2) + 1 + pos
    }

    /// `fuji_decode_interpolation_even`.
    fn interpolate(&mut self, p: &Params, line: usize, pos: usize) {
        let (i, stride) = (Self::at(p, line, pos), p.line_width + 2);
        let b = &self.buf;
        let (rb, rc, rd, rf) = (
            i32::from(b[i - stride]),
            i32::from(b[i - stride - 1]),
            i32::from(b[i - stride + 1]),
            i32::from(b[i - 2 * stride]),
        );
        self.buf[i] = (even_prediction(rb, rc, rd, rf) >> 2) as u16;
    }

    /// `fuji_decode_sample_even` and `fuji_decode_sample_odd`.
    fn sample(
        &mut self,
        p: &Params,
        line: usize,
        pos: usize,
        odd: bool,
        set: usize,
    ) -> Result<(), PsdError> {
        let (i, stride) = (Self::at(p, line, pos), p.line_width + 2);
        let b = &self.buf;
        let (rb, rc, rd) = (
            i32::from(b[i - stride]),
            i32::from(b[i - stride - 1]),
            i32::from(b[i - stride + 1]),
        );
        let (diff, g1, g2, predicted) = if odd {
            let (ra, rg) = (i32::from(b[i - 1]), i32::from(b[i + 1]));
            let predicted = if (rb > rc && rb > rd) || (rb < rc && rb < rd) {
                (rg + ra + 2 * rb) >> 2
            } else {
                (ra + rg) >> 1
            };
            (
                (rb - rc).abs() + (rc - ra).abs(),
                rb - rc,
                rc - ra,
                predicted,
            )
        } else {
            let rf = i32::from(b[i - 2 * stride]);
            let predicted = even_prediction(rb, rc, rd, rf) >> 2;
            (
                (rf - rb).abs() + (rc - rb).abs(),
                rb - rf,
                rc - rb,
                predicted,
            )
        };
        let grads = if odd {
            &mut self.odd[set]
        } else {
            &mut self.even[set]
        };
        let (mut qt, mut grad_set) = (&self.main, &mut grads.main[..]);
        if let Some(lossy) = &p.lossy {
            for (k, q) in lossy.iter().enumerate() {
                if self.main.q_base <= k as i32 {
                    break;
                }
                if diff <= q.max_grad {
                    qt = q;
                    grad_set = &mut grads.lossy[k][..];
                    break;
                }
            }
        }
        let max = p.max_value;
        let grad = qt.grad_mult * i32::from(qt.table[(max + g1) as usize])
            + i32::from(qt.table[(max + g2) as usize]);
        let g = &mut grad_set[grad.unsigned_abs() as usize];
        let zeros = self.bits.zeros()?;
        let code = if zeros < p.max_bits - qt.raw_bits as i32 - 1 {
            let n = bit_diff(g.sum, g.count);
            self.bits.read(n)? + (zeros << n)
        } else {
            self.bits.read(qt.raw_bits)? + 1
        };
        let code = if code & 1 == 1 {
            -1 - code / 2
        } else {
            code / 2
        };
        g.sum += code.abs();
        if g.count == MIN_VALUE {
            g.sum >>= 1;
            g.count >>= 1;
        }
        g.count += 1;
        let step = 2 * qt.q_base + 1;
        let mut v = if grad < 0 {
            predicted - code * step
        } else {
            predicted + code * step
        };
        if v < -qt.q_base {
            v += qt.total_values * step;
        } else if v > qt.q_base + max {
            v -= qt.total_values * step;
        }
        self.buf[i] = v.clamp(0, max) as u16;
        Ok(())
    }

    /// `fuji_extend_generic` over the group of `line`.
    fn extend(&mut self, p: &Params, line: usize) {
        let (first, last) = group(line);
        let lw = p.line_width;
        for l in first..=last {
            let (cur, prev) = (Self::at(p, l, 0) - 1, Self::at(p, l - 1, 0) - 1);
            self.buf[cur] = self.buf[prev + 1];
            self.buf[cur + lw + 1] = self.buf[prev + lw];
        }
    }

    fn even_step(&mut self, p: &Params, pass: &Pass, pos: usize) -> Result<(), PsdError> {
        for (&line, &rule) in pass.lines.iter().zip(&pass.even) {
            let sample = match rule {
                Sample => true,
                Interpolate => false,
                SampleOff4 => pos & 3 != 0,
                InterpolateAt2 => pos & 3 != 2,
            };
            if sample {
                self.sample(p, line, pos, false, pass.grads)?;
            } else {
                self.interpolate(p, line, pos);
            }
        }
        Ok(())
    }

    /// One block of six rows: the six passes, then the shift to the next.
    fn decode_line(&mut self, p: &Params, cfa: &CfaPattern, line: usize) -> Result<(), PsdError> {
        if let Some(q_bases) = self.q_bases {
            let q_base = i32::from(q_bases[line]);
            if line == 0 || q_base != self.main.q_base {
                self.main = main_qtable(q_base, p.max_value);
                self.reset_main_grads();
            }
        }
        let lw = p.line_width;
        let passes = if p.xtrans {
            &XTRANS_PASSES
        } else {
            &BAYER_PASSES
        };
        for pass in passes {
            let (mut even, mut odd) = (0, 1);
            while even < lw || odd < lw {
                if even < lw {
                    self.even_step(p, pass, even)?;
                    even += 2;
                }
                if even > 8 {
                    for &line in &pass.lines {
                        self.sample(p, line, odd, true, pass.grads)?;
                    }
                    odd += 2;
                }
            }
            for &line in &pass.lines {
                self.extend(p, line);
            }
        }
        let size = lw + 2;
        for (to, from) in [
            (R0, R3),
            (R0 + 1, R4),
            (G0, G6),
            (G0 + 1, G7),
            (B0, B3),
            (B0 + 1, B4),
        ] {
            self.buf
                .copy_within(from * size..(from + 1) * size, to * size);
        }
        self.copy_out(p, cfa);
        for first in [R2, G2, B2] {
            let (_, last) = group(first);
            self.buf[first * size..(last + 1) * size].fill(0);
            let prev = (first - 1) * size;
            self.buf[first * size] = self.buf[prev + 1];
            self.buf[first * size + lw + 1] = self.buf[prev + lw];
        }
        Ok(())
    }

    /// `copy_line_to_xtrans` and `copy_line_to_bayer`.
    fn copy_out(&mut self, p: &Params, cfa: &CfaPattern) {
        for row in 0..6 {
            let out = &mut self.out[row * self.width..][..self.width];
            for (x, o) in out.iter_mut().enumerate() {
                let colour = cfa.colors[(row % cfa.height) * cfa.width + x % cfa.width];
                let line = match colour {
                    0 => R2 + row / 2,
                    2 => B2 + row / 2,
                    _ => G2 + row,
                };
                let index = if p.xtrans {
                    (((x * 2 / 3) & !1) | ((x % 3) & 1)) + ((x % 3) >> 1)
                } else {
                    x >> 1
                };
                *o = self.buf[Self::at(p, line, index)];
            }
        }
    }
}

/// The even-position prediction sum, four times the predicted value.
fn even_prediction(rb: i32, rc: i32, rd: i32, rf: i32) -> i32 {
    let (cb, fb, db) = ((rc - rb).abs(), (rf - rb).abs(), (rd - rb).abs());
    if cb > fb && cb > db {
        rf + rd + 2 * rb
    } else if db > cb && db > fb {
        rf + rc + 2 * rb
    } else {
        rd + rc + 2 * rb
    }
}

/// Decodes the compressed data `data` (header included) into a
/// `header.width` × `header.height` plane laid out in `cfa`.
pub(super) fn decode(data: &[u8], h: &Header, cfa: &CfaPattern) -> Result<Vec<u16>, PsdError> {
    let pattern = if h.xtrans { 6 } else { 2 };
    if (cfa.width, cfa.height) != (pattern, pattern) {
        return Err(invalid("sensor type differs from the layout record"));
    }
    let body = data.get(HEADER_LEN..).ok_or(PsdError::Truncated)?;
    let table = body.get(..4 * h.blocks).ok_or(PsdError::Truncated)?;
    let mut at = (4 * h.blocks).next_multiple_of(16);
    let line_step = h.lines.next_multiple_of(16);
    let q_bases = if h.lossless {
        None
    } else {
        let n = h.blocks * line_step;
        let q = body.get(at..at + n).ok_or(PsdError::Truncated)?;
        at += n;
        Some(q)
    };
    let max_value = (1i32 << h.bits) - 1;
    let line_width = if h.xtrans {
        BLOCK_WIDTH * 2 / 3
    } else {
        BLOCK_WIDTH / 2
    };
    let p = Params {
        max_value,
        max_bits: 4 * log2ceil(max_value + 1) as i32,
        line_width,
        lossy: (!h.lossless).then(|| lossy_qtables(max_value)),
        xtrans: h.xtrans,
    };
    let mut strips = Vec::with_capacity(h.blocks);
    for (b, size) in table.as_chunks::<4>().0.iter().enumerate() {
        let size = u32::from_be_bytes(*size) as usize;
        let bytes = at
            .checked_add(size)
            .and_then(|end| body.get(at..end))
            .ok_or(PsdError::Truncated)?;
        at += size;
        let width = if b + 1 == h.blocks {
            h.width - BLOCK_WIDTH * b
        } else {
            BLOCK_WIDTH
        };
        let q = q_bases.map(|q| &q[b * line_step..][..h.lines]);
        strips.push(Strip::new(&p, bytes, width, q));
    }
    let mut plane = vec![0u16; h.width * h.height];
    for line in 0..h.lines {
        strips
            .par_iter_mut()
            .try_for_each(|s| s.decode_line(&p, cfa, line))?;
        for (b, s) in strips.iter().enumerate() {
            for row in 0..6 {
                let y = line * 6 + row;
                plane[y * h.width + b * BLOCK_WIDTH..][..s.width]
                    .copy_from_slice(&s.out[row * s.width..][..s.width]);
            }
        }
    }
    Ok(plane)
}
