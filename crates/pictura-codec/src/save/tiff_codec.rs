//! TIFF strip compressors that take their input a band at a time (#1320).
//! Each produces the bytes its one-shot form produces for the concatenated
//! input: PackBits decides a packet from at most 255 bytes ahead, so it holds
//! that much back until the input ends; LZW carries its table and bit buffer.

use std::collections::HashMap;

/// PackBits over a stream. A packet starting at `i` reads at most
/// `i + 128 + 127` bytes (a literal of up to 128 bytes, each checked for a run
/// of up to 128), so packets are only decided while that much is buffered.
#[derive(Default)]
pub(super) struct PackBits {
    pending: Vec<u8>,
}

const LOOKAHEAD: usize = 256;

impl PackBits {
    pub(super) fn push(&mut self, data: &[u8], out: &mut Vec<u8>) {
        self.pending.extend_from_slice(data);
        let used = encode(&self.pending, out, false);
        self.pending.drain(..used);
    }

    pub(super) fn finish(mut self, out: &mut Vec<u8>) {
        let used = encode(&self.pending, out, true);
        self.pending.drain(..used);
    }
}

/// Encode packets from the start of `data`; returns the bytes consumed. Unless
/// `last`, a packet starts only with [`LOOKAHEAD`] bytes available.
fn encode(data: &[u8], out: &mut Vec<u8>, last: bool) -> usize {
    let mut i = 0;
    while i < data.len() && (last || data.len() - i >= LOOKAHEAD) {
        let mut run = 1;
        while i + run < data.len() && data[i + run] == data[i] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            out.push((257 - run as u16) as u8);
            out.push(data[i]);
            i += run;
        } else {
            let start = i;
            i += 1;
            while i < data.len() && i - start < 128 {
                let mut next = 1;
                while i + next < data.len() && data[i + next] == data[i] && next < 128 {
                    next += 1;
                }
                if next >= 3 {
                    break;
                }
                i += 1;
            }
            let n = i - start;
            out.push((n - 1) as u8);
            out.extend_from_slice(&data[start..i]);
        }
    }
    i
}

/// TIFF LZW (MSB-first codes, 9–12 bits, a Clear at 4096 entries) over a
/// stream.
pub(super) struct Lzw {
    bit_buffer: u32,
    bit_count: u32,
    width: u32,
    table: HashMap<(u16, u8), u16>,
    next: u16,
    prefix: Option<u16>,
}

const CLEAR: u16 = 256;
const END: u16 = 257;

impl Lzw {
    pub(super) fn new(out: &mut Vec<u8>) -> Self {
        let mut lzw = Self {
            bit_buffer: 0,
            bit_count: 0,
            width: 9,
            table: HashMap::new(),
            next: 258,
            prefix: None,
        };
        lzw.emit(CLEAR, out);
        lzw
    }

    fn emit(&mut self, code: u16, out: &mut Vec<u8>) {
        self.bit_buffer |= u32::from(code) << (32 - self.width - self.bit_count);
        self.bit_count += self.width;
        while self.bit_count >= 8 {
            out.push((self.bit_buffer >> 24) as u8);
            self.bit_buffer <<= 8;
            self.bit_count -= 8;
        }
    }

    pub(super) fn push(&mut self, data: &[u8], out: &mut Vec<u8>) {
        for &b in data {
            let Some(prefix) = self.prefix else {
                self.prefix = Some(u16::from(b));
                continue;
            };
            if let Some(&code) = self.table.get(&(prefix, b)) {
                self.prefix = Some(code);
                continue;
            }
            self.emit(prefix, out);
            if self.next < 4096 {
                self.table.insert((prefix, b), self.next);
                self.next += 1;
                if self.next == (1u16 << self.width) && self.width < 12 {
                    self.width += 1;
                }
            } else {
                self.emit(CLEAR, out);
                self.table.clear();
                self.next = 258;
                self.width = 9;
            }
            self.prefix = Some(u16::from(b));
        }
    }

    pub(super) fn finish(mut self, out: &mut Vec<u8>) {
        if let Some(prefix) = self.prefix {
            self.emit(prefix, out);
        }
        self.emit(END, out);
        if self.bit_count > 0 {
            out.push((self.bit_buffer >> 24) as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| match i % 700 {
                0..=199 => 7,
                200..=459 => (i * 31 % 251) as u8,
                _ => (i / 3 % 2) as u8,
            })
            .collect()
    }

    #[test]
    fn streamed_packbits_equals_one_shot() {
        let data = sample(10_000);
        let whole = super::super::tiff::packbits(&data);
        for chunk in [1, 7, 129, 255, 256, 1000] {
            let mut stream = PackBits::default();
            let mut out = Vec::new();
            for piece in data.chunks(chunk) {
                stream.push(piece, &mut out);
            }
            stream.finish(&mut out);
            assert_eq!(out, whole, "chunks of {chunk}");
        }
    }

    #[test]
    fn streamed_lzw_equals_one_shot() {
        let data = sample(60_000);
        let whole = super::super::tiff::lzw(&data);
        for chunk in [1, 333, 4096] {
            let mut out = Vec::new();
            let mut stream = Lzw::new(&mut out);
            for piece in data.chunks(chunk) {
                stream.push(piece, &mut out);
            }
            stream.finish(&mut out);
            assert_eq!(out, whole, "chunks of {chunk}");
        }
        let mut out = Vec::new();
        Lzw::new(&mut out).finish(&mut out);
        assert_eq!(out, super::super::tiff::lzw(&[]));
    }
}
