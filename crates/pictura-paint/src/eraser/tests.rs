use super::*;
use crate::stamp::layer_surface;
use crate::stroke::layer_rgba;
use crate::{Rgba, StrokeSample};
use pictura_core::{BitDepth, Channel, ColorMode, LockFlags, PsdRect};

const GREY: [u8; 4] = [90, 90, 90, 255];
const TEAL: Rgba = Rgba {
    r: 0,
    g: 128,
    b: 128,
    a: 255,
};

fn doc(background: bool) -> Document {
    let (w, h) = (40, 20);
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let ids: &[(i16, usize)] = if background {
        &[(0, 0), (1, 1), (2, 2)]
    } else {
        &[(0, 0), (1, 1), (2, 2), (-1, 3)]
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        },
        channels: ids
            .iter()
            .map(|&(id, c)| Channel {
                id,
                data: vec![GREY[c]; (w * h) as usize].into(),
            })
            .collect(),
        background,
        ..Default::default()
    }]);
    doc
}

fn erase(doc: &Document, mode: EraserMode, history: Option<StampSource>) -> Document {
    let cfg = StrokeConfig {
        diameter: 8,
        background: TEAL,
        ..StrokeConfig::default()
    };
    let mut stroke = begin_erase(doc, "0", cfg, mode, history, 1.0).expect("begin");
    let mut out = doc.clone();
    for x in [8.0, 16.0, 24.0] {
        stroke.sample(
            &mut out,
            StrokeSample {
                x,
                y: 10.0,
                pressure: 1.0,
            },
        );
    }
    stroke.finish().expect("erased");
    out
}

fn pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
    layer_rgba(&doc.layers[0]).get(x, y)
}

/// The layer's pixels the stroke cleared.
fn erased_pixels(doc: &Document) -> usize {
    let layer = layer_rgba(&doc.layers[0]);
    (0..layer.height)
        .flat_map(|y| (0..layer.width).map(move |x| (x, y)))
        .filter(|&(x, y)| layer.get(x, y)[3] == 0)
        .count()
}

#[test]
fn a_layer_erases_to_transparency_and_the_background_to_the_background_colour() {
    let layer = erase(&doc(false), EraserMode::Brush, None);
    assert_eq!(pixel(&layer, 16, 10)[3], 0, "the stroke left alpha behind");
    assert_eq!(pixel(&layer, 16, 18), GREY, "erased outside the tip");

    let background = erase(&doc(true), EraserMode::Brush, None);
    assert_eq!(pixel(&background, 16, 10), [0, 128, 128, 255]);
}

#[test]
fn a_transparency_lock_paints_the_background_colour() {
    let mut locked = doc(false);
    locked.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
    assert_eq!(
        pixel(&erase(&locked, EraserMode::Brush, None), 16, 10),
        [0, 128, 128, 255]
    );
}

#[test]
fn half_opacity_leaves_half_the_alpha() {
    let cfg = StrokeConfig {
        diameter: 8,
        opacity: 50,
        ..StrokeConfig::default()
    };
    let mut out = doc(false);
    let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Brush, None, 1.0).unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 16.0,
            y: 10.0,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    let a = pixel(&out, 16, 10)[3];
    assert!((a as i32 - 128).abs() <= 1, "alpha {a}");
}

#[test]
fn block_erases_a_hard_square_at_full_strength() {
    // Settings inherited from a pencil or wet-edged preset must not leak in.
    let cfg = StrokeConfig {
        opacity: 10,
        flow: 50,
        grain: 55,
        wet_edges: true,
        ..StrokeConfig::default()
    };
    let mut out = doc(false);
    let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Block, None, 1.0).unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 20.0,
            y: 10.0,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    // The square's corner is erased; a round 16 px tip would miss it.
    assert_eq!(pixel(&out, 13, 3)[3], 0);
    assert_eq!(pixel(&out, 20, 10)[3], 0);
    assert_eq!(pixel(&out, 11, 10)[3], 255);

    // A half-pixel dab covers the same square: it is exactly 16 px wide,
    // not one wider on the far side.
    let mut out = doc(false);
    let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Block, None, 1.0).unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 20.5,
            y: 10.5,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    assert_eq!(pixel(&out, 13, 3)[3], 0, "the square's corner");
    assert_eq!(pixel(&out, 27, 17)[3], 0, "the square's far corner");
    assert_eq!(pixel(&out, 28, 10)[3], 255, "the square is 16 px wide");
    assert_eq!(pixel(&out, 20, 18)[3], 255, "the square is 16 px tall");
    assert_eq!(erased_pixels(&out), 16 * 16, "exactly one square");
}

