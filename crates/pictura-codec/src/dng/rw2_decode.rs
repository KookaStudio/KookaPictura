//! Panasonic's RW2 sensor codings other than the plain ones (#1541): the
//! RawFormat 3/4, 5, 6 and 8 decoders.
//!
//! Ported to Rust from darktable's rawspeed, commit
//! 718eae230df9ebb1750755b775e52185f2342a46, files
//! `src/librawspeed/decompressors/PanasonicV{4,5,6,8}Decompressor.{h,cpp}`,
//! `src/librawspeed/decoders/Rw2Decoder.cpp` and, for the bad-pixel fill,
//! `src/librawspeed/common/RawImageDataU16.cpp`
//! (<https://github.com/darktable-org/rawspeed>). Not a clean-room
//! implementation: the owner chose an open, attributed port for this format.
//!
//! Copyright (C) 2009-2014 Klaus Post
//! Copyright (C) 2014 Pedro Côrte-Real
//! Copyright (C) 2017 Axel Waggershauser
//! Copyright (C) 2017-2018, 2020 Roman Lebedev
//! Copyright (C) 2018 Alexey Danilchenko
//! Copyright (C) 2018 Stefan Hoffmeister
//! Copyright (C) 2019, 2022-2024 LibRaw LLC (info@libraw.org)
//! Copyright (C) 2024 Daniel Vogelbacher
//! Copyright (C) 2025 Kolton Yager
//!
//! rawspeed is licensed under the GNU Lesser General Public License, version
//! 2.1 or later. As its section 3 permits, this port is distributed under the
//! GNU General Public License, version 3 or later, like the rest of Kooka
//! Pictura. See `assets/PROVENANCE.md`.

use rayon::prelude::*;

use super::{to_usize, Tags};
use crate::error::PsdError;

/// The codings' input blocks; a block's two sections are stored swapped.
const BLOCK: usize = 0x4000;
const SPLIT: usize = 0x1FF8;
const PACKET: usize = 16;
/// RawFormat 3 and 4 photosites per packet.
const PREDICTIVE_PIXELS: usize = 14;

const V8_CLIP: u16 = 0x003B;
/// Red, first green, second green, blue.
const V8_INITIAL: [u16; 4] = [0x003C, 0x003D, 0x003E, 0x003F];
const V8_GAMMA_SLOPES: u16 = 0x0039;
const V8_GAMMA_POINTS: u16 = 0x003A;
const V8_CODES: u16 = 0x0040;
const V8_SHIFT_DOWN: u16 = 0x0041;
const V8_STRIPS_H: u16 = 0x0042;
const V8_STRIPS_V: u16 = 0x0043;
const V8_STRIP_OFFSETS: u16 = 0x0044;
const V8_STRIP_POSITIONS: u16 = 0x0045;
const V8_STRIP_BITS: u16 = 0x0046;
const V8_STRIP_WIDTHS: u16 = 0x0047;
const V8_STRIP_HEIGHTS: u16 = 0x0048;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("RW2: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("RW2 {what}"))
}

/// As many `bits`-bit samples as fit in 16 bytes, from the low bit up, then
/// padding; `dst` takes whole groups.
pub(super) fn unpack_groups(src: &[u8], bits: u32, dst: &mut [u16]) {
    let per_group = (128 / bits) as usize;
    let mask = (1u128 << bits) - 1;
    for (out, &group) in dst
        .chunks_exact_mut(per_group)
        .zip(src.as_chunks::<PACKET>().0)
    {
        let mut value = u128::from_le_bytes(group);
        for d in out {
            *d = (value & mask) as u16;
            value >>= bits;
        }
    }
}

/// The input bytes RawFormat 3 and 4 need for `pixels` photosites.
pub(super) fn predictive_len(pixels: usize, split: usize) -> usize {
    let len = pixels / PREDICTIVE_PIXELS * PACKET;
    if split == 0 {
        len
    } else {
        len.div_ceil(BLOCK) * BLOCK
    }
}

