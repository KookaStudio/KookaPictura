//! Canon CRX entropy coding (#1474): the bit reader, the adaptive
//! Golomb-Rice line decoders of one subband, and its inverse quantization.
//!
//! Ported to Rust from LibRaw 0.22.1, file `src/decoders/crx.cpp`
//! (<https://github.com/LibRaw/LibRaw>, commit
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
//!
//! Arithmetic wraps where LibRaw's 32-bit integers would, so a hostile file
//! decodes to noise or an error and never panics.

use crate::error::PsdError;

const JS: [u32; 32] = [
    1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 8, 8, 8, 8, 0x10, 0x10, 0x20, 0x20, 0x40, 0x40, 0x80, 0x80,
    0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000,
];
const J: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 0x0A, 0x0B, 0x0C,
    0x0D, 0x0E, 0x0F,
];
const Q_STEP: [i32; 6] = [0x28, 0x2D, 0x33, 0x39, 0x40, 0x48];

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("CR3: {what}"))
}

/// A big-endian bit stream over one subband's bytes.
pub(super) struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    bit_data: u32,
    bits_left: i32,
}

impl<'a> Bits<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_data: 0,
            bits_left: 0,
        }
    }

    fn word(&mut self) -> Option<u32> {
        let w = self.data.get(self.pos..)?.first_chunk::<4>()?;
        self.pos += 4;
        Some(u32::from_be_bytes(*w))
    }

    fn byte(&mut self) -> Result<u32, PsdError> {
        let b = *self.data.get(self.pos).ok_or(PsdError::Truncated)?;
        self.pos += 1;
        Ok(b as u32)
    }

    /// The count of zero bits before the next one bit, which is consumed.
    pub fn zeros(&mut self) -> Result<u32, PsdError> {
        if self.bit_data != 0 {
            let top = 31 - self.bit_data.leading_zeros();
            self.bit_data = self.bit_data.checked_shl(32 - top).unwrap_or(0);
            self.bits_left -= (32 - top) as i32;
            return Ok(31 - top);
        }
        let mut zeros = self.bits_left as u32;
        loop {
            while let Some(w) = self.word() {
                if w != 0 {
                    let top = 31 - w.leading_zeros();
                    self.bit_data = ((w as u64) << (32 - top)) as u32;
                    self.bits_left = top as i32;
                    return Ok(zeros.wrapping_add(31 - top));
                }
                zeros = zeros.wrapping_add(32);
            }
            let b = self.byte()?;
            if b != 0 {
                let top = 31 - b.leading_zeros();
                self.bit_data = ((b as u64) << (32 - top)) as u32;
                self.bits_left = top as i32;
                return Ok(zeros.wrapping_add(7 - top));
            }
            zeros = zeros.wrapping_add(8);
        }
    }

    /// The next `bits` bits (1 to 31), most significant first.
    pub fn bits(&mut self, bits: u32) -> Result<u32, PsdError> {
        let need = bits as i32;
        let mut left = self.bits_left;
        let mut data = self.bit_data;
        if left < need {
            if let Some(w) = self.word() {
                self.bits_left = 32 - (need - left);
                self.bit_data = w << (need - left);
                return Ok(((w >> left) | data) >> (32 - bits));
            }
            while left < need {
                left += 8;
                data |= self.byte()?.checked_shl((32 - left) as u32).unwrap_or(0);
            }
        }
        self.bit_data = data << bits;
        self.bits_left = left - need;
        Ok(data >> (32 - bits))
    }

    /// One Golomb-Rice code with parameter `k`: an escape of `escape_bits`
    /// raw bits at `escape` zeros or more.
    pub fn code(&mut self, k: i32, escape: u32, escape_bits: u32) -> Result<u32, PsdError> {
        if !(0..32).contains(&k) {
            return Err(invalid("Golomb parameter out of range"));
        }
        let zeros = self.zeros()?;
        if zeros >= escape {
            self.bits(escape_bits)
        } else if k != 0 {
            Ok(self.bits(k as u32)? | (zeros << k))
        } else {
            Ok(zeros)
        }
    }

    /// The run of repeated symbols that a set bit starts, at most `length`.
    fn run(&mut self, s: &mut usize, length: i64) -> Result<i64, PsdError> {
        if self.bits(1)? == 0 {
            return Ok(0);
        }
        let mut n: i64 = 1;
        while self.bits(1)? != 0 {
            n += JS[*s] as i64;
            if n > length {
                n = length;
                break;
            }
            if *s < 31 {
                *s += 1;
            }
            if n == length {
                break;
            }
        }
        if n < length {
            if J[*s] != 0 {
                n += self.bits(J[*s])? as i64;
            }
            if *s > 0 {
                *s -= 1;
            }
            if n > length {
                return Err(invalid("symbol run past the line"));
            }
        }
        Ok(n)
    }
}

