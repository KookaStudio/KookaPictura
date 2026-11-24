//! Canon CRX sensor coding (#1474): the `CMP1` image header, the tile,
//! plane and subband headers, the C-RAW quantization tables, and the
//! per-plane decode into the sensor plane.
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

use rayon::prelude::*;

use super::crx_lines::{predict_k, q_step, BandLines, Bits, QSteps, Quant, Subband};
use super::crx_wavelet::{Level, TilePlane, BOTTOM, LEFT, RIGHT, TOP};
use crate::error::PsdError;

/// Extra coefficients a band carries per neighbouring tile, by level count,
/// the tile size's remainder modulo 8, and level (`exCoefNumTbl`).
const EX_COEF: [u8; 144] = [
    1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0,
    0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 2, 2, 1,
    0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 2, 2, 1, 0, 0, 1, 1, 1, 1, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 2, 2, 2, 2, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 2, 2, 1, 1, 0,
    1, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("CR3: {what}"))
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("CR3 {what}"))
}

fn be16(b: &[u8], at: usize) -> Result<u32, PsdError> {
    let v = b.get(at..at + 2).ok_or(PsdError::Truncated)?;
    Ok(u16::from_be_bytes([v[0], v[1]]) as u32)
}

fn be32(b: &[u8], at: usize) -> Result<u32, PsdError> {
    let v = b.get(at..at + 4).ok_or(PsdError::Truncated)?;
    Ok(u32::from_be_bytes([v[0], v[1], v[2], v[3]]))
}

/// A CRAW track's `CMP1` box: how its frames are coded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Header {
    pub version: u32,
    /// The full sensor plane, all four colour planes together.
    pub width: usize,
    pub height: usize,
    pub tile_width: usize,
    pub tile_height: usize,
    pub bits: u32,
    pub planes: u32,
    /// 0 RGGB, 1 GRBG, 2 GBRG, 3 BGGR, from the plane's corner.
    pub cfa: u32,
    pub encoding: u32,
    pub levels: usize,
    /// Bytes of tile, plane and subband headers at the start of a frame.
    pub header_size: usize,
    /// Encoding 3's output precision.
    pub median_bits: u32,
}

impl Header {
    /// The output samples' precision.
    pub fn precision(&self) -> u32 {
        if self.encoding == 3 {
            self.median_bits
        } else {
            self.bits
        }
    }
}

/// `crxParseImageHeader`: reads a `CMP1` box's content, refusing what LibRaw
/// refuses and anything that is not a four-plane Bayer frame.
pub(super) fn parse_header(c: &[u8]) -> Result<Header, PsdError> {
    // LibRaw reads at least 40 bytes and at most 85, zero-filled.
    if c.len() < 40 {
        return Err(invalid("CMP1 box too short"));
    }
    let byte = |at: usize| c.get(at).copied().unwrap_or(0);
    let planes = (c[25] >> 4) as u32;
    let use_median = c[32] >> 7 == 1 && c.len() >= 56 && planes == 4 && (byte(56) >> 6) & 1 == 1;
    let bits = c[24] as u32;
    let h = Header {
        version: be16(c, 4)?,
        width: be32(c, 8)? as usize,
        height: be32(c, 12)? as usize,
        tile_width: be32(c, 16)? as usize,
        tile_height: be32(c, 20)? as usize,
        bits,
        planes,
        cfa: (c[25] & 0xF) as u32,
        encoding: (c[26] >> 4) as u32,
        levels: (c[26] & 0xF) as usize,
        header_size: be32(c, 28)? as usize,
        median_bits: if use_median && c.len() >= 84 {
            byte(84) as u32
        } else {
            bits
        },
    };
    let (cols, rows) = (c[27] >> 7, (c[27] >> 6) & 1);
    let bad = (h.version != 0x100 && h.version != 0x200)
        || h.header_size == 0
        || h.tile_width > h.width
        || h.tile_height > h.height
        || h.levels > 3
        || cols > 1
        || rows > 1;
    if bad {
        return Err(invalid("CMP1 header"));
    }
    // Encoding 1 (signed samples, LibRaw reads up to 15 bits) is refused: no
    // still camera is known to write it.
    match h.encoding {
        0 | 3 if h.bits <= 14 => {}
        e => return Err(unsupported(&format!("coding {e} at {} bits", h.bits))),
    }
    if h.planes != 4 {
        return Err(unsupported(&format!("{}-plane frame", h.planes)));
    }
    if h.width & 1 == 1 || h.height & 1 == 1 || h.tile_width & 1 == 1 || h.tile_height & 1 == 1 {
        return Err(invalid("odd frame or tile size"));
    }
    if h.cfa > 3 || h.bits == 8 || h.median_bits == 0 || h.median_bits > 16 {
        return Err(invalid("CMP1 header"));
    }
    Ok(h)
}

