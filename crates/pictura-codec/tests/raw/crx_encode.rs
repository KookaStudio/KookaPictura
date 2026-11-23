//! A CRX encoder for the CR3 tests: the line coder mirrors the decoder's
//! adaptive state (prediction, runs, Golomb-Rice parameter) and writes the
//! codes it would read, so a written plane must decode exactly. Wavelet
//! frames use a textbook reversible 5/3 transform with symmetric edges.

const JS: [usize; 32] = [
    1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 8, 8, 8, 8, 0x10, 0x10, 0x20, 0x20, 0x40, 0x40, 0x80, 0x80,
    0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000,
];
const J: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13,
    14, 15,
];

#[derive(Default)]
pub struct BitWriter {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitWriter {
    pub fn put(&mut self, v: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.acc = self.acc << 1 | (v.checked_shr(i).unwrap_or(0) & 1) as u64;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }

    /// A Golomb-Rice code: `code >> k` zeros, a one, then `k` low bits, or
    /// `escape` zeros, a one and `escape_bits` raw bits.
    pub fn code(&mut self, code: u32, k: i32, escape: u32, escape_bits: u32) {
        let zeros = code >> k;
        if zeros >= escape {
            self.put(0, escape);
            self.put(1, 1);
            self.put(code, escape_bits);
        } else {
            self.put(0, zeros);
            self.put(1, 1);
            self.put(code & ((1 << k) - 1), k as u32);
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            let n = self.n;
            self.put(0, 8 - n);
        }
        self.out
    }
}

fn zigzag(v: i32) -> u32 {
    if v >= 0 {
        2 * v as u32
    } else {
        (-2 * v - 1) as u32
    }
}

fn predict_k(prev: i32, code: i32, max: i32) -> i32 {
    let k = prev - (code < (1 << prev >> 1)) as i32
        + ((code >> prev) > 2) as i32
        + ((code >> prev) > 5) as i32;
    if max == 0 || k < max {
        k
    } else {
        max
    }
}

fn median(top_left: i32, top: i32, left: i32) -> i32 {
    let delta = top - top_left;
    let at =
        (((top_left < left) ^ (delta < 0)) as usize) << 1 | ((left < top) ^ (delta < 0)) as usize;
    [delta + left, delta + left, left, top][at]
}

/// One subband's coder state, the decoder's mirror.
pub struct Lines {
    w: usize,
    partial: bool,
    line: usize,
    s: usize,
    k: i32,
    buf: Vec<i32>,
    ks: Vec<i32>,
    pub bits: BitWriter,
}

impl Lines {
    pub fn new(w: usize, partial: bool) -> Self {
        Self {
            w,
            partial,
            line: 0,
            s: 0,
            k: 0,
            buf: vec![0; 2 * w + 4],
            ks: vec![0; w],
            bits: BitWriter::default(),
        }
    }

    /// Writes a run of `n` (at most `length`) as the decoder reads it.
    fn run(&mut self, n: usize, length: usize) {
        if n == 0 {
            self.bits.put(0, 1);
            return;
        }
        self.bits.put(1, 1);
        let mut cur = 1;
        while cur + JS[self.s] <= n {
            self.bits.put(1, 1);
            cur += JS[self.s];
            if self.s < 31 {
                self.s += 1;
            }
            if cur == length {
                return;
            }
        }
        self.bits.put(0, 1);
        if J[self.s] != 0 {
            self.bits.put((n - cur) as u32, J[self.s]);
        }
        if self.s > 0 {
            self.s -= 1;
        }
    }

    fn sym(&mut self, value: i32, pred: i32) -> u32 {
        let code = zigzag(value - pred);
        self.bits.code(code, self.k, 41, 21);
        code
    }

    /// Codes `values` (`w` of them) as the next line.
    pub fn line(&mut self, values: &[i32]) {
        let stride = self.w + 2;
        let (p0, p1) = if self.line & 1 == 1 {
            (stride, 0)
        } else {
            (0, stride)
        };
        if self.line == 0 {
            self.s = 0;
            self.k = 0;
            if self.partial {
                self.top(stride, values);
            } else {
                self.top_no_ref(0, stride, values);
            }
        } else if self.partial {
            self.median_line(p0, p1, values);
        } else {
            self.no_ref(p0, p1, values);
        }
        self.line += 1;
    }