#[test]
fn the_block_square_scales_with_the_zoom() {
    // The Block is 16 screen pixels wide, so its document side is
    // 16 / zoom.
    assert_eq!(block_side_doc_px(1.0), 16);
    assert_eq!(block_side_doc_px(4.0), 4);
    assert_eq!(block_side_doc_px(0.25), 64);
    assert_eq!(block_side_doc_px(32.0), 1, "a document pixel is the floor");
    assert_eq!(block_side_doc_px(0.0), 16, "a bad zoom is 100 %");
    assert_eq!(block_side_doc_px(f64::NAN), 16, "a bad zoom is 100 %");

    // At 400 % the block is 4 document pixels a side, so it covers 16 of
    // them whatever the dab's position in its pixel.
    let mut out = doc(false);
    let mut stroke = begin_erase(
        &out,
        "0",
        StrokeConfig::default(),
        EraserMode::Block,
        None,
        4.0,
    )
    .unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 20.5,
            y: 10.5,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    assert_eq!(erased_pixels(&out), 4 * 4, "4 document px a side at 400 %");

    // At 3200 % the block is one document pixel, so it covers one: the
    // inclusive boundary would have covered two.
    let mut out = doc(false);
    let mut stroke = begin_erase(
        &out,
        "0",
        StrokeConfig::default(),
        EraserMode::Block,
        None,
        32.0,
    )
    .unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 20.5,
            y: 10.5,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    assert_eq!(erased_pixels(&out), 1, "one document px at 3200 %");

    // The brush preset's shape dynamics do not apply: the Block is one
    // square of one size.
    let mut out = doc(false);
    let mut stroke = begin_erase(
        &out,
        "0",
        StrokeConfig {
            scatter: 120,
            count: 3,
            size_jitter: 60,
            ..StrokeConfig::default()
        },
        EraserMode::Block,
        None,
        1.0,
    )
    .unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 20.5,
            y: 10.5,
            pressure: 1.0,
        },
    );
    stroke.finish().unwrap();
    assert_eq!(erased_pixels(&out), 16 * 16, "the dynamics do not apply");
}

#[test]
fn pencil_erases_without_a_soft_edge() {
    let out = erase(&doc(false), EraserMode::Pencil, None);
    let layer = layer_rgba(&out.layers[0]);
    assert!(layer.data.iter().all(|p| p[3] == 0 || p[3] == 255));
}

#[test]
fn erase_to_history_paints_the_source_back() {
    let mut past = doc(false);
    for c in &mut past.layers[0].channels {
        if c.id == 0 {
            c.data.fill(250);
        }
    }
    let source = StampSource::new(layer_surface(&past, "0").unwrap(), (0, 0));
    let out = erase(&doc(false), EraserMode::Brush, Some(source));
    assert_eq!(pixel(&out, 16, 10), [250, 90, 90, 255]);
}

fn split() -> Document {
    // Sky (blue) on the left, a subject (yellow) from x = 20.
    let mut d = doc(false);
    let w = 40;
    for c in &mut d.layers[0].channels {
        let (sky, subject) = match c.id {
            0 => (30, 220),
            1 => (120, 200),
            2 => (220, 40),
            _ => (255, 255),
        };
        for (i, v) in c.data.iter_mut().enumerate() {
            *v = if i % w < 20 { sky } else { subject };
        }
    }
    d
}

fn background_erase(doc: &Document, options: BackgroundEraseOptions, x: f32) -> Document {
    let cfg = StrokeConfig {
        diameter: 16,
        ..StrokeConfig::default()
    };
    let mut stroke = Stroke::begin_kind(doc, "0", cfg, crate::StrokeKind::BackgroundErase(options))
        .expect("begin");
    let mut out = doc.clone();
    stroke.sample(
        &mut out,
        StrokeSample {
            x,
            y: 10.0,
            pressure: 1.0,
        },
    );
    stroke.finish();
    out
}

