//! CCITT Group 4 (ITU-T T.6, "MMR") encoder for 1-bit images, written as a
//! PDF `CCITTFaxDecode` stream with `K -1`. Run-length code tables are T.4's.

/// White terminating codes for runs 0–63.
const WHITE_TERM: [&str; 64] = [
    "00110101", "000111", "0111", "1000", "1011", "1100", "1110", "1111", "10011", "10100",
    "00111", "01000", "001000", "000011", "110100", "110101", "101010", "101011", "0100111",
    "0001100", "0001000", "0010111", "0000011", "0000100", "0101000", "0101011", "0010011",
    "0100100", "0011000", "00000010", "00000011", "00011010", "00011011", "00010010", "00010011",
    "00010100", "00010101", "00010110", "00010111", "00101000", "00101001", "00101010", "00101011",
    "00101100", "00101101", "00000100", "00000101", "00001010", "00001011", "01010010", "01010011",
    "01010100", "01010101", "00100100", "00100101", "01011000", "01011001", "01011010", "01011011",
    "01001010", "01001011", "00110010", "00110011", "00110100",
];

/// White make-up codes for runs 64, 128, … 1728.
const WHITE_MAKEUP: [&str; 27] = [
    "11011",
    "10010",
    "010111",
    "0110111",
    "00110110",
    "00110111",
    "01100100",
    "01100101",
    "01101000",
    "01100111",
    "011001100",
    "011001101",
    "011010010",
    "011010011",
    "011010100",
    "011010101",
    "011010110",
    "011010111",
    "011011000",
    "011011001",
    "011011010",
    "011011011",
    "010011000",
    "010011001",
    "010011010",
    "011000",
    "010011011",
];

/// Black terminating codes for runs 0–63.
const BLACK_TERM: [&str; 64] = [
    "0000110111",
    "010",
    "11",
    "10",
    "011",
    "0011",
    "0010",
    "00011",
    "000101",
    "000100",
    "0000100",
    "0000101",
    "0000111",
    "00000100",
    "00000111",
    "000011000",
    "0000010111",
    "0000011000",
    "0000001000",
    "00001100111",
    "00001101000",
    "00001101100",
    "00000110111",
    "00000101000",
    "00000010111",
    "00000011000",
    "000011001010",
    "000011001011",
    "000011001100",
    "000011001101",
    "000001101000",
    "000001101001",
    "000001101010",
    "000001101011",
    "000011010010",
    "000011010011",
    "000011010100",
    "000011010101",
    "000011010110",
    "000011010111",
    "000001101100",
    "000001101101",
    "000011011010",
    "000011011011",
    "000001010100",
    "000001010101",
    "000001010110",
    "000001010111",
    "000001100100",
    "000001100101",
    "000001010010",
    "000001010011",
    "000000100100",
    "000000110111",
    "000000111000",
    "000000100111",
    "000000101000",
    "000001011000",
    "000001011001",
    "000000101011",
    "000000101100",
    "000001011010",
    "000001100110",
    "000001100111",
];

/// Black make-up codes for runs 64, 128, … 1728.
const BLACK_MAKEUP: [&str; 27] = [
    "0000001111",
    "000011001000",
    "000011001001",
    "000001011011",
    "000000110011",
    "000000110100",
    "000000110101",
    "0000001101100",
    "0000001101101",
    "0000001001010",
    "0000001001011",
    "0000001001100",
    "0000001001101",
    "0000001110010",
    "0000001110011",
    "0000001110100",
    "0000001110101",
    "0000001110110",
    "0000001110111",
    "0000001010010",
    "0000001010011",
    "0000001010100",
    "0000001010101",
    "0000001011010",
    "0000001011011",
    "0000001100100",
    "0000001100101",
];

/// Extended make-up codes (both colours) for runs 1792, 1856, … 2560.
const EXTENDED_MAKEUP: [&str; 13] = [
    "00000001000",
    "00000001100",
    "00000001101",
    "000000010010",
    "000000010011",
    "000000010100",
    "000000010101",
    "000000010110",
    "000000010111",
    "000000011100",
    "000000011101",
    "000000011110",
    "000000011111",
];

