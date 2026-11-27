//! Olympus-compressed ORF sensor data (#1535): one bit stream over the whole
//! plane, each photosite an adaptive Golomb-like code predicted from its
//! same-colour neighbours two columns left and two rows up, parameterized by
//! the maker note's ImageProcessing tags `0x0640`-`0x0653`.
//!
//! Ported to Rust from LibRaw 0.22.1, file `src/decoders/olympus14.cpp`
//! (<https://github.com/LibRaw/LibRaw>, commit
//! b860248a89d9082b8e0a1e202e516f46af9adb29). Not a clean-room
//! implementation: the owner chose an open, attributed port for this format.
//!
//! Copyright (C) 2024-2025 Alex Tutubalin, LibRaw LLC
//!
//! LibRaw is licensed under the GNU Lesser General Public License, version
//! 2.1, or the Common Development and Distribution License, version 1.0.
//! This port takes the LGPL-2.1 and, as its section 3 permits, is distributed
//! under the GNU General Public License, version 3 or later, like the rest of
//! Kooka Pictura. See `assets/PROVENANCE.md`.

use crate::error::PsdError;

/// The ImageProcessing tags `0x0640`-`0x0649` and `0x0650`-`0x0653`, in
/// that order; LibRaw names them only by number. 0 where a tag is absent.
pub(super) type Params = [u32; 14];

/// The tag numbers of `Params`, in order.
pub(super) const PARAM_TAGS: [u16; 14] = [
    0x0640, 0x0641, 0x0642, 0x0643, 0x0644, 0x0645, 0x0646, 0x0647, 0x0648, 0x0649, 0x0650, 0x0651,
    0x0652, 0x0653,
];

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("ORF compressed data: {what}"))
}

/// LibRaw's buffered bit pump: big-endian words read most significant bit
/// first. Reads past the end of the data yield zeros, where LibRaw's yield
/// stale buffer bytes.
struct Pump<'a> {
    data: &'a [u8],
    pos: usize,
    storage: u32,
    count: u32,
}

/// `x << n` as x86 shifts a 32-bit register: by `n` modulo 32. LibRaw relies
/// on it when all 32 stored bits are valid.
fn shl(x: u32, n: u32) -> u32 {
    x.wrapping_shl(n)
}

impl Pump<'_> {
    fn byte(&mut self) -> u32 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        u32::from(b)
    }

    fn replace_data(&mut self) {
        self.storage = (0..4).fold(0, |acc, _| acc << 8 | self.byte());
        self.count = 32;
    }

    fn add_data(&mut self) -> u32 {
        let word = self.byte() << 8 | self.byte();
        self.storage = self.storage << 16 | word;
        self.count += 16;
        self.count
    }

    fn skip_bit(&mut self, vbits: &mut u32) {
        if *vbits == 0 {
            self.replace_data();
            self.count -= 1;
            *vbits = self.count;
        } else {
            *vbits -= 1;
            self.count = *vbits;
        }
    }

    /// `bits` of 1 to 16.
    fn getbits(&mut self, bits: u32) -> u32 {
        while self.count < bits {
            self.add_data();
        }
        let ret = shl(self.storage, 32 - (self.count & 0x1F)) >> (32 - bits);
        self.count -= bits;
        ret
    }
}

/// The checked parameters, named after LibRaw's uses of them.
struct Coding {
    /// Low bits stored raw below each predicted value (`0x0640`).
    low_bits: u32,
    /// The gradient beyond which the predictor follows an edge (`1 << 0x0641`).
    edge: i32,
    /// The neighbour taken beyond the plane's first two rows or columns.
    edge_value: i32,
    /// Bits of the code's fine part (`0x0643`, 0 to 2).
    fine_bits: u32,
    /// `0x0644`; 15 turns the fine context off.
    fine_shift: u32,
    /// The code length up to which a run of small codes grows (`1 << 0x0645`).
    small: i32,
    run_min: i32,
    run_t2: i32,
    run_t1: i32,
    t2: i32,
    t1: i32,
    max_bits: u32,
    /// Zeros before an escape (`0x0652`, at least 1).
    escape: u32,
    /// Bytes of the stream's header (`0x0653`).
    header: usize,
}

