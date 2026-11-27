//! SuperCCD SR RAFs (#1648): the R frame merged into extended highlight
//! range (`codec/vendor-raw-containers`) on synthetic RAFs, and, opt-in, the
//! S3 Pro, S5 Pro and F700 sample files. The merge has no oracle (LibRaw
//! reads one frame at a time, Adobe's merge is closed), so these check the
//! specified rule.

use pictura_codec::{read_raw, DngImage, ImageBudget, PsdError};

#[allow(dead_code)]
#[path = "raw/raf_builder.rs"]
mod raf_builder;

use raf_builder::{primary_only, raf, Raf};

const WHITE: u16 = 15872;
const S_BLACK: u16 = 20;
const R_BLACK: u16 = 40;
const RANGE: f64 = (WHITE - S_BLACK) as f64;

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

/// A deterministic hash of photosite `i`.
fn hash(i: usize) -> usize {
    let mut x = (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (x ^ (x >> 32)) as usize
}

/// A SuperCCD layout whose plane is `width × height`, all of it the active
/// area, with LibRaw's `fw` for it.
struct Lattice {
    width: usize,
    height: usize,
    shift: bool,
}

impl Lattice {
    fn fw(&self) -> usize {
        if self.shift {
            self.width
        } else {
            self.width / 2
        }
    }

    fn grid_width(&self) -> usize {
        (self.height >> self.shift as u32) + self.fw()
    }

    /// LibRaw's grid cell of photosite `(row, col)`, if in the grid.
    fn cell(&self, row: usize, col: usize) -> Option<usize> {
        let fw = self.fw();
        let (r, c) = if self.shift {
            (fw - 1 - col + row / 2, col + row.div_ceil(2))
        } else {
            (fw - 1 + row - col / 2, row + col.div_ceil(2))
        };
        let w = self.grid_width();
        (r < w - 1 && c < w).then_some(r * w + c)
    }

    /// The grid colour (0 red, 1 green, 2 blue) of photosite `i`.
    fn colour(&self, i: usize) -> Option<usize> {
        let cell = self.cell(i / self.width, i % self.width)?;
        let w = self.grid_width();
        let pattern = if self.fw() % 2 == 1 {
            [0, 1, 1, 2]
        } else {
            [1, 2, 0, 1]
        };
        Some(pattern[(cell / w % 2) * 2 + cell % w % 2])
    }

    fn photosites(&self) -> usize {
        self.width * self.height
    }

    /// An SR RAF: S samples `s` and R samples `r`, R in each row's second
    /// half when `shift` is set (the F700), in a section of its own
    /// otherwise (the S3 Pro and S5 Pro); S black 20, R black 40.
    fn sr(&self, s: Vec<u16>, r: Vec<u16>) -> Raf {
        let mut f = Raf::words(self.width, self.height, s);
        f.crop = None;
        f.layout = if self.shift {
            [0x80, 0x81, 0x82, 0x83]
        } else {
            [0x00, 0x01, 0x02, 0x01]
        };
        let (h, w) = if self.shift {
            (self.height / 2, self.width * 2)
        } else {
            (self.height, self.width)
        };
        f.record(0x0121, &[h as u16, w as u16]);
        let mut second = f.extra.clone();
        f.record(0x4000, &[S_BLACK; 4]);
        second.push((
            0x4000,
            [R_BLACK; 4].iter().flat_map(|v| v.to_be_bytes()).collect(),
        ));
        f.second_extra = Some(second);
        f.second_frame = true;
        if self.shift {
            f.second_samples = Some(r);
        } else {
            f.second_section = Some(r);
        }
        f
    }
}

fn lattices() -> [Lattice; 2] {
    [
        Lattice {
            width: 96,
            height: 64,
            shift: false,
        },
        Lattice {
            width: 48,
            height: 128,
            shift: true,
        },
    ]
}

/// The merged read, the primary-only read, and the headroom between them.
fn reads(bytes: Vec<u8>) -> (DngImage, DngImage, f64) {
    let merged = read(&bytes).expect("merged");
    let primary = read(&primary_only(bytes)).expect("primary");
    let h = (merged.color.baseline_exposure - primary.color.baseline_exposure).exp2();
    (merged, primary, h)
}

/// `counts` above S black, normalized over the headroom `h`.
fn normal(counts: f64, h: f64) -> f64 {
    (counts * 65535.0 / RANGE / h).round().clamp(0.0, 65535.0)
}

fn near(got: u16, want: f64, what: &str) {
    assert!(
        (f64::from(got) - want).abs() <= 1.0,
        "{what}: {got} vs {want}"
    );
}

// raf_sr_unclipped
#[test]
fn sr_unclipped_samples_keep_the_primary_frame() {
    for l in lattices() {
        // Below 0.80 of the range, multiples of 16 so R is exact.
        let scene: Vec<u16> = (0..l.photosites())
            .map(|i| 16 * (40 + hash(i) % 740) as u16)
            .collect();
        let s = scene.iter().map(|&v| v + S_BLACK).collect();
        let r = scene.iter().map(|&v| v / 16 + R_BLACK).collect();
        let (merged, primary, h) = reads(raf(&l.sr(s, r)));
        assert_eq!(h, 1.0, "shift {}", l.shift);
        assert_eq!(merged.samples, primary.samples, "shift {}", l.shift);
    }
}

// raf_sr_clipped
#[test]
fn sr_clipped_samples_come_from_the_r_frame() {
    const RATIOS: [u16; 3] = [12, 16, 20];
    for l in lattices() {
        // Multiples of 240 (divisible by every ratio); half in the fit
        // band, the rest up to three times the range.
        let scene: Vec<u16> = (0..l.photosites())
            .map(|i| {
                let n = if hash(i).is_multiple_of(2) {
                    20 + hash(i + 7) % 27
                } else {
                    20 + hash(i + 7) % 180
                };
                240 * n as u16
            })
            .collect();
        let s = scene.iter().map(|&v| (v + S_BLACK).min(WHITE)).collect();
        // Every 17th clipped photosite has its R at black: the clip is
        // then the best estimate.
        let dark = |i: usize| hash(i).is_multiple_of(17) && f64::from(scene[i]) >= RANGE;
        let r = scene
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if dark(i) {
                    R_BLACK
                } else {
                    v / RATIOS[l.colour(i).unwrap_or(1)] + R_BLACK
                }
            })
            .collect();
        let want: Vec<f64> = (0..l.photosites())
            .map(|i| if dark(i) { RANGE } else { f64::from(scene[i]) })
            .collect();
        let (merged, _, h) = reads(raf(&l.sr(s, r)));
        let peak = (0..l.photosites())
            .filter(|&i| l.cell(i / l.width, i % l.width).is_some())
            .map(|i| want[i])
            .fold(0.0, f64::max);
        assert!((h - peak / RANGE).abs() < 1e-9, "headroom {h}");
        assert!(h > 2.0, "{h}");
        assert!((0..l.photosites()).any(dark));
        for (i, &v) in want.iter().enumerate() {
            if let Some(cell) = l.cell(i / l.width, i % l.width) {
                near(merged.samples[cell], normal(v, h), "scene");
            }
        }
    }
}

