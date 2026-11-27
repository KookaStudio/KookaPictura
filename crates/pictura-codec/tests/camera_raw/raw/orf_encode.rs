//! Writes Olympus-compressed ORF streams for tests: the decoder's predictor
//! run forwards, choosing for each photosite the code that reproduces it.
//! Codes past the escape length use the escape whenever they fit it.

/// ImageProcessing `0x0640`-`0x0649` and `0x0650`-`0x0653` as every camera in
/// the raw.pixls.us sample set writes them.
pub const CAMERA: [u32; 14] = [0, 5, 0, 2, 5, 4, 3, 2, 3, 4, 7, 15, 12, 5];

/// Bits written most significant first.
#[derive(Default)]
struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.acc = self.acc << 1 | u64::from(value >> i & 1);
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.put(0, 8 - self.n);
        }
        self.out
    }
}

/// The stream, header included, that decodes to `samples` under `p`.
pub fn encode(samples: &[u16], width: usize, height: usize, p: &[u32; 14]) -> Vec<u8> {
    let [low_bits, edge, edge_value, fine_bits, fine_shift, small, run_min, run_t2, run_t1, t2, t1, max_bits, escape, header] =
        *p;
    let (edge, small, edge_value) = (1i32 << edge, 1i32 << (small & 0x1F), edge_value as i32);
    let escape = escape.max(1);
    let mut out = vec![0u8; header as usize - 1];
    out.extend([1, 0, 0]);
    let mut bits = Bits::default();

    let mut context = [0i32; 4];
    let (mut lpred, mut pred1, mut glc, mut gcode) = (0i32, 0i32, 0u32, 0u32);
    for row in 0..height {
        for col in 0..width {
            let prev = gcode as i32;
            gcode = glc;
            let psel = if col > 1 && small >= prev {
                context[3] + 1
            } else {
                0
            };
            context[3] = context[2];
            context[2] = psel;
            let wbits = if col > 1 {
                let (t1, t2) = if psel >= run_min as i32 {
                    (run_t1, run_t2)
                } else {
                    (t1, t2)
                };
                let length = 32 - ((prev & 0xFFFF) as u32).leading_zeros() as i32;
                t2 as i32 + (length - t1 as i32).max(-1) + 1
            } else {
                t2 as i32
            }
            .min(max_bits as i32) as u32;

            let stored = |r: usize, x: usize| i32::from(samples[r * width + x]) >> low_bits;
            let w = if col < 2 { edge_value } else { context[1] };
            let mut n = if row < 2 {
                edge_value
            } else {
                stored(row - 2, col)
            };
            let nw = if row < 2 || col < 2 {
                edge_value
            } else {
                stored(row - 2, col - 2)
            };
            if w < n || nw < w {
                if nw <= n && w >= n {
                    n = w;
                } else if nw < n || w >= n {
                    if nw > w || w >= n {
                        if (n - nw).abs() > edge {
                            n += w - nw;
                        } else if (w - nw).abs() <= edge {
                            n = (n + w) >> 1;
                        } else {
                            n = n + w - nw;
                        }
                    }
                } else {
                    n = w;
                }
            }

            let target = samples[row * width + col];
            let low = u32::from(target) & ((1 << low_bits) - 1);
            let g = (i32::from(target) >> low_bits) - n;
            let v = if col < 2 { 0 } else { pred1 };
            let (value, fine, vbits) = match fine_bits {
                0 => (g, 0, 0),
                1 => {
                    let fine = (g - 2 * v).rem_euclid(2);
                    ((g - 2 * v - fine) / 2, fine, v + (g >> 1))
                }
                _ => {
                    let fine = (g - 4 * v).rem_euclid(4);
                    let vbits = ((g >> 1) & !1) + v + (g >> 2);
                    ((g - 4 * v - fine) / 4, fine, vbits)
                }
            };
            let (negative, magnitude) = if value >= 0 {
                (0, value as u32)
            } else {
                (1, !value as u32)
            };
            let high = magnitude >> wbits;
            bits.put(negative, 1);
            bits.put(fine as u32, fine_bits);
            bits.put(low, low_bits);
            if high < escape {
                bits.put(0, high);
                bits.put(1, 1);
            } else {
                assert!(
                    wbits < 15 && high < 1 << (15 - wbits),
                    "{magnitude} does not fit the escape at {wbits} bits"
                );
                bits.put(0, escape);
                bits.put(high, 15 - wbits);
                bits.put(0, 1);
            }
            bits.put(magnitude & ((1 << wbits) - 1), wbits);

            glc = magnitude;
            pred1 = context[0];
            context[0] = if fine_shift == 15 {
                0
            } else {
                vbits >> fine_shift
            };
            context[1] = lpred;
            lpred = g + n;
        }
    }
    out.extend(bits.finish());
    out.extend([0; 8]);
    out
}
