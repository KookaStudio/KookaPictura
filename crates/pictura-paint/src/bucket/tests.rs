use super::*;
use crate::fill::tests::{doc, lock, pixel, RED};
use crate::stamp::layer_surface;
use pictura_core::LockFlags;

const WHITE: [u8; 4] = [255; 4];
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// Two white rooms with a 2 px black wall between them.
fn rooms() -> Document {
    let mut d = doc(40, 20, WHITE);
    for c in &mut d.layers[0].channels[..3] {
        for (i, v) in c.data.iter_mut().enumerate() {
            if (19..21).contains(&(i % 40)) {
                *v = 0;
            }
        }
    }
    d
}

fn canvas(d: &Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: d.height as i32,
        right: d.width as i32,
    }
}

/// The box of the covered pixels of a `w`-wide coverage.
fn bounds_of(coverage: &[u8], w: i32) -> PsdRect {
    let mut b: Option<PsdRect> = None;
    for (i, _) in coverage.iter().enumerate().filter(|(_, &v)| v > 0) {
        let (x, y) = (i as i32 % w, i as i32 / w);
        let px = PsdRect {
            top: y,
            left: x,
            bottom: y + 1,
            right: x + 1,
        };
        b = Some(b.map_or(px, |b| b.union(px)));
    }
    b.expect("a covered pixel")
}

/// Flood layer 0 of `d` over the whole canvas, with no selection.
fn flood_layer(d: &Document, seed: (i32, i32), tolerance: u8, contiguous: bool) -> Option<Vec<u8>> {
    let source = BucketSource::layer(d, "0").unwrap();
    flood(
        source,
        d.width,
        canvas(d),
        None,
        seed,
        tolerance,
        contiguous,
    )
}

/// Flood `d` from `seed` and fill it red.
fn bucket(d: &mut Document, seed: (i32, i32), tolerance: u8, contiguous: bool) {
    let mask = flood_layer(d, seed, tolerance, contiguous).unwrap();
    let red = BucketPaint::Foreground(RED);
    fill(
        d,
        "0",
        &mask,
        red,
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    );
}

/// A `w × h` document whose selection covers the rectangle at `(left, top)`.
fn selected(w: i32, h: i32, left: i32, top: i32, sw: i32, sh: i32) -> (Document, Vec<u8>) {
    let d = doc(w, h, WHITE);
    let mut selection = vec![0u8; (w * h) as usize];
    for y in top..top + sh {
        for x in left..left + sw {
            selection[(y * w + x) as usize] = 255;
        }
    }
    (d, selection)
}

#[test]
fn the_outside_band_stops_at_the_selections_edge() {
    // An 8×1 row: the selection covers the middle four pixels.
    let (mut d, selection) = selected(8, 1, 2, 0, 4, 1);
    stroke_selection(
        &mut d,
        "0",
        &selection,
        bounds_of(&selection, 8),
        RED,
        1,
        StrokeAlign::Outside,
        PaintMode::Normal,
        1.0,
    )
    .expect("outside stroke");
    assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255], "the band's left arm");
    assert_eq!(pixel(&d, 6, 0), [220, 0, 0, 255], "the band's right arm");
    assert_eq!(pixel(&d, 7, 0), WHITE, "the band reached past the edge");
}

#[test]
fn the_inside_band_stops_at_the_selections_edge() {
    let (mut d, selection) = selected(8, 1, 2, 0, 4, 1);
    stroke_selection(
        &mut d,
        "0",
        &selection,
        bounds_of(&selection, 8),
        RED,
        1,
        StrokeAlign::Inside,
        PaintMode::Normal,
        1.0,
    )
    .expect("inside stroke");
    assert_eq!(pixel(&d, 2, 0), [220, 0, 0, 255], "the band's left arm");
    assert_eq!(pixel(&d, 5, 0), [220, 0, 0, 255], "the band's right arm");
    assert_eq!(pixel(&d, 3, 0), WHITE, "the band covered the interior");
    assert_eq!(pixel(&d, 6, 0), WHITE, "the band reached past the edge");
}