struct BandHeader {
    width: usize,
    height: usize,
    data_offset: usize,
    data_size: usize,
    quant: Quant,
}

struct PlaneHeader {
    data_offset: usize,
    partial: bool,
    rounded_mask: i32,
    bands: Vec<BandHeader>,
}

struct Tile {
    flags: u32,
    width: usize,
    height: usize,
    data_offset: usize,
    qp_size: usize,
    extra: usize,
    has_qp: bool,
    planes: Vec<PlaneHeader>,
    steps: Option<QSteps>,
}

/// The geometry: plane size (half the sensor), and tile size and count.
struct Grid {
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    cols: usize,
    rows: usize,
}

fn grid(h: &Header) -> Result<Grid, PsdError> {
    let g = Grid {
        width: h.width / 2,
        height: h.height / 2,
        tile_width: h.tile_width / 2,
        tile_height: h.tile_height / 2,
        cols: 0,
        rows: 0,
    };
    if g.tile_width < 0x16 || g.tile_height < 0x16 || g.width > 0x7FFF || g.height > 0x7FFF {
        return Err(invalid("tile size"));
    }
    let cols = g.width.div_ceil(g.tile_width);
    let rows = g.height.div_ceil(g.tile_height);
    if cols > 0xFF
        || rows > 0xFF
        || g.width - g.tile_width * (cols - 1) < 0x16
        || g.height - g.tile_height * (rows - 1) < 0x16
    {
        return Err(invalid("tile grid"));
    }
    Ok(Grid { cols, rows, ..g })
}

/// `crxReadSubbandHeaders`.
fn band_headers(
    p: &[u8],
    at: &mut usize,
    count: usize,
) -> Result<Vec<(usize, usize, Quant)>, PsdError> {
    let mut out = Vec::with_capacity(count);
    let mut offset = 0usize;
    for n in 0..count {
        let (sign, size) = (be16(p, *at)?, be16(p, *at + 2)? as usize);
        if (sign != 0xFF03 || size != 8) && (sign != 0xFF13 || size != 16) {
            return Err(invalid("subband header"));
        }
        let h = p.get(*at..*at + 4 + size).ok_or(PsdError::Truncated)?;
        let total = be32(h, 4)? as usize;
        if n != (h[8] >> 4) as usize {
            return Err(invalid("subband order"));
        }
        let (pad, quant) = if sign == 0xFF03 {
            let bits = be32(h, 8)?;
            let quant = Quant {
                q: ((bits >> 19) & 0xFF) as i32,
                partial: bits & 0x800_0000 != 0,
                ..Quant::default()
            };
            ((bits & 0x7FFFF) as usize, quant)
        } else {
            if be16(h, 8)? & 0xFFF != 0 || be16(h, 18)? != 0 {
                return Err(unsupported("partial subband"));
            }
            let quant = Quant {
                step_base: be32(h, 12)? as i32,
                step_mult: be16(h, 10)?,
                ..Quant::default()
            };
            (be16(h, 16)? as usize, quant)
        };
        let size_out = total
            .checked_sub(pad)
            .ok_or_else(|| invalid("subband size"))?;
        out.push((offset, size_out, quant));
        offset = offset
            .checked_add(total)
            .ok_or_else(|| invalid("subband size"))?;
        *at += 4 + size;
    }
    Ok(out)
}