/// Vertical mode codes for `a1 - b1` = -3 … 3.
const VERTICAL: [&str; 7] = ["0000010", "000010", "010", "1", "011", "000011", "0000011"];

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u8,
}

impl Bits {
    fn put(&mut self, code: &str) {
        for c in code.bytes() {
            if self.used == 0 {
                self.bytes.push(0);
            }
            if c == b'1' {
                *self.bytes.last_mut().expect("pushed") |= 0x80 >> self.used;
            }
            self.used = (self.used + 1) % 8;
        }
    }

    fn run(&mut self, mut run: usize, black: bool) {
        while run > 2560 {
            self.put(EXTENDED_MAKEUP[12]);
            run -= 2560;
        }
        if run >= 64 {
            let makeup = run / 64 * 64;
            if makeup <= 1728 {
                let table = if black { &BLACK_MAKEUP } else { &WHITE_MAKEUP };
                self.put(table[makeup / 64 - 1]);
            } else {
                self.put(EXTENDED_MAKEUP[(makeup - 1792) / 64]);
            }
            run -= makeup;
        }
        self.put(if black {
            BLACK_TERM[run]
        } else {
            WHITE_TERM[run]
        });
    }
}

/// The first changing element after `from` (`-1` = before the line): the
/// first `i > from` whose colour differs from pixel `i - 1` (white before the
/// line); `line.len()` when there is none.
fn next_change(line: &[bool], from: isize) -> usize {
    let start = (from + 1).max(0) as usize;
    (start..line.len())
        .find(|&i| line[i] != (i > 0 && line[i - 1]))
        .unwrap_or(line.len())
}

/// Encode rows of `width` pixels (`true` = black), `rows.len() / width` rows,
/// as a T.6 stream ending with EOFB.
pub(crate) fn encode_g4(pixels: &[bool], width: usize) -> Vec<u8> {
    let mut bits = Bits::default();
    let white = vec![false; width];
    let mut reference: &[bool] = &white;
    for line in pixels.chunks_exact(width.max(1)) {
        let mut a0: isize = -1;
        let mut black = false;
        while a0 < width as isize {
            let a1 = next_change(line, a0);
            let mut b1 = next_change(reference, a0);
            if b1 < width && reference[b1] == black {
                b1 = next_change(reference, b1 as isize);
            }
            let b2 = next_change(reference, b1 as isize);
            if b2 < a1 {
                bits.put("0001");
                a0 = b2 as isize;
            } else if (a1 as isize - b1 as isize).abs() <= 3 {
                bits.put(VERTICAL[(a1 as isize - b1 as isize + 3) as usize]);
                a0 = a1 as isize;
                black = !black;
            } else {
                let a2 = next_change(line, a1 as isize);
                bits.put("001");
                bits.run(a1 - a0.max(0) as usize, black);
                bits.run(a2 - a1, !black);
                a0 = a2 as isize;
            }
        }
        reference = line;
    }
    bits.put("000000000001000000000001");
    bits.bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_white_row_is_one_vertical_code() {
        // An all-white row: a1 = b1 = width, so V0; then EOFB.
        let out = encode_g4(&[false; 8], 8);
        assert_eq!(out[0] >> 7, 1, "V0 first");
        assert_eq!(out.len(), 4, "1 bit + 24 EOFB bits");
    }

    #[test]
    fn horizontal_mode_codes_both_runs() {
        // White 10 then black to the end, against a white reference: b1 is the
        // line end, too far for vertical mode.
        let mut row = [false; 20];
        row[10..].fill(true);
        let mut bits = Bits::default();
        bits.put("001");
        bits.run(10, false);
        bits.run(10, true);
        let out = encode_g4(&row, 20);
        assert_eq!(&out[..2], &bits.bytes[..2]);
    }

    #[test]
    fn long_runs_use_make_up_codes() {
        // 3000 = 2560 extended make-up + 384 make-up + 56 terminating.
        let mut bits = Bits::default();
        bits.run(3000, false);
        let mut manual = Bits::default();
        manual.put(EXTENDED_MAKEUP[12]);
        manual.put(WHITE_MAKEUP[5]);
        manual.put(WHITE_TERM[56]);
        assert_eq!(bits.bytes, manual.bytes);
    }
}