#[test]
fn the_band_does_not_reach_the_canvas_edge() {
    // A 10×10 selection with room to its right on a 24×24 canvas.
    let (mut d, selection) = selected(24, 24, 7, 7, 10, 10);
    stroke_selection(
        &mut d,
        "0",
        &selection,
        bounds_of(&selection, 24),
        RED,
        2,
        StrokeAlign::Outside,
        PaintMode::Normal,
        1.0,
    )
    .expect("outside stroke");
    assert_eq!(pixel(&d, 5, 12), [220, 0, 0, 255], "the band's left arm");
    assert_eq!(pixel(&d, 17, 12), [220, 0, 0, 255], "the band's right arm");
    assert_eq!(pixel(&d, 19, 12), WHITE, "the band reached past the edge");
    assert_eq!(pixel(&d, 23, 23), WHITE, "the band reached the corner");
}

#[test]
fn a_contiguous_fill_stops_at_the_wall_and_a_global_one_does_not() {
    let mut d = rooms();
    bucket(&mut d, (5, 10), 32, true);
    assert_eq!(pixel(&d, 5, 10), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 30, 10), WHITE, "the fill leaked past the wall");
    assert_eq!(pixel(&d, 19, 10), BLACK, "the wall itself was filled");

    let mut d = rooms();
    bucket(&mut d, (5, 10), 32, false);
    assert_eq!(pixel(&d, 30, 10), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 20, 10), BLACK);
}

#[test]
fn tolerance_is_inclusive_per_channel() {
    let patched = || {
        let mut d = doc(20, 20, [100, 100, 100, 255]);
        // A patch 20 levels off on one channel only.
        for (i, v) in d.layers[0].channels[1].data.iter_mut().enumerate() {
            if i % 20 >= 10 {
                *v = 120;
            }
        }
        d
    };
    let mut tight = patched();
    bucket(&mut tight, (2, 10), 19, true);
    assert_eq!(pixel(&tight, 15, 10), [100, 120, 100, 255]);
    let mut exact = patched();
    bucket(&mut exact, (2, 10), 20, true);
    assert_eq!(pixel(&exact, 15, 10), [220, 0, 0, 255]);
}

#[test]
fn transparency_is_not_black() {
    // A black outline on an empty layer: the inside fills, the outline and
    // the outside do not.
    let mut d = doc(20, 20, [0; 4]);
    for y in 5..15 {
        for x in 5..15 {
            if x == 5 || x == 14 || y == 5 || y == 14 {
                d.layers[0].channels[3].data[y * 20 + x] = 255;
            }
        }
    }
    bucket(&mut d, (10, 10), 32, true);
    assert_eq!(pixel(&d, 10, 10), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 5, 10), BLACK);
    assert_eq!(pixel(&d, 1, 1)[3], 0);
}

#[test]
fn a_seed_off_the_surface_floods_nothing() {
    let d = rooms();
    assert!(flood_layer(&d, (40, 10), 32, true).is_none());
    assert!(flood_layer(&d, (-1, 10), 32, false).is_none());
}

#[test]
fn an_empty_layer_floods_whole_and_an_enclosed_seed_does_not() {
    let empty = doc(30, 20, [0; 4]);
    for contiguous in [true, false] {
        let all = flood_layer(&empty, (3, 4), 0, contiguous).unwrap();
        assert_eq!(all, vec![255u8; 30 * 20]);
    }
    // An opaque ring: the pixels outside it match the seed inside but
    // are not connected to it.
    let mut d = doc(30, 20, [0; 4]);
    for y in 5..15 {
        for x in 5..15 {
            if x == 5 || x == 14 || y == 5 || y == 14 {
                d.layers[0].channels[3].data[y * 30 + x] = 255;
            }
        }
    }
    let inside = flood_layer(&d, (10, 10), 0, true).unwrap();
    assert_eq!(inside.iter().filter(|&&v| v == 255).count(), 8 * 8);
    assert_eq!(inside[0], 0);
    let global = flood_layer(&d, (10, 10), 0, false).unwrap();
    assert_eq!(global.iter().filter(|&&v| v == 255).count(), 30 * 20 - 36);
}