/// A code's signed value: even codes positive, odd ones negative.
fn signed(code: u32) -> i32 {
    -((code & 1) as i32) ^ ((code >> 1) as i32)
}

pub(super) fn predict_k(prev: i32, code: i32, max: i32) -> i32 {
    let k = prev - (code < (1i32.wrapping_shl(prev as u32) >> 1)) as i32
        + ((code >> prev) > 2) as i32
        + ((code >> prev) > 5) as i32;
    if max == 0 || k < max {
        k
    } else {
        max
    }
}

/// The 3-way median predictor of `left`, `top` and their gradients.
fn median(top_left: i32, top: i32, left: i32) -> i32 {
    let delta = top.wrapping_sub(top_left);
    let at =
        (((top_left < left) ^ (delta < 0)) as usize) << 1 | ((left < top) ^ (delta < 0)) as usize;
    let grad = delta.wrapping_add(left);
    [grad, grad, left, top][at]
}

/// One subband's line decoder: the previous and current line, each with a
/// guard column at both ends, and the coder's adaptive state.
pub(super) struct BandLines<'a> {
    bits: Bits<'a>,
    width: usize,
    height: usize,
    rounded_mask: i32,
    rounded_bits: i32,
    line: usize,
    s: usize,
    k: i32,
    partial: bool,
    buf: Vec<i32>,
    ks: Vec<i32>,
}

impl<'a> BandLines<'a> {
    pub fn new(
        data: &'a [u8],
        width: usize,
        height: usize,
        partial: bool,
        rounded_mask: i32,
    ) -> Self {
        Self {
            bits: Bits::new(data),
            width,
            height,
            rounded_mask,
            rounded_bits: 0,
            line: 0,
            s: 0,
            k: 0,
            partial,
            buf: vec![0; 2 * width + 4],
            ks: if partial { Vec::new() } else { vec![0; width] },
        }
    }

