//! In-tree approximate watermark (`FILT-105`).
//!
//! This is the project's own deterministic spread-spectrum carrier, **not**
//! Adobe's Digimarc Barcode and not compatible with it. Adobe's Embed Watermark
//! called a licensed third-party service whose algorithm is closed, so exact
//! parity is a non-goal (see `docs/06-filters/digimarc-watermark.md` and issue
//! #350). The scheme here writes a small, payload-carrying carrier into a
//! layer's colour planes and reads it back; it never claims Digimarc
//! compatibility.
//!
//! Payload: a 66-bit codeword — an 8-bit magic byte, a 2-bit durability field, a
//! 1-bit watermark type, a 23-bit creator ID, a 24-bit ID, and an 8-bit CRC-8.
//! The bits are repeated by the durability's repetition factor and tiled across
//! 8×8 cells; every pixel takes `amplitude * chip(x, y) * sign(bit)`, where the
//! chip is a deterministic ±1 hash of the pixel's `(x, y)`. Reading correlates
//! each cell against the same chip, mean-removed, and majority-votes the bits.

use pictura_core::PixelBuffer;

use crate::FilterError;

/// The watermark's type, mirroring CS6's Image ID / Transaction ID choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkKind {
    ImageId,
    TransactionId,
}

/// The CS6 durability levels. Higher durability is more redundant and slightly
/// more visible, matching CS6's note that a more durable watermark is more
/// visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    Low,
    Medium,
    High,
}

impl Durability {
    pub const ALL: [Durability; 3] = [Durability::Low, Durability::Medium, Durability::High];

    /// The `(repetition, amplitude)` the level maps to.
    pub const fn params(self) -> (u32, i32) {
        match self {
            Durability::Low => (1, 4),
            Durability::Medium => (2, 6),
            Durability::High => (4, 8),
        }
    }

    const fn code(self) -> u32 {
        match self {
            Durability::Low => 0,
            Durability::Medium => 1,
            Durability::High => 2,
        }
    }

    fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Durability::Low),
            1 => Some(Durability::Medium),
            2 => Some(Durability::High),
            _ => None,
        }
    }

    fn from_repetition(repetition: u32) -> Option<Self> {
        match repetition {
            1 => Some(Durability::Low),
            2 => Some(Durability::Medium),
            4 => Some(Durability::High),
            _ => None,
        }
    }
}

/// The largest creator ID the 23-bit field holds.
pub const MAX_CREATOR_ID: u32 = 8_388_607;
/// The largest ID the 24-bit field holds.
pub const MAX_ID: u32 = 16_777_215;

/// A watermark's decoded payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatermarkPayload {
    pub kind: WatermarkKind,
    pub creator_id: u32,
    pub id: u32,
}

const MAGIC: u32 = 0xA5;
const CELL: usize = 8;
const CODEWORD_BITS: usize = 66;
/// The codeword bits the CRC-8 covers (everything before the checksum).
const CRC_BITS: usize = CODEWORD_BITS - 8;

fn push_bits(out: &mut Vec<bool>, value: u32, count: u32) {
    for shift in (0..count).rev() {
        out.push((value >> shift) & 1 == 1);
    }
}

/// Bit-serial CRC-8 (polynomial `0x07`) over `bits`, MSB first.
fn crc8(bits: &[bool]) -> u8 {
    let mut crc = 0u8;
    for &bit in bits {
        let high = (crc >> 7) & 1;
        crc <<= 1;
        if u8::from(bit) ^ high != 0 {
            crc ^= 0x07;
        }
    }
    crc
}

fn kind_code(kind: WatermarkKind) -> u32 {
    match kind {
        WatermarkKind::ImageId => 0,
        WatermarkKind::TransactionId => 1,
    }
}

/// The 66-bit codeword for `payload` at `durability`, MSB first.
fn codeword(payload: &WatermarkPayload, durability: Durability) -> [bool; CODEWORD_BITS] {
    let mut bits = Vec::with_capacity(CRC_BITS);
    push_bits(&mut bits, MAGIC, 8);
    push_bits(&mut bits, durability.code(), 2);
    push_bits(&mut bits, kind_code(payload.kind), 1);
    push_bits(&mut bits, payload.creator_id & MAX_CREATOR_ID, 23);
    push_bits(&mut bits, payload.id & MAX_ID, 24);
    debug_assert_eq!(bits.len(), CRC_BITS);
    let crc = crc8(&bits);
    push_bits(&mut bits, u32::from(crc), 8);
    bits.try_into().expect("codeword is 66 bits")
}