#[test]
fn the_background_eraser_cuts_out_what_it_sampled() {
    let options = BackgroundEraseOptions {
        tolerance: 40,
        ..BackgroundEraseOptions::default()
    };
    // Crosshair on the sky, close enough that the dab overlaps the subject.
    let out = background_erase(&split(), options, 16.0);
    assert_eq!(pixel(&out, 14, 10)[3], 0, "the sampled sky survived");
    assert_eq!(pixel(&out, 22, 10)[3], 255, "the subject was erased too");
}

#[test]
fn protect_foreground_keeps_its_colour() {
    let options = BackgroundEraseOptions {
        sampling: Sampling::BackgroundSwatch,
        limits: Limits::Discontiguous,
        tolerance: 40,
        protect_foreground: true,
    };
    let sky = Rgba {
        r: 30,
        g: 120,
        b: 220,
        a: 255,
    };
    let cfg = StrokeConfig {
        diameter: 16,
        color: sky,
        background: sky,
        ..StrokeConfig::default()
    };
    let kind = crate::StrokeKind::BackgroundErase(options);
    let mut out = split();
    let mut stroke = Stroke::begin_kind(&out, "0", cfg, kind).unwrap();
    stroke.sample(
        &mut out,
        StrokeSample {
            x: 8.0,
            y: 10.0,
            pressure: 1.0,
        },
    );
    assert!(stroke.finish().is_none(), "a protected colour was erased");
}

#[test]
fn the_magic_eraser_takes_the_masked_region_scaled_by_opacity() {
    let mut d = doc(false);
    let mut mask = vec![0u8; 40 * 20];
    mask[10 * 40 + 5] = 255;
    let dirty = {
        let (width, height) = (d.width, d.height);
        match pictura_core::Coverage::for_canvas(&(TiledMask::from(mask)), width, height) {
            Some(coverage) => magic_erase(&mut d, "0", &coverage, 0.5, TEAL),
            None => None,
        }
    }
    .expect("erased");
    assert_eq!((dirty.left, dirty.top), (5, 10));
    let half = pixel(&d, 5, 10);
    assert!((half[3] as i32 - 128).abs() <= 1, "alpha {}", half[3]);
    assert_eq!(&half[..3], &GREY[..3], "erasing changed the colour");
    assert_eq!(pixel(&d, 6, 10)[3], 255);
}

/// A 512² layer whose planes are tiled uniform, so the erase's uniform
/// fast path applies.
fn uniform_tiled_doc(background: bool) -> Document {
    let (w, h) = (512usize, 512usize);
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let ids: &[(i16, usize)] = if background {
        &[(0, 0), (1, 1), (2, 2)]
    } else {
        &[(0, 0), (1, 1), (2, 2), (-1, 3)]
    };
    doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        channels: ids
            .iter()
            .map(|&(id, c)| Channel {
                id,
                data: Plane::filled(w, h, 1, GREY[c]),
            })
            .collect(),
        background,
        ..Default::default()
    }]);
    doc
}

#[test]
fn a_uniform_erase_writes_uniform_alpha_tiles() {
    let mut d = uniform_tiled_doc(false);
    let mask = TiledMask::filled(512, 512, 1, 255);
    let dirty = {
        let (width, height) = (d.width, d.height);
        match pictura_core::Coverage::for_canvas(&(mask), width, height) {
            Some(coverage) => magic_erase(&mut d, "0", &coverage, 1.0, TEAL),
            None => None,
        }
    }
    .expect("erased");
    assert_eq!(
        dirty,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 512,
            right: 512
        }
    );
    let alpha = d.layers[0]
        .channels
        .iter()
        .find(|c| c.id == -1)
        .expect("an alpha channel");
    for t in 0..alpha.data.tile_count() {
        assert!(
            matches!(
                alpha.data.tile_state(t),
                Some(TileState::Empty | TileState::Uniform(0))
            ),
            "tile {t} was allocated: {:?}",
            alpha.data.tile_state(t)
        );
    }
    assert_eq!(pixel(&d, 300, 300)[3], 0);
}

