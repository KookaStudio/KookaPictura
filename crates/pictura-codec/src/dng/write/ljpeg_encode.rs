//! Lossless JPEG encoding (ITU T.81 process 14, `SOF3`) for DNG compression
//! 7: 16-bit precision, predictor 1, point transform 0, no restarts, and one
//! Huffman table fitted to each frame's difference categories.

/// Difference categories 0–16, plus one reserved slot so no code is all ones
/// (T.81 Annex K.2).
const SYMBOLS: usize = 17;
const RESERVED: usize = SYMBOLS;

struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, len: u32) {
        if len == 0 {
            return;
        }
        self.acc = (self.acc << len) | (value as u64 & ((1u64 << len) - 1));
        self.n += len;
        while self.n >= 8 {
            self.n -= 8;
            let byte = (self.acc >> self.n) as u8;
            self.out.push(byte);
            if byte == 0xFF {
                self.out.push(0);
            }
        }
    }

    fn flush(&mut self) {
        if self.n > 0 {
            let pad = 8 - self.n;
            self.put((1 << pad) - 1, pad);
        }
    }
}

/// Code lengths per symbol (T.81 Annex K.2, as libjpeg builds them), limited
/// to 16 bits.
fn code_lengths(freq: &[u64; SYMBOLS]) -> ([u8; 16], Vec<u8>) {
    let mut freq: Vec<u64> = freq.iter().copied().chain([1]).collect();
    let mut size = [0usize; SYMBOLS + 1];
    let mut others = [usize::MAX; SYMBOLS + 1];
    loop {
        let smallest = |skip: usize, freq: &[u64]| {
            (0..=RESERVED)
                .filter(|&i| i != skip && freq[i] > 0)
                .min_by_key(|&i| (freq[i], usize::MAX - i))
        };
        let Some(c1) = smallest(usize::MAX, &freq) else {
            break;
        };
        let Some(c2) = smallest(c1, &freq) else {
            break;
        };
        freq[c1] += freq[c2];
        freq[c2] = 0;
        let mut c = c1;
        size[c] += 1;
        while others[c] != usize::MAX {
            c = others[c];
            size[c] += 1;
        }
        others[c] = c2;
        let mut c = c2;
        size[c] += 1;
        while others[c] != usize::MAX {
            c = others[c];
            size[c] += 1;
        }
    }
    let mut bits = [0usize; 33];
    for &s in &size {
        if s > 0 {
            bits[s] += 1;
        }
    }
    for i in (17..=32).rev() {
        while bits[i] > 0 {
            let mut j = i - 2;
            while bits[j] == 0 {
                j -= 1;
            }
            bits[i] -= 2;
            bits[i - 1] += 1;
            bits[j + 1] += 2;
            bits[j] -= 1;
        }
    }
    let mut longest = 16;
    while bits[longest] == 0 {
        longest -= 1;
    }
    bits[longest] -= 1;
    let mut counts = [0u8; 16];
    for (len, count) in counts.iter_mut().enumerate() {
        *count = bits[len + 1] as u8;
    }
    // Symbols in order of their unlimited size, which the limiting kept
    // monotone; the reserved slot sorts last and is dropped.
    let mut symbols: Vec<usize> = (0..SYMBOLS).filter(|&s| size[s] > 0).collect();
    symbols.sort_by_key(|&s| (size[s], s));
    (counts, symbols.into_iter().map(|s| s as u8).collect())
}

fn segment(out: &mut Vec<u8>, marker: u8, body: &[u8]) {
    out.extend([0xFF, marker]);
    out.extend(((body.len() + 2) as u16).to_be_bytes());
    out.extend(body);
}

fn category(d: i32) -> u32 {
    match d {
        0 => 0,
        32768 => 16,
        _ => 32 - d.unsigned_abs().leading_zeros(),
    }
}

/// Encodes `height` rows of `width × components` interleaved 16-bit samples.
pub(super) fn encode(samples: &[u16], width: usize, height: usize, components: usize) -> Vec<u8> {
    debug_assert_eq!(samples.len(), width * height * components);
    let stride = width * components;
    let mut diffs = Vec::with_capacity(samples.len());
    let mut freq = [0u64; SYMBOLS];
    for y in 0..height {
        for x in 0..width {
            for c in 0..components {
                let i = y * stride + x * components + c;
                let pred = match (x, y) {
                    (0, 0) => 1 << 15,
                    (0, _) => samples[i - stride] as i32,
                    _ => samples[i - components] as i32,
                };
                let mut d = (samples[i] as i32 - pred).rem_euclid(65536);
                if d > 32768 {
                    d -= 65536;
                }
                freq[category(d) as usize] += 1;
                diffs.push(d);
            }
        }
    }
    let (counts, symbols) = code_lengths(&freq);
    let mut codes = [(0u32, 0u32); SYMBOLS];
    let mut code = 0u32;
    let mut k = 0;
    for (len, &n) in counts.iter().enumerate() {
        for _ in 0..n {
            codes[symbols[k] as usize] = (code, len as u32 + 1);
            code += 1;
            k += 1;
        }
        code <<= 1;
    }

    let mut out = vec![0xFF, 0xD8];
    let mut dht = vec![0x00];
    dht.extend(counts);
    dht.extend(&symbols);
    segment(&mut out, 0xC4, &dht);
    let mut sof = vec![16];
    sof.extend((height as u16).to_be_bytes());
    sof.extend((width as u16).to_be_bytes());
    sof.push(components as u8);
    for c in 0..components {
        sof.extend([c as u8 + 1, 0x11, 0]);
    }
    segment(&mut out, 0xC3, &sof);
    let mut sos = vec![components as u8];
    for c in 0..components {
        sos.extend([c as u8 + 1, 0x00]);
    }
    sos.extend([1, 0, 0]);
    segment(&mut out, 0xDA, &sos);

    let mut bits = Bits { out, acc: 0, n: 0 };
    for d in diffs {
        let s = category(d);
        let (code, len) = codes[s as usize];
        bits.put(code, len);
        if (1..16).contains(&s) {
            let extra = if d > 0 { d } else { d + (1 << s) - 1 };
            bits.put(extra as u32, s);
        }
    }
    bits.flush();
    let mut out = bits.out;
    out.extend([0xFF, 0xD9]);
    out
}
