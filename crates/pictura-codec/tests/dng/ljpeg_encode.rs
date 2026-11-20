//! A lossless JPEG encoder written from ITU T.81 (process 14, `SOF3`) for the
//! DNG reader tests. Point transform 0; one Huffman table whose codes run
//! from 2 to 14 bits, so the decoder's lookup table and its slow path both
//! decode.

/// Code length of each difference category 0–16.
const LENGTHS: [u8; 17] = [2, 3, 3, 3, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

fn codes() -> [(u16, u8); 17] {
    let mut out = [(0u16, 0u8); 17];
    let mut code = 0u16;
    for len in 1..=16u8 {
        for (sym, &l) in LENGTHS.iter().enumerate() {
            if l == len {
                out[sym] = (code, len);
                code += 1;
            }
        }
        code <<= 1;
    }
    out
}

struct Writer {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Writer {
    fn put(&mut self, value: u32, len: u32) {
        for i in (0..len).rev() {
            self.acc = (self.acc << 1) | ((value >> i) & 1);
            self.n += 1;
            if self.n == 8 {
                let byte = self.acc as u8;
                self.out.push(byte);
                if byte == 0xFF {
                    self.out.push(0);
                }
                self.acc = 0;
                self.n = 0;
            }
        }
    }

    fn flush(&mut self) {
        while self.n != 0 {
            self.put(1, 1);
        }
    }
}

fn segment(out: &mut Vec<u8>, marker: u8, body: &[u8]) {
    out.extend([0xFF, marker]);
    out.extend(((body.len() + 2) as u16).to_be_bytes());
    out.extend(body);
}

/// Encodes `height` rows of `width × components` interleaved samples.
pub fn encode(
    samples: &[u16],
    width: usize,
    height: usize,
    components: usize,
    precision: u8,
    predictor: u8,
    restart: usize,
) -> Vec<u8> {
    assert_eq!(samples.len(), width * height * components);
    let mut out = vec![0xFF, 0xD8];
    let mut dht = vec![0x00];
    for len in 1..=16 {
        dht.push(LENGTHS.iter().filter(|&&l| l == len).count() as u8);
    }
    dht.extend(0u8..17);
    segment(&mut out, 0xC4, &dht);
    let mut sof = vec![precision];
    sof.extend((height as u16).to_be_bytes());
    sof.extend((width as u16).to_be_bytes());
    sof.push(components as u8);
    for c in 0..components {
        sof.extend([c as u8 + 1, 0x11, 0]);
    }
    segment(&mut out, 0xC3, &sof);
    if restart > 0 {
        segment(&mut out, 0xDD, &(restart as u16).to_be_bytes());
    }
    let mut sos = vec![components as u8];
    for c in 0..components {
        sos.extend([c as u8 + 1, 0x00]);
    }
    sos.extend([predictor, 0, 0]);
    segment(&mut out, 0xDA, &sos);

    let codes = codes();
    let mut w = Writer { out, acc: 0, n: 0 };
    let stride = width * components;
    let at = |x: usize, y: usize, c: usize| samples[y * stride + x * components + c] as i32;
    let mut interval_start_row = 0usize;
    let mut interval_start_col = 0usize;
    let mut markers = 0u8;
    for y in 0..height {
        for x in 0..width {
            let pixel = y * width + x;
            if restart > 0 && pixel > 0 && pixel.is_multiple_of(restart) {
                w.flush();
                w.out.extend([0xFF, 0xD0 + markers % 8]);
                markers += 1;
                interval_start_row = y;
                interval_start_col = x;
            }
            for c in 0..components {
                let first_line = y == interval_start_row;
                let pred = if y == interval_start_row && x == interval_start_col {
                    1 << (precision - 1)
                } else if first_line {
                    at(x - 1, y, c)
                } else if x == 0 {
                    at(x, y - 1, c)
                } else {
                    let (a, b, cc) = (at(x - 1, y, c), at(x, y - 1, c), at(x - 1, y - 1, c));
                    match predictor {
                        1 => a,
                        2 => b,
                        3 => cc,
                        4 => a + b - cc,
                        5 => a + ((b - cc) >> 1),
                        6 => b + ((a - cc) >> 1),
                        _ => (a + b) >> 1,
                    }
                };
                let mut d = (at(x, y, c) - pred).rem_euclid(65536);
                if d > 32768 {
                    d -= 65536;
                }
                let s = if d == 0 {
                    0
                } else if d == 32768 {
                    16
                } else {
                    32 - d.unsigned_abs().leading_zeros()
                };
                let (code, len) = codes[s as usize];
                w.put(code as u32, len as u32);
                if (1..16).contains(&s) {
                    let extra = if d > 0 { d } else { d + (1 << s) - 1 };
                    w.put(extra as u32, s);
                }
            }
        }
    }
    w.flush();
    let mut out = w.out;
    out.extend([0xFF, 0xD9]);
    out
}