#[test]
fn opacity_and_the_mask_scale_the_fill() {
    let mut d = doc(4, 1, WHITE);
    let black = BucketPaint::Foreground(Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    let dirty = fill(
        &mut d,
        "0",
        &[255, 128, 0, 0],
        black,
        PaintMode::Normal,
        0.5,
        None,
        Alpha::Blend,
    )
    .expect("filled");
    assert_eq!((dirty.left, dirty.right), (0, 2));
    assert!((pixel(&d, 0, 0)[0] as i32 - 128).abs() <= 1);
    assert!((pixel(&d, 1, 0)[0] as i32 - 191).abs() <= 1);
    assert_eq!(pixel(&d, 2, 0), WHITE);
    assert!(fill(
        &mut d,
        "0",
        &[255; 4],
        black,
        PaintMode::Normal,
        0.0,
        None,
        Alpha::Blend
    )
    .is_none());
    assert!(
        fill(
            &mut d,
            "0",
            &[255; 3],
            black,
            PaintMode::Normal,
            1.0,
            None,
            Alpha::Blend
        )
        .is_none(),
        "a short mask was read"
    );
}

#[test]
fn a_transparency_lock_keeps_the_fill_off_empty_pixels() {
    let mut d = doc(20, 20, [255, 255, 255, 0]);
    for a in &mut d.layers[0].channels[3].data[..200] {
        *a = 255;
    }
    lock(&mut d, LockFlags::TRANSPARENCY);
    let red = BucketPaint::Foreground(RED);
    fill(
        &mut d,
        "0",
        &[255; 400],
        red,
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    )
    .expect("filled");
    assert_eq!(pixel(&d, 10, 5), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 10, 15)[3], 0, "an empty pixel gained coverage");
}

#[test]
fn a_pattern_tiles_from_the_document_origin_inside_the_selection() {
    let tile = RgbaImage {
        width: 2,
        height: 1,
        data: vec![[10, 10, 10, 255], [200, 200, 200, 255]],
    };
    let mut d = doc(6, 2, WHITE);
    let selection = [255, 255, 255, 255, 0, 0].repeat(2);
    fill(
        &mut d,
        "0",
        &[255; 12],
        BucketPaint::Pattern(&tile),
        PaintMode::Normal,
        1.0,
        Some(&selection),
        Alpha::Blend,
    )
    .expect("filled");
    assert_eq!(pixel(&d, 0, 0)[0], 10);
    assert_eq!(pixel(&d, 1, 1)[0], 200);
    assert_eq!(pixel(&d, 2, 1)[0], 10);
    assert_eq!(pixel(&d, 4, 0), WHITE, "the fill escaped the selection");

    let empty = RgbaImage {
        width: 0,
        height: 0,
        data: Vec::new(),
    };
    let paint = BucketPaint::Pattern(&empty);
    assert!(fill(
        &mut d,
        "0",
        &[255; 12],
        paint,
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend
    )
    .is_none());
}

#[test]
fn a_selection_stroke_bands_the_edge_where_asked() {
    let red = RED;
    // An 8×1 row: selection covers the middle four pixels.
    let selection = [0, 0, 255, 255, 255, 255, 0, 0];
    let mut d = doc(8, 1, WHITE);

    // Outside 1 px: the two pixels just outside the selection take the
    // colour; the selection itself is untouched.
    stroke_selection(
        &mut d,
        "0",
        &selection,
        bounds_of(&selection, 8),
        red,
        1,
        StrokeAlign::Outside,
        PaintMode::Normal,
        1.0,
    )
    .expect("outside stroke");
    assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 6, 0), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 2, 0), WHITE, "the selection was painted");
    assert_eq!(pixel(&d, 0, 0), WHITE, "the band spread too far");

    // Inside 1 px: the selection's border takes the colour; its middle and
    // the outside stay white.
    let mut d = doc(8, 1, WHITE);
    stroke_selection(
        &mut d,
        "0",
        &selection,
        bounds_of(&selection, 8),
        red,
        1,
        StrokeAlign::Inside,
        PaintMode::Normal,
        1.0,
    )
    .expect("inside stroke");
    assert_eq!(pixel(&d, 2, 0), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 5, 0), [220, 0, 0, 255]);
    assert_eq!(pixel(&d, 1, 0), WHITE);
}

