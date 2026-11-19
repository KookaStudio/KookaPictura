use super::*;
use crate::replace::grow;
use crate::stroke::layer_rgba;
use crate::{PaintError, Stroke, StrokeSample};
use pictura_core::{BitDepth, Channel, ColorMode, Document};

const W: i32 = 64;
const H: i32 = 64;
const RED: [u8; 4] = [200, 30, 30, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn doc(px: [u8; 4]) -> Document {
    let mut doc = Document::new(W as u32, H as u32, ColorMode::Rgb, BitDepth::Eight);
    {
        let _ = doc.layers.push(pictura_core::Layer::from_parts(
            pictura_core::LayerParts {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: H,
                    right: W,
                },
                channels: [(0, 0), (1, 1), (2, 2), (-1, 3)]
                    .map(|(id, c)| Channel {
                        id,
                        data: vec![px[c]; (W * H) as usize].into(),
                    })
                    .into(),
                ..Default::default()
            },
            pictura_core::LayerContent::Pixel,
        ));
    };
    doc
}

fn surface(px: [u8; 4]) -> RgbaImage {
    RgbaImage {
        width: W,
        height: H,
        data: vec![px; (W * H) as usize],
    }
}

fn paint(doc: &Document, source: RgbaImage, options: ArtHistoryOptions) -> Option<Document> {
    let cfg = StrokeConfig {
        diameter: 4,
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_art_history(doc, "0", cfg, source.into(), options).ok()?;
    let mut out = doc.clone();
    for x in [24.0, 32.0, 40.0] {
        stroke.sample(
            &mut out,
            StrokeSample {
                x,
                y: 32.0,
                pressure: 1.0,
            },
        );
    }
    stroke.finish().map(|_| out)
}

fn painted(doc: &Document, colour: [u8; 4]) -> Vec<(i32, i32)> {
    let img = layer_rgba(&doc.layers[0]);
    (0..W * H)
        .map(|i| (i % W, i / W))
        .filter(|&(x, y)| img.get(x, y) == colour)
        .collect()
}

#[test]
fn strokes_take_their_colour_from_the_source_and_scatter_over_the_area() {
    let out = paint(&doc(WHITE), surface(RED), ArtHistoryOptions::default()).unwrap();
    let red = painted(&out, RED);
    assert!(!red.is_empty(), "nothing painted");
    // Area 50 reaches beyond the 4 px tip's own track.
    assert!(red.iter().any(|&(_, y)| (y - 32).abs() > 6), "no scatter");
}

#[test]
fn a_high_tolerance_leaves_pixels_that_match_the_source() {
    let options = ArtHistoryOptions {
        tolerance: 50,
        ..ArtHistoryOptions::default()
    };
    // Source and layer agree, so nothing has drifted far enough.
    assert!(paint(&doc(RED), surface(RED), options).is_none());
    // White is far from red: the gate lets it through.
    assert!(paint(&doc(WHITE), surface(RED), options).is_some());
}

#[test]
fn longer_styles_reach_further_than_dab() {
    let reach = |style| {
        let options = ArtHistoryOptions {
            style,
            area: 1,
            ..ArtHistoryOptions::default()
        };
        let out = paint(&doc(WHITE), surface(RED), options).unwrap();
        let red = painted(&out, RED);
        let spread = |f: fn(&(i32, i32)) -> i32| {
            red.iter().map(f).max().unwrap() - red.iter().map(f).min().unwrap()
        };
        spread(|p| p.0).max(spread(|p| p.1))
    };
    assert!(reach(ArtStyle::TightLong) > reach(ArtStyle::Dab));
}

#[test]
fn a_seed_replays_and_another_differs() {
    let at = |seed| {
        let options = ArtHistoryOptions {
            seed,
            ..ArtHistoryOptions::default()
        };
        painted(&paint(&doc(WHITE), surface(RED), options).unwrap(), RED)
    };
    assert_eq!(at(7), at(7));
    assert_ne!(at(7), at(8));
}

#[test]
fn a_deep_document_is_refused() {
    let mut deep = doc(WHITE);
    deep.depth = BitDepth::Sixteen;
    let cfg = StrokeConfig::default();
    let begun = Stroke::begin_art_history(&deep, "0", cfg, surface(RED).into(), Default::default());
    assert!(matches!(begun, Err(PaintError::UnsupportedDepth)));
}

#[test]
fn every_style_has_a_name() {
    for (i, _) in STYLE_NAMES.iter().enumerate() {
        assert!(ArtStyle::from_i32(i as i32).is_some());
    }
    assert!(ArtStyle::from_i32(STYLE_NAMES.len() as i32).is_none());
}

fn layer_local_serial(source: &RgbaImage, rect: PsdRect) -> RgbaImage {
    let (w, h) = (rect.width().max(0), rect.height().max(0));
    let mut out = RgbaImage {
        width: w,
        height: h,
        data: vec![[0; 4]; (w * h) as usize],
    };
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x + rect.left, y + rect.top);
            if sx >= 0 && sy >= 0 && sx < source.width && sy < source.height {
                out.set(x, y, source.get(sx, sy));
            }
        }
    }
    out
}

