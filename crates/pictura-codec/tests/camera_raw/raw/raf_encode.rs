//! A test encoder for Fujifilm-compressed RAF data: the header, block-size
//! table, lossy quantizer bases and the per-strip lines the reader decodes.
//! It runs the decoder's own prediction and adaptation, choosing each code
//! so the decoded sample lands nearest the target, and returns what a
//! correct decoder must produce.

const BLOCK_WIDTH: usize = 0x300;
const R2: usize = 2;
const R3: usize = 3;
const R4: usize = 4;
const G2: usize = 7;
const G3: usize = 8;
const G4: usize = 9;
const G5: usize = 10;
const G6: usize = 11;
const G7: usize = 12;
const B2: usize = 15;
const B3: usize = 16;
const B4: usize = 17;
const LINES: usize = 18;

/// The 6 × 6 CFA of compressed X-Trans bodies (X-H1, X-T5) at the plane's
/// origin.
pub const COMPRESSED_XTRANS: [u8; 36] = [
    1, 1, 0, 1, 1, 2, //
    1, 1, 2, 1, 1, 0, //
    2, 0, 1, 0, 2, 1, //
    1, 1, 2, 1, 1, 0, //
    1, 1, 0, 1, 1, 2, //
    0, 2, 1, 2, 0, 1, //
];
pub const RGGB: [u8; 4] = [0, 1, 1, 2];

pub struct Plane<'a> {
    pub width: usize,
    pub height: usize,
    pub samples: &'a [u16],
    /// `COMPRESSED_XTRANS` or `RGGB`.
    pub cfa: &'a [u8],
    pub bits: u32,
}

/// The data and the plane a decoder must produce from it. `q_bases` gives
/// lossy coding its base per block and line; `None` codes losslessly.
pub fn encode(p: &Plane, q_bases: Option<&dyn Fn(usize, usize) -> u8>) -> (Vec<u8>, Vec<u16>) {
    let xtrans = p.cfa.len() == 36;
    let blocks = p.width.div_ceil(BLOCK_WIDTH);
    let lines = p.height / 6;
    let mut out = Vec::new();
    out.extend(0x4953u16.to_be_bytes());
    out.push(u8::from(q_bases.is_none()));
    out.push(if xtrans { 16 } else { 0 });
    out.push(p.bits as u8);
    out.extend((p.height as u16).to_be_bytes());
    out.extend(((blocks * BLOCK_WIDTH) as u16).to_be_bytes());
    out.extend((p.width as u16).to_be_bytes());
    out.extend((BLOCK_WIDTH as u16).to_be_bytes());
    out.push(blocks as u8);
    out.extend((lines as u16).to_be_bytes());

    let line_step = lines.next_multiple_of(16);
    let mut table = vec![0u8; (4 * blocks).next_multiple_of(16)];
    let mut q_table = Vec::new();
    let mut streams = Vec::new();
    let mut decoded = vec![0u16; p.width * p.height];
    for b in 0..blocks {
        let width = (p.width - b * BLOCK_WIDTH).min(BLOCK_WIDTH);
        let bases: Option<Vec<u8>> = q_bases.map(|f| (0..lines).map(|l| f(b, l)).collect());
        if let Some(bases) = &bases {
            let mut padded = bases.clone();
            padded.resize(line_step, 0);
            q_table.extend(padded);
        }
        let mut strip = Strip::new(p, xtrans, bases);
        for line in 0..lines {
            strip.line(p, b, width, line, &mut decoded);
        }
        let bytes = strip.bits.finish();
        table[4 * b..4 * b + 4].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
        streams.push(bytes);
    }
    out.extend(table);
    out.extend(q_table);
    for s in streams {
        out.extend(s);
    }
    (out, decoded)
}

fn log2ceil(val: i32) -> u32 {
    if val <= 0 {
        0
    } else {
        32 - (val - 1).leading_zeros()
    }
}

struct QTable {
    table: Vec<i8>,
    q_base: i32,
    max_grad: i32,
    grad_mult: i32,
    total_values: i32,
    raw_bits: u32,
}