    fn equal_run(values: &[i32], from: usize, v: i32, cap: usize) -> usize {
        values[from..]
            .iter()
            .take(cap)
            .take_while(|&&x| x == v)
            .count()
    }

    fn top(&mut self, mut p1: usize, values: &[i32]) {
        let base = p1;
        self.buf[p1] = 0;
        let mut length = self.w;
        while length > 1 {
            let pred = if self.buf[p1] != 0 {
                self.buf[p1]
            } else {
                let n = Self::equal_run(values, p1 - base, 0, length);
                self.run(n, length);
                length -= n;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length == 0 {
                    break;
                }
                0
            };
            let v = values[p1 - base];
            let code = self.sym(v, pred);
            self.buf[p1 + 1] = v;
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
            length -= 1;
        }
        if length == 1 {
            let v = values[p1 - base];
            let code = self.sym(v, self.buf[p1]);
            self.buf[p1 + 1] = v;
            self.k = predict_k(self.k, code as i32, 15);
            p1 += 1;
        }
        self.buf[p1 + 1] = self.buf[p1] + 1;
    }

    fn sym_l1(
        &mut self,
        p0: &mut usize,
        p1: &mut usize,
        base: usize,
        values: &[i32],
        predict: bool,
        ahead: bool,
    ) {
        let b = &self.buf;
        let pred = if predict {
            median(b[*p0], b[*p0 + 1], b[*p1])
        } else {
            b[*p0 + 1]
        };
        let v = values[*p1 - base];
        let mut code = self.sym(v, pred);
        self.buf[*p1 + 1] = v;
        if ahead {
            let next = (self.buf[*p0 + 2] - self.buf[*p0 + 1]) << 1;
            code = (code + next.unsigned_abs()) >> 1;
            *p0 += 1;
        }
        self.k = predict_k(self.k, code as i32, 15);
        *p1 += 1;
    }

    fn median_line(&mut self, mut p0: usize, mut p1: usize, values: &[i32]) {
        let base = p1;
        self.buf[p1] = self.buf[p0 + 1];
        let mut length = self.w;
        while length > 1 {
            let b = &self.buf;
            if b[p1] != b[p0 + 1] || b[p1] != b[p0 + 2] {
                self.sym_l1(&mut p0, &mut p1, base, values, true, true);
            } else {
                let n = Self::equal_run(values, p1 - base, self.buf[p1], length);
                self.run(n, length);
                length -= n;
                p0 += n;
                for _ in 0..n {
                    self.buf[p1 + 1] = self.buf[p1];
                    p1 += 1;
                }
                if length > 0 {
                    self.sym_l1(&mut p0, &mut p1, base, values, false, length > 1);
                }
            }
            length = length.saturating_sub(1);
            if length == 0 {
                break;
            }
        }
        if length == 1 {
            self.sym_l1(&mut p0, &mut p1, base, values, true, false);
        }
        self.buf[p1 + 1] = self.buf[p1] + 1;
    }

    /// The code a value after a run of zeros takes (the decoder adds one).
    fn after_zeros(&mut self, v: i32) -> u32 {
        assert_ne!(v, 0, "a run of zeros stops at a nonzero value");
        let code = zigzag(v) - 1;
        self.bits.code(code, self.k, 41, 21);
        code
    }

    fn top_no_ref(&mut self, p0: usize, mut p1: usize, values: &[i32]) {
        let base = p1;
        self.buf[p0] = 0;
        self.buf[p1] = 0;
        let mut p2 = 0;
        let mut length = self.w;
        while length > 1 {
            if self.buf[p1] != 0 {
                let v = values[p1 - base];
                let code = self.sym(v, 0);
                self.buf[p1 + 1] = v;
                self.k = predict_k(self.k, code as i32, 15);
            } else {
                let n = Self::equal_run(values, p1 - base, 0, length);
                self.run(n, length);
                length -= n;
                for _ in 0..n {
                    self.ks[p2] = 0;
                    self.buf[p1 + 1] = 0;
                    p1 += 1;
                    p2 += 1;
                }
                if length == 0 {
                    break;
                }
                let v = values[p1 - base];
                let code = self.after_zeros(v);
                self.buf[p1 + 1] = v;
                self.k = predict_k(self.k, code as i32, 15);
            }
            self.ks[p2] = self.k;
            p2 += 1;
            p1 += 1;
            length -= 1;
        }
        if length == 1 {
            let v = values[p1 - base];
            let code = self.sym(v, 0);
            self.buf[p1 + 1] = v;
            self.k = predict_k(self.k, code as i32, 15);
            self.ks[p2] = self.k;
            p1 += 1;
        }
        self.buf[p1 + 1] = 0;
    }

