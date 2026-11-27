//! SuperCCD and EXR RAFs (#1621): LibRaw's 45° grid and the rotation back
//! upright (`codec/vendor-raw-containers`, `codec/raw-demosaic`) on
//! synthetic RAFs, and, opt-in, real camera files against LibRaw through
//! rawpy.

use pictura_codec::{
    demosaic, read_raw, CfaPattern, DemosaicMethod, DngColor, DngCrop, DngImage, ImageBudget,
    PsdError,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/raf_builder.rs"]
mod raf_builder;

use builder::noise_plane;
use raf_builder::{primary_only, raf, Raf};

const WHITE: f64 = 15872.0;
const STEP: f64 = std::f64::consts::FRAC_1_SQRT_2;

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

/// Distinct samples, so each lands somewhere recognisable.
fn distinct(width: usize, height: usize) -> Vec<u16> {
    (0..width * height).map(|i| 100 + 7 * i as u16).collect()
}

fn normal(v: u16) -> u16 {
    (v as f64 * 65535.0 / WHITE).round().min(65535.0) as u16
}

/// A SuperCCD word section `width × height` whose `0x0121` names the whole
/// plane as the active area.
fn superccd(width: usize, height: usize, shift: bool, samples: Vec<u16>) -> Raf {
    let mut r = Raf::words(width, height, samples);
    r.crop = None;
    r.layout = if shift {
        [0x80, 0x81, 0x82, 0x83]
    } else {
        [0x00, 0x01, 0x02, 0x01]
    };
    let (h, w) = if shift {
        (height / 2, width * 2)
    } else {
        (height, width)
    };
    r.record(0x0121, &[h as u16, w as u16]);
    r
}

/// LibRaw's grid cell of active-area photosite `(row, col)`.
fn cell(fw: usize, shift: bool, row: usize, col: usize) -> (usize, usize) {
    if shift {
        (fw - 1 - col + row / 2, col + row.div_ceil(2))
    } else {
        (fw - 1 + row - col / 2, row + col.div_ceil(2))
    }
}

fn rotated(fw: usize, grid_height: usize) -> (usize, usize) {
    let f = (fw - 1) as f64;
    (
        (f / STEP) as usize,
        ((grid_height as f64 - f) / STEP) as usize,
    )
}

/// The four layouts the tests cover: each layout bit, odd and even `fw`.
fn layouts() -> [(usize, usize, bool, usize); 4] {
    // (plane width, plane height, shift, fw)
    [
        (16, 24, true, 16),
        (15, 24, true, 15),
        (16, 24, false, 8),
        (18, 24, false, 9),
    ]
}

// raf_superccd_repack
#[test]
fn superccd_rafs_repack_onto_the_grid() {
    for (width, height, shift, fw) in layouts() {
        let r = superccd(width, height, shift, distinct(width, height));
        let image = read(&raf(&r)).unwrap_or_else(|e| panic!("{width} {shift}: {e}"));
        let grid_width = (height >> shift as u32) + fw;
        assert_eq!(image.fuji_width, Some(fw), "{width} {shift}");
        assert_eq!((image.width, image.height), (grid_width, grid_width - 1));
        let mut want = vec![0u16; image.width * image.height];
        for row in 0..height {
            for col in 0..(fw << (!shift) as u32) {
                let (gr, gc) = cell(fw, shift, row, col);
                if gr < image.height && gc < image.width && col < width {
                    want[gr * image.width + gc] = normal(r.samples[row * width + col]);
                }
            }
        }
        assert_eq!(image.samples, want, "{width} {shift}");
        let colors = if fw % 2 == 1 {
            [0, 1, 1, 2]
        } else {
            [1, 2, 0, 1]
        };
        assert_eq!(image.cfa.colors, colors);
        let (w, h) = rotated(fw, image.height);
        let crop = image.default_crop;
        assert_eq!(
            [crop.x, crop.y, crop.width, crop.height],
            [0.0, 0.0, w as f64, h as f64]
        );
    }
}

// raf_superccd_area
#[test]
fn superccd_active_area_follows_libraw() {
    let (width, height) = (20, 28);
    // An inset that fits: rows 2.., columns 2.., 24 × 16.
    let mut inset = superccd(width, height, false, distinct(width, height));
    inset.crop = Some([2, 2, 24, 16]);
    let image = read(&raf(&inset)).expect("inset");
    assert_eq!((image.width, image.fuji_width), (24 + 8, Some(8)));
    assert_eq!(
        image.samples[7 * image.width],
        normal(inset.samples[2 * width + 2])
    );

    // An inset wider than the plane: the centred `0x0121` area, 20 × 16.
    let mut upright = superccd(width, height, false, distinct(width, height));
    upright.crop = Some([2, 2, 24, 40]);
    upright.extra.clear();
    upright.record(0x0121, &[20, 16]);
    let image = read(&raf(&upright)).expect("0x0121");
    assert_eq!((image.width, image.fuji_width), (20 + 8, Some(8)));
    assert_eq!(
        image.samples[7 * image.width],
        normal(upright.samples[4 * width + 2])
    );

    // The S2 Pro: a plane 2944 wide without `0xC000`.
    let (width, height) = (2944, 2200);
    let samples: Vec<u16> = (0..width * height).map(|i| (i % 9973) as u16 + 1).collect();
    let mut s2 = superccd(width, height, false, samples);
    s2.extra.clear();
    let image = read(&raf(&s2)).expect("S2 Pro");
    assert_eq!((image.width, image.height), (3584, 3583));
    assert_eq!((image.fuji_width, image.orientation), (Some(1440), 6));
    assert_eq!(
        image.samples[1439 * image.width],
        normal(s2.samples[28 * width + 32])
    );
    s2.extra.push((0xC000, vec![0; 8]));
    assert!(read(&raf(&s2)).is_err(), "no 0x0121 and not the S2 Pro");
}

fn grid_image(width: usize, height: usize, fw: usize, samples: Vec<u16>) -> DngImage {
    let colors = if fw % 2 == 1 {
        vec![0, 1, 1, 2]
    } else {
        vec![1, 2, 0, 1]
    };
    let (w, h) = rotated(fw, height);
    DngImage {
        width,
        height,
        samples,
        cfa: CfaPattern {
            width: 2,
            height: 2,
            colors,
        },
        fuji_width: Some(fw),
        default_crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: w as f64,
            height: h as f64,
        },
        orientation: 1,
        make: None,
        model: None,
        color: DngColor::default(),
    }
}

