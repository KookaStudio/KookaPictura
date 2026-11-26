//! Synthetic Sony ARWs for the vendor reader tests: IFD0 naming a raw SubIFD
//! (uncompressed 16-bit words, ARW2 blocks, or lossless JPEG tiles) and,
//! optionally, an enciphered SR2 private block carrying the levels.

use super::builder::{write_ifd, IfdSpec, Plane, Val};
use super::ljpeg_encode;

#[derive(Clone, Copy)]
pub enum Coding {
    /// One sample per little-endian 16-bit word.
    Uncompressed,
    /// The plane holds 11-bit codes; `knees` is tag `0x7010`.
    Arw2 { knees: [u16; 4] },
    /// Tiles of lossless JPEG, four components to a frame sample; the plane
    /// is padded to whole tiles and tag `0x7038` names its size.
    Lossless { tile_w: usize, tile_h: usize },
}

/// Black per 2 × 2 position, white, and as-shot RGGB multipliers.
#[derive(Clone, Copy)]
pub struct Levels {
    pub black: [u16; 4],
    pub white: u16,
    pub wb_rggb: [u16; 4],
}

#[derive(Clone, Copy, PartialEq)]
pub enum Place {
    Nowhere,
    /// In the raw IFD (`0x7310`, `0x7313`, `0xC61D`).
    RawIfd,
    /// In the SR2SubIFD (`0x7310`, `0x7313`, `0x787F`).
    Sr2,
    /// In the SR2SubIFD with the older tags (`0x7300`, GRBG `0x7303`).
    Sr2Old,
}

pub struct Arw {
    pub plane: Plane,
    pub bits: u16,
    pub coding: Coding,
    pub levels: Levels,
    pub place: Place,
    /// Levels a second place carries, to test which one wins.
    pub sr2_too: Option<Levels>,
    pub sr2_key: u32,
    pub make: &'static str,
    pub model: &'static str,
    pub orientation: u16,
    pub cfa: [u8; 4],
    pub crop: Option<([u32; 2], [u32; 2])>,
    pub photometric: u16,
    pub compression: Option<u16>,
    /// Applied last to the raw IFD.
    pub raw_tags: Vec<(u16, Val)>,
}

impl Arw {
    pub fn new(plane: Plane, coding: Coding) -> Self {
        Self {
            plane,
            bits: 14,
            coding,
            levels: Levels {
                black: [512; 4],
                white: 16383,
                wb_rggb: [2272, 1024, 1024, 1800],
            },
            place: Place::RawIfd,
            sr2_too: None,
            sr2_key: 0x4433_2211,
            make: "SONY",
            model: "ILCE-7M3",
            orientation: 1,
            cfa: [0, 1, 1, 2],
            crop: None,
            photometric: 32803,
            compression: None,
            raw_tags: Vec::new(),
        }
    }
}

/// The ARW2 shift for a block whose codes span `range`.
fn shift(range: u32) -> u32 {
    let mut s = 0;
    while s < 4 && (0x80 << s) <= range {
        s += 1;
    }
    s
}

/// Encodes 16 same-colour codes; returns the block and the codes a decoder
/// reconstructs from it.
fn encode_block(v: &[u16; 16]) -> ([u8; 16], [u16; 16]) {
    let max = *v.iter().max().unwrap() as u32;
    let min = *v.iter().min().unwrap() as u32;
    let imax = v.iter().position(|&x| x as u32 == max).unwrap();
    let mut imin = v.iter().position(|&x| x as u32 == min).unwrap();
    if imin == imax {
        imin = (imax + 1) % 16;
    }
    let s = shift(max - min);
    let mut bits: u128 = max as u128 | (min as u128) << 11 | (imax as u128) << 22;
    bits |= (imin as u128) << 26;
    let mut out = [0u16; 16];
    let mut at = 30;
    for (i, &x) in v.iter().enumerate() {
        out[i] = if i == imax {
            max as u16
        } else if i == imin {
            min as u16
        } else {
            let d = ((x as u32 - min) >> s).min(127);
            bits |= (d as u128) << at;
            at += 7;
            ((d << s) + min).min(0x7FF) as u16
        };
    }
    (bits.to_le_bytes(), out)
}

/// ARW2 data for a plane of 11-bit codes (width a multiple of 32), and the
/// codes a decoder reconstructs.
pub fn encode_arw2(plane: &Plane) -> (Vec<u8>, Vec<u16>) {
    let mut data = Vec::with_capacity(plane.samples.len());
    let mut decoded = vec![0u16; plane.samples.len()];
    for y in 0..plane.height {
        for g in (0..plane.width).step_by(32) {
            for phase in 0..2 {
                let at = |k: usize| y * plane.width + g + 2 * k + phase;
                let codes: [u16; 16] = std::array::from_fn(|k| plane.samples[at(k)]);
                let (block, back) = encode_block(&codes);
                data.extend(block);
                for (k, c) in back.into_iter().enumerate() {
                    decoded[at(k)] = c;
                }
            }
        }
    }
    (data, decoded)
}

