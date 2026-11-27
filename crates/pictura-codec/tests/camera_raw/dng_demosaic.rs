//! The demosaic stage (`codec/raw-demosaic`, #397) on synthetic CFA planes,
//! and, opt-in, on real camera files.

use pictura_codec::{
    demosaic, read_dng, CameraRgb, CfaPattern, DemosaicMethod, DngColor, DngCrop, DngImage,
    ImageBudget, PsdError,
};

const METHODS: [DemosaicMethod; 2] = [DemosaicMethod::Bilinear, DemosaicMethod::Ahd];

/// RGGB, BGGR, GRBG, GBRG.
const BAYER: [[u8; 4]; 4] = [[0, 1, 1, 2], [2, 1, 1, 0], [1, 0, 2, 1], [1, 2, 0, 1]];

/// A 6×6 X-Trans-shaped layout: every row and column holds all three colours.
const XTRANS: [u8; 36] = [
    1, 2, 1, 1, 0, 1, //
    0, 1, 0, 2, 1, 2, //
    1, 2, 1, 1, 0, 1, //
    1, 0, 1, 1, 2, 1, //
    2, 1, 2, 0, 1, 0, //
    1, 0, 1, 1, 2, 1, //
];

fn pattern(colors: &[u8]) -> CfaPattern {
    let side = if colors.len() == 36 { 6 } else { 2 };
    CfaPattern {
        width: side,
        height: side,
        colors: colors.to_vec(),
    }
}

fn patterns() -> Vec<CfaPattern> {
    let mut all: Vec<_> = BAYER.iter().map(|p| pattern(p)).collect();
    all.push(pattern(&XTRANS));
    all
}

fn image(width: usize, height: usize, samples: Vec<u16>, cfa: CfaPattern) -> DngImage {
    DngImage {
        width,
        height,
        samples,
        cfa,
        fuji_width: None,
        default_crop: DngCrop {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        },
        orientation: 1,
        make: None,
        model: None,
        color: DngColor::default(),
    }
}

fn color_at(cfa: &CfaPattern, x: usize, y: usize) -> usize {
    cfa.colors[(y % cfa.height) * cfa.width + x % cfa.width] as usize
}

/// Samples `rgb(x, y)` through the pattern.
fn mosaic(
    width: usize,
    height: usize,
    cfa: &CfaPattern,
    rgb: impl Fn(usize, usize) -> [u16; 3],
) -> Vec<u16> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| rgb(x, y)[color_at(cfa, x, y)])
        .collect()
}

fn noise(width: usize, height: usize, seed: u64) -> Vec<u16> {
    let mut s = seed;
    (0..width * height)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (s >> 48) as u16
        })
        .collect()
}

fn run(img: &DngImage, method: DemosaicMethod) -> CameraRgb {
    demosaic(img, method, &ImageBudget::default()).unwrap()
}

fn px(out: &CameraRgb, x: usize, y: usize) -> [u16; 3] {
    let o = (y * out.width + x) * 3;
    [out.rgb[o], out.rgb[o + 1], out.rgb[o + 2]]
}

#[test]
fn sampled_channels_are_kept() {
    let (w, h) = (40, 30);
    for cfa in patterns() {
        let img = image(w, h, noise(w, h, 7), cfa.clone());
        for method in METHODS {
            let out = run(&img, method);
            assert_eq!((out.width, out.height, out.rgb.len()), (w, h, w * h * 3));
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        px(&out, x, y)[color_at(&cfa, x, y)],
                        img.samples[y * w + x],
                        "{method:?} {:?} at {x},{y}",
                        cfa.colors
                    );
                }
            }
        }
    }
}

#[test]
fn flat_field_is_unchanged() {
    let (w, h) = (36, 30);
    let flat = [1000, 20000, 5000];
    for cfa in patterns() {
        let img = image(w, h, mosaic(w, h, &cfa, |_, _| flat), cfa.clone());
        for method in METHODS {
            let out = run(&img, method);
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        px(&out, x, y),
                        flat,
                        "{method:?} {:?} at {x},{y}",
                        cfa.colors
                    );
                }
            }
        }
    }
}