    fn bump(&mut self, i: usize) {
        if self.ks[i + 1] - self.k <= 1 {
            if self.k >= 15 {
                self.k = 15;
            }
        } else {
            self.k += 1;
        }
    }

    fn no_ref(&mut self, p0: usize, p1: usize, values: &[i32]) {
        let w = self.w;
        let mut i = 0;
        while i + 1 < w {
            let b = &self.buf;
            if b[p0 + i + 2] | b[p0 + i + 1] | b[p1 + i] != 0 {
                let code = self.sym(values[i], 0);
                self.buf[p1 + i + 1] = values[i];
                self.k = predict_k(self.k, code as i32, 0);
                self.bump(i);
            } else {
                let n = Self::equal_run(values, i, 0, w - i);
                self.run(n, w - i);
                for j in 0..n {
                    self.buf[p1 + i + 1 + j] = 0;
                    self.ks[i + j] = 0;
                }
                i += n;
                if i + 1 >= w {
                    if i + 1 == w {
                        let code = self.after_zeros(values[i]);
                        self.buf[p1 + i + 1] = values[i];
                        self.k = predict_k(self.k, code as i32, 15);
                        self.ks[i] = self.k;
                    }
                    i += 1;
                    continue;
                }
                let code = self.after_zeros(values[i]);
                self.buf[p1 + i + 1] = values[i];
                self.k = predict_k(self.k, code as i32, 0);
                self.bump(i);
            }
            self.ks[i] = self.k;
            i += 1;
        }
        if i + 1 == w {
            let code = self.sym(values[i], 0);
            self.buf[p1 + i + 1] = values[i];
            self.k = predict_k(self.k, code as i32, 15);
            self.ks[i] = self.k;
        }
    }
}

/// A row-major 2D array.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub v: Vec<i32>,
}

impl Grid {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            v: vec![0; w * h],
        }
    }

    fn at(&self, x: usize, y: usize) -> i32 {
        self.v[y * self.w + x]
    }

    pub fn map(&self, f: impl Fn(i32) -> i32) -> Self {
        Self {
            w: self.w,
            h: self.h,
            v: self.v.iter().map(|&x| f(x)).collect(),
        }
    }
}

