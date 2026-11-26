//! The RW2 codings ported from rawspeed (`codec/vendor-raw-containers`,
//! #1541) against synthetic RW2s from `tests/camera_raw/raw/rw2_encode.rs`. Real files
//! run in `raw_rw2.rs`'s corpus test.

use pictura_codec::{read_raw, DngImage, ImageBudget, PsdError};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/rw2_builder.rs"]
mod rw2_builder;
#[path = "raw/rw2_encode.rs"]
mod rw2_encode;

use builder::noise_plane;
use rw2_builder::{data_start, rw2, Data, Rw2};

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

/// A seeded generator for the walks below.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
}

/// A step of `k` units of `scale`, up or down, kept inside `lo..=hi`.
fn step(
    rng: &mut Rng,
    from: i32,
    scale: i32,
    (k_lo, k_hi): (i32, i32),
    (lo, hi): (i32, i32),
) -> i32 {
    let k = rng.range(k_lo, k_hi);
    let up = from + k * scale;
    if up <= hi && (rng.next().is_multiple_of(2) || from - k * scale < lo) {
        up
    } else {
        from - k * scale
    }
}

/// Packets of 14 that RawFormat 4 codes exactly: each triple after the first
/// two photosites moves by steps of one shift's size.
fn predictive_plane(len: usize, seed: u64) -> Vec<u16> {
    let mut rng = Rng(seed);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len / 14 {
        let mut last = [rng.range(1000, 3000), rng.range(1000, 3000)];
        out.extend(last.map(|v| v as u16));
        for triple in 0..4 {
            // Each scale's steps are too large for the one below it.
            let (scale, k) = [
                (1, (0, 127)),
                (2, (64, 127)),
                (4, (64, 127)),
                (16, (32, 60)),
            ][rng.range(0, 3) as usize];
            for p in 0..3 {
                let c = (2 + triple * 3 + p) % 2;
                last[c] = step(&mut rng, last[c], scale, k, (600, 3500));
                out.push(last[c] as u16);
            }
        }
    }
    out
}

/// Packets RawFormat 6 codes exactly at `bits`.
fn paired_plane(len: usize, bits: u32, seed: u64) -> Vec<u16> {
    let mut rng = Rng(seed);
    let (per, range, top) = if bits == 14 {
        (11, (3000, 14000), 511)
    } else {
        (14, (700, 3500), 127)
    };
    let mut out = Vec::with_capacity(len);
    for _ in 0..len / per {
        let mut last = [rng.range(range.0, range.1), rng.range(range.0, range.1)];
        out.extend(last.map(|v| v as u16));
        for triple in 0..(per - 2) / 3 {
            let base = [0, 1, 2, 4][rng.range(0, 3) as usize];
            for p in 0..3 {
                let c = (2 + triple * 3 + p) % 2;
                last[c] = if base == 4 {
                    // Stored whole, in sixteens: value + 15 is a multiple.
                    16 * rng.range(range.0 / 16 + 1, range.1 / 16) - 15
                } else {
                    step(&mut rng, last[c], 1 << base, (top / 2, top), range)
                };
                out.push(last[c] as u16);
            }
        }
    }
    out
}

/// What the reader owes for `r`: the samples inside the borders, normalized,
/// with `pedestal` added to the black tags.
fn normalized(r: &Rw2, pedestal: u16) -> Vec<u16> {
    let pattern = [[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]][r.cfa as usize - 1];
    let [top, left, bottom, right] = r.borders.map(usize::from);
    let white = f64::from(r.white);
    let mut out = Vec::new();
    for y in top..bottom {
        for x in left..right {
            let black = f64::from(r.black[pattern[y % 2 * 2 + x % 2] as usize] + pedestal);
            let stored = f64::from(r.samples[y * r.width + x]);
            let v = ((stored - black) * 65535.0 / (white - black)).round();
            out.push(v.clamp(0.0, 65535.0) as u16);
        }
    }
    out
}

fn predictive(width: usize, height: usize, offset: bool) -> Rw2 {
    let mut r = Rw2::new(
        width,
        height,
        predictive_plane(width * height, 5),
        Data::Predictive,
    );
    r.raw_data_offset = offset;
    r.raw_format = Some(if offset { 4 } else { 3 });
    r
}

#[test]
fn predictive_rw2s_decode_exactly() {
    // 252 × 64 spans two blocks, the second one short.
    for offset in [true, false] {
        let mut r = predictive(252, 64, offset);
        r.black = [100, 110, 120];
        r.borders = [1, 3, 63, 250];
        let image = read(&rw2(&r)).unwrap();
        assert_eq!(image.samples, normalized(&r, 15), "RawDataOffset {offset}");
    }
}