#[test]
fn a_uniform_erase_matches_the_per_pixel_path() {
    let mut fast = uniform_tiled_doc(false);
    let mut dense = fast.clone();
    let tiled = TiledMask::filled(512, 512, 1, 255);
    // A flat mask has no uniform tile state, so it takes the per-pixel
    // path with the same coverage.
    let flat = TiledMask::from(vec![255u8; 512 * 512]);
    assert!({
        let (width, height) = (fast.width, fast.height);
        match pictura_core::Coverage::for_canvas(&(tiled), width, height) {
            Some(coverage) => magic_erase(&mut fast, "0", &coverage, 0.5, TEAL),
            None => None,
        }
    }
    .is_some());
    assert!({
        let (width, height) = (dense.width, dense.height);
        match pictura_core::Coverage::for_canvas(&(flat), width, height) {
            Some(coverage) => magic_erase(&mut dense, "0", &coverage, 0.5, TEAL),
            None => None,
        }
    }
    .is_some());
    for (a, b) in fast.layers[0]
        .channels
        .iter()
        .zip(&dense.layers[0].channels)
    {
        assert!(a.data == b.data, "channel {}", a.id);
    }
}

#[test]
fn ensure_alpha_gives_a_uniform_plane() {
    let d = doc(true);
    let mut layer = d.layers[0].clone();
    ensure_alpha(&mut layer);
    let alpha = layer
        .channels
        .iter()
        .find(|c| c.id == -1)
        .expect("an alpha channel");
    assert_eq!(alpha.data.tile_state(0), Some(TileState::Uniform(255)));
    assert_eq!(alpha.data.tile_count(), 1, "one uniform tile");
}

#[test]
fn the_magic_eraser_fills_a_locked_layer_with_the_background_colour() {
    let mut d = doc(true);
    let mask = vec![255u8; 40 * 20];
    {
        let (width, height) = (d.width, d.height);
        match pictura_core::Coverage::for_canvas(&(TiledMask::from(mask)), width, height) {
            Some(coverage) => magic_erase(&mut d, "0", &coverage, 1.0, TEAL),
            None => None,
        }
    }
    .expect("filled");
    assert_eq!(pixel(&d, 3, 3), [0, 128, 128, 255]);
    let mut layer = d.layers[0].clone();
    layer.background = false;
    ensure_alpha(&mut layer);
    assert!(
        !erases_to_background(&layer),
        "an alpha channel was not added"
    );
}

#[test]
fn an_antialiased_mask_softens_only_its_edge() {
    let mut mask = vec![0u8; 9 * 9];
    for y in 2..7 {
        for x in 2..7 {
            mask[y * 9 + x] = 255;
        }
    }
    let soft = antialias_mask(&mask, 9);
    assert_eq!(soft[4 * 9 + 4], 255);
    assert!(soft[2 * 9 + 2] < 255 && soft[2 * 9 + 2] >= 128);
    assert_eq!(soft[0], 0);
}

/// The per-pixel 3×3 box the parallel one replaced.
fn antialias_reference(mask: &[u8], w: usize) -> Vec<u8> {
    let h = mask.len() / w;
    let mut out = mask.to_vec();
    for y in 0..h {
        for x in 0..w {
            let (mut sum, mut count) = (0u32, 0u32);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    sum += u32::from(mask[yy * w + xx]);
                    count += 1;
                }
            }
            if mask[y * w + x] > 0 {
                out[y * w + x] = (sum / count).max(128) as u8;
            }
        }
    }
    out
}

