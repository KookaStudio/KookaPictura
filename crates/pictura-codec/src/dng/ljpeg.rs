//! Lossless JPEG (ITU T.81 process 14, `SOF3`) as DNG compression 7 uses it.
//!
//! Huffman-coded prediction differences, predictors 1–7, interleaved
//! components, restart intervals and point transform. Difference category 16
//! means 32768 with no extra bits (the DNG convention).

use crate::error::PsdError;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("lossless JPEG: {what}"))
}

const FAST_BITS: u32 = 9;

pub(super) struct Huffman {
    /// `(code length, symbol)` for every 9-bit prefix; length 0 is a miss.
    fast: Vec<(u8, u8)>,
    maxcode: [i32; 17],
    offset: [i32; 17],
    symbols: Vec<u8>,
}

impl Huffman {
    pub(super) fn new(counts: &[u8; 16], symbols: Vec<u8>) -> Result<Self, PsdError> {
        let mut fast = vec![(0u8, 0u8); 1 << FAST_BITS];
        let mut maxcode = [-1i32; 17];
        let mut offset = [0i32; 17];
        let mut code = 0i32;
        let mut k = 0usize;
        for len in 1..=16usize {
            let n = counts[len - 1] as usize;
            if n > 0 {
                offset[len] = k as i32 - code;
                for _ in 0..n {
                    if code >= 1 << len {
                        return Err(invalid("Huffman table overflows its code space"));
                    }
                    if len as u32 <= FAST_BITS {
                        let shift = FAST_BITS - len as u32;
                        let first = (code as usize) << shift;
                        for slot in &mut fast[first..first + (1 << shift)] {
                            *slot = (len as u8, symbols[k]);
                        }
                    }
                    code += 1;
                    k += 1;
                }
                maxcode[len] = code - 1;
            }
            code <<= 1;
        }
        Ok(Self {
            fast,
            maxcode,
            offset,
            symbols,
        })
    }
}

pub(super) struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u64,
    count: u32,
    /// Whether `FF 00` stands for `FF` and `FF` with any other byte is a
    /// marker (JPEG), or every byte is data (Nikon NEF).
    stuffed: bool,
    /// Bytes fed from the data and zero bytes fed past a marker or the end.
    real: u64,
    padded: u64,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            acc: 0,
            count: 0,
            stuffed: true,
            real: 0,
            padded: 0,
        }
    }

    /// A reader of `data` with no byte stuffing and no markers.
    pub(super) fn plain(data: &'a [u8]) -> Self {
        Self {
            stuffed: false,
            ..Self::new(data, 0)
        }
    }

    fn fill(&mut self) {
        while self.count <= 56 {
            let byte = match self.data.get(self.pos) {
                Some(&0xFF) if self.stuffed => match self.data.get(self.pos + 1) {
                    Some(0) => {
                        self.pos += 2;
                        Some(0xFF)
                    }
                    _ => None,
                },
                Some(&b) => {
                    self.pos += 1;
                    Some(b)
                }
                None => None,
            };
            match byte {
                Some(b) => {
                    self.real += 1;
                    self.acc = (self.acc << 8) | b as u64;
                }
                None => {
                    self.padded += 1;
                    self.acc <<= 8;
                }
            }
            self.count += 8;
        }
    }

    fn peek16(&mut self) -> u32 {
        if self.count < 16 {
            self.fill();
        }
        ((self.acc >> (self.count - 16)) & 0xFFFF) as u32
    }

    pub(super) fn take(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if self.count < n {
            self.fill();
        }
        self.count -= n;
        ((self.acc >> self.count) & ((1u64 << n) - 1)) as u32
    }

    pub(super) fn overran(&self) -> bool {
        (self.real + self.padded) * 8 - self.count as u64 > self.real * 8
    }

    pub(super) fn decode(&mut self, h: &Huffman) -> Result<u8, PsdError> {
        let peek = self.peek16();
        let (len, sym) = h.fast[(peek >> (16 - FAST_BITS)) as usize];
        if len > 0 {
            self.count -= len as u32;
            return Ok(sym);
        }
        for len in FAST_BITS as usize + 1..=16 {
            let code = (peek >> (16 - len)) as i32;
            if code <= h.maxcode[len] {
                self.count -= len as u32;
                return h
                    .symbols
                    .get((code + h.offset[len]) as usize)
                    .copied()
                    .ok_or_else(|| invalid("Huffman code"));
            }
        }
        Err(invalid("Huffman code"))
    }

    /// Steps over a restart marker at the next byte boundary.
    fn restart(&mut self) -> Result<(), PsdError> {
        if self.overran() {
            return Err(PsdError::Truncated);
        }
        while self.data.get(self.pos) == Some(&0xFF) && self.data.get(self.pos + 1) == Some(&0xFF) {
            self.pos += 1;
        }
        match (self.data.get(self.pos), self.data.get(self.pos + 1)) {
            (Some(0xFF), Some(0xD0..=0xD7)) => self.pos += 2,
            _ => return Err(invalid("missing restart marker")),
        }
        self.acc = 0;
        self.count = 0;
        self.real = 0;
        self.padded = 0;
        Ok(())
    }
}