// superccd_rotate_size
#[test]
fn superccd_demosaic_rotates_to_libraw_size() {
    for (width, height, shift, fw) in layouts() {
        let r = superccd(
            width,
            height,
            shift,
            noise_plane(width, height, 15872, 3).samples,
        );
        let image = read(&raf(&r)).expect("SuperCCD");
        for method in [DemosaicMethod::Bilinear, DemosaicMethod::Ahd] {
            let rgb = demosaic(&image, method, &ImageBudget::default()).expect("demosaic");
            assert_eq!((rgb.width, rgb.height), rotated(fw, image.height));
            assert_eq!(rgb.rgb.len(), rgb.width * rgb.height * 3);
        }
    }
    let (width, height, fw) = (40, 39, 20);
    let image = grid_image(width, height, fw, vec![1000; width * height]);
    let (w, h) = rotated(fw, height);
    let grid_out = (width * height * 6) as u64;
    let input = (width * height * 2) as u64;
    let mut budget = ImageBudget {
        max_document_bytes: grid_out + input + (w * h * 6) as u64 - 1,
        ..ImageBudget::default()
    };
    assert!(demosaic(&image, DemosaicMethod::Ahd, &budget).is_err());
    budget.max_document_bytes += 1;
    demosaic(&image, DemosaicMethod::Ahd, &budget).expect("just within the budget");
}