/// The input bytes RawFormat 5 needs for `pixels` photosites.
pub(super) fn blocked_len(pixels: usize, bits: u32) -> usize {
    let packets = pixels / (128 / bits) as usize;
    packets.div_ceil(BLOCK / PACKET) * BLOCK
}

/// RawFormat 6 photosites per 16-byte packet.
pub(super) fn paired_pixels(bits: u32) -> usize {
    if bits == 14 {
        11
    } else {
        14
    }
}

/// A block with its swapped sections put back in order, padded with zeros.
fn unswap(block: &[u8], split: usize, len: usize) -> Vec<u8> {
    let split = split.min(block.len());
    let mut buf = Vec::with_capacity(len);
    buf.extend_from_slice(&block[split..]);
    buf.extend_from_slice(&block[..split]);
    buf.resize(len.max(buf.len()), 0);
    buf
}

/// RawFormat 3 and 4's bit reader: each 16-byte packet is read from its last
/// byte down, across the whole unswapped block.
struct PredictiveBits {
    buf: Vec<u8>,
    vbits: u32,
}

impl PredictiveBits {
    fn get(&mut self, n: u32) -> u32 {
        self.vbits = self.vbits.wrapping_sub(n) & 0x1ffff;
        let byte = (self.vbits >> 3) as usize ^ 0x3ff0;
        let word = u32::from(self.buf[byte]) | u32::from(self.buf[byte + 1]) << 8;
        (word >> (self.vbits & 7)) & ((1 << n) - 1)
    }
}

fn predictive_packet(bits: &mut PredictiveBits, out: &mut [u16]) {
    let mut sh = 0;
    let mut pred = [0i32; 2];
    let mut nonz = [0i32; 2];
    for (p, out) in out.iter_mut().enumerate() {
        let c = p & 1;
        if p % 3 == 2 {
            sh = 4 >> (3 - bits.get(2));
        }
        if nonz[c] != 0 {
            let j = bits.get(8) as i32;
            if j != 0 {
                pred[c] -= 0x80 << sh;
                if pred[c] < 0 || sh == 4 {
                    pred[c] &= (1 << sh) - 1;
                }
                pred[c] += j << sh;
            }
        } else {
            nonz[c] = bits.get(8) as i32;
            if nonz[c] != 0 || p > 11 {
                pred[c] = nonz[c] << 4 | bits.get(4) as i32;
            }
        }
        *out = pred[c] as u16;
    }
}

/// RawFormat 3 and 4: `plane` is a whole number of 14-photosite packets.
pub(super) fn predictive(data: &[u8], split: usize, plane: &mut [u16]) {
    let per_block = BLOCK / PACKET * PREDICTIVE_PIXELS;
    plane
        .par_chunks_mut(per_block)
        .zip(data.par_chunks(BLOCK))
        .for_each(|(out, block)| {
            let mut bits = PredictiveBits {
                buf: unswap(block, split, BLOCK + 1),
                vbits: 0,
            };
            for packet in out.as_chunks_mut::<PREDICTIVE_PIXELS>().0 {
                predictive_packet(&mut bits, packet);
            }
        });
}

/// For each sample of `line`, the nearest non-zero sample of its colour
/// before it and after it, as (value, distance).
type Neighbours = (Option<(u16, usize)>, Option<(u16, usize)>);

fn neighbours(line: &[u16]) -> Vec<Neighbours> {
    let n = line.len();
    let mut out = vec![(None, None); n];
    for i in 2..n {
        let before = match line[i - 2] {
            0 => out[i - 2].0.map(|(v, d)| (v, d + 2)),
            v => Some((v, 2)),
        };
        out[i].0 = before;
    }
    for i in (0..n.saturating_sub(2)).rev() {
        let after = match line[i + 2] {
            0 => out[i + 2].1.map(|(v, d)| (v, d + 2)),
            v => Some((v, 2)),
        };
        out[i].1 = after;
    }
    out
}