#[test]
fn predictive_zeros_take_their_neighbours() {
    let (width, height) = (28, 6);
    let mut samples = vec![1000u16; width * height];
    // A packet starting at zero, and one whose even photosites are all zero
    // up to the last, which only then may be small.
    let first = 2 * width;
    samples[first] = 0;
    let second = 4 * width + 14;
    for p in (0..12).step_by(2) {
        samples[second + p] = 0;
    }
    samples[second + 12] = 5;
    let mut r = Rw2::new(width, height, samples, Data::Predictive);
    r.white = 2000;
    let image = read(&rw2(&r)).unwrap();
    let at = |i: usize| image.samples[i];
    let thousand = at(0);
    assert_eq!(at(first), thousand, "filled from four neighbours of 1000");
    assert_eq!(at(second + 12), 0, "5 is under the pedestal");
    for p in (0..12).step_by(2) {
        assert!(at(second + p) > 0, "photosite {p} filled");
    }
}

#[test]
fn blocked_rw2s_decode_exactly() {
    // Two blocks each: 10 × 1024 and 9 × 1024 photosites per block.
    for (bits, width) in [(12, 100), (14, 99)] {
        let max = (1 << bits) - 1;
        let samples = noise_plane(width, 120, max, 11).samples;
        let mut r = Rw2::new(width, 120, samples, Data::Blocked(bits));
        r.black = [128, 129, 130];
        let image = read(&rw2(&r)).unwrap();
        assert_eq!(image.samples, normalized(&r, 0), "{bits} bits");
    }
}

#[test]
fn paired_rw2s_decode_exactly() {
    for (bits, width) in [(12, 140), (14, 132)] {
        let mut r = Rw2::new(
            width,
            10,
            paired_plane(width * 10, bits, 17),
            Data::Paired(bits),
        );
        r.black = [500, 510, 520];
        r.cfa = 2;
        let image = read(&rw2(&r)).unwrap();
        assert_eq!(image.samples, normalized(&r, 0), "{bits} bits");
    }
}

#[test]
fn strip_rw2s_decode_exactly() {
    for (columns, rows) in [(1, 1), (2, 2), (3, 1)] {
        let samples = noise_plane(96, 20, u16::MAX, 29).samples;
        let mut r = Rw2::new(96, 20, samples, Data::Strips { columns, rows });
        r.black = [2048; 3];
        r.initial = [100, 7000, 300, 65000];
        r.borders = [2, 4, 18, 92];
        let image = read(&rw2(&r)).unwrap();
        assert_eq!(image.samples, normalized(&r, 0), "{columns} × {rows}");
    }
}

#[test]
fn strip_tags_are_checked() {
    let rect_mismatch = |r: &mut Rw2| r.width += 2;
    let odd = |r: &mut Rw2| {
        r.width = 95;
        r.samples.truncate(95 * r.height);
    };
    for (what, change) in [
        (
            "plane wider than the strips",
            &rect_mismatch as &dyn Fn(&mut Rw2),
        ),
        ("odd width", &odd),
    ] {
        let samples = noise_plane(96, 20, u16::MAX, 29).samples;
        let mut r = Rw2::new(
            96,
            20,
            samples,
            Data::Strips {
                columns: 2,
                rows: 1,
            },
        );
        let bytes_before = rw2(&r);
        change(&mut r);
        let mut bytes = bytes_before;
        // Patch SensorWidth (tag 2, the first entry) in place.
        bytes[10 + 8..10 + 10].copy_from_slice(&(r.width as u16).to_le_bytes());
        assert!(read(&bytes).is_err(), "{what}");
    }
}

#[test]
fn hostile_compressed_rw2s_fail_cleanly() {
    let small = [
        predictive(28, 4, true),
        predictive(28, 4, false),
        Rw2::new(
            20,
            4,
            noise_plane(20, 4, 4095, 3).samples,
            Data::Blocked(12),
        ),
        Rw2::new(28, 4, paired_plane(28 * 4, 12, 3), Data::Paired(12)),
        Rw2::new(22, 4, paired_plane(22 * 4, 14, 3), Data::Paired(14)),
        Rw2::new(
            8,
            4,
            noise_plane(8, 4, u16::MAX, 3).samples,
            Data::Strips {
                columns: 2,
                rows: 2,
            },
        ),
    ];
    for r in small {
        let bytes = rw2(&r);
        read(&bytes).unwrap_or_else(|e| panic!("{:?}: {e}", r.data));
        for len in 0..bytes.len() {
            assert!(read(&bytes[..len]).is_err(), "{:?} cut at {len}", r.data);
        }
        for at in 4..bytes.len() {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                broken[at] ^= flip;
                // Any answer but a panic.
                let _ = read(&broken);
            }
        }
        assert!(data_start(&r) < bytes.len());
    }
}