fn lookup(qp: [i32; 5]) -> Vec<i8> {
    (-qp[4]..=qp[4])
        .map(|v| {
            if v <= -qp[3] {
                -4
            } else if v <= -qp[2] {
                -3
            } else if v <= -qp[1] {
                -2
            } else if v < -qp[0] {
                -1
            } else if v <= qp[0] {
                0
            } else if v < qp[1] {
                1
            } else if v < qp[2] {
                2
            } else if v < qp[3] {
                3
            } else {
                4
            }
        })
        .collect()
}

fn main_qtable(q_base: i32, max: i32) -> QTable {
    let mut qp = [
        q_base,
        3 * q_base + 0x12,
        5 * q_base + 0x43,
        7 * q_base + 0x114,
        max,
    ];
    if qp[1] > max || qp[1] < q_base + 1 {
        qp[1] = q_base + 1;
    }
    if qp[2] < qp[1] || qp[2] > max {
        qp[2] = qp[1];
    }
    if qp[3] < qp[2] || qp[3] > max {
        qp[3] = qp[2];
    }
    let total_values = (max + 2 * q_base) / (2 * q_base + 1) + 1;
    QTable {
        table: lookup(qp),
        q_base,
        max_grad: 0,
        grad_mult: 9,
        total_values,
        raw_bits: log2ceil(total_values),
    }
}

fn lossy_qtable(k: i32, max: i32) -> QTable {
    let t = [
        [0x12, 0x43, 0x114],
        [0x15, 0x48, 0x11B],
        [0x18, 0x4D, 0x122],
    ][k as usize];
    let q1 = if max >= t[0] { t[0] } else { k + 1 };
    let q2 = if max >= t[1] { t[1] } else { q1 };
    let q3 = if max >= t[2] { t[2] } else { q2 };
    let total_values = (max + 2 * k) / (2 * k + 1) + 1;
    QTable {
        table: lookup([k, q1, q2, q3, max]),
        q_base: k,
        max_grad: 5 + k,
        grad_mult: 3,
        total_values,
        raw_bits: log2ceil(total_values),
    }
}

#[derive(Clone, Copy, Default)]
struct Grad(i32, i32);

#[derive(Clone, Copy)]
struct Grads {
    main: [Grad; 41],
    lossy: [[Grad; 5]; 3],
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    have: u32,
}