/// rawspeed's `RawImageDataU16::fixBadPixel` weights for one axis.
fn axis(pair: Neighbours) -> Option<i32> {
    let (dist_a, dist_b) = (pair.0.map_or(0, |p| p.1), pair.1.map_or(0, |p| p.1));
    let total = (dist_a + dist_b) as i32;
    if total == 0 {
        return None;
    }
    let weight_a = if dist_a == 0 {
        0
    } else {
        (total - dist_a as i32) * 256 / total
    };
    let value = |p: Option<(u16, usize)>| p.map_or(0, |(v, _)| i32::from(v));
    Some(value(pair.0) * weight_a + value(pair.1) * (256 - weight_a))
}

/// RawFormat 3 and 4's zero photosites are bad pixels: each takes the
/// distance-weighted mean of the nearest good one of its colour left, right,
/// up and down, as rawspeed's `fixBadPixel` does.
pub(super) fn fill_zeros(plane: &mut [u16], width: usize) {
    let bad: Vec<usize> = plane
        .par_iter()
        .enumerate()
        .filter_map(|(i, &v)| (v == 0).then_some(i))
        .collect();
    if bad.is_empty() {
        return;
    }
    let height = plane.len() / width;
    let source: &[u16] = plane;
    // Each line with a bad pixel is swept once.
    let mut horizontal = Vec::with_capacity(bad.len());
    for row in bad.chunk_by(|a, b| a / width == b / width) {
        let y = row[0] / width;
        let found = neighbours(&source[y * width..][..width]);
        horizontal.extend(row.iter().map(|&i| found[i % width]));
    }
    let mut by_column: Vec<usize> = (0..bad.len()).collect();
    by_column.sort_by_key(|&k| (bad[k] % width, bad[k] / width));
    let mut vertical = vec![(None, None); bad.len()];
    for column in by_column.chunk_by(|&a, &b| bad[a] % width == bad[b] % width) {
        let x = bad[column[0]] % width;
        let line: Vec<u16> = (0..height).map(|y| source[y * width + x]).collect();
        let found = neighbours(&line);
        for &k in column {
            vertical[k] = found[bad[k] / width];
        }
    }
    let fixed: Vec<u16> = horizontal
        .iter()
        .zip(&vertical)
        .map(|(&h, &v)| {
            let (h, v) = (axis(h), axis(v));
            let shifts = 7 + u32::from(h.is_some()) + u32::from(v.is_some());
            ((h.unwrap_or(0) + v.unwrap_or(0)) >> shifts).clamp(0, 0xffff) as u16
        })
        .collect();
    for (&i, v) in bad.iter().zip(fixed) {
        plane[i] = v;
    }
}

/// RawFormat 5: plain groups inside swapped blocks.
pub(super) fn blocked(data: &[u8], bits: u32, plane: &mut [u16]) {
    let per_block = BLOCK / PACKET * (128 / bits) as usize;
    plane
        .par_chunks_mut(per_block)
        .zip(data.par_chunks_exact(BLOCK))
        .for_each(|(out, block)| unpack_groups(&unswap(block, SPLIT, BLOCK), bits, out));
}

struct PairedCoding {
    /// Field widths in the order they are read from the low bit, after `skip`.
    fields: &'static [u32],
    skip: u32,
    base_zero: u32,
    base_compare: u32,
    spix_compare: u32,
    mask: u16,
}

const PAIRED_12: PairedCoding = PairedCoding {
    fields: &[8, 8, 8, 2, 8, 8, 8, 2, 8, 8, 8, 2, 8, 8, 8, 2, 12, 12],
    skip: 0,
    base_zero: 0x80,
    base_compare: 0x800,
    spix_compare: 0x3fff,
    mask: 0xfff,
};

const PAIRED_14: PairedCoding = PairedCoding {
    fields: &[10, 10, 10, 2, 10, 10, 10, 2, 10, 10, 10, 2, 14, 14],
    skip: 4,
    base_zero: 0x200,
    base_compare: 0x2000,
    spix_compare: 0xffff,
    mask: 0x3fff,
};