struct Frame {
    precision: u32,
    width: usize,
    height: usize,
    components: Vec<u8>,
}

fn diff(bits: &mut Bits, h: &Huffman) -> Result<i32, PsdError> {
    let s = bits.decode(h)? as u32;
    Ok(match s {
        0 => 0,
        16 => 32768,
        1..=15 => {
            let v = bits.take(s) as i32;
            if v < 1 << (s - 1) {
                v - (1 << s) + 1
            } else {
                v
            }
        }
        _ => return Err(invalid("difference category")),
    })
}

fn parse_frame(seg: &[u8]) -> Result<Frame, PsdError> {
    if seg.len() < 6 {
        return Err(PsdError::Truncated);
    }
    let precision = seg[0] as u32;
    let height = u16::from_be_bytes([seg[1], seg[2]]) as usize;
    let width = u16::from_be_bytes([seg[3], seg[4]]) as usize;
    let n = seg[5] as usize;
    if !(2..=16).contains(&precision) || !(1..=4).contains(&n) {
        return Err(invalid("frame header"));
    }
    if width == 0 || height == 0 {
        return Err(PsdError::Unsupported(
            "lossless JPEG without a height".into(),
        ));
    }
    let mut components = Vec::with_capacity(n);
    for c in 0..n {
        let spec = seg.get(6 + 3 * c..9 + 3 * c).ok_or(PsdError::Truncated)?;
        if spec[1] != 0x11 {
            return Err(PsdError::Unsupported("subsampled lossless JPEG".into()));
        }
        components.push(spec[0]);
    }
    Ok(Frame {
        precision,
        width,
        height,
        components,
    })
}

/// The frame header of `data` without decoding it: `(precision, width,
/// height, components)`, the width counted in samples per component.
pub(super) fn frame(data: &[u8]) -> Result<(u32, usize, usize, usize), PsdError> {
    if data.get(..2) != Some(&[0xFF, 0xD8]) {
        return Err(invalid("missing SOI"));
    }
    let mut at = 2usize;
    loop {
        while data.get(at) == Some(&0xFF) && data.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        let marker = match data.get(at..at + 2) {
            Some([0xFF, m]) => *m,
            Some(_) => return Err(invalid("expected a marker")),
            None => return Err(PsdError::Truncated),
        };
        at += 2;
        match marker {
            0xC3 => {
                let f = parse_frame(segment(data, at)?)?;
                return Ok((f.precision, f.width, f.height, f.components.len()));
            }
            0xC0..=0xCF if marker != 0xC4 && marker != 0xC8 && marker != 0xCC => {
                return Err(PsdError::Unsupported(format!(
                    "JPEG process SOF{}",
                    marker - 0xC0
                )));
            }
            0xDA | 0xD8 | 0xD9 | 0x01 | 0xD0..=0xD7 => return Err(invalid("scan before frame")),
            _ => at += segment(data, at)?.len() + 2,
        }
    }
}

fn segment(data: &[u8], at: usize) -> Result<&[u8], PsdError> {
    let len = data
        .get(at..at + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]) as usize)
        .ok_or(PsdError::Truncated)?;
    if len < 2 {
        return Err(invalid("segment length"));
    }
    data.get(at + 2..at + len).ok_or(PsdError::Truncated)
}