    pub fn bits(&mut self) -> &mut Bits<'a> {
        &mut self.bits
    }

    pub fn line(&self) -> usize {
        self.line
    }

    /// Decodes the next line into `out` (`width` values).
    pub fn decode(&mut self, out: &mut [i32]) -> Result<(), PsdError> {
        if self.line >= self.height {
            return Err(invalid("subband read past its height"));
        }
        let stride = self.width + 2;
        let (p0, p1) = if self.line & 1 == 1 {
            (stride, 0)
        } else {
            (0, stride)
        };
        if self.line == 0 {
            self.s = 0;
            self.k = 0;
            let p1 = stride;
            if !self.partial {
                self.top_line_no_ref(0, p1)?;
            } else if self.rounded_mask <= 0 {
                self.top_line(p1)?;
            } else {
                self.rounded_bits = 1;
                if self.rounded_mask & !1 != 0 {
                    while self.rounded_mask >> self.rounded_bits != 0 {
                        self.rounded_bits += 1;
                    }
                }
                self.top_line_rounded(p1)?;
            }
            out[..self.width].copy_from_slice(&self.buf[p1 + 1..p1 + 1 + self.width]);
        } else {
            if !self.partial {
                self.line_no_ref(p0, p1)?;
            } else if self.rounded_mask <= 0 {
                self.line_median(p0, p1)?;
            } else {
                self.line_rounded(p0, p1)?;
            }
            out[..self.width].copy_from_slice(&self.buf[p1 + 1..p1 + 1 + self.width]);
        }
        self.line += 1;
        Ok(())
    }

    /// `crxDecodeSymbolL1`: the symbol after `p1`, from the median or the
    /// line above, then the next K from one symbol ahead unless at the end.
    fn symbol_l1(
        &mut self,
        p0: &mut usize,
        p1: &mut usize,
        predict: bool,
        ahead: bool,
    ) -> Result<(), PsdError> {
        let b = &mut self.buf;
        b[*p1 + 1] = if predict {
            median(b[*p0], b[*p0 + 1], b[*p1])
        } else {
            b[*p0 + 1]
        };
        let mut code = self.bits.code(self.k, 41, 21)?;
        let b = &mut self.buf;
        b[*p1 + 1] = b[*p1 + 1].wrapping_add(signed(code));
        if ahead {
            let next = b[*p0 + 2].wrapping_sub(b[*p0 + 1]).wrapping_shl(1);
            code = code.wrapping_add(next.wrapping_abs() as u32) >> 1;
            *p0 += 1;
        }
        self.k = predict_k(self.k, code as i32, 15);
        *p1 += 1;
        Ok(())
    }

    /// `crxDecodeLine`: a line predicted from the one above.
    fn line_median(&mut self, mut p0: usize, mut p1: usize) -> Result<(), PsdError> {
        self.buf[p1] = self.buf[p0 + 1];
        let mut length = self.width as i64;
        while length > 1 {
            let b = &self.buf;
            if b[p1] != b[p0 + 1] || b[p1] != b[p0 + 2] {
                self.symbol_l1(&mut p0, &mut p1, true, true)?;
            } else {
                let n = self.bits.run(&mut self.s, length)?;
                length -= n;
                p0 += n as usize;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length > 0 {
                    self.symbol_l1(&mut p0, &mut p1, false, length > 1)?;
                }
            }
            length -= 1;
        }
        if length == 1 {
            self.symbol_l1(&mut p0, &mut p1, true, false)?;
        }
        self.buf[p1 + 1] = self.buf[p1].wrapping_add(1);
        Ok(())
    }

    /// `crxDecodeSymbolL1Rounded`.
    fn symbol_rounded(
        &mut self,
        p0: usize,
        p1: &mut usize,
        predict: bool,
        ahead: bool,
    ) -> Result<(), PsdError> {
        let b = &self.buf;
        let sym = if predict {
            median(b[p0], b[p0 + 1], b[*p1])
        } else {
            b[p0 + 1]
        };
        let code = self.bits.code(self.k, 41, 21)?;
        let value = signed(code);
        let mask = self.rounded_mask;
        let b = &mut self.buf;
        b[*p1 + 1] = mask
            .wrapping_mul(2)
            .wrapping_mul(value)
            .wrapping_add(value >> 31)
            .wrapping_add(sym);
        let next = if ahead {
            let d = if b[p0 + 2] > b[p0 + 1] {
                b[p0 + 2].wrapping_sub(b[p0 + 1]).wrapping_add(mask - 1) >> self.rounded_bits
            } else {
                (b[p0 + 1].wrapping_sub(b[p0 + 2]).wrapping_add(mask) >> self.rounded_bits)
                    .wrapping_neg()
            };
            code.wrapping_add(2u32.wrapping_mul(d.wrapping_abs() as u32)) >> 1
        } else {
            code
        };
        self.k = predict_k(self.k, next as i32, 15);
        *p1 += 1;
        Ok(())
    }

    /// `crxDecodeLineRounded`.
    fn line_rounded(&mut self, mut p0: usize, mut p1: usize) -> Result<(), PsdError> {
        let mask = self.rounded_mask;
        let mut reached = false;
        self.buf[p0] = self.buf[p0 + 1];
        self.buf[p1] = self.buf[p0 + 1];
        let mut length = self.width as i64;
        while length > 1 {
            let b = &self.buf;
            if b[p0 + 2].wrapping_sub(b[p0 + 1]).wrapping_abs() > mask {
                self.symbol_rounded(p0, &mut p1, true, true)?;
                p0 += 1;
                reached = true;
            } else if reached || b[p0].wrapping_sub(b[p1]).wrapping_abs() > mask {
                self.symbol_rounded(p0, &mut p1, true, true)?;
                p0 += 1;
                reached = false;
            } else {
                let n = self.bits.run(&mut self.s, length)?;
                length -= n;
                p0 += n as usize;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length > 1 {
                    self.symbol_rounded(p0, &mut p1, false, true)?;
                    p0 += 1;
                    let b = &self.buf;
                    reached = b[p0 + 1].wrapping_sub(b[p0]).wrapping_abs() > mask;
                } else if length == 1 {
                    self.symbol_rounded(p0, &mut p1, false, false)?;
                }
            }
            length -= 1;
        }
        if length == 1 {
            self.symbol_rounded(p0, &mut p1, true, false)?;
        }
        self.buf[p1 + 1] = self.buf[p1].wrapping_add(1);
        Ok(())
    }

    /// `crxDecodeLineNoRefPrevLine`: a line coded without reference to the
    /// one above, its K seeded from the K each column ended the line above at.
    fn line_no_ref(&mut self, p0: usize, p1: usize) -> Result<(), PsdError> {
        let w = self.width;
        let mut i = 0usize;
        while i + 1 < w {
            let b = &self.buf;
            if b[p0 + i + 2] | b[p0 + i + 1] | b[p1 + i] != 0 {
                let code = self.bits.code(self.k, 41, 21)?;
                self.buf[p1 + i + 1] = signed(code);
                self.k = predict_k(self.k, code as i32, 0);
                self.bump_k(i);
            } else {
                let n = self.bits.run(&mut self.s, (w - i) as i64)? as usize;
                if n > 0 {
                    self.buf[p1 + i + 1..p1 + i + 1 + n].fill(0);
                    self.ks[i..i + n].fill(0);
                    i += n;
                }
                if i + 1 >= w {
                    if i + 1 == w {
                        let code = self.bits.code(self.k, 41, 21)?;
                        self.buf[p1 + i + 1] = signed(code.wrapping_add(1));
                        self.k = predict_k(self.k, code as i32, 15);
                        self.ks[i] = self.k;
                    }
                    i += 1;
                    continue;
                }
                let code = self.bits.code(self.k, 41, 21)?;
                self.buf[p1 + i + 1] = signed(code.wrapping_add(1));
                self.k = predict_k(self.k, code as i32, 0);
                self.bump_k(i);
            }
            self.ks[i] = self.k;
            i += 1;
        }
        if i + 1 == w {
            // LibRaw holds this code signed, so its sign shifts in.
            let code = self.bits.code(self.k, 41, 21)? as i32;
            self.buf[p1 + i + 1] = -(code & 1) ^ (code >> 1);
            self.k = predict_k(self.k, code, 15);
            self.ks[i] = self.k;
        }
        Ok(())
    }

    /// Raises K by one when the column to the right ended the line above more
    /// than one above it, else caps it at 15.
    fn bump_k(&mut self, i: usize) {
        if self.ks[i + 1].wrapping_sub(self.k) <= 1 {
            if self.k >= 15 {
                self.k = 15;
            }
        } else {
            self.k += 1;
        }
    }

    /// `crxDecodeTopLine`: the first line, predicted from the left only.
    fn top_line(&mut self, mut p1: usize) -> Result<(), PsdError> {
        self.buf[p1] = 0;
        let mut length = self.width as i64;
        while length > 1 {
            if self.buf[p1] != 0 {
                self.buf[p1 + 1] = self.buf[p1];
            } else {
                let n = self.bits.run(&mut self.s, length)?;
                length -= n;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length <= 0 {
                    break;
                }
                self.buf[p1 + 1] = 0;
            }
            let code = self.bits.code(self.k, 41, 21)?;
            self.buf[p1 + 1] = self.buf[p1 + 1].wrapping_add(signed(code));
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
            length -= 1;
        }
        if length == 1 {
            self.buf[p1 + 1] = self.buf[p1];
            let code = self.bits.code(self.k, 41, 21)?;
            self.buf[p1 + 1] = self.buf[p1 + 1].wrapping_add(signed(code));
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
        }
        self.buf[p1 + 1] = self.buf[p1].wrapping_add(1);
        Ok(())
    }

    /// `crxDecodeTopLineRounded`.
    fn top_line_rounded(&mut self, mut p1: usize) -> Result<(), PsdError> {
        let mask = self.rounded_mask;
        self.buf[p1] = 0;
        let mut length = self.width as i64;
        let rounded = |v: i32| mask.wrapping_mul(2).wrapping_mul(v).wrapping_add(v >> 31);
        while length > 1 {
            if self.buf[p1].wrapping_abs() > mask {
                self.buf[p1 + 1] = self.buf[p1];
            } else {
                let n = self.bits.run(&mut self.s, length)?;
                length -= n;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length <= 0 {
                    break;
                }
                self.buf[p1 + 1] = 0;
            }
            let code = self.bits.code(self.k, 41, 21)?;
            self.buf[p1 + 1] = self.buf[p1 + 1].wrapping_add(rounded(signed(code)));
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
            length -= 1;
        }
        if length == 1 {
            let code = self.bits.code(self.k, 41, 21)?;
            self.buf[p1 + 1] = self.buf[p1 + 1].wrapping_add(rounded(signed(code)));
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
        }
        self.buf[p1 + 1] = self.buf[p1].wrapping_add(1);
        Ok(())
    }

    /// `crxDecodeTopLineNoRefPrevLine`.
    fn top_line_no_ref(&mut self, p0: usize, mut p1: usize) -> Result<(), PsdError> {
        self.buf[p0] = 0;
        self.buf[p1] = 0;
        let mut p2 = 0usize;
        let mut length = self.width as i64;
        while length > 1 {
            if self.buf[p1] != 0 {
                let code = self.bits.code(self.k, 41, 21)?;
                self.buf[p1 + 1] = signed(code);
                self.k = predict_k(self.k, code as i32, 15);
            } else {
                let n = self.bits.run(&mut self.s, length)?;
                length -= n;
                for _ in 0..n {
                    self.ks[p2] = 0;
                    self.buf[p1 + 1] = 0;
                    p1 += 1;
                    p2 += 1;
                }
                if length <= 0 {
                    break;
                }
                let code = self.bits.code(self.k, 41, 21)?;
                self.buf[p1 + 1] = signed(code.wrapping_add(1));
                self.k = predict_k(self.k, code as i32, 15);
            }
            self.ks[p2] = self.k;
            p2 += 1;
            p1 += 1;
            length -= 1;
        }
        if length == 1 {
            let code = self.bits.code(self.k, 41, 21)?;
            self.buf[p1 + 1] = signed(code);
            self.k = predict_k(self.k, code as i32, 15);
            self.ks[p2] = self.k;
            p1 += 1;
        }
        self.buf[p1 + 1] = 0;
        Ok(())
    }
}