// superccd_rotate_positions
#[test]
fn superccd_rotation_samples_libraw_positions() {
    let value = |ch: usize, r: f64, c: f64| match ch {
        0 => 1000.0 + 40.0 * r + 25.0 * c,
        1 => 20000.0 - 30.0 * r + 50.0 * c,
        _ => 5000.0 + 10.0 * r + 10.0 * c,
    };
    for fw in [30, 31] {
        let (width, height) = (60, 59);
        let image = grid_image(width, height, fw, Vec::new());
        let samples = (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let ch = image.cfa.colors[(y % 2) * 2 + x % 2] as usize;
                value(ch, y as f64, x as f64).round() as u16
            })
            .collect();
        let image = DngImage { samples, ..image };
        for method in [DemosaicMethod::Bilinear, DemosaicMethod::Ahd] {
            let rgb = demosaic(&image, method, &ImageBudget::default()).expect("demosaic");
            let mut checked = 0;
            for row in 0..rgb.height {
                for col in 0..rgb.width {
                    let r = ((fw - 1) as f64 + (row as f64 - col as f64) * STEP) as f32 as f64;
                    let c = ((row + col) as f64 * STEP) as f32 as f64;
                    if r < 6.0 || c < 6.0 || r > (height - 7) as f64 || c > (width - 7) as f64 {
                        continue;
                    }
                    for ch in 0..3 {
                        let got = rgb.rgb[(row * rgb.width + col) * 3 + ch] as f64;
                        let want = value(ch, r, c);
                        assert!(
                            (got - want).abs() <= 2.0,
                            "{method:?} fw {fw} ({row}, {col}) ch {ch}: {got} vs {want}"
                        );
                    }
                    checked += 1;
                }
            }
            assert!(checked > 50, "{checked} interior pixels");
        }
    }
}