/// Decodes `data`, calling `row(first_sample_index, samples)` for every frame
/// row in order; a row holds `width × components` interleaved samples. A frame
/// of more than `limit` samples is refused before decoding.
pub(super) fn decode(
    data: &[u8],
    limit: usize,
    mut row: impl FnMut(usize, &[u16]),
) -> Result<(), PsdError> {
    if data.get(..2) != Some(&[0xFF, 0xD8]) {
        return Err(invalid("missing SOI"));
    }
    let mut tables: [Option<Huffman>; 4] = [None, None, None, None];
    let mut frame: Option<Frame> = None;
    let mut restart = 0usize;
    let mut at = 2usize;
    loop {
        while data.get(at) == Some(&0xFF) && data.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        let marker = match data.get(at..at + 2) {
            Some([0xFF, m]) => *m,
            Some(_) => return Err(invalid("expected a marker")),
            None => return Err(PsdError::Truncated),
        };
        at += 2;
        match marker {
            0xC4 => {
                let seg = segment(data, at)?;
                let mut p = 0;
                while p < seg.len() {
                    let id = (seg[p] & 0x0F) as usize;
                    let counts: [u8; 16] = seg
                        .get(p + 1..p + 17)
                        .ok_or(PsdError::Truncated)?
                        .try_into()
                        .unwrap();
                    let total: usize = counts.iter().map(|&c| c as usize).sum();
                    let symbols = seg
                        .get(p + 17..p + 17 + total)
                        .ok_or(PsdError::Truncated)?
                        .to_vec();
                    if id > 3 {
                        return Err(invalid("Huffman table id"));
                    }
                    tables[id] = Some(Huffman::new(&counts, symbols)?);
                    p += 17 + total;
                }
                at += seg.len() + 2;
            }
            0xC3 => {
                let seg = segment(data, at)?;
                let f = parse_frame(seg)?;
                if f.width * f.height * f.components.len() > limit {
                    return Err(invalid("frame larger than its tile"));
                }
                frame = Some(f);
                at += seg.len() + 2;
            }
            0xC0..=0xCF if marker != 0xC8 && marker != 0xCC => {
                return Err(PsdError::Unsupported(format!(
                    "JPEG process SOF{}",
                    marker - 0xC0
                )));
            }
            0xDD => {
                let seg = segment(data, at)?;
                if seg.len() < 2 {
                    return Err(PsdError::Truncated);
                }
                restart = u16::from_be_bytes([seg[0], seg[1]]) as usize;
                at += seg.len() + 2;
            }
            0xDA => {
                let seg = segment(data, at)?;
                let frame = frame.as_ref().ok_or_else(|| invalid("scan before frame"))?;
                let n = *seg.first().ok_or(PsdError::Truncated)? as usize;
                if n != frame.components.len() || seg.len() < 1 + 2 * n + 3 {
                    return Err(PsdError::Unsupported(
                        "non-interleaved lossless JPEG".into(),
                    ));
                }
                let mut huff = Vec::with_capacity(n);
                for c in 0..n {
                    if seg[1 + 2 * c] != frame.components[c] {
                        return Err(invalid("scan component order"));
                    }
                    let id = (seg[2 + 2 * c] >> 4) as usize;
                    huff.push(
                        tables
                            .get(id)
                            .and_then(Option::as_ref)
                            .ok_or_else(|| invalid("missing Huffman table"))?,
                    );
                }
                let predictor = seg[1 + 2 * n];
                let pt = (seg[3 + 2 * n] & 0x0F) as u32;
                if !(1..=7).contains(&predictor) || pt >= frame.precision {
                    return Err(invalid("scan header"));
                }
                let bits = Bits::new(data, at + seg.len() + 2);
                return scan(frame, &huff, predictor, pt, restart, bits, &mut row);
            }
            0xD8 | 0xD9 | 0x01 | 0xD0..=0xD7 => return Err(invalid("unexpected marker")),
            _ => at += segment(data, at)?.len() + 2,
        }
    }
}

fn scan(
    frame: &Frame,
    huff: &[&Huffman],
    predictor: u8,
    pt: u32,
    restart: usize,
    mut bits: Bits,
    row: &mut impl FnMut(usize, &[u16]),
) -> Result<(), PsdError> {
    let n = frame.components.len();
    let stride = frame.width * n;
    let default = 1i32 << (frame.precision - pt - 1);
    let mut prev = vec![0u16; stride];
    let mut cur = vec![0u16; stride];
    let mut out = vec![0u16; stride];
    let mut first_line = true;
    let mut use_default = true;
    let mut left_in_interval = restart;
    for y in 0..frame.height {
        for x in 0..frame.width {
            if restart > 0 {
                if left_in_interval == 0 {
                    bits.restart()?;
                    left_in_interval = restart;
                    first_line = true;
                    use_default = true;
                }
                left_in_interval -= 1;
            }
            for (c, table) in huff.iter().enumerate() {
                let i = x * n + c;
                let pred = if use_default {
                    default
                } else if first_line {
                    cur[i - n] as i32
                } else if x == 0 {
                    prev[i] as i32
                } else {
                    let (ra, rb, rc) = (cur[i - n] as i32, prev[i] as i32, prev[i - n] as i32);
                    match predictor {
                        1 => ra,
                        2 => rb,
                        3 => rc,
                        4 => ra + rb - rc,
                        5 => ra + ((rb - rc) >> 1),
                        6 => rb + ((ra - rc) >> 1),
                        _ => (ra + rb) >> 1,
                    }
                };
                let value = (pred + diff(&mut bits, table)?) as u16;
                cur[i] = value;
                out[i] = value << pt;
            }
            use_default = false;
        }
        if bits.overran() {
            return Err(PsdError::Truncated);
        }
        row(y * stride, &out);
        first_line = false;
        std::mem::swap(&mut prev, &mut cur);
    }
    Ok(())
}
