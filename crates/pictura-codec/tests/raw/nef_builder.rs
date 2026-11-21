//! Synthetic Nikon NEFs for the vendor reader tests: IFD0 with an Exif IFD
//! holding a Nikon maker note (its own embedded TIFF), and SubIFDs for a
//! preview and the CFA sensor image, stored by the packers and the Nikon
//! Huffman encoder below.

use super::builder::{write_ifd, write_tiff, IfdSpec, Plane, Val};

/// Nikon's code tables as JPEG `BITS` and `HUFFVAL`: 12-bit lossy, lossy
/// after the split, lossless, then the same for 14 bits.
const TABLES: [([u8; 16], &[u8]); 6] = [
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[5, 4, 3, 6, 2, 7, 1, 0, 8, 9, 11, 10, 12, 0],
    ),
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[0x39, 0x5a, 0x38, 0x27, 0x16, 5, 4, 3, 2, 1, 0, 11, 12, 12],
    ),
    (
        [0, 1, 4, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[5, 4, 6, 3, 7, 2, 8, 1, 9, 0, 10, 11, 12],
    ),
    (
        [0, 1, 4, 3, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[5, 6, 4, 7, 8, 3, 9, 2, 1, 0, 10, 11, 12, 13, 14],
    ),
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0],
        &[8, 0x5c, 0x4b, 0x3a, 0x29, 7, 6, 5, 4, 3, 2, 1, 0, 13, 14],
    ),
    (
        [0, 1, 4, 2, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0],
        &[7, 6, 8, 5, 9, 4, 10, 3, 11, 12, 2, 0, 1, 13, 14],
    ),
];

/// A linearization table (maker-note tag `0x0096`).
#[derive(Clone)]
pub struct Lin {
    pub version: [u8; 2],
    pub vpred: [u16; 4],
    /// The listed curve entries (stepped for `0x44 0x20`, else direct).
    pub curve: Vec<u16>,
    /// The split row, written at byte 562.
    pub split: Option<u16>,
}

impl Lin {
    pub fn lossless(bits: u8) -> Self {
        let p = 1u16 << (bits - 3);
        Self {
            version: [0x46, 0x30],
            vpred: [p; 4],
            curve: Vec::new(),
            split: None,
        }
    }

    /// A `0x44 0x20` curve of 257 points over `2^bits` inputs: identity up
    /// to a quarter, then compressing.
    pub fn stepped(bits: u8) -> Self {
        let max = 1u32 << bits;
        let step = max / 256;
        let curve = (0..257u32)
            .map(|k| {
                let x = k * step;
                let knee = max / 4;
                let y = if x <= knee { x } else { knee + (x - knee) * 2 };
                y.min(max - 1) as u16
            })
            .collect();
        Self {
            version: [0x44, 0x20],
            vpred: [1 << (bits - 4); 4],
            curve,
            split: None,
        }
    }

    /// The curve the decoder builds, by predicted value.
    pub fn full_curve(&self, bits: u8) -> Vec<u16> {
        let max = 1usize << bits;
        if self.version[0] == 0x46 {
            return (0..max as u32).map(|v| v as u16).collect();
        }
        if self.version == [0x44, 0x20] {
            let n = self.curve.len();
            let step = max / (n - 1);
            let last = (n - 1) * step;
            return (0..max.max(last + 1))
                .map(|i| {
                    if i >= last {
                        return self.curve[n - 1];
                    }
                    let (k, f) = (i / step, i % step);
                    let (a, b) = (self.curve[k] as usize, self.curve[k + 1] as usize);
                    ((a * (step - f) + b * f) / step) as u16
                })
                .collect();
        }
        self.curve.clone()
    }

    fn bytes(&self, be: bool) -> Vec<u8> {
        let w = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
        let mut out = self.version.to_vec();
        self.vpred.iter().for_each(|&v| out.extend(w(v)));
        out.extend(w(self.curve.len() as u16));
        self.curve.iter().for_each(|&v| out.extend(w(v)));
        if let Some(split) = self.split {
            assert!(out.len() <= 562, "the curve overlaps the split field");
            out.resize(562, 0);
            out.extend(w(split));
        }
        out
    }
}

pub enum Data {
    /// Compression 1, packed most-significant bit first, in strips.
    Msb { rows_per_strip: usize },
    /// Compression 1, 16-bit containers in the file's byte order.
    Containers,
    /// Compression 34713 without a linearization table, packed
    /// least-significant bit first.
    Lsb,
    /// Compression 34713, Huffman-coded; the plane holds the curve inputs.
    Huffman(Lin),
}

pub struct Nef {
    pub plane: Plane,
    pub bits: u8,
    pub be: bool,
    pub data: Data,
    /// Overrides the raw IFD's compression tag.
    pub compression: Option<u16>,
    pub nef_compression: Option<u16>,
    pub cfa: [u8; 4],
    pub ifd0_tags: Vec<(u16, Val)>,
    pub black: Option<Vec<u16>>,
    pub wb: Option<Vec<(u32, u32)>>,
    /// Replaces the linearization table's bytes.
    pub lin_bytes: Option<Vec<u8>>,
    /// Overrides the raw IFD's `ImageLength`.
    pub height_tag: Option<u32>,
}