fn corpus_with_rawpy(what: &str) -> Option<Vec<std::path::PathBuf>> {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP {what}: PICTURA_RAW_CORPUS is not set");
        return None;
    };
    let rawpy = std::process::Command::new("python3")
        .args(["-c", "import rawpy"])
        .status()
        .is_ok_and(|s| s.success());
    if !rawpy {
        eprintln!("SKIP {what}: python3 with rawpy is not available");
        return None;
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.expect("corpus entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("raf")))
        .collect();
    paths.sort();
    Some(paths)
}

/// Runs `script` on `path`; its stdout is little-endian words: a kind (0 to
/// skip the file), then the script's own fields.
fn python(script: &str, path: &std::path::Path) -> Vec<u8> {
    let out = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(path)
        .output()
        .expect("python3");
    assert!(
        out.status.success(),
        "{}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn word(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[4 * i..4 * i + 4].try_into().unwrap())
}

fn words16(b: &[u8]) -> Vec<u16> {
    b.as_chunks::<2>()
        .0
        .iter()
        .map(|w| u16::from_le_bytes(*w))
        .collect()
}

/// The parts of a RAF the dumps share: its directory records and whether
/// the sensor section is a TIFF.
const RAF_HEAD: &str = r#"
import numpy as np, rawpy, struct, sys
b = open(sys.argv[1], 'rb').read()
ro, rl = struct.unpack('>II', b[0x5c:0x64])
rec, at = {}, ro + 4
for _ in range(struct.unpack('>I', b[ro:ro + 4])[0]):
    t, l = struct.unpack('>HH', b[at:at + 4])
    rec[t] = b[at + 4:at + 4 + l]
    at += 4 + l
so = struct.unpack('>I', b[0x64:0x68])[0]
tiff = b[so:so + 2] in (b'II', b'MM')
superccd = not rec[0x130][1] & 8
out = sys.stdout.buffer
"#;

/// LibRaw's raw plane repacked onto its own grid with its own margins and
/// size, normalized by its black and white: the SuperCCD oracle.
const SUPERCCD_DUMP: &str = r#"
if not superccd:
    out.write(struct.pack('<I', 0)); sys.exit()
L = rec[0x130][0] >> 7
with rawpy.imread(sys.argv[1]) as r:
    a, s = r.raw_image.astype(np.float64), r.sizes
    black, white = r.black_level_per_channel, r.white_level
W, H, top, left = s.width, s.height, s.top_margin, s.left_margin
h = s.raw_height - 2 * top
fw = W - (h >> L)
grid = np.zeros((H, W))
cols = np.arange(fw << (1 - L))
for row in range(h):
    if L:
        gr, gc = fw - 1 - cols + (row >> 1), cols + ((row + 1) >> 1)
    else:
        gr, gc = fw - 1 + row - (cols >> 1), row + ((cols + 1) >> 1)
    ok = (gr < H) & (gc < W) & (cols + left < s.raw_width)
    grid[gr[ok], gc[ok]] = a[row + top, cols[ok] + left]
y, x = np.mgrid[0:H, 0:W]
pat = [0, 1, 1, 2] if fw & 1 else [1, 2, 0, 1]
colour = np.choose((y % 2) * 2 + x % 2, pat)
# LibRaw's second green (index 3) sits on the rows without red.
red_row = (y % 2) == (0 if 0 in pat[:2] else 1)
blk = np.where(colour == 1, np.where(red_row, black[1], black[3]),
               np.choose(colour, [black[0], 0, black[2]]))
norm = np.clip(np.round((grid - blk) * 65535 / (white - blk)), 0, 65535)
norm[grid == 0] = 0
out.write(struct.pack('<III', 1, W, H))
out.write(norm.astype('<u2').tobytes())
"#;

// raf_superccd_libraw
#[test]
fn superccd_rafs_match_libraw_when_present() {
    let Some(paths) = corpus_with_rawpy("SuperCCD raf corpus") else {
        return;
    };
    let mut compared = 0;
    for path in paths {
        let b = python(&format!("{RAF_HEAD}{SUPERCCD_DUMP}"), &path);
        if word(&b, 0) == 0 {
            continue;
        }
        let name = path.display();
        let (width, height) = (word(&b, 1) as usize, word(&b, 2) as usize);
        let want = words16(&b[12..]);
        // LibRaw reads an SR body's primary frame alone.
        let image = read(&primary_only(std::fs::read(&path).unwrap()))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (width, height), "{name}");
        let differ = image
            .samples
            .iter()
            .zip(&want)
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count();
        assert_eq!(differ, 0, "{name}: grid cells differing from LibRaw");
        eprintln!("{name}: {width} x {height} grid equal to LibRaw");
        compared += 1;
    }
    eprintln!("{compared} SuperCCD RAFs compared");
}

/// LibRaw's raw plane, its colours at the plane's origin, and the plane
/// normalized by LibRaw's black and the white the RAF's own coding names:
/// the oracle for uncompressed 2 × 2 files (an X-Trans file's black is
/// per position, which LibRaw averages by channel; the X100S DNG pair
/// covers X-Trans).
const UPRIGHT_DUMP: &str = r#"
if superccd or 0x131 in rec:
    out.write(struct.pack('<I', 0)); sys.exit()
if tiff:
    o = '<' if b[so:so + 2] == b'II' else '>'
    def ifd(at):
        d = {}
        for i in range(struct.unpack(o + 'H', b[so + at:so + at + 2])[0]):
            e = b[so + at + 2 + 12 * i:so + at + 14 + 12 * i]
            tag, typ, n = struct.unpack(o + 'HHI', e[:8])
            d[tag] = struct.unpack(o + 'I', e[8:])[0] if typ in (4, 13) and n == 1 else None
        return d
    raw = ifd(ifd(struct.unpack(o + 'I', b[so + 4:so + 8])[0])[0xF000])
    if b[so + raw[0xF007]:so + raw[0xF007] + 2] == b'IS':
        out.write(struct.pack('<I', 0)); sys.exit()
    white = (1 << (raw.get(0xF003) or 14)) - 1
else:
    jo = struct.unpack('>I', b[0x54:0x58])[0]
    j = b[jo:jo + 64]
    white = 4095 if j[j.find(b'Exif') + 6:][:2] == b'MM' else 15872
with rawpy.imread(sys.argv[1]) as r:
    a = r.raw_image.astype(np.float64)
    raw_colors = r.raw_colors.copy()
    black = r.black_level_per_channel
colours = np.where(raw_colors == 3, 1, raw_colors)
side = 2
blk = np.choose(raw_colors, black)
norm = np.clip(np.round((a - blk) * 65535 / (white - blk)), 0, 65535)
out.write(struct.pack('<IIII', 1, a.shape[1], a.shape[0], side))
out.write(colours[:side, :side].astype('<u4').tobytes())
out.write(norm.astype('<u2').tobytes())
"#;

// raf_upright_libraw
#[test]
fn upright_rafs_match_libraw_when_present() {
    let Some(paths) = corpus_with_rawpy("upright raf corpus") else {
        return;
    };
    let mut compared = 0;
    for path in paths {
        let b = python(&format!("{RAF_HEAD}{UPRIGHT_DUMP}"), &path);
        if word(&b, 0) == 0 {
            continue;
        }
        let name = path.display();
        let (width, height, side) = (
            word(&b, 1) as usize,
            word(&b, 2) as usize,
            word(&b, 3) as usize,
        );
        let colours: Vec<u8> = (0..side * side).map(|i| word(&b, 4 + i) as u8).collect();
        let want = words16(&b[4 * (4 + side * side)..]);
        let image = read(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (width, height), "{name}");
        assert_eq!((image.cfa.width, image.cfa.height), (side, side), "{name}");
        assert_eq!(image.cfa.colors, colours, "{name}: CFA");
        let differ = image
            .samples
            .iter()
            .zip(&want)
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count();
        assert_eq!(differ, 0, "{name}: samples differing from LibRaw");
        eprintln!("{name}: {width} x {height} equal to LibRaw");
        compared += 1;
    }
    eprintln!("{compared} upright RAFs compared");
}

/// LibRaw's linear, unscaled camera-RGB rendering of a SuperCCD file: its
/// size and each channel's 8 × 8 block means.
const RENDER_DUMP: &str = r#"
if not superccd:
    out.write(struct.pack('<I', 0)); sys.exit()
with rawpy.imread(sys.argv[1]) as r:
    rgb = r.postprocess(output_color=rawpy.ColorSpace.raw, gamma=(1, 1),
        no_auto_bright=True, user_wb=[1, 1, 1, 1], output_bps=16,
        demosaic_algorithm=rawpy.DemosaicAlgorithm.LINEAR, no_auto_scale=True)
h, w = rgb.shape[:2]
bh, bw = h // 8, w // 8
out.write(struct.pack('<IIIII', 1, w, h, bw, bh))
for c in range(3):
    m = rgb[:bh * 8, :bw * 8, c].astype(np.float64).reshape(bh, 8, bw, 8).mean((1, 3))
    out.write(m.astype('<f8').tobytes())
"#;

/// Channel `c` of `rgb` (`w × h`) shown under TIFF orientation 1 or 6.
fn oriented(
    rgb: &[u16],
    (w, h): (usize, usize),
    c: usize,
    orientation: u16,
) -> (usize, usize, Vec<f64>) {
    let at = |x: usize, y: usize| rgb[(y * w + x) * 3 + c] as f64;
    match orientation {
        1 => (w, h, (0..w * h).map(|i| at(i % w, i / w)).collect()),
        6 => (
            h,
            w,
            (0..w * h)
                .map(|i| {
                    let (x, y) = (i % h, i / h);
                    at(y, h - 1 - x)
                })
                .collect(),
        ),
        other => panic!("orientation {other} is not in the corpus"),
    }
}

fn correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
    }
    sab / (saa * sbb).sqrt()
}