/// The tone curve of tag `0x7010` over 11-bit codes, as the format describes
/// it: slopes 1, 2, 4, 8, 16 per half-code between knees at 0, each tag value
/// over 4, and 4095; a code reads the curve at twice its value.
pub fn curve(knees: [u16; 4]) -> Vec<u16> {
    let k = [
        0,
        knees[0] as u32 / 4,
        knees[1] as u32 / 4,
        knees[2] as u32 / 4,
        knees[3] as u32 / 4,
        4095,
    ];
    let value = |j: u32| -> u32 {
        (0..5)
            .map(|i| (j.min(k[i + 1]).saturating_sub(k[i])) << i)
            .sum()
    };
    (0..2048).map(|c| value(2 * c).min(65535) as u16).collect()
}

fn shorts(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A little-endian IFD at `at` with every value after the entry table, so
/// the whole directory lies inside `[at, at + len)` (the SR2 layout).
fn sr2_ifd(at: u32, tags: &[(u16, &[u16])]) -> Vec<u8> {
    let table = 2 + 12 * tags.len() + 4;
    let mut head = (tags.len() as u16).to_le_bytes().to_vec();
    let mut values = Vec::new();
    for &(tag, v) in tags {
        head.extend(tag.to_le_bytes());
        head.extend(3u16.to_le_bytes());
        head.extend((v.len() as u32).to_le_bytes());
        if v.len() <= 2 {
            let mut inline = shorts(v);
            inline.resize(4, 0);
            head.extend(inline);
        } else {
            head.extend((at + (table + values.len()) as u32).to_le_bytes());
            values.extend(shorts(v));
        }
    }
    head.extend([0; 4]);
    head.extend(values);
    head
}

/// The SR2 cipher key stream XORed onto big-endian words (its own inverse).
pub fn cipher(data: &mut [u8], mut key: u32) {
    let mut pad = [0u32; 128];
    for w in &mut pad[..4] {
        key = key.wrapping_mul(48_828_125).wrapping_add(1);
        *w = key;
    }
    pad[3] = (pad[3] << 1) | ((pad[0] ^ pad[2]) >> 31);
    for p in 4..127 {
        pad[p] = ((pad[p - 4] ^ pad[p - 2]) << 1) | ((pad[p - 3] ^ pad[p - 1]) >> 31);
    }
    for (p, w) in (127usize..).zip(data.as_chunks_mut::<4>().0) {
        pad[p & 127] = pad[(p + 1) & 127] ^ pad[(p + 65) & 127];
        *w = (u32::from_be_bytes(*w) ^ pad[p & 127]).to_be_bytes();
    }
}

fn sr2_tags(l: &Levels, old: bool) -> Vec<(u16, Vec<u16>)> {
    let [r, g1, g2, b] = l.wb_rggb;
    if old {
        vec![
            (0x7300, l.black.to_vec()),
            (0x7303, vec![g1, r, b, g2]),
            (0x787F, vec![l.white; 3]),
        ]
    } else {
        vec![
            (0x7310, l.black.to_vec()),
            (0x7313, l.wb_rggb.to_vec()),
            (0x787F, vec![l.white; 3]),
        ]
    }
}

/// `(offsets tag, byte counts tag, chunks)`, as `IfdSpec::chunks` takes them.
type Chunks = (u16, u16, Vec<Vec<u8>>);

/// The raw chunks and the tags that describe them.
fn chunks(spec: &Arw) -> (Vec<(u16, Val)>, Chunks) {
    let p = &spec.plane;
    match spec.coding {
        Coding::Uncompressed => (
            vec![(278, Val::Long(vec![p.height as u32]))],
            (273, 279, vec![shorts(&p.samples)]),
        ),
        Coding::Arw2 { knees } => (
            vec![
                (278, Val::Long(vec![p.height as u32])),
                (0x7010, Val::Short(knees.to_vec())),
            ],
            (273, 279, vec![encode_arw2(p).0]),
        ),
        Coding::Lossless { tile_w, tile_h } => {
            let (pw, ph) = (
                p.width.div_ceil(tile_w) * tile_w,
                p.height.div_ceil(tile_h) * tile_h,
            );
            let mut tiles = Vec::new();
            for ty in (0..ph).step_by(tile_h) {
                for tx in (0..pw).step_by(tile_w) {
                    // Each four-component frame sample is one 2 × 2 quad,
                    // its components row-major.
                    let mut tile = Vec::with_capacity(tile_w * tile_h);
                    for r in 0..tile_h / 2 {
                        for q in 0..tile_w / 2 {
                            for c in 0..4 {
                                let (y, x) = (ty + 2 * r + c / 2, tx + 2 * q + c % 2);
                                let inside = y < p.height && x < p.width;
                                tile.push(if inside {
                                    p.samples[y * p.width + x]
                                } else {
                                    0
                                });
                            }
                        }
                    }
                    tiles.push(ljpeg_encode::encode(
                        &tile,
                        tile_w / 2,
                        tile_h / 2,
                        4,
                        spec.bits as u8,
                        1,
                        0,
                    ));
                }
            }
            (
                vec![
                    (256, Val::Long(vec![pw as u32])),
                    (257, Val::Long(vec![ph as u32])),
                    (322, Val::Long(vec![tile_w as u32])),
                    (323, Val::Long(vec![tile_h as u32])),
                    (0x7038, Val::Long(vec![p.width as u32, p.height as u32])),
                ],
                (324, 325, tiles),
            )
        }
    }
}

pub fn arw(spec: &Arw) -> Vec<u8> {
    let p = &spec.plane;
    let mut out = b"II\x2a\x00\0\0\0\0".to_vec();
    let mut ifd0_tags = vec![
        (254, Val::Long(vec![1])),
        (271, Val::Ascii(spec.make)),
        (272, Val::Ascii(spec.model)),
        (274, Val::Short(vec![spec.orientation])),
    ];

    let sr2 = match spec.place {
        Place::Sr2 => Some((spec.levels, false)),
        Place::Sr2Old => Some((spec.levels, true)),
        _ => spec.sr2_too.map(|l| (l, false)),
    };
    if let Some((levels, old)) = sr2 {
        let at = out.len() as u32;
        let tags = sr2_tags(&levels, old);
        let refs: Vec<(u16, &[u16])> = tags.iter().map(|(t, v)| (*t, v.as_slice())).collect();
        let mut block = sr2_ifd(at, &refs);
        let len = block.len() as u32;
        cipher(&mut block, spec.sr2_key);
        out.extend(block);
        let private = IfdSpec {
            tags: vec![
                (0x7200, Val::Long(vec![at])),
                (0x7201, Val::Long(vec![len])),
                (0x7221, Val::Byte(spec.sr2_key.to_le_bytes().to_vec())),
            ],
            ..IfdSpec::default()
        };
        let private_at = write_ifd(&mut out, &private, false, false) as u32;
        ifd0_tags.push((0xC634, Val::Byte(private_at.to_le_bytes().to_vec())));
    }

    let compression = spec.compression.unwrap_or(match spec.coding {
        Coding::Uncompressed => 1,
        Coding::Arw2 { .. } => 32767,
        Coding::Lossless { .. } => 7,
    });
    let (coding_tags, data) = chunks(spec);
    let mut raw_tags = vec![
        (254, Val::Long(vec![0])),
        (256, Val::Long(vec![p.width as u32])),
        (257, Val::Long(vec![p.height as u32])),
        (258, Val::Short(vec![spec.bits])),
        (259, Val::Short(vec![compression])),
        (262, Val::Short(vec![spec.photometric])),
        (277, Val::Short(vec![1])),
        (33421, Val::Short(vec![2, 2])),
        (33422, Val::Byte(spec.cfa.to_vec())),
    ];
    raw_tags.extend(coding_tags);
    if spec.place == Place::RawIfd {
        let l = spec.levels;
        raw_tags.extend([
            (0x7310, Val::Short(l.black.to_vec())),
            (0x7313, Val::Short(l.wb_rggb.to_vec())),
            (0xC61D, Val::Short(vec![l.white])),
        ]);
    }
    if let Some((origin, size)) = spec.crop {
        raw_tags.push((0xC61F, Val::Long(origin.to_vec())));
        raw_tags.push((0xC620, Val::Long(size.to_vec())));
    }
    raw_tags.extend(spec.raw_tags.iter().cloned());
    let mut dedup: Vec<(u16, Val)> = Vec::new();
    for (t, v) in raw_tags {
        dedup.retain(|(u, _)| *u != t);
        dedup.push((t, v));
    }
    let raw = IfdSpec {
        tags: dedup,
        chunks: Some(data),
        subs: Vec::new(),
    };
    let ifd0 = IfdSpec {
        tags: ifd0_tags,
        chunks: None,
        subs: vec![raw],
    };
    let ifd0_at = write_ifd(&mut out, &ifd0, false, false) as u32;
    out[4..8].copy_from_slice(&ifd0_at.to_le_bytes());
    out
}