impl Nef {
    pub fn new(plane: Plane, bits: u8, data: Data) -> Self {
        Self {
            plane,
            bits,
            be: true,
            data,
            compression: None,
            nef_compression: None,
            cfa: [0, 1, 1, 2],
            ifd0_tags: vec![
                (271, Val::Ascii("NIKON CORPORATION")),
                (272, Val::Ascii("NIKON SYNTHETIC")),
            ],
            black: None,
            wb: None,
            lin_bytes: None,
            height_tag: None,
        }
    }
}

struct BitWriter {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitWriter {
    fn put(&mut self, v: u32, len: u32) {
        if len == 0 {
            return;
        }
        self.acc = (self.acc << len) | (v as u64 & ((1u64 << len) - 1));
        self.n += len;
        while self.n >= 8 {
            self.n -= 8;
            self.out.push((self.acc >> self.n) as u8);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push((self.acc << (8 - self.n)) as u8);
        }
        self.out
    }
}

/// `(symbol, code, code length)` for a table, the first code of a repeated
/// symbol only.
fn codes(index: usize) -> Vec<(u8, u32, u32)> {
    let (counts, symbols) = TABLES[index];
    let mut out: Vec<(u8, u32, u32)> = Vec::new();
    let (mut code, mut k) = (0u32, 0usize);
    for len in 1..=16u32 {
        for _ in 0..counts[len as usize - 1] {
            if out.iter().all(|c| c.0 != symbols[k]) {
                out.push((symbols[k], code, len));
            }
            code += 1;
            k += 1;
        }
        code <<= 1;
    }
    out
}

/// The difference symbol `s` and bits `v` decode to.
fn decoded(s: u8, v: i32) -> i32 {
    let (len, shl) = ((s & 15) as u32, (s >> 4) as u32);
    if len == 0 {
        return 0;
    }
    let m = if shl == 0 {
        v
    } else {
        (v << shl) | (1 << (shl - 1))
    };
    if m & (1 << (len - 1)) == 0 {
        m - (1 << len) + (shl == 0) as i32
    } else {
        m
    }
}

/// The symbol, code and bits whose decoded difference is nearest `d`.
fn nearest(table: &[(u8, u32, u32)], d: i32) -> (u8, u32, u32, i32, u32, i32) {
    let mut best: Option<(u8, u32, u32, i32, u32, i32)> = None;
    for &(s, code, clen) in table {
        let (len, shl) = ((s & 15) as u32, (s >> 4) as u32);
        let nbits = len.saturating_sub(shl);
        let top = (1i32 << nbits) - 1;
        let guesses = if len == 0 {
            vec![0]
        } else {
            let pos = d >> shl;
            let neg = (d + (1 << len) - (shl == 0) as i32) >> shl;
            vec![pos - 1, pos, pos + 1, neg - 1, neg, neg + 1]
        };
        for v in guesses.into_iter().map(|v| v.clamp(0, top)) {
            let got = decoded(s, v);
            let cost = ((got - d).abs(), clen + nbits);
            if best.is_none_or(|b| cost < ((b.5 - d).abs(), b.2 + b.4)) {
                best = Some((s, code, clen, v, nbits, got));
            }
        }
    }
    best.expect("a table has codes")
}

/// Huffman-codes `plane` (curve inputs) closed-loop; returns the stream and
/// the predicted values a decoder reconstructs.
pub fn encode_huffman(plane: &Plane, lin: &Lin, bits: u8) -> (Vec<u8>, Vec<i32>) {
    let base = if bits == 14 { 3 } else { 0 };
    let lossless = lin.version[0] == 0x46;
    let mut table = codes(base + if lossless { 2 } else { 0 });
    let split = lin
        .split
        .filter(|_| lin.version == [0x44, 0x20])
        .unwrap_or(0) as usize;
    let mut vpred = [
        [lin.vpred[0] as i16 as i32, lin.vpred[1] as i16 as i32],
        [lin.vpred[2] as i16 as i32, lin.vpred[3] as i16 as i32],
    ];
    let mut w = BitWriter {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    let mut recon = Vec::with_capacity(plane.samples.len());
    for y in 0..plane.height {
        if split > 0 && y == split {
            table = codes(base + 1);
        }
        let mut hpred = [0i32; 2];
        for x in 0..plane.width {
            let pred = if x < 2 { vpred[y & 1][x] } else { hpred[x & 1] };
            let target = plane.samples[y * plane.width + x] as i32 - pred;
            let (_, code, clen, v, nbits, got) = nearest(&table, target);
            w.put(code, clen);
            w.put(v as u32, nbits);
            if x < 2 {
                vpred[y & 1][x] += got;
                hpred[x] = vpred[y & 1][x];
            } else {
                hpred[x & 1] += got;
            }
            recon.push(hpred[x & 1]);
        }
    }
    (w.finish(), recon)
}

fn pack_msb(plane: &Plane, bits: u8, rows: std::ops::Range<usize>) -> Vec<u8> {
    let mut out = Vec::new();
    for y in rows {
        let mut w = BitWriter {
            out: Vec::new(),
            acc: 0,
            n: 0,
        };
        for &v in &plane.samples[y * plane.width..][..plane.width] {
            w.put(v as u32, bits as u32);
        }
        out.extend(w.finish());
    }
    out
}

fn pack_lsb(plane: &Plane, bits: u8) -> Vec<u8> {
    let mut out = Vec::new();
    for row in plane.samples.chunks(plane.width) {
        let (mut acc, mut n) = (0u64, 0u32);
        for &v in row {
            acc |= (v as u64) << n;
            n += bits as u32;
            while n >= 8 {
                out.push(acc as u8);
                acc >>= 8;
                n -= 8;
            }
        }
        if n > 0 {
            out.push(acc as u8);
        }
    }
    out
}

fn containers(plane: &Plane, be: bool) -> Vec<u8> {
    plane
        .samples
        .iter()
        .flat_map(|&v| if be { v.to_be_bytes() } else { v.to_le_bytes() })
        .collect()
}

pub fn nef(spec: &Nef) -> Vec<u8> {
    let plane = &spec.plane;
    let be = spec.be;
    let mut out = if be {
        b"MM\0\x2a\0\0\0\0".to_vec()
    } else {
        b"II\x2a\0\0\0\0\0".to_vec()
    };

    let mut note_tags = Vec::new();
    if let Some(black) = &spec.black {
        note_tags.push((0x003D, Val::Short(black.clone())));
    }
    if let Some(wb) = &spec.wb {
        note_tags.push((0x000C, Val::Rational(wb.clone())));
    }
    if let Some(c) = spec.nef_compression {
        note_tags.push((0x0093, Val::Short(vec![c])));
    }
    if let Data::Huffman(lin) = &spec.data {
        let bytes = spec.lin_bytes.clone().unwrap_or_else(|| lin.bytes(be));
        note_tags.push((0x0096, Val::Byte(bytes)));
    }
    let mut note = b"Nikon\0\x02\x10\0\0".to_vec();
    note.extend(write_tiff(
        &IfdSpec {
            tags: note_tags,
            ..IfdSpec::default()
        },
        be,
        false,
    ));
    let exif_tags = vec![(0x927C, Val::Byte(note))];
    let exif_at = write_ifd(
        &mut out,
        &IfdSpec {
            tags: exif_tags,
            ..IfdSpec::default()
        },
        be,
        false,
    ) as u32;

    let (compression, rows_per_strip, chunks) = match &spec.data {
        Data::Msb { rows_per_strip } => {
            let chunks = (0..plane.height)
                .step_by(*rows_per_strip)
                .map(|y0| {
                    pack_msb(
                        plane,
                        spec.bits,
                        y0..(y0 + rows_per_strip).min(plane.height),
                    )
                })
                .collect();
            (1, *rows_per_strip, chunks)
        }
        Data::Containers => (1, plane.height, vec![containers(plane, be)]),
        Data::Lsb => (34713, plane.height, vec![pack_lsb(plane, spec.bits)]),
        Data::Huffman(lin) => (
            34713,
            plane.height,
            vec![encode_huffman(plane, lin, spec.bits).0],
        ),
    };
    let raw = IfdSpec {
        tags: vec![
            (254, Val::Long(vec![0])),
            (256, Val::Long(vec![plane.width as u32])),
            (
                257,
                Val::Long(vec![spec.height_tag.unwrap_or(plane.height as u32)]),
            ),
            (258, Val::Short(vec![spec.bits as u16])),
            (
                259,
                Val::Short(vec![spec.compression.unwrap_or(compression)]),
            ),
            (262, Val::Short(vec![32803])),
            (278, Val::Long(vec![rows_per_strip as u32])),
            (33421, Val::Short(vec![2, 2])),
            (33422, Val::Byte(spec.cfa.to_vec())),
        ],
        chunks: Some((273, 279, chunks)),
        subs: Vec::new(),
    };
    let preview = IfdSpec {
        tags: vec![(254, Val::Long(vec![1])), (259, Val::Short(vec![6]))],
        ..IfdSpec::default()
    };
    let mut ifd0_tags = vec![(34665, Val::Long(vec![exif_at]))];
    ifd0_tags.extend(spec.ifd0_tags.iter().cloned());
    let ifd0 = IfdSpec {
        tags: ifd0_tags,
        chunks: None,
        subs: vec![preview, raw],
    };
    let ifd0_at = write_ifd(&mut out, &ifd0, be, false) as u32;
    let first = if be {
        ifd0_at.to_be_bytes()
    } else {
        ifd0_at.to_le_bytes()
    };
    out[4..8].copy_from_slice(&first);
    out
}