#[test]
fn a_boxed_fill_matches_the_document_mask_and_forks_only_its_bands() {
    use crate::fill::tests::doc_at;
    let (w, h) = (90i32, 300i32);
    // A soft box crossing two bands, on a layer offset from the canvas
    // and on a Background, both shared with a history snapshot.
    let bounds = PsdRect {
        top: 70,
        left: 12,
        bottom: 140,
        right: 61,
    };
    let bw = bounds.width();
    let boxed: Vec<u8> = (0..bw * bounds.height())
        .map(|i| ((i * 37) % 256) as u8)
        .collect();
    let mut whole = vec![0u8; (w * h) as usize];
    for y in bounds.top..bounds.bottom {
        for x in bounds.left..bounds.right {
            whole[(y * w + x) as usize] = boxed[((y - bounds.top) * bw + x - bounds.left) as usize];
        }
    }
    let red = BucketPaint::Foreground(RED);
    for (origin, background) in [((0, 0), false), ((-5, 10), false), ((0, 0), true)] {
        let base = doc_at((w, h), origin, WHITE, background);
        let mut expected = base.clone();
        let dirty = fill(
            &mut expected,
            "0",
            &whole,
            red,
            PaintMode::Normal,
            0.8,
            None,
            Alpha::Blend,
        );
        let mut history = base.clone();
        for c in &mut history.layers[0].channels {
            c.data.fracture(w as usize);
        }
        let mut got = history.clone();
        let boxed_dirty = fill_within(
            &mut got,
            "0",
            bounds,
            Some(&boxed),
            red,
            PaintMode::Normal,
            0.8,
            None,
            Alpha::Blend,
        );
        assert_eq!(boxed_dirty, dirty, "origin {origin:?}");
        for (a, b) in got.layers[0]
            .channels
            .iter()
            .zip(&expected.layers[0].channels)
        {
            assert_eq!(
                a.data.as_slice(),
                b.data.as_slice(),
                "channel {} at {origin:?}",
                a.id
            );
        }
        for (a, b) in got.layers[0]
            .channels
            .iter()
            .zip(&history.layers[0].channels)
        {
            assert!(a.data.shares_band(4, &b.data), "band 4 stayed shared");
            assert!(!a.data.shares_band(1, &b.data), "band 1 was written");
        }
    }
}

/// A deterministic three-colour noise layer at `origin` on a `w × h` canvas.
fn noise(w: i32, h: i32, origin: (i32, i32), background: bool) -> Document {
    let mut d = crate::fill::tests::doc_at((w, h), origin, WHITE, background);
    let mut state = 0x5eed_u64;
    let n = (w * h) as usize;
    let picks: Vec<u8> = (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) % 3) as u8
        })
        .collect();
    for c in &mut d.layers[0].channels {
        for (v, &k) in c.data.iter_mut().zip(&picks) {
            *v = match (c.id, k) {
                (-1, 1) => 0,
                (-1, _) => 255,
                (_, 0) => 0,
                _ => 255,
            };
        }
    }
    d
}

/// The match test against the whole interleaved surface: the oracle the
/// plane reads must agree with.
fn surface_matches(d: &Document, seed: (i32, i32), tolerance: u8) -> Vec<u8> {
    let surface = layer_surface(d, "0").unwrap();
    let target = surface.get(seed.0, seed.1);
    surface
        .data
        .iter()
        .map(|p| {
            p.iter()
                .zip(&target)
                .all(|(a, b)| a.abs_diff(*b) <= tolerance) as u8
                * 255
        })
        .collect()
}

fn crop(whole: &[u8], w: i32, b: PsdRect) -> Vec<u8> {
    (b.top..b.bottom)
        .flat_map(|y| (b.left..b.right).map(move |x| whole[(y * w + x) as usize]))
        .collect()
}