// raf_sr_blend
#[test]
fn sr_blend_is_continuous() {
    for l in lattices() {
        // Even photosites fit the ratio (R = S / 16); odd ones ramp through
        // the blend with R 4 % above.
        let n = l.photosites();
        let s_counts: Vec<f64> = (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    16.0 * (300 + hash(i) % 390) as f64
                } else {
                    (RANGE * (0.75 + 0.24 * i as f64 / n as f64)).round()
                }
            })
            .collect();
        let r_counts: Vec<f64> = s_counts
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                if i % 2 == 0 {
                    s / 16.0
                } else {
                    (1.04 * s / 16.0).round()
                }
            })
            .collect();
        let s = s_counts.iter().map(|&v| v as u16 + S_BLACK).collect();
        let r = r_counts.iter().map(|&v| v as u16 + R_BLACK).collect();
        let (merged, _, h) = reads(raf(&l.sr(s, r)));
        let merge = |s: f64, r: f64| {
            let t = ((s / RANGE - 0.80) / 0.15).clamp(0.0, 1.0);
            s + t * (16.0 * r - s)
        };
        let mut ramp = Vec::new();
        for i in 0..n {
            if let Some(cell) = l.cell(i / l.width, i % l.width) {
                let want = merge(s_counts[i], r_counts[i]);
                near(merged.samples[cell], normal(want, h), "blend");
                if i % 2 == 1 {
                    ramp.push((s_counts[i], merged.samples[cell]));
                }
            }
        }
        ramp.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            ramp.windows(2)
                .all(|w| u32::from(w[1].1) + 1 >= u32::from(w[0].1)),
            "monotone"
        );
        assert!(ramp.len() > 100);
    }
}

// raf_sr_nominal
#[test]
fn sr_too_few_samples_use_the_nominal_ratio() {
    let l = &lattices()[0];
    let r_counts: Vec<u16> = (0..l.photosites())
        .map(|i| 2000 + (hash(i) % 900) as u16)
        .collect();
    let peak = (0..l.photosites())
        .filter(|&i| l.cell(i / l.width, i % l.width).is_some())
        .map(|i| f64::from(r_counts[i]))
        .fold(0.0, f64::max);
    for (records, ratio) in [(Some((256u16, 32u16)), 8.0), (None, 16.0)] {
        let s = vec![WHITE; l.photosites()];
        let r = r_counts.iter().map(|&v| v + R_BLACK).collect();
        let mut f = l.sr(s, r);
        if let Some((primary, second)) = records {
            f.record(0x9200, &[primary, 0x0100]);
            if let Some(extra) = &mut f.second_extra {
                extra.push((
                    0x9200,
                    [second, 0x0100u16]
                        .iter()
                        .flat_map(|v| v.to_be_bytes())
                        .collect(),
                ));
            }
        }
        let (merged, _, h) = reads(raf(&f));
        assert!(
            (h - ratio * peak / RANGE).abs() < 1e-9,
            "ratio {ratio}: {h}"
        );
        for (i, &v) in r_counts.iter().enumerate() {
            if let Some(cell) = l.cell(i / l.width, i % l.width) {
                near(
                    merged.samples[cell],
                    normal(ratio * f64::from(v), h),
                    "nominal",
                );
            }
        }
    }
}