/// The quantizer step of a quantization parameter.
pub(super) fn q_step(q: i32) -> Result<u32, PsdError> {
    if q < 0 {
        return Err(invalid("negative quantization"));
    }
    let step = Q_STEP[(q % 6) as usize] as u32;
    Ok(if q / 6 >= 6 {
        step.wrapping_shl(((q / 6 - 6) & 0x1f) as u32)
    } else {
        step >> (6 - q / 6)
    })
}

/// Where a subband's quantization comes from.
#[derive(Clone, Copy, Default)]
pub(super) struct Quant {
    /// The old header's parameter, adapted per line when `partial`.
    pub q: i32,
    pub partial: bool,
    /// The new header's base and multiplier of the tile's step table.
    pub step_base: i32,
    pub step_mult: u32,
    pub row_start: i32,
    pub row_end: i32,
    pub col_start: i32,
    pub col_end: i32,
    pub level_shift: u32,
}

/// A tile's quantizer step tables, coarsest level first, in one run as
/// LibRaw lays them out: a level's rows index from its own `starts` entry.
pub(super) struct QSteps {
    pub table: Vec<u32>,
    pub starts: Vec<usize>,
    /// Values per row: one per 8 columns of the tile.
    pub width: usize,
}

/// One subband of one tile and plane: its geometry, coder and line buffer.
pub(super) struct Subband<'a> {
    pub width: usize,
    pub height: usize,
    pub quant: Quant,
    q_k: i32,
    pub lines: Option<BandLines<'a>>,
    pub buf: Vec<i32>,
}