impl Coding {
    fn new(p: &Params) -> Result<Self, PsdError> {
        let [low_bits, edge, edge_value, fine_bits, fine_shift, small, run_min, run_t2, run_t1, t2, t1, max_bits, escape, header] =
            *p;
        // LibRaw takes any values; these bounds keep every shift defined and
        // hold all known cameras' parameters.
        if low_bits > 16
            || edge > 30
            || fine_bits > 16
            || fine_shift > 15
            || small & 0x1F > 30
            || max_bits > 15
            || header == 0
        {
            return Err(PsdError::Unsupported(format!(
                "ORF compressed data with coding parameters {p:?}"
            )));
        }
        let int = |v: u32| v as i32;
        Ok(Self {
            low_bits,
            edge: 1 << edge,
            edge_value: int(edge_value),
            fine_bits,
            fine_shift,
            small: 1 << (small & 0x1F),
            run_min: int(run_min),
            run_t2: int(run_t2),
            run_t1: int(run_t1),
            t2: int(t2),
            t1: int(t1),
            max_bits,
            escape: escape.max(1),
            header: header as usize,
        })
    }
}

/// One code: the coded value, its magnitude `glc`, and the raw low and fine
/// bits read before it.
struct Code {
    value: i32,
    glc: u32,
    low: u32,
    fine: u32,
}

/// LibRaw's `oly_code`.
fn code(pump: &mut Pump, wbits: u32, c: &Coding) -> Code {
    if pump.count == 0 {
        pump.replace_data();
    }
    pump.count -= 1;
    let negative = (pump.storage >> pump.count) & 1;
    let fine = if c.fine_bits > 0 {
        pump.getbits(c.fine_bits)
    } else {
        0
    };
    let low = if c.low_bits > 0 {
        pump.getbits(c.low_bits)
    } else {
        0
    };

    let mut vbits = pump.count;
    let mut high = 0u32;
    loop {
        pump.skip_bit(&mut vbits);
        if (pump.storage >> vbits) & 1 != 0 {
            break;
        }
        high += 1;
        if high == c.escape {
            high = 0;
            if wbits != 15 {
                let i = 15 - wbits;
                while vbits < i {
                    vbits = pump.add_data();
                }
                high = shl(pump.storage, 32 - (vbits & 0x1F)) >> (wbits + 17);
                vbits -= i;
                pump.count = vbits;
            }
            pump.skip_bit(&mut vbits);
            break;
        }
    }

    let rest = if wbits == 0 {
        0
    } else {
        while vbits < wbits {
            vbits = pump.add_data();
        }
        let r = shl(pump.storage, 32 - (vbits & 0x1F)) >> (32 - wbits);
        pump.count = vbits - wbits;
        r
    };
    let glc = rest | high << wbits;
    Code {
        value: (glc ^ 0u32.wrapping_sub(negative)) as i32,
        glc,
        low,
        fine,
    }
}

/// The bit length of `v`'s low 16 bits.
fn bit_length(v: i32) -> i32 {
    32 - ((v & 0xFFFF) as u32).leading_zeros() as i32
}