#[test]
fn gradient_stays_monotonic() {
    let (w, h) = (48, 40);
    for horizontal in [true, false] {
        let ramp = |x: usize, y: usize| 1000 + 1000 * if horizontal { x } else { y } as u16;
        for colors in BAYER {
            let cfa = pattern(&colors);
            let img = image(w, h, mosaic(w, h, &cfa, |x, y| [ramp(x, y); 3]), cfa);
            for method in METHODS {
                let out = run(&img, method);
                for y in 0..h {
                    for x in 0..w {
                        let here = px(&out, x, y);
                        let (nx, ny) = if horizontal { (x + 1, y) } else { (x, y + 1) };
                        if nx < w && ny < h {
                            let next = px(&out, nx, ny);
                            assert!(
                                (0..3).all(|c| next[c] >= here[c]),
                                "{method:?} {colors:?} {horizontal}: {here:?} then {next:?} at {x},{y}"
                            );
                        }
                        let interior = (3..w - 3).contains(&x) && (3..h - 3).contains(&y);
                        if interior {
                            let want = ramp(x, y);
                            assert!(
                                here.iter().all(|&v| v.abs_diff(want) <= 1),
                                "{method:?} {colors:?} {horizontal}: {here:?} vs {want} at {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// The largest channel deviation from the grey truth within 2 px of the edge.
fn edge_error(vertical_edge: bool, cfa: &CfaPattern, method: DemosaicMethod) -> u16 {
    let (w, h) = (40, 40);
    let grey = |x: usize, y: usize| {
        if (if vertical_edge { x } else { y }) < 20 {
            4000
        } else {
            60000
        }
    };
    let img = image(w, h, mosaic(w, h, cfa, |x, y| [grey(x, y); 3]), cfa.clone());
    let out = run(&img, method);
    let mut worst = 0;
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            let across = if vertical_edge { x } else { y };
            if (18..22).contains(&across) {
                let want = grey(x, y);
                worst = px(&out, x, y)
                    .iter()
                    .map(|v| v.abs_diff(want))
                    .fold(worst, u16::max);
            }
        }
    }
    worst
}

fn reduces_false_colour(cfa: &CfaPattern) {
    for vertical_edge in [true, false] {
        let bilinear = edge_error(vertical_edge, cfa, DemosaicMethod::Bilinear);
        let ahd = edge_error(vertical_edge, cfa, DemosaicMethod::Ahd);
        assert!(
            u32::from(ahd) * 4 < u32::from(bilinear),
            "{:?} vertical={vertical_edge}: AHD {ahd} vs bilinear {bilinear}",
            cfa.colors
        );
    }
}

#[test]
fn ahd_reduces_false_colour_at_edges() {
    for colors in BAYER {
        reduces_false_colour(&pattern(&colors));
    }
}

#[test]
fn xtrans_reduces_false_colour_at_edges() {
    reduces_false_colour(&pattern(&XTRANS));
}

#[test]
fn xtrans_ramp_is_reproduced() {
    let (w, h) = (48, 40);
    let cfa = pattern(&XTRANS);
    for horizontal in [true, false] {
        let ramp = |x: usize, y: usize| 1000 + 1000 * if horizontal { x } else { y } as u16;
        let img = image(
            w,
            h,
            mosaic(w, h, &cfa, |x, y| [ramp(x, y); 3]),
            cfa.clone(),
        );
        let out = run(&img, DemosaicMethod::Ahd);
        for y in 0..h {
            for x in 0..w {
                let here = px(&out, x, y);
                let want = ramp(x, y);
                assert!(
                    here.iter().all(|&v| v.abs_diff(want) <= 1),
                    "{horizontal}: {here:?} vs {want} at {x},{y}"
                );
                let (nx, ny) = if horizontal { (x + 1, y) } else { (x, y + 1) };
                if nx < w && ny < h {
                    let next = px(&out, nx, ny);
                    assert!(
                        (0..3).all(|c| next[c] >= here[c]),
                        "{horizontal}: {here:?} then {next:?} at {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn non_xtrans_six_by_six_keeps_bilinear() {
    let mut colors = XTRANS;
    colors[..24].fill(1);
    let (w, h) = (24, 24);
    let img = image(w, h, noise(w, h, 5), pattern(&colors));
    assert_eq!(
        run(&img, DemosaicMethod::Ahd),
        run(&img, DemosaicMethod::Bilinear)
    );
}

#[test]
fn ahd_falls_back_to_bilinear() {
    let three = CfaPattern {
        width: 3,
        height: 3,
        colors: vec![0, 1, 2, 1, 2, 0, 2, 0, 1],
    };
    let cases = [
        (24, 24, three),
        (1, 1, pattern(&BAYER[0])),
        (3, 2, pattern(&BAYER[1])),
        (7, 7, pattern(&BAYER[2])),
    ];
    for (w, h, cfa) in cases {
        let img = image(w, h, noise(w, h, 11), cfa);
        assert_eq!(
            run(&img, DemosaicMethod::Ahd),
            run(&img, DemosaicMethod::Bilinear),
            "{w}×{h}"
        );
    }
}

#[test]
fn invalid_patterns_and_budgets_are_refused() {
    let (w, h) = (16, 16);
    let red_green = image(w, h, noise(w, h, 3), pattern(&[0, 1, 1, 0]));
    for method in METHODS {
        assert!(matches!(
            demosaic(&red_green, method, &ImageBudget::default()),
            Err(PsdError::Invalid(_))
        ));
    }
    let valid = image(w, h, noise(w, h, 3), pattern(&BAYER[0]));
    let tight = ImageBudget {
        max_alloc_bytes: (w * h * 6 - 1) as u64,
        ..ImageBudget::default()
    };
    assert!(matches!(
        demosaic(&valid, DemosaicMethod::Ahd, &tight),
        Err(PsdError::Invalid(_))
    ));
}

#[test]
fn leica_samples_demosaic_when_present() {
    let Some(root) = std::env::var_os("PICTURA_DNG_CORPUS") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    for name in ["L1.dng", "L2.dng"] {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let img = read_dng(&bytes, &ImageBudget::default()).unwrap();
        for method in METHODS {
            let out = run(&img, method);
            assert_eq!((out.width, out.height), (5216, 3472), "{name} {method:?}");
            for i in (0..img.samples.len()).step_by(7) {
                let (x, y) = (i % img.width, i / img.width);
                assert_eq!(
                    px(&out, x, y)[color_at(&img.cfa, x, y)],
                    img.samples[i],
                    "{name} {method:?} at {x},{y}"
                );
            }
        }
    }
}