impl<'a> Subband<'a> {
    pub fn new(width: usize, height: usize, quant: Quant, lines: Option<BandLines<'a>>) -> Self {
        Self {
            width,
            height,
            quant,
            q_k: 0,
            lines,
            // Two guard values: tile edges read one past a band's last value.
            buf: vec![0; width + 2],
        }
    }

    /// `crxDecodeLineWithIQuantization`: the next line, scaled back by its
    /// quantizer step; zeros for a band without data.
    pub fn decode(&mut self, steps: Option<(&QSteps, usize)>) -> Result<(), PsdError> {
        let Some(lines) = self.lines.as_mut() else {
            self.buf.fill(0);
            return Ok(());
        };
        if self.quant.partial && steps.is_none() {
            let code = lines.bits().code(self.q_k, 23, 8)?;
            self.quant.q = self.quant.q.wrapping_add(signed(code));
            self.q_k = predict_k(self.q_k, code as i32, 0);
            if self.q_k > 7 {
                return Err(invalid("quantization parameter"));
            }
        }
        lines.decode(&mut self.buf)?;
        let row = lines.line() as i32 - 1;
        let w = self.width;
        let Some(steps) = steps else {
            let q = self.quant.q;
            let scale = if q / 6 >= 6 {
                (Q_STEP[(q % 6) as usize]).wrapping_mul(1i32.wrapping_shl((q / 6 + 26) as u32))
            } else {
                q_step(q)? as i32
            };
            if scale != 1 {
                for v in &mut self.buf[..w] {
                    *v = v.wrapping_mul(scale);
                }
            }
            return Ok(());
        };
        let qz = &self.quant;
        let h = self.height as i32;
        let srow = if row < qz.row_start {
            0
        } else if row < h - qz.row_end {
            row - qz.row_end
        } else {
            h - qz.row_end - qz.row_start - 1
        };
        let (steps, level) = steps;
        let base = usize::try_from(srow)
            .ok()
            .and_then(|r| r.checked_mul(steps.width))
            .and_then(|r| r.checked_add(*steps.starts.get(level)?))
            .ok_or_else(|| invalid("quantization row"))?;
        let row_steps = steps
            .table
            .get(base..)
            .ok_or_else(|| invalid("quantization row"))?;
        let step = |at: i64| -> Result<i32, PsdError> {
            let s = *usize::try_from(at)
                .ok()
                .and_then(|a| row_steps.get(a))
                .ok_or_else(|| invalid("quantization column"))?;
            let v = qz
                .step_base
                .wrapping_add((s.wrapping_mul(qz.step_mult) >> 3) as i32);
            Ok(v.clamp(1, 0x168000))
        };
        let (cs, ce) = (qz.col_start as i64, w as i64 - qz.col_end as i64);
        let last = (ce - cs - 1) >> qz.level_shift;
        for i in 0..w as i64 {
            let at = if i < cs {
                0
            } else if i < ce {
                (i - cs) >> qz.level_shift
            } else {
                last
            };
            let v = &mut self.buf[i as usize];
            *v = v.wrapping_mul(step(at)?);
        }
        Ok(())
    }
}