// raf_sr_missing_rows
#[test]
fn sr_missing_r_rows_keep_the_primary_frame() {
    let l = &lattices()[0];
    let last = l.height - 1;
    let scene: Vec<u16> = (0..l.photosites())
        .map(|i| {
            if i / l.width == last {
                3 * WHITE
            } else {
                16 * (300 + hash(i) % 900) as u16
            }
        })
        .collect();
    let s = scene.iter().map(|&v| (v + S_BLACK).min(WHITE)).collect();
    let r = scene.iter().map(|&v| v / 16 + R_BLACK).collect();
    let mut bytes = raf(&l.sr(s, r));
    bytes.truncate(bytes.len() - 2 * l.width);
    let (merged, primary, h) = reads(bytes);
    assert!(h > 1.0, "the other rows still reach above the S range");
    for col in 0..l.width {
        if let Some(cell) = l.cell(last, col) {
            near(
                merged.samples[cell],
                (f64::from(primary.samples[cell]) / h).round(),
                "row",
            );
        }
    }
}

// raf_sr_upright
#[test]
fn sr_merge_needs_a_superccd_r_frame() {
    let (width, height) = (40, 24);
    let samples: Vec<u16> = (0..width * height)
        .map(|i| (hash(i) % 15872) as u16)
        .collect();
    let mut paired = Raf::words(width, height, samples.clone());
    paired.second_samples = Some(vec![15000; width * height]);
    paired.second_frame = true;
    paired.layout = [0x80, 0x0B, 0x09, 0x0A];
    let bytes = raf(&paired);
    let image = read(&bytes).expect("Bayer with a second frame");
    let alone = read(&primary_only(bytes)).expect("Bayer alone");
    assert_eq!(image.samples, alone.samples);
    assert_eq!(image.color.baseline_exposure, alone.color.baseline_exposure);

    // An SR layout whose 0x80 is 0 reads like a file without a second frame.
    let l = &lattices()[0];
    let s: Vec<u16> = (0..l.photosites())
        .map(|i| (hash(i) % 15872) as u16)
        .collect();
    let mut f = l.sr(s, vec![8000; l.photosites()]);
    let with_r = read(&raf(&f)).expect("merged");
    f.second_section = None;
    let without = read(&raf(&f)).expect("0x80 is 0");
    f.second_frame = false;
    let single = read(&raf(&f)).expect("one frame");
    assert_eq!(without.samples, single.samples);
    assert_ne!(with_r.samples, single.samples);
}

// raf_short_header
#[test]
fn short_header_names_no_second_directory() {
    let l = Lattice {
        width: 16,
        height: 24,
        shift: true,
    };
    let s: Vec<u16> = (0..l.photosites()).map(|i| 100 + 7 * i as u16).collect();
    let mut f = l.sr(s, Vec::new());
    f.second_frame = false;
    f.second_samples = None;
    f.exif = None;
    let bytes = raf(&f);
    let want = read(&bytes).expect("one directory").samples;
    let mut short = bytes;
    // The camera JPEG (empty) starts at 0x6C, and 0x78 holds its bytes.
    short[0x54..0x58].copy_from_slice(&0x6Cu32.to_be_bytes());
    short[0x58..0x5C].fill(0);
    short[0x78..0x7C].copy_from_slice(&0x0000_0100u32.to_be_bytes());
    let image = read(&short).expect("short header");
    assert_eq!(image.samples, want);
}

// raf_sr_corpus
#[test]
fn sr_rafs_gain_highlight_range_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP SR raf corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.expect("corpus entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("raf")))
        .collect();
    paths.sort();
    let mut merged_files = 0;
    for path in paths {
        let name = path.display();
        let bytes = std::fs::read(&path).unwrap();
        let (Ok(merged), Ok(primary)) = (read(&bytes), read(&primary_only(bytes.clone()))) else {
            continue;
        };
        if merged.samples == primary.samples {
            continue;
        }
        let h = (merged.color.baseline_exposure - primary.color.baseline_exposure).exp2();
        assert!(h > 1.0, "{name}: headroom {h}");
        assert_eq!(
            (merged.width, merged.height),
            (primary.width, primary.height)
        );
        let (mut unclipped, mut clipped) = (0usize, 0usize);
        for (&m, &p) in merged.samples.iter().zip(&primary.samples) {
            if f64::from(p) < 0.80 * 65535.0 {
                near(m, (f64::from(p) / h).round(), &format!("{name} unclipped"));
                unclipped += 1;
            } else if p == 65535 {
                assert!(
                    f64::from(m) >= (65535.0 / h).round() - 1.0,
                    "{name}: clipped sample {m} under headroom {h}"
                );
                clipped += 1;
            }
        }
        eprintln!(
            "{name}: {:.2} stops of headroom, {unclipped} unclipped and {clipped} clipped samples checked",
            h.log2()
        );
        merged_files += 1;
    }
    eprintln!("{merged_files} SR RAFs merged");
}