/// Decodes the `width × height` plane from the `stored` bytes of its strip,
/// which starts `data`; reads run on past the strip to the end of the file,
/// as LibRaw's do.
pub(super) fn decode(
    data: &[u8],
    stored: usize,
    params: &Params,
    (width, height): (usize, usize),
) -> Result<Vec<u16>, PsdError> {
    let c = Coding::new(params)?;
    // Every code holds at least its sign, fine and low bits and a stop bit.
    let least = (2 + c.fine_bits + c.low_bits) as u64 * (width * height) as u64;
    if (stored as u64) * 8 < least {
        return Err(invalid("too short for the plane"));
    }
    // A big-endian 1 over the header's bytes, then two zero bytes.
    let header = data
        .get(..c.header + 2)
        .ok_or_else(|| invalid("missing header"))?;
    let one = header[..c.header]
        .iter()
        .fold(0u64, |acc, &b| acc.wrapping_shl(8) | u64::from(b));
    if one != 1 || header[c.header..] != [0, 0] {
        return Err(invalid("header"));
    }
    let mut pump = Pump {
        data,
        pos: c.header + 2,
        storage: 0,
        count: 0,
    };

    let mut plane = vec![0u16; width * height];
    // LibRaw's `context` (unsigned there, the same bits here) and the state
    // carried from photosite to photosite, and from row to row.
    let mut context = [0i32; 4];
    let (mut lpred, mut pred1, mut glc, mut gcode) = (0i32, 0i32, 0u32, 0u32);
    for row in 0..height {
        for col in 0..width {
            let p = gcode as i32;
            gcode = glc;
            let psel = if col > 1 && c.small >= p {
                context[3].wrapping_add(1)
            } else {
                0
            };
            context[3] = context[2];
            context[2] = psel;
            let mut wbits = if col > 1 {
                let (t1, t2) = if psel >= c.run_min {
                    (c.run_t1, c.run_t2)
                } else {
                    (c.t1, c.t2)
                };
                let tt = (bit_length(p) - t1).max(-1);
                t2.wrapping_add(tt).wrapping_add(1)
            } else {
                c.t2
            };
            if wbits > c.max_bits as i32 {
                wbits = c.max_bits as i32;
            }

            let coded = code(&mut pump, wbits as u32, &c);
            glc = coded.glc;
            let mut g = coded.value;
            let fine = coded.fine as i32;
            let vbits = if c.fine_bits == 0 {
                0
            } else {
                let v = if col < 2 { 0 } else { pred1 };
                if c.fine_bits == 1 {
                    g = fine.wrapping_add(v.wrapping_add(g).wrapping_mul(2));
                    v.wrapping_add(g >> 1)
                } else {
                    g = fine.wrapping_add(v.wrapping_add(g).wrapping_mul(4));
                    ((g >> 1) & !1).wrapping_add(v).wrapping_add(g >> 2)
                }
            };

            pred1 = context[0];
            context[0] = if c.fine_shift == 15 {
                0
            } else {
                vbits >> c.fine_shift
            };
            let stored = |r: usize, x: usize| i32::from(plane[r * width + x]) >> c.low_bits;
            let w = if col < 2 { c.edge_value } else { context[1] };
            let mut n = if row < 2 {
                c.edge_value
            } else {
                stored(row - 2, col)
            };
            let nw = if row < 2 || col < 2 {
                c.edge_value
            } else {
                stored(row - 2, col - 2)
            };
            context[1] = lpred;

            if w < n || nw < w {
                if nw <= n && w >= n {
                    n = w;
                } else if nw < n || w >= n {
                    if nw > w || w >= n {
                        if n.wrapping_sub(nw).wrapping_abs() > c.edge {
                            n = n.wrapping_add(w.wrapping_sub(nw));
                        } else if w.wrapping_sub(nw).wrapping_abs() <= c.edge {
                            n = n.wrapping_add(w) >> 1;
                        } else {
                            n = n.wrapping_add(w).wrapping_sub(nw);
                        }
                    }
                } else {
                    n = w;
                }
            }
            lpred = g.wrapping_add(n);
            plane[row * width + col] = coded.low.wrapping_add(shl(lpred as u32, c.low_bits)) as u16;
        }
    }
    Ok(plane)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAMERA: Params = [0, 5, 0, 2, 5, 4, 3, 2, 3, 4, 7, 15, 12, 5];

    fn stream(header: &[u8]) -> Vec<u8> {
        [header, &[0xA5; 64]].concat()
    }

    #[test]
    fn the_header_is_checked() {
        let plane = decode(&stream(&[0, 0, 0, 0, 1, 0, 0]), 71, &CAMERA, (4, 2));
        assert!(plane.is_ok());
        for header in [
            &[0, 0, 0, 0, 2, 0, 0][..],
            &[0, 0, 0, 0, 1, 0, 1],
            &[0, 0, 0],
        ] {
            assert!(matches!(
                decode(&stream(header), 67, &CAMERA, (4, 2)),
                Err(PsdError::Invalid(_))
            ));
        }
        assert!(decode(&[0, 0, 0, 0, 1, 0], 6, &CAMERA, (4, 2)).is_err());
        assert!(matches!(
            decode(&stream(&[0, 0, 0, 0, 1, 0, 0]), 3, &CAMERA, (4, 2)),
            Err(PsdError::Invalid(_))
        ));
    }

    #[test]
    fn out_of_range_parameters_are_refused() {
        for (i, v) in [
            (0, 17),
            (1, 31),
            (3, 17),
            (4, 16),
            (5, 31),
            (11, 16),
            (13, 0),
        ] {
            let mut p = CAMERA;
            p[i] = v;
            assert!(
                matches!(
                    decode(&stream(&[0, 0, 0, 0, 1, 0, 0]), 71, &p, (4, 2)),
                    Err(PsdError::Unsupported(_))
                ),
                "parameter {i} = {v}"
            );
        }
    }

    #[test]
    fn bit_lengths_match_libraws_table() {
        let mut table = vec![0; 65536];
        let mut n = 1;
        for i in 0..16 {
            for _ in 0..1 << i {
                table[n] = i + 1;
                n += 1;
            }
        }
        for v in [0, 1, 2, 3, 4, 255, 256, 65535, 65536, 65537, -1] {
            assert_eq!(bit_length(v), table[(v & 0xFFFF) as usize], "{v}");
        }
    }
}