#[test]
fn a_boxed_flood_reads_the_planes_as_the_surface_does() {
    let (w, h) = (37, 29);
    let bounds = PsdRect {
        top: 4,
        left: 6,
        bottom: 25,
        right: 30,
    };
    for (origin, background) in [((0, 0), false), ((-5, 3), false), ((0, 0), true)] {
        let d = noise(w, h, origin, background);
        let seed = (11, 9);
        let source = BucketSource::layer(&d, "0").unwrap();
        let whole = flood(source, d.width, canvas(&d), None, seed, 0, false).unwrap();
        assert_eq!(whole, surface_matches(&d, seed, 0), "origin {origin:?}");
        let boxed = flood(source, d.width, bounds, None, seed, 0, false).unwrap();
        assert_eq!(boxed, crop(&whole, w, bounds), "origin {origin:?}");
        // A contiguous flood in the box is the whole flood of the box alone.
        let mut seeds = vec![0u8; (bounds.width() * bounds.height()) as usize];
        seeds[((seed.1 - 4) * bounds.width() + seed.0 - 6) as usize] = 1;
        let cropped = crop(&whole, w, bounds);
        let expected = mask_ops::flood(
            bounds.width() as usize,
            bounds.height() as usize,
            &seeds,
            |_, to| cropped[to] != 0,
        );
        let contiguous = flood(source, d.width, bounds, None, seed, 0, true);
        assert_eq!(contiguous, expected, "origin {origin:?}");
    }
}

#[test]
fn a_composite_source_reads_its_planes() {
    let (w, h) = (23u32, 17u32);
    let d = noise(w as i32, h as i32, (0, 0), false);
    let surface = layer_surface(&d, "0").unwrap();
    let mut buf = PixelBuffer::new(w, h, 4);
    let n = (w * h) as usize;
    for (i, px) in surface.data.iter().enumerate() {
        for (k, &v) in px.iter().enumerate() {
            buf.data[k * n + i] = v;
        }
    }
    let seed = (5, 6);
    let all = PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    };
    let got = flood(BucketSource::Composite(&buf), w, all, None, seed, 0, false).unwrap();
    assert_eq!(got, surface_matches(&d, seed, 0));
}

#[test]
fn the_selection_is_a_wall_for_the_contiguous_flood() {
    // Two selected rooms on one white layer: white connects them outside the
    // selection, but the flood may not leave it.
    let (w, h) = (30, 10);
    let d = doc(w, h, WHITE);
    let mut selection = vec![0u8; (w * h) as usize];
    for y in 2..8 {
        for x in (2..8).chain(20..28) {
            selection[(y * w + x) as usize] = 255;
        }
    }
    let bounds = bounds_of(&selection, w);
    let source = BucketSource::layer(&d, "0").unwrap();
    let count = |m: &[u8]| m.iter().filter(|&&v| v == 255).count();
    let left = flood(source, d.width, bounds, Some(&selection), (4, 4), 0, true).unwrap();
    assert_eq!(count(&left), 36, "the flood crossed the unselected gap");
    let both = flood(source, d.width, bounds, Some(&selection), (4, 4), 0, false).unwrap();
    assert_eq!(count(&both), 36 + 48);
    assert!(
        flood(source, d.width, bounds, Some(&selection), (12, 4), 0, true).is_none(),
        "a seed outside the selection floods nothing"
    );
}

#[test]
fn the_selection_edge_is_not_softened() {
    // A uniform layer in a diagonal selection: every selected pixel matches,
    // so the anti-aliased mask is solid wherever the selection is.
    let (w, h) = (20, 20);
    let d = doc(w, h, WHITE);
    let selection: Vec<u8> = (0..w * h)
        .map(|i| {
            if i % w + i / w < 25 && i % w > 2 {
                255
            } else {
                0
            }
        })
        .collect();
    let bounds = bounds_of(&selection, w);
    let source = BucketSource::layer(&d, "0").unwrap();
    let mask = flood(source, d.width, bounds, Some(&selection), (5, 5), 0, true).unwrap();
    let soft = antialias(mask, d.width, bounds, Some(&selection));
    let selected = crop(&selection, w, bounds);
    for (i, (&m, &s)) in soft.iter().zip(&selected).enumerate() {
        if s > 0 {
            assert_eq!(m, 255, "box pixel {i} was softened");
        }
    }
}