impl BitWriter {
    fn put(&mut self, value: u32, n: u32) {
        for i in (0..n).rev() {
            self.acc = self.acc << 1 | (u64::from(value) >> i & 1);
            self.have += 1;
            if self.have == 8 {
                self.bytes.push(self.acc as u8);
                self.acc = 0;
                self.have = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.have > 0 {
            self.put(0, 8 - self.have);
        }
        self.bytes
    }
}

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

/// 0 sample, 1 interpolate, 2 sample off multiples of four, 3 interpolate
/// two past multiples of four.
type Pass = ([usize; 2], [u8; 2], usize);
const XTRANS_PASSES: [Pass; 6] = [
    ([R2, G2], [1, 0], 0),
    ([G3, B2], [0, 1], 1),
    ([R3, G4], [2, 1], 2),
    ([G5, B3], [0, 3], 0),
    ([R4, G6], [3, 0], 1),
    ([G7, B4], [1, 2], 2),
];
const BAYER_PASSES: [Pass; 6] = [
    ([R2, G2], [0, 0], 0),
    ([G3, B2], [0, 0], 1),
    ([R3, G4], [0, 0], 2),
    ([G5, B3], [0, 0], 0),
    ([R4, G6], [0, 0], 1),
    ([G7, B4], [0, 0], 2),
];

fn group(line: usize) -> (usize, usize) {
    match line {
        0..5 => (R2, R4),
        5..13 => (G2, G7),
        _ => (B2, B4),
    }
}

struct Strip {
    xtrans: bool,
    lw: usize,
    max: i32,
    max_bits: i32,
    buf: Vec<u16>,
    /// The target sample at each line position, when a photosite maps there.
    target: Vec<Option<u16>>,
    main: QTable,
    lossy: Vec<QTable>,
    even: [Grads; 3],
    odd: [Grads; 3],
    bases: Option<Vec<u8>>,
    bits: BitWriter,
}

impl Strip {
    fn new(p: &Plane, xtrans: bool, bases: Option<Vec<u8>>) -> Self {
        let max = (1i32 << p.bits) - 1;
        let lw = if xtrans {
            BLOCK_WIDTH * 2 / 3
        } else {
            BLOCK_WIDTH / 2
        };
        let lossy: Vec<QTable> = if bases.is_some() {
            (0..3).map(|k| lossy_qtable(k, max)).collect()
        } else {
            Vec::new()
        };
        let zero = Grads {
            main: [Grad::default(); 41],
            lossy: [[Grad::default(); 5]; 3],
        };
        let mut s = Self {
            xtrans,
            lw,
            max,
            max_bits: 4 * log2ceil(max + 1) as i32,
            buf: vec![0; LINES * (lw + 2)],
            target: vec![None; LINES * (lw + 2)],
            main: main_qtable(0, max),
            lossy,
            even: [zero; 3],
            odd: [zero; 3],
            bases,
            bits: BitWriter::default(),
        };
        if s.bases.is_none() {
            s.reset_main();
        } else {
            for k in 0..3 {
                let d = ((s.lossy[k].total_values + 0x20) >> 6).max(2);
                for g in s.even.iter_mut().chain(s.odd.iter_mut()) {
                    g.lossy[k] = [Grad(d, 1); 5];
                }
            }
        }
        s
    }

    fn reset_main(&mut self) {
        let d = ((self.main.total_values + 0x20) >> 6).max(2);
        for g in self.even.iter_mut().chain(self.odd.iter_mut()) {
            g.main = [Grad(d, 1); 41];
        }
    }

    fn at(&self, line: usize, pos: usize) -> usize {
        line * (self.lw + 2) + 1 + pos
    }

    fn index(&self, x: usize) -> usize {
        if self.xtrans {
            (((x * 2 / 3) & !1) | ((x % 3) & 1)) + ((x % 3) >> 1)
        } else {
            x >> 1
        }
    }

    fn line_of(cfa: &[u8], row: usize, x: usize) -> usize {
        let side = if cfa.len() == 36 { 6 } else { 2 };
        match cfa[(row % side) * side + x % side] {
            0 => R2 + row / 2,
            2 => B2 + row / 2,
            _ => G2 + row,
        }
    }

    fn line(&mut self, p: &Plane, block: usize, width: usize, line: usize, decoded: &mut [u16]) {
        if let Some(bases) = &self.bases {
            let q = i32::from(bases[line]);
            if line == 0 || q != self.main.q_base {
                self.main = main_qtable(q, self.max);
                self.reset_main();
            }
        }
        self.target.fill(None);
        for row in 0..6 {
            for x in 0..width {
                let i = self.at(Self::line_of(p.cfa, row, x), self.index(x));
                let y = line * 6 + row;
                self.target[i] = Some(p.samples[y * p.width + block * BLOCK_WIDTH + x]);
            }
        }
        let passes = if self.xtrans {
            &XTRANS_PASSES
        } else {
            &BAYER_PASSES
        };
        for &(lines, rules, set) in passes {
            let (mut even, mut odd) = (0, 1);
            while even < self.lw || odd < self.lw {
                if even < self.lw {
                    for (&l, &rule) in lines.iter().zip(&rules) {
                        let sample = match rule {
                            0 => true,
                            1 => false,
                            2 => even & 3 != 0,
                            _ => even & 3 != 2,
                        };
                        if sample {
                            self.sample(l, even, false, set);
                        } else {
                            self.interpolate(l, even);
                        }
                    }
                    even += 2;
                }
                if even > 8 {
                    for &l in &lines {
                        self.sample(l, odd, true, set);
                    }
                    odd += 2;
                }
            }
            for &l in &lines {
                self.extend(l);
            }
        }
        let size = self.lw + 2;
        for (to, from) in [(0, R3), (1, R4), (5, G6), (6, G7), (13, B3), (14, B4)] {
            self.buf
                .copy_within(from * size..(from + 1) * size, to * size);
        }
        for row in 0..6 {
            for x in 0..width {
                let i = self.at(Self::line_of(p.cfa, row, x), self.index(x));
                decoded[(line * 6 + row) * p.width + block * BLOCK_WIDTH + x] = self.buf[i];
            }
        }
        for first in [R2, G2, B2] {
            let (_, last) = group(first);
            self.buf[first * size..(last + 1) * size].fill(0);
            let prev = (first - 1) * size;
            self.buf[first * size] = self.buf[prev + 1];
            self.buf[first * size + self.lw + 1] = self.buf[prev + self.lw];
        }
    }

    fn extend(&mut self, line: usize) {
        let (first, last) = group(line);
        for l in first..=last {
            let (cur, prev) = (self.at(l, 0) - 1, self.at(l - 1, 0) - 1);
            self.buf[cur] = self.buf[prev + 1];
            self.buf[cur + self.lw + 1] = self.buf[prev + self.lw];
        }
    }

    fn interpolate(&mut self, line: usize, pos: usize) {
        let (i, s) = (self.at(line, pos), self.lw + 2);
        let v = |j: usize| i32::from(self.buf[j]);
        let sum = even_prediction(v(i - s), v(i - s - 1), v(i - s + 1), v(i - 2 * s));
        self.buf[i] = (sum >> 2) as u16;
    }

    fn sample(&mut self, line: usize, pos: usize, odd: bool, set: usize) {
        let (i, s) = (self.at(line, pos), self.lw + 2);
        let v = |j: usize| i32::from(self.buf[j]);
        let (rb, rc, rd) = (v(i - s), v(i - s - 1), v(i - s + 1));
        let (diff, g1, g2, predicted) = if odd {
            let (ra, rg) = (v(i - 1), v(i + 1));
            let pr = if (rb > rc && rb > rd) || (rb < rc && rb < rd) {
                (rg + ra + 2 * rb) >> 2
            } else {
                (ra + rg) >> 1
            };
            ((rb - rc).abs() + (rc - ra).abs(), rb - rc, rc - ra, pr)
        } else {
            let rf = v(i - 2 * s);
            let pr = even_prediction(rb, rc, rd, rf) >> 2;
            ((rf - rb).abs() + (rc - rb).abs(), rb - rf, rc - rb, pr)
        };
        let mut which = None;
        for k in 0..self.lossy.len() {
            if self.main.q_base <= k as i32 {
                break;
            }
            if diff <= self.lossy[k].max_grad {
                which = Some(k);
                break;
            }
        }
        let qt = which.map_or(&self.main, |k| &self.lossy[k]);
        let max = self.max;
        let grad = qt.grad_mult * i32::from(qt.table[(max + g1) as usize])
            + i32::from(qt.table[(max + g2) as usize]);
        let sign = if grad < 0 { -1 } else { 1 };
        let step = 2 * qt.q_base + 1;
        let span = qt.total_values * step;
        let decode = |c: i32| {
            let mut v = predicted + sign * c * step;
            if v < -qt.q_base {
                v += span;
            } else if v > qt.q_base + max {
                v -= span;
            }
            v.clamp(0, max)
        };
        let target = self.target[i].map_or(predicted.clamp(0, max), i32::from);
        let near = ((sign * (target - predicted)) as f64 / step as f64).round() as i32;
        let to_code = |c: i32| if c >= 0 { 2 * c } else { -2 * c - 1 };
        let limit = 1i64 << qt.raw_bits;
        let c = [near, near - qt.total_values, near + qt.total_values]
            .into_iter()
            .filter(|&c| i64::from(to_code(c)) <= limit)
            .min_by_key(|&c| ((decode(c) - target).abs(), to_code(c)))
            .expect("an encodable code");
        let code = to_code(c);
        let raw_bits = qt.raw_bits;
        let reconstructed = decode(c);
        let grads = if odd {
            &mut self.odd[set]
        } else {
            &mut self.even[set]
        };
        let g = match which {
            Some(k) => &mut grads.lossy[k][grad.unsigned_abs() as usize],
            None => &mut grads.main[grad.unsigned_abs() as usize],
        };
        let n = bit_diff(g.0, g.1);
        let escape = self.max_bits - raw_bits as i32 - 1;
        if (code >> n) < escape {
            self.bits.put(0, (code >> n) as u32);
            self.bits.put(1, 1);
            self.bits.put(code as u32 & ((1u32 << n) - 1), n);
        } else {
            self.bits.put(0, escape as u32);
            self.bits.put(1, 1);
            self.bits.put((code - 1) as u32, raw_bits);
        }
        g.0 += c.abs();
        if g.1 == 0x40 {
            g.0 >>= 1;
            g.1 >>= 1;
        }
        g.1 += 1;
        self.buf[i] = reconstructed as u16;
    }
}