#[test]
fn layer_local_matches_serial_builder() {
    let source = surface(RED);
    for rect in [
        PsdRect {
            top: 0,
            left: 0,
            bottom: 64,
            right: 64,
        },
        PsdRect {
            top: 5,
            left: 7,
            bottom: 20,
            right: 30,
        },
        PsdRect {
            top: -4,
            left: -6,
            bottom: 10,
            right: 12,
        },
    ] {
        assert_eq!(
            layer_local(&source, rect),
            layer_local_serial(&source, rect),
            "{rect:?}"
        );
    }
    let empty = RgbaImage {
        width: 0,
        height: 0,
        data: Vec::new(),
    };
    let zero = PsdRect {
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
    };
    assert!(layer_local(&empty, zero).data.is_empty());
}

#[test]
fn sampled_surface_matches_two_pass_crop() {
    let mut buf = pictura_core::PixelBuffer::new(16, 12, 4);
    buf.data = (0..16 * 12 * 4)
        .map(|i| (i as u8).wrapping_mul(3))
        .collect::<Vec<u8>>()
        .into();
    for rect in [
        PsdRect {
            top: 0,
            left: 0,
            bottom: 12,
            right: 16,
        },
        PsdRect {
            top: 3,
            left: 2,
            bottom: 9,
            right: 11,
        },
        PsdRect {
            top: -2,
            left: -3,
            bottom: 5,
            right: 6,
        },
        PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        PsdRect {
            top: 3,
            left: 2,
            bottom: 9,
            right: 2,
        },
    ] {
        let one_pass = crate::stamp::surface_from_composite_rect(&buf, rect);
        let two_pass = layer_local(&crate::stamp::surface_from_composite(&buf), rect);
        assert_eq!(one_pass, two_pass, "{rect:?}");
        if rect.width() == 0 || rect.height() == 0 {
            assert!(one_pass.data.is_empty(), "{rect:?}");
            assert!(two_pass.data.is_empty(), "{rect:?}");
        }
    }
}