fn paired_packet(coding: &PairedCoding, packet: [u8; PACKET], out: &mut [u16]) {
    let mut value = u128::from_le_bytes(packet) >> coding.skip;
    // The first field read is the last one used.
    let mut fields = [0u16; 18];
    let n = coding.fields.len();
    for (i, &width) in coding.fields.iter().enumerate() {
        fields[n - 1 - i] = (value & ((1 << width) - 1)) as u16;
        value >>= width;
    }
    let mut next = fields.iter().copied();
    let mut field = || next.next().unwrap_or(0);

    let mut odd_even = [0u32; 2];
    let mut nonzero = [0u32; 2];
    let (mut pmul, mut pixel_base) = (0u32, 0u32);
    for (pix, out) in out.iter_mut().enumerate() {
        let c = pix % 2;
        if pix % 3 == 2 {
            let base = match field() {
                3 => 4,
                base => u32::from(base),
            };
            pixel_base = coding.base_zero << base;
            pmul = 1 << base;
        }
        // rawspeed keeps these in 16 bits.
        let mut epixel = field();
        if odd_even[c] != 0 {
            epixel = (u32::from(epixel) * pmul) as u16;
            if pixel_base < coding.base_compare && nonzero[c] > pixel_base {
                epixel = (u32::from(epixel) + nonzero[c] - pixel_base) as u16;
            }
            nonzero[c] = u32::from(epixel);
        } else {
            odd_even[c] = u32::from(epixel);
            if epixel != 0 {
                nonzero[c] = u32::from(epixel);
            } else {
                epixel = nonzero[c] as u16;
            }
        }
        *out = match u32::from(epixel).checked_sub(0xf) {
            None => 0,
            Some(spix) if spix <= coding.spix_compare => spix as u16,
            Some(_) => coding.mask,
        };
    }
}

/// RawFormat 6: `plane` is a whole number of packets.
pub(super) fn paired(data: &[u8], bits: u32, plane: &mut [u16]) {
    let coding = if bits == 14 { &PAIRED_14 } else { &PAIRED_12 };
    plane
        .par_chunks_mut(paired_pixels(bits))
        .zip(data.par_chunks_exact(PACKET))
        .for_each(|(out, packet)| {
            paired_packet(coding, packet.try_into().expect("packet"), out);
        });
}

/// A Panasonic list tag: a 16-bit count, then that many `N`-byte values.
fn counted<const N: usize>(tags: &Tags, tag: u16) -> Result<Vec<u64>, PsdError> {
    let entry = tags
        .ifd
        .get(tag)
        .ok_or_else(|| invalid(&format!("missing tag {tag:#06x}")))?;
    let raw = tags.tiff.raw(entry);
    let (count, values) = raw
        .split_first_chunk::<2>()
        .ok_or_else(|| invalid(&format!("tag {tag:#06x} is empty")))?;
    let count = usize::from(u16::from_le_bytes(*count));
    let values = values
        .get(..count * N)
        .ok_or_else(|| invalid(&format!("tag {tag:#06x} count")))?;
    Ok(values
        .as_chunks::<N>()
        .0
        .iter()
        .map(|v| v.iter().rev().fold(0u64, |a, &b| a << 8 | u64::from(b)))
        .collect())
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Code {
    bits: u8,
    category: u8,
}

/// rawspeed's empty table slot.
const NO_CODE: Code = Code {
    bits: 7,
    category: 0,
};

/// The difference-category codes, looked up by the next 16 bits. The table is
/// matched in order and is not a prefix code.
fn code_table(raw: &[u8]) -> Result<Vec<Code>, PsdError> {
    let words: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&w| u16::from_le_bytes(w))
        .collect();
    let symbols = usize::from(*words.first().ok_or_else(|| invalid("empty code table"))?);
    if !(1..=17).contains(&symbols) || words.len() < 1 + 2 * symbols {
        return Err(invalid("code table size"));
    }
    let mut table = Vec::with_capacity(symbols);
    for (category, pair) in words[1..=2 * symbols].as_chunks::<2>().0.iter().enumerate() {
        let (len, code) = (u32::from(pair[0]), u32::from(pair[1]));
        if !(1..=16).contains(&len) || code >> len != 0 {
            return Err(invalid("bad code"));
        }
        let entry = Code {
            bits: len as u8,
            category: category as u8,
        };
        if entry == NO_CODE {
            return Err(invalid("code table holds the empty slot"));
        }
        let mask = (0xffffu32 << (16 - len)) as u16;
        table.push((entry, (code << (16 - len)) as u16, mask));
    }
    Ok((0..=u16::MAX)
        .map(|word| {
            table
                .iter()
                .find(|&&(_, symbol, mask)| word & mask == symbol)
                .map_or(NO_CODE, |&(entry, ..)| entry)
        })
        .collect())
}