/// The textbook forward 5/3 lifting of one line with symmetric edges.
pub fn forward1(x: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let n = x.len();
    let at = |i: isize| {
        let i = if i < 0 {
            -i
        } else if i as usize >= n {
            2 * (n as isize - 1) - i
        } else {
            i
        };
        x[i as usize]
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

/// Its inverse.
pub fn inverse1(low: &[i32], high: &[i32]) -> Vec<i32> {
    let n = low.len() + high.len();
    let h = |k: isize| high[k.clamp(0, high.len() as isize - 1) as usize];
    let mut x = vec![0; n];
    for k in 0..low.len() {
        x[2 * k] = low[k] - ((h(k as isize - 1) + h(k as isize) + 2) >> 2);
    }
    for k in 0..high.len() {
        let next = if 2 * k + 2 < n {
            x[2 * k + 2]
        } else {
            x[2 * k]
        };
        x[2 * k + 1] = high[k] + ((x[2 * k] + next) >> 1);
    }
    x
}

fn split_cols(g: &Grid, f: impl Fn(&[i32]) -> (Vec<i32>, Vec<i32>)) -> (Grid, Grid) {
    let (lw, hw) = (g.w.div_ceil(2), g.w / 2);
    let (mut l, mut h) = (Grid::new(lw, g.h), Grid::new(hw, g.h));
    for y in 0..g.h {
        let (a, b) = f(&g.v[y * g.w..][..g.w]);
        l.v[y * lw..][..lw].copy_from_slice(&a);
        h.v[y * hw..][..hw].copy_from_slice(&b);
    }
    (l, h)
}

fn transpose(g: &Grid) -> Grid {
    let mut t = Grid::new(g.h, g.w);
    for y in 0..g.h {
        for x in 0..g.w {
            t.v[x * g.h + y] = g.at(x, y);
        }
    }
    t
}

/// The bands of `levels` 2D levels, in the file's order: the low band, then
/// (high-low, low-high, high-high) per level, coarsest first. Vertical
/// lifting first, so the decoder's horizontal-then-vertical inverse undoes it.
pub fn forward(g: &Grid, levels: usize) -> Vec<Grid> {
    let mut per_level = Vec::new();
    let mut cur = g.clone();
    for _ in 0..levels {
        let (lo, hi) = split_cols(&transpose(&cur), forward1);
        let (lo, hi) = (transpose(&lo), transpose(&hi));
        let (ll, hl) = split_cols(&lo, forward1);
        let (lh, hh) = split_cols(&hi, forward1);
        per_level.push([hl, lh, hh]);
        cur = ll;
    }
    let mut out = vec![cur];
    for bands in per_level.into_iter().rev() {
        out.extend(bands);
    }
    out
}

fn join_cols(l: &Grid, h: &Grid) -> Grid {
    let w = l.w + h.w;
    let mut g = Grid::new(w, l.h);
    for y in 0..l.h {
        let row = inverse1(&l.v[y * l.w..][..l.w], &h.v[y * h.w..][..h.w]);
        g.v[y * w..][..w].copy_from_slice(&row);
    }
    g
}

/// The textbook inverse of `forward`.
pub fn inverse(bands: &[Grid]) -> Grid {
    let mut cur = bands[0].clone();
    for b in bands[1..].chunks(3) {
        let lo = join_cols(&cur, &b[0]);
        let hi = join_cols(&b[1], &b[2]);
        cur = transpose(&join_cols(&transpose(&lo), &transpose(&hi)));
    }
    cur
}

/// How a frame is coded.
#[derive(Clone)]
pub struct Frame {
    pub version: u16,
    pub bits: u8,
    pub cfa: u8,
    pub encoding: u8,
    pub levels: usize,
    /// The tile, in sensor photosites (twice the plane's).
    pub tile: (usize, usize),
    /// The plane header's flag: the low band predicted from the line above.
    pub partial: bool,
    /// Version 1 band quantizer (4 is a unit step).
    pub q: u8,
    /// Version 2: a constant quantizer table with this step base and
    /// multiplier.
    pub qp: Option<(i32, u16)>,
    /// Encoding 3's median precision, in the extended header.
    pub median_bits: Option<u8>,
}

impl Frame {
    pub fn lossless(tile: (usize, usize)) -> Self {
        Self {
            version: 0x100,
            bits: 14,
            cfa: 0,
            encoding: 0,
            levels: 0,
            tile,
            partial: true,
            q: 4,
            qp: None,
            median_bits: None,
        }
    }
}

/// The `CMP1` box content for a `w × h` sensor frame.
pub fn cmp1(f: &Frame, w: usize, h: usize, header_size: usize) -> Vec<u8> {
    let mut c = vec![0u8; if f.median_bits.is_some() { 85 } else { 52 }];
    c[4..6].copy_from_slice(&f.version.to_be_bytes());
    c[8..12].copy_from_slice(&(w as u32).to_be_bytes());
    c[12..16].copy_from_slice(&(h as u32).to_be_bytes());
    c[16..20].copy_from_slice(&(f.tile.0 as u32).to_be_bytes());
    c[20..24].copy_from_slice(&(f.tile.1 as u32).to_be_bytes());
    c[24] = f.bits;
    c[25] = 4 << 4 | f.cfa;
    c[26] = f.encoding << 4 | f.levels as u8;
    c[27] = ((w > f.tile.0) as u8) << 7 | ((h > f.tile.1) as u8) << 6;
    c[28..32].copy_from_slice(&(header_size as u32).to_be_bytes());
    if let Some(m) = f.median_bits {
        c[32] = 0x80;
        c[56] = 0x40;
        c[84] = m;
    }
    c
}

fn band_header(f: &Frame, n: usize, size: usize) -> Vec<u8> {
    let mut b = Vec::new();
    if f.version == 0x200 {
        let (base, mult) = f.qp.unwrap_or((0, 8));
        b.extend(0xFF13u16.to_be_bytes());
        b.extend(16u16.to_be_bytes());
        b.extend((size as u32).to_be_bytes());
        b.extend(((n as u16) << 12).to_be_bytes());
        b.extend(mult.to_be_bytes());
        b.extend(base.to_be_bytes());
        b.extend([0, 0, 0, 0]);
    } else {
        b.extend(0xFF03u16.to_be_bytes());
        b.extend(8u16.to_be_bytes());
        b.extend((size as u32).to_be_bytes());
        b.extend(((n as u32) << 28 | (f.q as u32) << 19).to_be_bytes());
    }
    b
}

/// The coded bytes of one band.
pub fn band_bytes(g: &Grid, partial: bool) -> Vec<u8> {
    let mut lines = Lines::new(g.w, partial);
    for y in 0..g.h {
        lines.line(&g.v[y * g.w..][..g.w]);
    }
    lines.bits.finish()
}

/// A constant quantizer table: every value 4 (a unit step), all codes zero.
fn qp_bytes(tile_w: usize, tile_h: usize) -> Vec<u8> {
    let mut bits = BitWriter::default();
    for _ in 0..tile_w.div_ceil(8) * tile_h.div_ceil(2) {
        bits.put(1, 1);
    }
    bits.finish()
}

/// One frame of the four `planes` (each `pw × ph`, values as the decoder's
/// lines see them: relative to half the range for encoding 0): its `CMP1`
/// box and its sample.
pub fn frame(f: &Frame, planes: &[Grid; 4]) -> (Vec<u8>, Vec<u8>) {
    let (pw, ph) = (planes[0].w, planes[0].h);
    let (tw, th) = (f.tile.0 / 2, f.tile.1 / 2);
    let (cols, rows) = (pw.div_ceil(tw), ph.div_ceil(th));
    assert!(
        f.levels == 0 || cols * rows == 1,
        "wavelet frames are one tile"
    );
    let mut header = Vec::new();
    let mut data = Vec::new();
    for t in 0..cols * rows {
        let (c, r) = (t % cols, t / cols);
        let (x0, y0) = (c * tw, r * th);
        let (w, h) = (tw.min(pw - x0), th.min(ph - y0));
        let mut tile_data = Vec::new();
        let mut plane_headers = Vec::new();
        if f.qp.is_some() {
            tile_data.extend(qp_bytes(w, h));
        }
        let qp_size = tile_data.len();
        for (p, plane) in planes.iter().enumerate() {
            let mut tile = Grid::new(w, h);
            for y in 0..h {
                tile.v[y * w..][..w].copy_from_slice(&plane.v[(y0 + y) * pw + x0..][..w]);
            }
            let bands = if f.levels == 0 {
                vec![tile]
            } else {
                forward(&tile, f.levels)
            };
            let mut plane_data = Vec::new();
            let mut band_headers = Vec::new();
            for (n, band) in bands.iter().enumerate() {
                let bytes = band_bytes(band, f.partial && n == 0);
                band_headers.extend(band_header(f, n, bytes.len()));
                plane_data.extend(bytes);
            }
            plane_headers.extend(0xFF02u16.to_be_bytes());
            plane_headers.extend(8u16.to_be_bytes());
            plane_headers.extend((plane_data.len() as u32).to_be_bytes());
            plane_headers.extend([(p as u8) << 4 | (f.partial as u8) << 3, 0, 0, 0]);
            plane_headers.extend(band_headers);
            tile_data.extend(plane_data);
        }
        if f.qp.is_some() {
            header.extend(0xFF11u16.to_be_bytes());
            header.extend(16u16.to_be_bytes());
            header.extend((tile_data.len() as u32).to_be_bytes());
            header.extend((t as u16).to_be_bytes());
            header.extend(0x4000u16.to_be_bytes());
            header.extend((qp_size as u32).to_be_bytes());
            header.extend([0, 0, 0, 0]);
        } else {
            header.extend(0xFF01u16.to_be_bytes());
            header.extend(8u16.to_be_bytes());
            header.extend((tile_data.len() as u32).to_be_bytes());
            header.extend((t as u16).to_be_bytes());
            header.extend([0, 0]);
        }
        header.extend(plane_headers);
        data.extend(tile_data);
    }
    let cmp1 = cmp1(f, 2 * pw, 2 * ph, header.len());
    header.extend(data);
    (cmp1, header)
}
