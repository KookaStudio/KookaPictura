//! A minimal TIFF/BigTIFF writer that lays out synthetic DNGs for the reader
//! tests: image chunks first, then each IFD after its value blobs.

use super::ljpeg_encode;

#[derive(Clone)]
pub enum Val {
    Byte(Vec<u8>),
    Ascii(&'static str),
    Short(Vec<u16>),
    Long(Vec<u32>),
    Rational(Vec<(u32, u32)>),
    SRational(Vec<(i32, i32)>),
}

impl Val {
    fn encode(&self, be: bool) -> (u16, u64, Vec<u8>) {
        let mut out = Vec::new();
        let (typ, count) = match self {
            Val::Byte(v) => {
                out.extend(v);
                (1, v.len())
            }
            Val::Ascii(s) => {
                out.extend(s.bytes());
                out.push(0);
                (2, s.len() + 1)
            }
            Val::Short(v) => {
                v.iter().for_each(|&x| out.extend(u16b(x, be)));
                (3, v.len())
            }
            Val::Long(v) => {
                v.iter().for_each(|&x| out.extend(u32b(x, be)));
                (4, v.len())
            }
            Val::Rational(v) => {
                v.iter().for_each(|&(n, d)| {
                    out.extend(u32b(n, be));
                    out.extend(u32b(d, be));
                });
                (5, v.len())
            }
            Val::SRational(v) => {
                v.iter().for_each(|&(n, d)| {
                    out.extend(u32b(n as u32, be));
                    out.extend(u32b(d as u32, be));
                });
                (10, v.len())
            }
        };
        (typ, count as u64, out)
    }
}

fn u16b(v: u16, be: bool) -> [u8; 2] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn u32b(v: u32, be: bool) -> [u8; 4] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn u64b(v: u64, be: bool) -> [u8; 8] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

#[derive(Default)]
pub struct IfdSpec {
    pub tags: Vec<(u16, Val)>,
    /// `(offsets tag, byte counts tag, chunks)`.
    pub chunks: Option<(u16, u16, Vec<Vec<u8>>)>,
    pub subs: Vec<IfdSpec>,
}

fn set(tags: &mut Vec<(u16, Val)>, tag: u16, val: Val) {
    tags.retain(|(t, _)| *t != tag);
    tags.push((tag, val));
}

fn align(out: &mut Vec<u8>) {
    if out.len() % 2 == 1 {
        out.push(0);
    }
}

fn write_ifd(out: &mut Vec<u8>, spec: &IfdSpec, be: bool, big: bool) -> u64 {
    let subs: Vec<u32> = spec
        .subs
        .iter()
        .map(|s| write_ifd(out, s, be, big) as u32)
        .collect();
    let mut tags = Vec::new();
    if let Some((off_tag, cnt_tag, chunks)) = &spec.chunks {
        let mut offsets = Vec::new();
        for c in chunks {
            align(out);
            offsets.push(out.len() as u32);
            out.extend(c);
        }
        tags.push((*off_tag, Val::Long(offsets)));
        tags.push((
            *cnt_tag,
            Val::Long(chunks.iter().map(|c| c.len() as u32).collect()),
        ));
    }
    if !subs.is_empty() {
        tags.push((330, Val::Long(subs)));
    }
    for (t, v) in &spec.tags {
        set(&mut tags, *t, v.clone());
    }
    tags.sort_by_key(|(t, _)| *t);
    let inline = if big { 8 } else { 4 };
    let mut entries = Vec::new();
    for (tag, val) in &tags {
        let (typ, count, data) = val.encode(be);
        let value = if data.len() > inline {
            align(out);
            let at = out.len() as u64;
            out.extend(&data);
            if big {
                u64b(at, be).to_vec()
            } else {
                u32b(at as u32, be).to_vec()
            }
        } else {
            let mut v = data;
            v.resize(inline, 0);
            v
        };
        entries.push((*tag, typ, count, value));
    }
    align(out);
    let at = out.len() as u64;
    if big {
        out.extend(u64b(entries.len() as u64, be));
    } else {
        out.extend(u16b(entries.len() as u16, be));
    }
    for (tag, typ, count, value) in entries {
        out.extend(u16b(tag, be));
        out.extend(u16b(typ, be));
        if big {
            out.extend(u64b(count, be));
        } else {
            out.extend(u32b(count as u32, be));
        }
        out.extend(value);
    }
    out.extend(vec![0; inline]);
    at
}

pub fn write_tiff(ifd0: &IfdSpec, be: bool, big: bool) -> Vec<u8> {
    let mut out = if be { b"MM".to_vec() } else { b"II".to_vec() };
    if big {
        out.extend(u16b(43, be));
        out.extend(u16b(8, be));
        out.extend(u16b(0, be));
        out.extend([0; 8]);
    } else {
        out.extend(u16b(42, be));
        out.extend([0; 4]);
    }
    let first = write_ifd(&mut out, ifd0, be, big);
    if big {
        out[8..16].copy_from_slice(&u64b(first, be));
    } else {
        out[4..8].copy_from_slice(&u32b(first as u32, be));
    }
    out
}

pub struct Plane {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<u16>,
}

/// A deterministic plane of values in `0..=max` (fixed LCG seed).
pub fn noise_plane(width: usize, height: usize, max: u16, seed: u64) -> Plane {
    let mut state = seed;
    let samples = (0..width * height)
        .map(|i| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // A smooth ramp plus noise, so predictions see real neighbours.
            let ramp = (i % width) as u64 * max as u64 / width as u64;
            ((ramp + (state >> 33) % (max as u64 / 4 + 1)) % (max as u64 + 1)) as u16
        })
        .collect();
    Plane {
        width,
        height,
        samples,
    }
}