/// Reads bytes in order and each byte from its low bit, as one MSB-first
/// stream; past the end it reads zeros.
struct StripBits<'a> {
    data: &'a [u8],
    pos: usize,
    cache: u64,
    fill: u32,
    used: usize,
}

impl StripBits<'_> {
    fn refill(&mut self) {
        while self.fill <= 56 {
            let byte = self.data.get(self.pos).copied().unwrap_or(0).reverse_bits();
            self.cache |= u64::from(byte) << (56 - self.fill);
            self.pos += 1;
            self.fill += 8;
        }
    }

    fn take(&mut self, n: u32) -> Result<u32, PsdError> {
        self.used += n as usize;
        if self.used > 8 * self.data.len() {
            return Err(invalid("strip ends early"));
        }
        let value = if n == 0 {
            0
        } else {
            (self.cache >> (64 - n)) as u32
        };
        self.cache <<= n;
        self.fill -= n;
        Ok(value)
    }

    fn difference(&mut self, codes: &[Code]) -> Result<i32, PsdError> {
        self.refill();
        let code = codes[(self.cache >> 48) as usize];
        if code == NO_CODE {
            return Err(invalid("undefined code"));
        }
        self.take(u32::from(code.bits))?;
        let len = u32::from(code.category);
        if len == 0 {
            return Ok(0);
        }
        let diff = self.take(len)? as i32;
        Ok(if diff & (1 << (len - 1)) == 0 {
            diff - ((1 << len) - 1)
        } else {
            diff
        })
    }
}

struct Strip<'a> {
    data: &'a [u8],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

/// Decodes one strip in 2 × 2 blocks, each column-major; `pred` is indexed
/// row-major.
fn decode_strip(strip: &Strip, codes: &[Code], mut pred: [u16; 4]) -> Result<Vec<u16>, PsdError> {
    let w = strip.width;
    let mut out = vec![0u16; w * strip.height];
    let mut bits = StripBits {
        data: strip.data,
        pos: 0,
        cache: 0,
        fill: 0,
        used: 0,
    };
    for y in (0..strip.height).step_by(2) {
        for x in (0..w).step_by(2) {
            for col in 0..2 {
                for row in 0..2 {
                    let p = &mut pred[row * 2 + col];
                    let value = (i32::from(*p) + bits.difference(codes)?).clamp(0, 0xffff);
                    *p = value as u16;
                    out[(y + row) * w + x + col] = *p;
                }
            }
        }
        // Each row of blocks starts from the first block of the one before.
        for row in 0..2 {
            for col in 0..2 {
                pred[row * 2 + col] = out[(y + row) * w + col];
            }
        }
    }
    Ok(out)
}

/// The strips must tile the plane in rows of equal height, left to right.
fn check_grid(strips: &[Strip], (width, height): (usize, usize)) -> Result<(), PsdError> {
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    for s in strips {
        if x == width {
            (x, y) = (0, y + row_height);
        }
        if x == 0 {
            row_height = s.height;
        }
        let even = [s.x, s.y, s.width, s.height].iter().all(|v| v % 2 == 0);
        if s.x != x || s.y != y || s.height != row_height || s.width == 0 || s.height == 0 || !even
        {
            return Err(invalid("strips do not tile the plane"));
        }
        if s.x + s.width > width || s.y + s.height > height {
            return Err(invalid("strip outside the plane"));
        }
        x += s.width;
    }
    if x != width || y + row_height != height {
        return Err(invalid("strips do not cover the plane"));
    }
    Ok(())
}