#[test]
fn a_fill_without_a_mask_covers_its_box() {
    let bounds = PsdRect {
        top: 70,
        left: 12,
        bottom: 140,
        right: 61,
    };
    let full = vec![255u8; (bounds.width() * bounds.height()) as usize];
    let red = BucketPaint::Foreground(RED);
    let mut a = doc(90, 300, WHITE);
    let mut b = a.clone();
    let da = fill_within(
        &mut a,
        "0",
        bounds,
        None,
        red,
        PaintMode::Normal,
        0.7,
        None,
        Alpha::Blend,
    );
    let db = fill_within(
        &mut b,
        "0",
        bounds,
        Some(&full),
        red,
        PaintMode::Normal,
        0.7,
        None,
        Alpha::Blend,
    );
    assert_eq!(da, Some(bounds));
    assert_eq!(da, db);
    for (x, y) in a.layers[0].channels.iter().zip(&b.layers[0].channels) {
        assert_eq!(x.data.as_slice(), y.data.as_slice());
    }
}

/// The stroke band as it was built before #1207: over the whole document.
fn whole_document_stroke(
    d: &mut Document,
    selection: &[u8],
    width: u32,
    position: StrokeAlign,
) -> Option<PsdRect> {
    let (w, h) = (d.width as usize, d.height as usize);
    let radius = width as usize;
    let (out_r, in_r) = match position {
        StrokeAlign::Outside => (radius, 0),
        StrokeAlign::Inside => (0, radius),
        StrokeAlign::Center => (radius.div_ceil(2), radius / 2),
    };
    let dilated = mask_ops::dilate(selection, w, h, out_r, Border::Clamp);
    let eroded = mask_ops::erode(selection, w, h, in_r, Border::Clamp);
    let band: Vec<u8> = selection
        .iter()
        .zip(&dilated)
        .zip(&eroded)
        .map(|((&m, &dv), &e)| match position {
            StrokeAlign::Outside => dv.saturating_sub(m),
            StrokeAlign::Inside => m.saturating_sub(e),
            StrokeAlign::Center => dv.saturating_sub(e),
        })
        .collect();
    fill(
        d,
        "0",
        &band,
        BucketPaint::Foreground(RED),
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    )
}

#[test]
fn a_windowed_stroke_equals_the_whole_document_stroke() {
    let (w, h) = (48, 40);
    // Inside the canvas, against its left and top edges, and a soft ellipse.
    let rect = |l: i32, t: i32, r: i32, b: i32| -> Vec<u8> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if (l..r).contains(&x) && (t..b).contains(&y) {
                    255
                } else {
                    0
                }
            })
            .collect()
    };
    let ellipse: Vec<u8> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f32 - 30.0, (i / w) as f32 - 22.0);
            let r = (x * x / 81.0 + y * y / 49.0).sqrt();
            ((1.5 - r).clamp(0.0, 1.0) * 255.0) as u8
        })
        .collect();
    for selection in [rect(14, 12, 30, 25), rect(0, 0, 9, 7), ellipse] {
        let bounds = bounds_of(&selection, w);
        for position in [
            StrokeAlign::Inside,
            StrokeAlign::Center,
            StrokeAlign::Outside,
        ] {
            for width in [1u32, 3, 6] {
                let mut expected = doc(w, h, WHITE);
                let mut got = expected.clone();
                let dirty_expected =
                    whole_document_stroke(&mut expected, &selection, width, position);
                let dirty = stroke_selection(
                    &mut got,
                    "0",
                    &selection,
                    bounds,
                    RED,
                    width,
                    position,
                    PaintMode::Normal,
                    1.0,
                );
                assert_eq!(dirty, dirty_expected, "{position:?} {width}");
                for (a, b) in got.layers[0]
                    .channels
                    .iter()
                    .zip(&expected.layers[0].channels)
                {
                    assert_eq!(a.data.as_slice(), b.data.as_slice(), "{position:?} {width}");
                }
            }
        }
    }
}