#[test]
fn the_parallel_antialias_matches_the_per_pixel_box() {
    let mut seed = 0x2545_f491_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for (w, h) in [(1, 1), (1, 7), (7, 1), (2, 2), (37, 23)] {
        // Solid runs, gaps, and partial coverage, plus a ragged tail.
        let mut mask: Vec<u8> = (0..w * h)
            .map(|_| match next() % 5 {
                0 => 0,
                1 => (next() % 256) as u8,
                _ => 255,
            })
            .collect();
        for v in &mut mask[..w.min(w * h)] {
            *v = 255;
        }
        // A tail shorter than a row is not part of the mask.
        let tail: &[u8] = if w > 1 { &[77] } else { &[] };
        mask.extend_from_slice(tail);
        assert_eq!(
            antialias_mask(&mask, w),
            antialias_reference(&mask[..w * h], w)
                .into_iter()
                .chain(tail.iter().copied())
                .collect::<Vec<_>>(),
            "{w}x{h}"
        );
    }
    assert_eq!(antialias_mask(&[5, 6], 0), vec![5, 6]);
}

/// The Magic Eraser as it was before #1214: the wand over the whole
/// interleaved surface, a dense mask, and a per-pixel erase.
fn magic_erase_reference(
    d: &mut Document,
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
    soften: bool,
) {
    let surface = layer_surface(d, "0").unwrap();
    let (w, h) = (d.width as usize, d.height as usize);
    let at = |p: usize| surface.data[p];
    let target = at(seed.1 as usize * w + seed.0 as usize);
    let near = |p: usize| (0..3).all(|k| at(p)[k].abs_diff(target[k]) <= tolerance);
    let mut flood: Vec<u8> = (0..w * h).map(|p| u8::from(near(p)) * 255).collect();
    if contiguous {
        flood = pictura_core::mask_ops::flood_from(
            w,
            h,
            seed.1 as usize * w + seed.0 as usize,
            |_, to| near(to),
        )
        .unwrap();
    }
    let mask = if soften {
        antialias_mask(&flood, w)
    } else {
        flood
    };
    let layer = &mut d.layers[0];
    let rect = layer.rect;
    let lw = rect.width() as usize;
    let alpha = layer.channels.iter().position(|c| c.id == -1).unwrap();
    let data = layer.channels[alpha].data.as_mut_slice();
    for y in rect.top.max(0)..rect.bottom.min(h as i32) {
        for x in rect.left.max(0)..rect.right.min(w as i32) {
            let cover = mask[y as usize * w + x as usize] as f32 / 255.0 * 0.8;
            if cover <= 0.0 {
                continue;
            }
            let i = (y - rect.top) as usize * lw + (x - rect.left) as usize;
            data[i] = (data[i] as f32 * (1.0 - cover)).round() as u8;
        }
    }
}

#[test]
fn magic_erase_tiled_matches_dense() {
    use crate::bucket::{antialias, magic_flood, BucketSource};
    let (w, h) = (600i32, 420i32);
    let mut base = crate::fill::tests::doc_at((w, h), (-20, 15), [90, 90, 90, 255], false);
    let n = base.layers[0].rect.width() * base.layers[0].rect.height();
    let lw = base.layers[0].rect.width();
    let mut state = 0x77u64;
    let cells: Vec<u8> = (0..64)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            [90u8, 92, 200, 10][(state % 4) as usize]
        })
        .collect();
    for c in &mut base.layers[0].channels[..3] {
        let data: Vec<u8> = (0..n)
            .map(|i| cells[(((i / lw) / 70) * 8 + (i % lw) / 90) as usize % 64])
            .collect();
        c.data = data.into();
    }
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: h,
        right: w,
    };
    for contiguous in [true, false] {
        for soften in [true, false] {
            let mut expected = base.clone();
            magic_erase_reference(&mut expected, (100, 100), 4, contiguous, soften);
            let mut got = base.clone();
            let size = (w as u32, h as u32);
            let source = BucketSource::layer(&got, "0").unwrap();
            let flood = magic_flood(source, size, (100, 100), 4, contiguous).unwrap();
            let mask = if soften {
                antialias(flood, size, canvas, None)
            } else {
                flood
            };
            {
                let (width, height) = (got.width, got.height);
                match pictura_core::Coverage::for_canvas(&(mask), width, height) {
                    Some(coverage) => magic_erase(&mut got, "0", &coverage, 0.8, TEAL),
                    None => None,
                }
            };
            for (a, b) in got.layers[0]
                .channels
                .iter()
                .zip(&expected.layers[0].channels)
            {
                assert!(a.data == b.data, "channel {} {contiguous} {soften}", a.id);
            }
        }
    }
}