// superccd_rotate_libraw
#[test]
fn superccd_renders_match_libraw_when_present() {
    let Some(paths) = corpus_with_rawpy("SuperCCD render corpus") else {
        return;
    };
    let budget = ImageBudget::default();
    let mut compared = 0;
    for path in paths {
        let b = python(&format!("{RAF_HEAD}{RENDER_DUMP}"), &path);
        if word(&b, 0) == 0 {
            continue;
        }
        let name = path.display();
        let (w, h, bw, bh) = (
            word(&b, 1) as usize,
            word(&b, 2) as usize,
            word(&b, 3) as usize,
            word(&b, 4) as usize,
        );
        let want: Vec<f64> = b[20..]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|v| f64::from_le_bytes(*v))
            .collect();
        // LibRaw reads an SR body's primary frame alone.
        let image = read(&primary_only(std::fs::read(&path).unwrap()))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let rgb = demosaic(&image, DemosaicMethod::Ahd, &budget).expect("demosaic");
        let mut worst = 1.0f64;
        for c in 0..3 {
            let (ow, oh, plane) = oriented(&rgb.rgb, (rgb.width, rgb.height), c, image.orientation);
            assert_eq!((ow, oh), (w, h), "{name}: rendered size");
            let mut got = Vec::with_capacity(bw * bh);
            for by in 0..bh {
                for bx in 0..bw {
                    let mut sum = 0.0;
                    for y in by * 8..by * 8 + 8 {
                        sum += plane[y * ow + bx * 8..][..8].iter().sum::<f64>();
                    }
                    got.push(sum / 64.0);
                }
            }
            let r = correlation(&got, &want[c * bw * bh..][..bw * bh]);
            assert!(r >= 0.99, "{name}: channel {c} block correlation {r}");
            worst = worst.min(r);
        }
        eprintln!("{name}: {w} x {h}, worst channel block correlation {worst:.5}");
        compared += 1;
    }
    eprintln!("{compared} SuperCCD renders compared");
}