/// The tip-by-tip stamping `dab` replaced (#1238): every pixel of each
/// tip's box through `tip_coverage`, one tip after another.
fn stamp_reference(
    brush: &ArtHistoryBrush,
    img: &mut RgbaImage,
    cfg: &StrokeConfig,
    (cx, cy): (f32, f32),
    paint: [u8; 4],
) -> Option<PsdRect> {
    let region = dab_bounds(img, cfg, cx, cy)?;
    let strength = cfg.opacity.min(100) as f32 / 100.0 * cfg.flow.min(100) as f32 / 100.0;
    let mut dirty = None;
    for y in region.top..region.bottom {
        for x in region.left..region.right {
            let tip = crate::tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let a = tip * strength * paint[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let dest = img.get(x, y);
            if brush.preserve_alpha && dest[3] == 0 {
                continue;
            }
            let mut out = over(dest, paint, a);
            if brush.preserve_alpha {
                out[3] = dest[3];
            }
            if out != dest {
                img.set(x, y, out);
                dirty = Some(grow(dirty, x, y));
            }
        }
    }
    dirty
}

/// [`ArtHistoryBrush::dab`] with [`stamp_reference`] for each tip.
fn dab_reference(
    brush: &mut ArtHistoryBrush,
    img: &mut RgbaImage,
    cfg: &StrokeConfig,
    cx: f32,
    cy: f32,
) -> Option<PsdRect> {
    let diameter = cfg.diameter.max(1) as f32;
    let area = brush.options.area.max(1) as f32;
    let count = (area / diameter).ceil().clamp(1.0, MAX_STROKES_PER_DAB) as usize;
    let shape = brush.options.style.shape();
    let step = (diameter * 0.25).max(1.0);
    let steps = (shape.length * diameter / step).round() as usize;
    let mut dirty = None;
    for _ in 0..count {
        let (angle, reach) = (brush.next() * TAU, brush.next().sqrt() * area * 0.5);
        let (mut x, mut y) = (cx + reach * angle.cos(), cy + reach * angle.sin());
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        if ix < 0 || iy < 0 || ix >= img.width || iy >= img.height {
            continue;
        }
        let paint = brush.source.get(ix, iy);
        if paint[3] == 0 || !brush.differs(paint, img.get(ix, iy)) {
            continue;
        }
        let mut heading = match brush.edge_heading(ix, iy) {
            Some(h) => h,
            None => brush.next() * TAU,
        };
        let turn = if brush.next() < 0.5 {
            -shape.curl
        } else {
            shape.curl
        };
        for i in 0..=steps {
            if i > 0 {
                heading += turn + (brush.next() - 0.5) * 2.0 * shape.jitter;
                x += step * heading.cos();
                y += step * heading.sin();
            }
            if let Some(d) = stamp_reference(brush, img, cfg, (x, y), paint) {
                dirty = Some(dirty.map_or(d, |r: PsdRect| r.union(d)));
            }
        }
    }
    dirty
}

/// A deterministic picture: bands, a diagonal edge and per-pixel grain,
/// with an alpha ramp when `translucent`.
fn textured(w: i32, h: i32, seed: u32, translucent: bool) -> RgbaImage {
    let data = (0..h)
        .flat_map(|y| {
            (0..w).map(move |x| {
                let grain = (x as u32 ^ y as u32).wrapping_mul(2_654_435_761) ^ seed;
                let edge = if x > y { 180 } else { 40 };
                let a = if translucent {
                    ((x * 255) / w.max(1)) as u8
                } else {
                    255
                };
                [edge, ((y * 7 + x) % 251) as u8, (grain >> 24) as u8, a]
            })
        })
        .collect();
    RgbaImage {
        width: w,
        height: h,
        data,
    }
}

#[test]
fn one_pass_strokes_match_tip_by_tip_stamping() {
    let (w, h) = (160, 120);
    let tips = [
        StrokeConfig {
            diameter: 9,
            hardness: 0,
            ..StrokeConfig::default()
        },
        StrokeConfig {
            diameter: 24,
            hardness: 100,
            ..StrokeConfig::default()
        },
        StrokeConfig {
            diameter: 31,
            hardness: 40,
            roundness: 35,
            angle_deg: 57,
            opacity: 63,
            flow: 81,
            ..StrokeConfig::default()
        },
        StrokeConfig {
            diameter: 17,
            square: true,
            aliased: true,
            ..StrokeConfig::default()
        },
        StrokeConfig {
            diameter: 1,
            ..StrokeConfig::default()
        },
    ];
    let mut checked = 0;
    for (t, cfg) in tips.iter().enumerate() {
        for style in 0..STYLE_NAMES.len() as i32 {
            for (preserve_alpha, translucent) in [(false, false), (true, true), (false, true)] {
                let options = ArtHistoryOptions {
                    style: ArtStyle::from_i32(style).expect("a style"),
                    area: [12, 50, 140][t % 3],
                    tolerance: [0, 30][style as usize % 2],
                    seed: (t * 31 + style as usize) as u64,
                };
                let source = textured(w, h, 7, false);
                let start = textured(w, h, 99, translucent);
                let mut fast = ArtHistoryBrush::new(options, source.clone(), preserve_alpha);
                let mut slow = ArtHistoryBrush::new(options, source, preserve_alpha);
                let (mut a, mut b) = (start.clone(), start);
                for i in 0..12 {
                    let (x, y) = (-6.0 + i as f32 * 15.3, 10.0 + i as f32 * 9.1);
                    let got = fast.dab(&mut a, cfg, x, y);
                    let want = dab_reference(&mut slow, &mut b, cfg, x, y);
                    assert_eq!(got, want, "dirty: tip {t} style {style} dab {i}");
                    checked += usize::from(want.is_some());
                }
                assert!(
                    a.data == b.data,
                    "pixels: tip {t} style {style} preserve {preserve_alpha}"
                );
            }
        }
    }
    assert!(checked > 500, "too few dabs painted ({checked})");
}

/// The `profile-tools.py` Art History stroke (#1238): an 8000×6000 layer,
/// a 200 px hard tip, Tight Short over 50 px, and the harness's 60-move
/// zigzag, timed through `Stroke::sample` alone (no presentation).
#[test]
#[ignore = "profiling: 8000x6000 Art History motion; run with --ignored --nocapture"]
fn motion_profile() {
    let (w, h) = (8000_i32, 6000_i32);
    let layer = |px: &dyn Fn(i32, i32) -> [u8; 4]| {
        let mut planes: Vec<Vec<u8>> = (0..4)
            .map(|_| Vec::with_capacity((w * h) as usize))
            .collect();
        for y in 0..h {
            for x in 0..w {
                let p = px(x, y);
                for (c, plane) in planes.iter_mut().enumerate() {
                    plane.push(p[c]);
                }
            }
        }
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
        let _ = doc.layers.push(pictura_core::Layer::from_parts(
            pictura_core::LayerParts {
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: h,
                    right: w,
                },
                channels: [0, 1, 2, -1]
                    .into_iter()
                    .zip(planes)
                    .map(|(id, data)| Channel {
                        id,
                        data: data.into(),
                    })
                    .collect(),
                ..Default::default()
            },
            pictura_core::LayerContent::Pixel,
        ));
        doc
    };
    let texture = |x: i32, y: i32| {
        let v = ((x / 37 + y / 23) % 7 * 30 + (x ^ y) % 17) as u8;
        [v, 255 - v, v / 2, 255]
    };
    let cases: [(&str, Document, RgbaImage); 2] = [
        ("uniform", layer(&|_, _| WHITE), {
            RgbaImage {
                width: w,
                height: h,
                data: vec![WHITE; (w * h) as usize],
            }
        }),
        ("textured", layer(&|x, y| texture(y, x)), {
            let data = (0..h)
                .flat_map(|y| (0..w).map(move |x| texture(x, y)))
                .collect();
            RgbaImage {
                width: w,
                height: h,
                data,
            }
        }),
    ];
    let cfg = StrokeConfig {
        diameter: 200,
        hardness: 100,
        ..StrokeConfig::default()
    };
    for (name, doc, source) in cases {
        let mut out = doc.clone();
        let t = std::time::Instant::now();
        let mut stroke =
            Stroke::begin_art_history(&doc, "0", cfg, source.into(), ArtHistoryOptions::default())
                .expect("begin");
        let begin = t.elapsed();
        let mut moves = Vec::new();
        for i in 0..=60 {
            let t = i as f32 / 60.0;
            let wave = ((t * 3.0) % 2.0 - 1.0).abs();
            let s = StrokeSample {
                x: w as f32 * (0.15 + 0.7 * t),
                y: h as f32 * (0.35 + 0.3 * wave),
                pressure: 1.0,
            };
            let t = std::time::Instant::now();
            stroke.sample(&mut out, s);
            moves.push(t.elapsed().as_secs_f64() * 1e3);
        }
        let t = std::time::Instant::now();
        let _ = stroke.finish();
        let finish = t.elapsed();
        let sum: f64 = moves.iter().sum();
        moves.sort_by(f64::total_cmp);
        eprintln!(
            "art-history {name}: begin {:.1} ms, move p50 {:.2} p95 {:.2} max {:.2} sum {:.1} ms, finish {:.1} ms",
            begin.as_secs_f64() * 1e3,
            moves[30],
            moves[57],
            moves[60],
            sum,
            finish.as_secs_f64() * 1e3
        );
    }
}