/// `crxProcessSubbands`: each band's size and the overlap with neighbouring
/// tiles, from the finest level's bands down to the low band.
fn band_geometry(h: &Header, tile: &Tile, bands: &mut [BandHeader]) {
    let flags = tile.flags;
    let mut bw = tile.width;
    let mut bh = tile.height;
    let mut b = bands.len() - 1;
    if h.levels == 0 {
        bands[0].width = bw;
        bands[0].height = bh;
        return;
    }
    let row_ex = &EX_COEF[0x30 * (h.levels - 1) + 6 * (tile.width & 7)..];
    let col_ex = &EX_COEF[0x30 * (h.levels - 1) + 6 * (tile.height & 7)..];
    let version2 = h.version == 0x200;
    let set = |band: &mut BandHeader, level: usize, cs: usize, ce: usize, rs: usize, re: usize| {
        if version2 {
            band.quant.col_start = cs as i32;
            band.quant.col_end = ce as i32;
            band.quant.row_start = rs as i32;
            band.quant.row_end = re as i32;
            band.quant.level_shift = 3 - level as u32;
        }
    };
    for level in 0..h.levels {
        let (odd_w, odd_h) = (bw & 1, bh & 1);
        bw = (bw + odd_w) >> 1;
        bh = (bh + odd_h) >> 1;
        let (mut w0, mut w1, mut h0, mut h1, mut cs, mut rs) = (0, 0, 0, 0, 0, 0);
        if flags & RIGHT != 0 {
            w0 = row_ex[2 * level] as usize;
            w1 = row_ex[2 * level + 1] as usize;
        }
        if flags & LEFT != 0 {
            w0 += 1;
            cs = 1;
        }
        if flags & BOTTOM != 0 {
            h0 = col_ex[2 * level] as usize;
            h1 = col_ex[2 * level + 1] as usize;
        }
        if flags & TOP != 0 {
            h0 += 1;
            rs = 1;
        }
        bands[b].width = bw + w0 - odd_w;
        bands[b].height = bh + h0 - odd_h;
        set(&mut bands[b], level + 1, cs, w0 - cs, rs, h0 - rs);
        bands[b - 1].width = bw + w1;
        bands[b - 1].height = bh + h0 - odd_h;
        set(&mut bands[b - 1], level + 1, 0, w1, rs, h0 - rs);
        bands[b - 2].width = bw + w0 - odd_w;
        bands[b - 2].height = bh + h1;
        set(&mut bands[b - 2], level + 1, cs, w0 - cs, 0, h1);
        b -= 3;
    }
    let we = if flags & RIGHT != 0 {
        row_ex[2 * h.levels - 1] as usize
    } else {
        0
    };
    let he = if flags & BOTTOM != 0 {
        col_ex[2 * h.levels - 1] as usize
    } else {
        0
    };
    bands[b].width = we + bw;
    bands[b].height = he + bh;
    set(&mut bands[b], h.levels, 0, we, 0, he);
}