struct BitReader<'a> {
    bits: &'a [bool],
    at: usize,
}

impl BitReader<'_> {
    fn read(&mut self, count: u32) -> u32 {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.bits[self.at]);
            self.at += 1;
        }
        value
    }
}

/// Decode a codeword, validating magic and CRC. Returns the payload and the
/// durability the codeword declares.
fn decode(bits: &[bool; CODEWORD_BITS]) -> Option<(WatermarkPayload, Durability)> {
    let mut reader = BitReader {
        bits: &bits[..],
        at: 0,
    };
    if reader.read(8) != MAGIC {
        return None;
    }
    let durability = Durability::from_code(reader.read(2))?;
    let kind = match reader.read(1) {
        0 => WatermarkKind::ImageId,
        _ => WatermarkKind::TransactionId,
    };
    let creator_id = reader.read(23);
    let id = reader.read(24);
    let checksum = reader.read(8) as u8;
    if crc8(&bits[..CRC_BITS]) != checksum {
        return None;
    }
    Some((
        WatermarkPayload {
            kind,
            creator_id,
            id,
        },
        durability,
    ))
}

/// A deterministic ±1 chip for pixel `(x, y)` (splitmix64 finalizer).
fn chip(x: usize, y: usize) -> i32 {
    let mut h = 0x9E37_79B9_7F4A_7C15u64
        ^ (x as u64).wrapping_mul(0x0000_0100_0000_01B3)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    if h & 1 == 0 {
        1
    } else {
        -1
    }
}

/// The colour planes the carrier touches: the first three for an RGB(A) buffer,
/// the first for a Grayscale(+alpha) buffer. The transparency plane is never
/// touched.
fn color_planes(channels: u8) -> usize {
    if channels >= 3 {
        3
    } else {
        1
    }
}

/// Embed `payload` into `buf`'s colour planes at `durability`, in place.
pub fn embed(
    buf: &mut PixelBuffer,
    payload: &WatermarkPayload,
    durability: Durability,
) -> Result<(), FilterError> {
    crate::validate(buf)?;
    let (repetition, amplitude) = durability.params();
    let message_len = CODEWORD_BITS * repetition as usize;
    let code = codeword(payload, durability);
    let width = buf.width as usize;
    let height = buf.height as usize;
    let cols = width.div_ceil(CELL);
    let planes = color_planes(buf.channels);
    let n = width * height;
    for y in 0..height {
        for x in 0..width {
            let cell = (y / CELL) * cols + (x / CELL);
            let bit = (cell % message_len) / repetition as usize;
            let sign = if code[bit] { 1 } else { -1 };
            let delta = amplitude * chip(x, y) * sign;
            for plane in 0..planes {
                let index = plane * n + y * width + x;
                let value = i32::from(buf.data[index]) + delta;
                buf.data[index] = value.clamp(0, 255) as u8;
            }
        }
    }
    Ok(())
}

/// Read a payload from `buf`, or `None` when no valid carrier is present.
pub fn read(buf: &PixelBuffer) -> Option<WatermarkPayload> {
    read_with_margin(buf).map(|(payload, _)| payload)
}