#[derive(Clone, Copy)]
pub enum Codec {
    Raw {
        bits: u16,
    },
    Ljpeg {
        bits: u8,
        predictor: u8,
        components: usize,
        restart: usize,
    },
}

pub struct Options {
    pub codec: Codec,
    pub tile: Option<(usize, usize)>,
    pub rows_per_strip: Option<usize>,
    pub big_endian: bool,
    pub bigtiff: bool,
    pub in_subifd: bool,
    pub dng_version: bool,
    /// Applied last, so they override the generated tags.
    pub raw_tags: Vec<(u16, Val)>,
    pub ifd0_tags: Vec<(u16, Val)>,
    /// Extra zero rows encoded into every lossless JPEG frame.
    pub ljpeg_pad_rows: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            codec: Codec::Raw { bits: 16 },
            tile: None,
            rows_per_strip: None,
            big_endian: false,
            bigtiff: false,
            in_subifd: false,
            dng_version: true,
            raw_tags: Vec::new(),
            ifd0_tags: Vec::new(),
            ljpeg_pad_rows: 0,
        }
    }
}

fn pack(samples: &[u16], width: usize, rows: usize, bits: u16, be: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for r in 0..rows {
        let row = &samples[r * width..][..width];
        match bits {
            8 => out.extend(row.iter().map(|&v| v as u8)),
            16 => row.iter().for_each(|&v| out.extend(u16b(v, be))),
            _ => {
                let mut acc = 0u64;
                let mut n = 0;
                for &v in row {
                    acc = (acc << bits) | v as u64;
                    n += bits;
                    while n >= 8 {
                        n -= 8;
                        out.push((acc >> n) as u8);
                    }
                }
                if n > 0 {
                    out.push((acc << (8 - n)) as u8);
                }
            }
        }
    }
    out
}

fn encode_chunk(samples: &[u16], tw: usize, rows: usize, opts: &Options) -> Vec<u8> {
    match opts.codec {
        Codec::Raw { bits } => pack(samples, tw, rows, bits, opts.big_endian),
        Codec::Ljpeg {
            bits,
            predictor,
            components,
            restart,
        } => {
            let mut s = samples.to_vec();
            s.resize((rows + opts.ljpeg_pad_rows) * tw, 0);
            ljpeg_encode::encode(
                &s,
                tw / components,
                rows + opts.ljpeg_pad_rows,
                components,
                bits,
                predictor,
                restart,
            )
        }
    }
}

pub fn dng(plane: &Plane, opts: Options) -> Vec<u8> {
    let (w, h) = (plane.width, plane.height);
    let (bits, compression) = match opts.codec {
        Codec::Raw { bits } => (bits, 1),
        Codec::Ljpeg { bits, .. } => (bits as u16, 7),
    };
    let chunks = match opts.tile {
        Some((tw, th)) => {
            let mut chunks = Vec::new();
            for ty in (0..h).step_by(th) {
                for tx in (0..w).step_by(tw) {
                    let mut tile = vec![0u16; tw * th];
                    for r in 0..th.min(h - ty) {
                        for c in 0..tw.min(w - tx) {
                            tile[r * tw + c] = plane.samples[(ty + r) * w + tx + c];
                        }
                    }
                    chunks.push(encode_chunk(&tile, tw, th, &opts));
                }
            }
            (324, 325, chunks)
        }
        None => {
            let rps = opts.rows_per_strip.unwrap_or(h);
            let chunks = (0..h)
                .step_by(rps)
                .map(|y| {
                    let rows = rps.min(h - y);
                    encode_chunk(&plane.samples[y * w..(y + rows) * w], w, rows, &opts)
                })
                .collect();
            (273, 279, chunks)
        }
    };
    let mut raw_tags = vec![
        (254, Val::Long(vec![0])),
        (256, Val::Long(vec![w as u32])),
        (257, Val::Long(vec![h as u32])),
        (258, Val::Short(vec![bits])),
        (259, Val::Short(vec![compression])),
        (262, Val::Short(vec![32803])),
        (277, Val::Short(vec![1])),
        (33421, Val::Short(vec![2, 2])),
        (33422, Val::Byte(vec![0, 1, 1, 2])),
    ];
    match opts.tile {
        Some((tw, th)) => {
            raw_tags.push((322, Val::Long(vec![tw as u32])));
            raw_tags.push((323, Val::Long(vec![th as u32])));
        }
        None => raw_tags.push((
            278,
            Val::Long(vec![opts.rows_per_strip.unwrap_or(h) as u32]),
        )),
    }
    let raw = IfdSpec {
        tags: raw_tags,
        chunks: Some(chunks),
        subs: Vec::new(),
    };
    let mut ifd0_tags = Vec::new();
    if opts.dng_version {
        ifd0_tags.push((50706, Val::Byte(vec![1, 4, 0, 0])));
    }
    let mut ifd0 = if opts.in_subifd {
        ifd0_tags.extend([
            (254, Val::Long(vec![1])),
            (256, Val::Long(vec![8])),
            (257, Val::Long(vec![8])),
            (262, Val::Short(vec![2])),
        ]);
        IfdSpec {
            tags: ifd0_tags,
            chunks: None,
            subs: vec![raw],
        }
    } else {
        let mut raw = raw;
        raw.tags.extend(ifd0_tags);
        raw
    };
    for (t, v) in opts.raw_tags {
        if ifd0.subs.is_empty() {
            set(&mut ifd0.tags, t, v);
        } else {
            set(&mut ifd0.subs[0].tags, t, v);
        }
    }
    for (t, v) in opts.ifd0_tags {
        set(&mut ifd0.tags, t, v);
    }
    write_tiff(&ifd0, opts.big_endian, opts.bigtiff)
}