/// `crxReadImageHeaders`: the tiles' flags, sizes and offsets, each plane's
/// subbands, and the version 2 quantization tables.
fn tiles(h: &Header, g: &Grid, frame: &[u8]) -> Result<Vec<Tile>, PsdError> {
    let hdr = frame.get(..h.header_size).ok_or(PsdError::Truncated)?;
    let data = &frame[h.header_size..];
    let n = g.cols * g.rows;
    let band_count = 3 * h.levels + 1;
    let mut at = 0usize;
    let mut tile_offset = 0usize;
    let mut out = Vec::with_capacity(n);
    for t in 0..n {
        let (col, row) = (t % g.cols, t / g.cols);
        let mut flags = 0;
        if g.cols > 1 {
            flags |= if col + 1 < g.cols { RIGHT } else { 0 } | if col > 0 { LEFT } else { 0 };
        }
        if g.rows > 1 {
            flags |= if row + 1 < g.rows { BOTTOM } else { 0 } | if row > 0 { TOP } else { 0 };
        }
        let width = if col + 1 < g.cols {
            g.tile_width
        } else {
            g.width - g.tile_width * (g.cols - 1)
        };
        let height = if row + 1 < g.rows {
            g.tile_height
        } else {
            g.height - g.tile_height * (g.rows - 1)
        };

        let (sign, size) = (be16(hdr, at)?, be16(hdr, at + 2)? as usize);
        if (sign != 0xFF01 || size != 8) && (sign != 0xFF11 || (size != 8 && size != 16)) {
            return Err(invalid("tile header"));
        }
        let th = hdr.get(at..at + 4 + size).ok_or(PsdError::Truncated)?;
        let tail = be16(th, 10)?;
        if (size == 8 && tail != 0) || (size == 16 && tail != 0x4000) || be16(th, 8)? as usize != t
        {
            return Err(invalid("tile header"));
        }
        let tile_size = be32(th, 4)? as usize;
        let (qp_size, extra) = if size == 16 {
            if be16(th, 18)? != 0 {
                return Err(invalid("tile header"));
            }
            (be32(th, 12)? as usize, be16(th, 16)? as usize)
        } else {
            (0, 0)
        };
        at += 4 + size;
        let mut tile = Tile {
            flags,
            width,
            height,
            data_offset: tile_offset,
            qp_size,
            extra,
            has_qp: size == 16,
            planes: Vec::with_capacity(4),
            steps: None,
        };
        tile_offset = tile_offset
            .checked_add(tile_size)
            .ok_or_else(|| invalid("tile size"))?;

        let mut plane_offset = 0usize;
        for p in 0..h.planes as usize {
            let ph = hdr.get(at..at + 12).ok_or(PsdError::Truncated)?;
            let (sign, size) = (be16(ph, 0)?, be16(ph, 2)?);
            if (sign != 0xFF02 && sign != 0xFF12)
                || size != 8
                || (ph[8] >> 4) as usize != p
                || ph[9..12] != [0, 0, 0]
            {
                return Err(invalid("plane header"));
            }
            let plane_size = be32(ph, 4)? as usize;
            let rounded = ((ph[8] >> 1) & 3) as i32;
            let partial = ph[8] & 8 != 0;
            if rounded != 0 && (h.levels != 0 || !partial) {
                return Err(invalid("plane header"));
            }
            at += 12;
            let mut bands: Vec<BandHeader> = band_headers(hdr, &mut at, band_count)?
                .into_iter()
                .map(|(data_offset, data_size, quant)| BandHeader {
                    width: 0,
                    height: 0,
                    data_offset,
                    data_size,
                    quant,
                })
                .collect();
            band_geometry(h, &tile, &mut bands);
            tile.planes.push(PlaneHeader {
                data_offset: plane_offset,
                partial,
                rounded_mask: if rounded != 0 { 1 << (rounded - 1) } else { 0 },
                bands,
            });
            plane_offset = plane_offset
                .checked_add(plane_size)
                .ok_or_else(|| invalid("plane size"))?;
        }
        out.push(tile);
    }
    if h.version == 0x200 {
        for tile in out.iter_mut().filter(|t| t.has_qp) {
            let qp = data
                .get(tile.data_offset..)
                .and_then(|d| d.get(..tile.qp_size))
                .ok_or(PsdError::Truncated)?;
            tile.steps = Some(q_steps(h.levels, tile.width, tile.height, qp)?);
        }
    }
    Ok(out)
}

/// `crxPrediction`.
fn predict(left: i32, top: i32, dh: i32, dv: i32) -> i32 {
    let grad = left.wrapping_add(dh);
    [grad, grad, left, top]
        [(((dv < 0) ^ (dh < 0)) as usize) << 1 | ((left < top) ^ (dh < 0)) as usize]
}