/// As [`read`], also returning the smallest per-bit absolute correlation (the
/// detection margin), which higher durabilities raise.
pub fn read_with_margin(buf: &PixelBuffer) -> Option<(WatermarkPayload, i64)> {
    let width = buf.width as usize;
    let height = buf.height as usize;
    if width == 0 || height == 0 || buf.data.len() < width * height * buf.channels as usize {
        return None;
    }
    let cols = width.div_ceil(CELL);
    let rows = height.div_ceil(CELL);
    let cells = cols * rows;
    let planes = color_planes(buf.channels);
    let n = width * height;

    // Lowest repetition first: a lower-durability carrier then decodes without a
    // scrambled higher-repetition trial. A durability whose message cannot fit
    // the image is skipped.
    for repetition in [1u32, 2, 4] {
        let message_len = CODEWORD_BITS * repetition as usize;
        if message_len > cells {
            continue;
        }
        let mut correlations = vec![0i64; message_len];
        for cy in 0..rows {
            for cx in 0..cols {
                let x0 = cx * CELL;
                let y0 = cy * CELL;
                let x1 = (x0 + CELL).min(width);
                let y1 = (y0 + CELL).min(height);
                let mut correlation = 0i64;
                for plane in 0..planes {
                    let base = plane * n;
                    let mut sum = 0i64;
                    let mut count = 0i64;
                    for y in y0..y1 {
                        for x in x0..x1 {
                            sum += i64::from(buf.data[base + y * width + x]);
                            count += 1;
                        }
                    }
                    let mean = sum / count;
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let residual = i64::from(buf.data[base + y * width + x]) - mean;
                            correlation += residual * i64::from(chip(x, y));
                        }
                    }
                }
                correlations[(cy * cols + cx) % message_len] += correlation;
            }
        }
        let mut margins = [0i64; CODEWORD_BITS];
        for (bit, margin) in margins.iter_mut().enumerate() {
            let mut total = 0i64;
            for copy in 0..repetition as usize {
                total += correlations[bit * repetition as usize + copy];
            }
            *margin = total;
        }
        let mut bits = [false; CODEWORD_BITS];
        for (bit, margin) in bits.iter_mut().zip(margins.iter()) {
            *bit = *margin > 0;
        }
        if let Some((payload, declared)) = decode(&bits) {
            if declared == Durability::from_repetition(repetition).expect("known repetition") {
                let margin = margins.iter().map(|m| m.abs()).min().unwrap_or(0);
                return Some((payload, margin));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic picture: a diagonal gradient with a little per-plane
    /// variation, so every cell has structure to correlate against.
    fn gradient(width: u32, height: u32) -> PixelBuffer {
        let mut buf = PixelBuffer::new(width, height, 4);
        let n = (width * height) as usize;
        for y in 0..height as usize {
            for x in 0..width as usize {
                for plane in 0..4usize {
                    buf.data[plane * n + y * width as usize + x] =
                        ((x * 3 + y * 5 + plane * 17) % 200 + 28) as u8;
                }
            }
        }
        buf
    }

    fn payload() -> WatermarkPayload {
        WatermarkPayload {
            kind: WatermarkKind::TransactionId,
            creator_id: 123_456,
            id: 7_654_321,
        }
    }

    #[test]
    fn watermark_round_trips_at_each_durability() {
        for durability in Durability::ALL {
            let mut buf = gradient(256, 256);
            embed(&mut buf, &payload(), durability).expect("embeds");
            assert_eq!(read(&buf), Some(payload()), "durability {durability:?}");
        }
    }

    #[test]
    fn corrupted_carrier_is_rejected() {
        let mut buf = gradient(256, 256);
        embed(&mut buf, &payload(), Durability::Medium).expect("embeds");
        // A flat field removes every cell's structure: the correlations vanish
        // and the codeword no longer validates.
        buf.data.as_mut_slice().fill(128);
        assert_eq!(read(&buf), None);
    }

    #[test]
    fn plain_image_has_no_watermark() {
        assert_eq!(read(&gradient(256, 256)), None);
    }

    #[test]
    fn carrier_is_deterministic_and_alpha_safe() {
        let before = gradient(128, 128);
        let mut first = before.clone();
        let mut second = before.clone();
        embed(&mut first, &payload(), Durability::High).expect("embeds");
        embed(&mut second, &payload(), Durability::High).expect("embeds");
        assert_eq!(first.data.as_slice(), second.data.as_slice());

        let amplitude = Durability::High.params().1;
        let n = 128 * 128;
        for index in 0..n {
            assert_eq!(
                first.data[3 * n + index],
                before.data[3 * n + index],
                "alpha changed at {index}"
            );
            for plane in 0..3 {
                let delta = (i32::from(first.data[plane * n + index])
                    - i32::from(before.data[plane * n + index]))
                .abs();
                assert!(delta <= amplitude, "delta {delta} exceeds {amplitude}");
            }
        }
    }

    #[test]
    fn higher_durability_has_a_larger_margin() {
        let mut low = gradient(320, 320);
        let mut high = low.clone();
        embed(&mut low, &payload(), Durability::Low).expect("embeds");
        embed(&mut high, &payload(), Durability::High).expect("embeds");
        let (_, low_margin) = read_with_margin(&low).expect("low reads");
        let (_, high_margin) = read_with_margin(&high).expect("high reads");
        assert!(
            high_margin > low_margin,
            "high margin {high_margin} is not greater than low margin {low_margin}"
        );
    }
}