// ponytail: each strip decodes into its own buffer before the copy, so the
// plane is briefly held twice.
/// RawFormat 8: independently coded strips, their positions in the tags.
pub(super) fn strips(
    bytes: &[u8],
    tags: &Tags,
    (width, height): (usize, usize),
    plane: &mut [u16],
) -> Result<(), PsdError> {
    let needed = |tag| -> Result<u64, PsdError> {
        tags.uint(tag)?
            .ok_or_else(|| invalid(&format!("missing tag {tag:#06x}")))
    };
    if needed(V8_CLIP)? != 0xffff {
        return Err(unsupported("RawFormat 8 clip value"));
    }
    if counted::<2>(tags, V8_SHIFT_DOWN)?.iter().any(|&v| v != 0) {
        return Err(unsupported("RawFormat 8 shift"));
    }
    let points = counted::<4>(tags, V8_GAMMA_POINTS)?;
    let slopes = counted::<4>(tags, V8_GAMMA_SLOPES)?;
    if points.iter().any(|&p| p != 0) || slopes.iter().any(|&s| s != 65536) {
        return Err(unsupported("RawFormat 8 gamma curve"));
    }
    let mut initial = [0u16; 4];
    for (value, tag) in initial.iter_mut().zip(V8_INITIAL) {
        *value = needed(tag)? as u16;
    }
    // rawspeed's set-up leaves the second green's value on both greens.
    let pred = [initial[0], initial[2], initial[2], initial[3]];
    let codes = code_table(
        tags.tiff.raw(
            tags.ifd
                .get(V8_CODES)
                .ok_or_else(|| invalid("missing code table"))?,
        ),
    )?;

    let total = to_usize(needed(V8_STRIPS_H)? * needed(V8_STRIPS_V)?)?;
    let offsets = counted::<4>(tags, V8_STRIP_OFFSETS)?;
    let positions = counted::<4>(tags, V8_STRIP_POSITIONS)?;
    let lengths = counted::<4>(tags, V8_STRIP_BITS)?;
    let widths = counted::<2>(tags, V8_STRIP_WIDTHS)?;
    let heights = counted::<2>(tags, V8_STRIP_HEIGHTS)?;
    let lists = [&offsets, &positions, &lengths, &widths, &heights];
    if total == 0 || lists.iter().any(|list| list.len() < total) {
        return Err(invalid("strip lists shorter than the strip count"));
    }
    let strips = (0..total)
        .map(|i| {
            let at = to_usize(offsets[i])?;
            let len = to_usize(lengths[i].div_ceil(8))?;
            let data = at
                .checked_add(len)
                .and_then(|end| bytes.get(at..end))
                .filter(|data| !data.is_empty())
                .ok_or(PsdError::Truncated)?;
            Ok(Strip {
                data,
                x: (positions[i] & 0xffff) as usize,
                y: (positions[i] >> 16) as usize,
                width: widths[i] as usize,
                height: heights[i] as usize,
            })
        })
        .collect::<Result<Vec<_>, PsdError>>()?;
    check_grid(&strips, (width, height))?;

    let decoded = strips
        .par_iter()
        .map(|strip| decode_strip(strip, &codes, pred))
        .collect::<Result<Vec<_>, PsdError>>()?;
    for (strip, values) in strips.iter().zip(decoded) {
        for (src, dst) in values
            .chunks_exact(strip.width)
            .zip(plane.chunks_exact_mut(width).skip(strip.y))
        {
            dst[strip.x..strip.x + strip.width].copy_from_slice(src);
        }
    }
    Ok(())
}