/// The tile's quantization parameters, a value per 8 × 2 block, coded as
/// lines of Golomb-Rice differences (`crxDecodeGolombTop`/`Normal`), then
/// `crxMakeQStep`'s step table per level, coarsest first.
fn q_steps(levels: usize, width: usize, height: usize, data: &[u8]) -> Result<QSteps, PsdError> {
    if !(1..=3).contains(&levels) {
        return Err(invalid("quantization levels"));
    }
    let qw = width.div_ceil(8);
    let qh = height.div_ceil(2);
    let mut table = Vec::with_capacity(qw * qh);
    let mut bits = Bits::new(data);
    let mut k = 0i32;
    let stride = qw + 2;
    let mut lines = vec![0i32; 2 * stride];
    for row in 0..qh {
        let (p0, p1) = if row & 1 == 1 {
            (stride, 0)
        } else {
            (0, stride)
        };
        let l = &mut lines;
        if row == 0 {
            l[p1] = 0;
            for i in 0..qw {
                l[p1 + i + 1] = l[p1 + i];
                let qp = bits.code(k, 23, 8)?;
                l[p1 + i + 1] = l[p1 + i + 1].wrapping_add(-((qp & 1) as i32) ^ (qp >> 1) as i32);
                k = predict_k(k, qp as i32, 7);
            }
        } else {
            l[p1] = l[p0 + 1];
            let mut dh = l[p0 + 1].wrapping_sub(l[p0]);
            for i in 0..qw {
                let v = predict(
                    l[p1 + i],
                    l[p0 + i + 1],
                    dh,
                    l[p0 + i].wrapping_sub(l[p1 + i]),
                );
                let qp = bits.code(k, 23, 8)?;
                l[p1 + i + 1] = v.wrapping_add(-((qp & 1) as i32) ^ (qp >> 1) as i32);
                if i + 1 < qw {
                    dh = l[p0 + i + 2].wrapping_sub(l[p0 + i + 1]);
                    k = predict_k(
                        k,
                        (qp.wrapping_add(2u32.wrapping_mul(dh.wrapping_abs() as u32)) >> 1) as i32,
                        7,
                    );
                } else {
                    k = predict_k(k, qp as i32, 7);
                }
            }
        }
        l[p1 + qw + 1] = l[p1 + qw].wrapping_add(1);
        table.extend((0..qw).map(|i| l[p1 + i + 1].wrapping_add(4)));
    }
    let row = |r: usize| &table[qw * r.min(qh - 1)..][..qw];
    let mut out = QSteps {
        table: Vec::new(),
        starts: Vec::with_capacity(levels),
        width: qw,
    };
    if levels == 3 {
        out.starts.push(out.table.len());
        for r in 0..height.div_ceil(8) {
            let rows = [row(4 * r), row(4 * r + 1), row(4 * r + 2), row(4 * r + 3)];
            for c in 0..qw {
                let q = rows.iter().fold(0i32, |s, r| s.wrapping_add(r[c]));
                out.table
                    .push(q_step(((q < 0) as i32 * 3).wrapping_add(q) >> 2)?);
            }
        }
    }
    if levels >= 2 {
        out.starts.push(out.table.len());
        for r in 0..height.div_ceil(4) {
            let (a, b) = (row(2 * r), row(2 * r + 1));
            for c in 0..qw {
                out.table.push(q_step(a[c].wrapping_add(b[c]) / 2)?);
            }
        }
    }
    out.starts.push(out.table.len());
    for &q in &table {
        out.table.push(q_step(q)?);
    }
    Ok(out)
}

/// Decodes one colour plane, tile by tile, into `out` (`g.width × g.height`
/// 16-bit values as LibRaw stores them for `h.encoding`).
fn decode_plane(
    h: &Header,
    g: &Grid,
    tiles: &[Tile],
    data: &[u8],
    plane: usize,
    out: &mut [u16],
) -> Result<(), PsdError> {
    let median = 1i32 << (h.bits - 1);
    let max = (1i32 << h.bits) - 1;
    // Encoding 3 keeps the plane values (as 16-bit) for `combine`.
    let store = |v: i32| -> u16 {
        if h.encoding == 3 {
            v as i16 as u16
        } else {
            median.wrapping_add(v).clamp(0, max) as u16
        }
    };
    let mut y0 = 0;
    for r in 0..g.rows {
        let mut x0 = 0;
        for c in 0..g.cols {
            let tile = &tiles[r * g.cols + c];
            let ph = &tile.planes[plane];
            let base = tile
                .data_offset
                .checked_add(tile.qp_size + tile.extra + ph.data_offset)
                .ok_or_else(|| invalid("plane offset"))?;
            let mut bands = Vec::with_capacity(ph.bands.len());
            for (n, b) in ph.bands.iter().enumerate() {
                let lines = if b.data_size == 0 {
                    None
                } else {
                    let start = base
                        .checked_add(b.data_offset)
                        .ok_or_else(|| invalid("subband offset"))?;
                    let bytes = data
                        .get(start..)
                        .and_then(|d| d.get(..b.data_size))
                        .ok_or(PsdError::Truncated)?;
                    let partial = ph.partial && n == 0;
                    let mask = if partial { ph.rounded_mask } else { 0 };
                    Some(BandLines::new(bytes, b.width, b.height, partial, mask))
                };
                bands.push(Subband::new(b.width, b.height, b.quant, lines));
            }
            let mut levels = Vec::with_capacity(h.levels);
            for l in 0..h.levels {
                levels.push(if l + 1 == h.levels {
                    Level::new(tile.width, tile.height)
                } else {
                    Level::new(ph.bands[3 * l + 5].width, ph.bands[3 * l + 4].height)
                });
            }
            let mut tp = TilePlane {
                bands,
                levels,
                flags: tile.flags,
                steps: tile.steps.as_ref(),
            };
            let mut put = |i: usize, row: &[i32]| {
                let dst = &mut out[(y0 + i) * g.width + x0..][..tile.width];
                for (d, &v) in dst.iter_mut().zip(row) {
                    *d = store(v);
                }
            };
            if h.levels > 0 {
                tp.initialize()?;
                for i in 0..tile.height {
                    let at = tp.next_row()?;
                    put(i, tp.line(h.levels - 1, at));
                }
            } else {
                let band = &mut tp.bands[0];
                let Some(lines) = band.lines.as_mut() else {
                    // LibRaw stops decoding the plane at a tile without data.
                    return Ok(());
                };
                for i in 0..tile.height {
                    lines.decode(&mut band.buf)?;
                    put(i, &band.buf);
                }
            }
            x0 += tile.width;
        }
        y0 += tiles[r * g.cols].height;
    }
    Ok(())
}

/// Decodes one CRAW frame (`frame`, from its tile headers on) into the
/// `h.width × h.height` sensor plane, each sample at `h.precision()` bits.
pub(super) fn decode(h: &Header, frame: &[u8]) -> Result<Vec<u16>, PsdError> {
    let g = grid(h)?;
    let tiles = tiles(h, &g, frame)?;
    let data = &frame[h.header_size..];
    let size = g.width * g.height;
    let planes: Vec<Vec<u16>> = (0..4)
        .into_par_iter()
        .map(|p| {
            let mut out = vec![0u16; size];
            decode_plane(h, &g, &tiles, data, p, &mut out).map(|()| out)
        })
        .collect::<Result<_, _>>()?;
    if h.encoding == 3 {
        let combined = combine(h, &planes);
        return Ok(interleave(h, &g, &combined));
    }
    Ok(interleave(h, &g, &planes))
}

/// Encoding 3's planes as red, two greens and blue (`crxConvertPlaneLine`'s
/// final pass): a luma-like plane and three differences.
fn combine(h: &Header, planes: &[Vec<u16>]) -> Vec<Vec<u16>> {
    let median = (1i32 << (h.median_bits - 1)) << 10;
    let max = (1i32 << h.median_bits) - 1;
    let p = |i: usize, at: usize| planes[i][at] as i16 as i32;
    let cells: Vec<[u16; 4]> = (0..planes[0].len())
        .map(|at| {
            let (p0, p1, p2, p3) = (p(0, at) << 10, p(1, at), p(2, at), p(3, at));
            let gr = median + p0 - 168 * p1 - 585 * p3;
            let gr = if gr < 0 {
                -(((gr.abs() + 512) >> 9) & !1)
            } else {
                ((gr + 512) >> 9) & !1
            };
            [
                (median + p0 + 1510 * p3 + 512) >> 10,
                (p2 + gr + 1) >> 1,
                (gr - p2 + 1) >> 1,
                (median + p0 + 1927 * p1 + 512) >> 10,
            ]
            .map(|v| v.clamp(0, max) as u16)
        })
        .collect();
    (0..4)
        .map(|i| cells.iter().map(|c| c[i]).collect())
        .collect()
}

/// The planes (red, first green, second green, blue) laid into the sensor
/// plane in the header's colour pattern.
fn interleave(h: &Header, g: &Grid, planes: &[Vec<u16>]) -> Vec<u16> {
    // Each plane's position in the 2 × 2 cell, row-major, per pattern.
    const AT: [[usize; 4]; 4] = [[0, 1, 2, 3], [1, 0, 3, 2], [2, 3, 0, 1], [3, 2, 1, 0]];
    let w = h.width;
    let mut out = vec![0u16; w * h.height];
    out.par_chunks_mut(2 * w).enumerate().for_each(|(y, rows)| {
        for (p, plane) in planes.iter().enumerate() {
            let cell = AT[h.cfa as usize][p];
            let row = &mut rows[(cell / 2) * w..][..w];
            let src = &plane[y * g.width..][..g.width];
            for (x, &v) in src.iter().enumerate() {
                row[2 * x + cell % 2] = v;
            }
        }
    });
    out
}
