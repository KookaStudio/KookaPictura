use super::*;

fn sel_from(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> Selection {
    let mut s = Selection::none(w, h);
    for y in 0..h {
        for x in 0..w {
            s.data[(y * w + x) as usize] = f(x, y);
        }
    }
    s
}

fn rgb_image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> PixelBuffer {
    let n = w as usize * h as usize;
    let mut img = PixelBuffer::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            let p = (y * w + x) as usize;
            let c = f(x, y);
            img.data[p] = c[0];
            img.data[n + p] = c[1];
            img.data[2 * n + p] = c[2];
        }
    }
    img
}

fn rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Selection {
    sel_from(w, h, |x, y| {
        if x >= x0 && x < x1 && y >= y0 && y < y1 {
            255
        } else {
            0
        }
    })
}

#[test]
fn combine_identities() {
    let a = rect(6, 5, 1, 1, 5, 4);
    let none = Selection::none(6, 5);

    let mut t = a.clone();
    t.combine(&none, SelectOp::Add).unwrap();
    assert_eq!(t, a, "A Add none = A");

    let mut t = a.clone();
    t.combine(&none, SelectOp::Intersect).unwrap();
    assert_eq!(t, none, "A Intersect none = none");

    let mut t = a.clone();
    t.combine(&a, SelectOp::Subtract).unwrap();
    assert_eq!(t, none, "A Subtract A = none");

    let mut t = a.clone();
    t.combine(&a, SelectOp::Intersect).unwrap();
    assert_eq!(t, a, "A Intersect A = A (binary)");

    let mut t = Selection::all(6, 5);
    t.combine(&a, SelectOp::Replace).unwrap();
    assert_eq!(t, a, "Replace = b");
}

#[test]
fn combine_soft_bounds() {
    let a = sel_from(8, 8, |x, _| (x * 17) as u8);
    let b = sel_from(8, 8, |_, y| (y * 23) as u8);

    let mut add = a.clone();
    add.combine(&b, SelectOp::Add).unwrap();
    for p in 0..64 {
        assert_eq!(add.data[p], a.data[p].max(b.data[p]));
        assert!(add.data[p] >= a.data[p] && add.data[p] >= b.data[p]);
    }

    let mut sub = a.clone();
    sub.combine(&b, SelectOp::Subtract).unwrap();
    for p in 0..64 {
        assert_eq!(sub.data[p], a.data[p].saturating_sub(b.data[p]));
        assert!(sub.data[p] <= a.data[p]);
    }

    let mut inter = a.clone();
    inter.combine(&b, SelectOp::Intersect).unwrap();
    for p in 0..64 {
        assert!(inter.data[p] <= a.data[p].min(b.data[p]) + 1);
    }
}

#[test]
fn combine_size_mismatch_errors() {
    let mut a = Selection::none(4, 4);
    let b = Selection::none(5, 4);
    assert!(a.combine(&b, SelectOp::Add).is_err());
}

#[test]
fn rect_selects_half_open_and_clips() {
    let s = Selection::rect(6, 5, 1, 1, 3, 2);
    assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 6);
    assert_eq!(s.data[7], 255);
    assert_eq!(s.data[(2 * 6 + 3) as usize], 255);
    assert_eq!(s.data[10], 0, "right edge is exclusive");
    assert_eq!(s.data[1], 0, "above the rect");
    assert_eq!(s.data[(3 * 6 + 1) as usize], 0, "below the rect");

    assert_eq!(
        Selection::rect(6, 5, 0, 0, 0, 2),
        Selection::none(6, 5),
        "zero width is empty"
    );
    let clipped = Selection::rect(6, 5, -2, -2, 4, 4);
    assert_eq!(clipped.data.iter().filter(|&&v| v > 0).count(), 4);
    assert_eq!(clipped.data[0], 255);
    assert_eq!(clipped.data[7], 255);
    assert_eq!(clipped.data[8], 0);
}

#[test]
fn rect_extreme_coords_do_not_overflow() {
    let s = Selection::rect(4, 4, 1, 1, i32::MAX, i32::MAX);
    assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 9);
}

#[test]
fn ellipse_is_inscribed_and_clipped() {
    let s = Selection::ellipse(4, 4, 0, 0, 4, 4);
    assert_eq!(s.data.iter().filter(|&&v| v > 0).count(), 12);
    assert_eq!(s.data[0], 0, "top-left corner is outside");
    assert_eq!(s.data[(3 * 4 + 3) as usize], 0, "bottom-right corner out");
    assert_eq!(s.data[5], 255, "centre is inside");
    assert_eq!(s.data[1], 255, "top edge midpoint in");
    assert_eq!(s.data[(3 * 4 + 2) as usize], 255, "bottom edge midpoint in");

    assert_eq!(Selection::ellipse(4, 4, 0, 0, 0, 4), Selection::none(4, 4));
}

#[test]
fn polygon_even_odd_fill_and_degenerate() {
    let square = Selection::polygon(4, 4, &[(0, 0), (4, 0), (4, 4), (0, 4)]);
    assert_eq!(square.data.iter().filter(|&&v| v > 0).count(), 16);

    let triangle = Selection::polygon(4, 4, &[(0, 0), (4, 0), (0, 4)]);
    assert_eq!(triangle.data.iter().filter(|&&v| v > 0).count(), 6);
    assert_eq!(triangle.data[0], 255);
    assert_eq!(triangle.data[7], 0);
    assert_eq!(triangle.data[(3 * 4 + 3) as usize], 0);

    assert_eq!(
        Selection::polygon(4, 4, &[(0, 0), (1, 1)]),
        Selection::none(4, 4),
        "fewer than three points is empty"
    );

    let over = Selection::polygon(4, 4, &[(-2, -2), (6, -2), (6, 6), (-2, 6)]);
    assert_eq!(over.data.iter().filter(|&&v| v > 0).count(), 16);
}

#[test]
fn combine_with_each_mode() {
    let a = Selection::rect(4, 4, 0, 0, 3, 3);
    let b = Selection::rect(4, 4, 1, 1, 3, 3);
    let count = |s: &Selection| s.data.iter().filter(|&&v| v > 0).count();

    let mut new = a.clone();
    new.combine_with(&b, CombineMode::New);
    assert_eq!(count(&new), 9, "New replaces");
    assert_eq!(new, b);

    let mut add = a.clone();
    add.combine_with(&b, CombineMode::Add);
    assert_eq!(count(&add), 14, "Add is union");

    let mut sub = a.clone();
    sub.combine_with(&b, CombineMode::Subtract);
    assert_eq!(count(&sub), 5, "Subtract removes the overlap");

    let mut inter = a.clone();
    inter.combine_with(&b, CombineMode::Intersect);
    assert_eq!(count(&inter), 4, "Intersect is the overlap");
    assert_eq!(inter.data[5], 255);
    assert_eq!(inter.data[10], 255);
    assert_eq!(inter.data[0], 0);
}

#[test]
fn channel_round_trips_a_selection() {
    let sel = rect(5, 3, 1, 1, 4, 2);
    let ch = sel.to_channel(-4);
    assert_eq!(ch.id, -4);
    assert_eq!(ch.data, sel.data);
    assert_eq!(Selection::from_channel(&ch, 5, 3).unwrap(), sel);
}

#[test]
fn from_channel_wrong_length_errors() {
    let ch = Channel {
        id: 0,
        data: vec![0; 5].into(),
    };
    assert!(Selection::from_channel(&ch, 4, 4).is_err());
}

#[test]
fn layer_alpha_selection_offsets_and_fills() {
    use pictura_core::{Layer, PsdRect};
    let mut layer = Layer {
        rect: PsdRect {
            top: 1,
            left: 2,
            bottom: 3,
            right: 5,
        }, // 3x2
        ..Default::default()
    };
    // No alpha channel -> fully opaque inside the rect, zero outside.
    let at = |x: usize, y: usize| y * 6 + x;
    let sel = selection_from_layer_alpha(&layer, 6, 4);
    assert_eq!(sel.data.iter().filter(|&&v| v == 255).count(), 6);
    assert_eq!(sel.data[at(2, 1)], 255);
    assert_eq!(sel.data[at(2, 0)], 0, "outside the rect");
    assert_eq!(sel.data[at(5, 1)], 0, "right edge exclusive");

    // Alpha channel -> layer-local coverage copied to the document offset.
    layer.channels = vec![Channel {
        id: -1,
        data: vec![0, 64, 255, 128, 0, 255].into(),
    }];
    let sel = selection_from_layer_alpha(&layer, 6, 4);
    assert_eq!(sel.data[at(2, 1)], 0);
    assert_eq!(sel.data[at(3, 1)], 64);
    assert_eq!(sel.data[at(4, 1)], 255);
    assert_eq!(sel.data[at(2, 2)], 128);
    assert_eq!(sel.data[at(3, 2)], 0);
    assert_eq!(sel.data[at(4, 2)], 255);
}

#[test]
fn mask_selection_uses_the_coverage_rule() {
    use pictura_core::{LayerMask, PsdRect};
    let rect = PsdRect {
        top: 1,
        left: 2,
        bottom: 3,
        right: 5,
    }; // 3x2
    let at = |x: usize, y: usize| y * 6 + x;
    let mask = LayerMask {
        rect,
        default_color: 0,
        data: Some(vec![0, 64, 255, 128, 0, 255].into()),
        ..Default::default()
    };
    let sel = selection_from_mask(&mask, 6, 4);
    assert_eq!(sel.data[at(2, 1)], 0);
    assert_eq!(sel.data[at(3, 1)], 64);
    assert_eq!(sel.data[at(4, 1)], 255);
    assert_eq!(sel.data[at(2, 2)], 128);
    assert_eq!(sel.data[at(4, 2)], 255);
    assert_eq!(sel.data[at(2, 0)], 0, "outside the rect uses default_color");
    assert_eq!(sel.data[at(5, 1)], 0, "right edge exclusive");
    // default_color fills outside the rect.
    let mask = LayerMask {
        default_color: 200,
        ..mask.clone()
    };
    let sel = selection_from_mask(&mask, 6, 4);
    assert_eq!(sel.data[at(0, 0)], 200);
    assert_eq!(sel.data[at(3, 1)], 64, "inside still uses the pixels");

    // A disabled or data-less mask selects the whole document.
    let disabled = LayerMask {
        disabled: true,
        ..mask.clone()
    };
    assert_eq!(selection_from_mask(&disabled, 6, 4), Selection::all(6, 4));
    let dataless = LayerMask { data: None, ..mask };
    assert_eq!(selection_from_mask(&dataless, 6, 4), Selection::all(6, 4));
}

#[test]
fn invert_involution() {
    let a = sel_from(7, 3, |x, y| ((x * 31 + y * 7) % 256) as u8);
    assert_eq!(a.invert().invert(), a);
    assert_eq!(Selection::none(4, 2).invert(), Selection::all(4, 2));
    assert_eq!(Selection::all(4, 2).invert(), Selection::none(4, 2));
}

#[test]
fn feather_zero_is_identity() {
    let a = rect(9, 5, 2, 1, 7, 4);
    assert_eq!(a.feather(0.0), a);
    assert_eq!(a.feather(-3.0), a);
}

#[test]
fn feather_makes_ramp() {
    let a = rect(16, 8, 0, 0, 8, 8);
    let f = a.feather(4.0);
    assert_ne!(f, a);
    let has_partial = f.data.iter().any(|&v| v > 0 && v < 255);
    assert!(has_partial, "hard edge should soften into a ramp");
    for y in 0..8 {
        for x in 1..16 {
            assert!(
                f.data[(y * 16 + x) as usize] <= f.data[(y * 16 + x - 1) as usize],
                "coverage must fall monotonically across the edge"
            );
        }
    }
}

#[test]
fn expand_contract_roundtrip() {
    let a = rect(24, 20, 5, 4, 16, 15);
    assert_eq!(a.expand(0), a);
    assert_eq!(a.contract(0), a);
    assert_eq!(a.expand(3).contract(3), a);
}

#[test]
fn border_is_centered_band() {
    let a = rect(24, 24, 8, 8, 16, 16);
    let b = a.border(6);
    assert_eq!(b.data[(12 * 24 + 12) as usize], 0, "interior stays clear");
    assert_eq!(b.data[(8 * 24 + 12) as usize], 255, "edge is selected");
    assert_eq!(b.data[(6 * 24 + 12) as usize], 255, "band extends outside");
    assert_eq!(b.data[(4 * 24 + 12) as usize], 0, "far outside stays clear");
    assert_eq!(b.data[(10 * 24 + 12) as usize], 255, "band extends inside");
    assert_eq!(Selection::none(4, 4).border(5).data.len(), 16);
}

#[test]
fn contract_all_shrinks() {
    let s = Selection::all(12, 8).contract(2);
    for x in 0..12 {
        assert_eq!(s.data[x], 0, "top row");
        assert_eq!(s.data[7 * 12 + x], 0, "bottom row");
    }
    for y in 0..8 {
        assert_eq!(s.data[y * 12], 0, "left column");
        assert_eq!(s.data[y * 12 + 11], 0, "right column");
    }
    assert_eq!(s.data[4 * 12 + 6], 255, "centre stays selected");
}

#[test]
fn border_edge_has_inner_band() {
    let b = Selection::all(12, 8).border(4);
    assert_eq!(b.data[0], 255, "outer edge row is in the band");
    assert_eq!(b.data[12 + 6], 255, "inner half row is in the band");
    assert_eq!(b.data[2 * 12 + 6], 0, "inside the band is clear");

    let s = Selection::rect(16, 16, 4, 4, 12, 12);
    let b = s.border(4);
    assert_eq!(b.data[8 * 16 + 2], 255, "outer half");
    assert_eq!(b.data[8 * 16 + 4], 255, "edge");
    assert_eq!(b.data[8 * 16 + 5], 255, "inner half");
    assert_eq!(b.data[8 * 16 + 6], 0, "beyond the band");
}

#[test]
fn smooth_removes_isolated_and_keeps_majority() {
    let mut isolated = Selection::none(9, 9);
    isolated.data[(4 * 9 + 4) as usize] = 255;
    let smoothed = isolated.smooth(1);
    assert_eq!(
        smoothed.data[(4 * 9 + 4) as usize],
        0,
        "isolated pixel removed"
    );

    let mut full = Selection::all(11, 11);
    full.data[(5 * 11 + 5) as usize] = 0;
    let filled = full.smooth(1);
    assert_eq!(filled.data[(5 * 11 + 5) as usize], 255, "lone hole filled");
    assert_eq!(filled.data[(3 * 11 + 3) as usize], 255, "interior kept");

    let block = rect(9, 9, 2, 2, 7, 7);
    assert_eq!(block.smooth(1).data[(4 * 9 + 4) as usize], 255);
    assert_eq!(block.smooth(0), block);
}

#[test]
fn magic_wand_contiguous_and_global() {
    let img = rgb_image(8, 8, |x, _| if x < 4 { [255, 0, 0] } else { [0, 0, 255] });
    let red = magic_wand(&img, 1, 1, 10, true).unwrap();
    for y in 0..8 {
        for x in 0..8 {
            let expected = if x < 4 { 255 } else { 0 };
            assert_eq!(red.data[(y * 8 + x) as usize], expected);
        }
    }
    let blue = magic_wand(&img, 6, 6, 10, false).unwrap();
    assert_eq!(blue.data[0], 0);
    assert_eq!(blue.data[(3 * 8 + 6) as usize], 255);
}

#[test]
fn magic_wand_rejects_tolerance_and_bounds() {
    let img = rgb_image(4, 4, |_, _| [100, 100, 100]);
    let picky = magic_wand(&img, 0, 0, 5, true).unwrap();
    assert_eq!(picky, Selection::all(4, 4));

    let img2 = rgb_image(4, 4, |x, _| {
        if x == 0 {
            [100, 100, 100]
        } else {
            [140, 100, 100]
        }
    });
    let strict = magic_wand(&img2, 0, 0, 10, true).unwrap();
    assert_eq!(strict.data[0], 255);
    assert_eq!(strict.data[1], 0, "colour outside tolerance is rejected");

    assert!(magic_wand(&img, 9, 9, 0, true).is_err());
}

#[test]
fn grow_is_contiguous_similar_is_global() {
    let img = rgb_image(12, 4, |x, _| {
        if x < 3 || (8..11).contains(&x) {
            [10, 200, 10]
        } else {
            [200, 10, 10]
        }
    });
    let seed = sel_from(12, 4, |x, y| if x == 1 && y == 2 { 255 } else { 0 });

    let grown = grow(&seed, &img, 20).unwrap();
    assert_eq!(grown.data[0], 255, "grows within first patch");
    assert_eq!(grown.data[3], 0, "stops at different colour");
    assert_eq!(grown.data[9], 0, "does not jump globally");

    let sim = similar(&seed, &img, 20).unwrap();
    assert_eq!(sim.data[9], 255, "similar reaches far patch");
    assert_eq!(sim.data[4], 0, "similar rejects other colour");
}

#[test]
fn grow_similar_size_mismatch_errors() {
    let img = rgb_image(4, 4, |_, _| [0, 0, 0]);
    let sel = Selection::none(5, 4);
    assert!(grow(&sel, &img, 0).is_err());
    assert!(similar(&sel, &img, 0).is_err());
}

#[test]
fn similar_matches_the_per_colour_scan() {
    let (w, h) = (64u32, 64u32);
    let colour = |x: u32, y: u32| {
        [
            (x * 5 % 256) as u8,
            (y * 7 % 256) as u8,
            ((x * 11 + y * 13) % 256) as u8,
        ]
    };
    let img = rgb_image(w, h, colour);
    let sel = sel_from(w, h, |x, y| {
        if (8..56).contains(&x) && (8..56).contains(&y) {
            255
        } else {
            0
        }
    });
    let got = similar(&sel, &img, 17).unwrap();
    let mut references = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if sel.data[(y * w + x) as usize] > 0 {
                references.push(colour(x, y));
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let p = (y * w + x) as usize;
            let expected = if sel.data[p] > 0 {
                sel.data[p]
            } else if references
                .iter()
                .any(|reference| chebyshev(*reference, colour(x, y)) <= 17)
            {
                255
            } else {
                0
            };
            assert_eq!(got.data[p], expected, "({x},{y})");
        }
    }
}

#[test]
fn similar_zero_tolerance_selects_only_the_exact_colour() {
    let img = rgb_image(6, 3, |x, _| if x < 3 { [12, 34, 56] } else { [12, 34, 57] });
    let seed = sel_from(6, 3, |x, y| if x == 0 && y == 1 { 255 } else { 0 });
    let out = similar(&seed, &img, 0).unwrap();
    for y in 0..3 {
        for x in 0..6 {
            let expected = if x < 3 { 255 } else { 0 };
            assert_eq!(out.data[(y * 6 + x) as usize], expected, "({x},{y})");
        }
    }
}

#[test]
fn similar_preserves_soft_selected_coverage() {
    let img = rgb_image(6, 6, |_, _| [40, 40, 40]);
    let mut sel = Selection::none(6, 6);
    sel.data[2 * 6 + 2] = 128;
    let out = similar(&sel, &img, 0).unwrap();
    assert_eq!(out.data[2 * 6 + 2], 128, "soft seed coverage survives");
    assert_eq!(out.data[0], 255, "an in-tolerance pixel is fully selected");
}

#[test]
fn similar_with_an_empty_selection_adds_nothing() {
    let img = rgb_image(5, 5, |x, y| [(x * 20) as u8, (y * 20) as u8, 7]);
    let empty = Selection::none(5, 5);
    assert_eq!(similar(&empty, &img, 255).unwrap(), empty);
}

#[test]
fn grow_preserves_soft_selected_coverage() {
    let img = rgb_image(8, 4, |_, _| [90, 90, 90]);
    let mut sel = Selection::none(8, 4);
    sel.data[8 + 1] = 128;
    let out = grow(&sel, &img, 0).unwrap();
    assert_eq!(out.data[8 + 1], 128, "the seed keeps its coverage");
    assert_eq!(out.data[0], 255, "the connected region grows");
}

#[test]
fn parameters_are_clamped_not_panicking() {
    let a = rect(8, 8, 1, 1, 7, 7);
    assert_eq!(a.expand(10_000).data.len(), 64);
    assert_eq!(a.contract(10_000).data.len(), 64);
    assert_eq!(a.border(10_000).data.len(), 64);
    assert_eq!(a.smooth(10_000).data.len(), 64);
    assert_eq!(a.feather(f64::MAX).data.len(), 64);
    assert_eq!(a.feather(1e9).data.len(), 64);
}

/// The per-pixel definitions the bounded `rect`/`ellipse` fills must match.
fn reference_shape(w: u32, h: u32, f: impl Fn(i64, i64) -> bool) -> Selection {
    sel_from(w, h, |x, y| if f(x as i64, y as i64) { 255 } else { 0 })
}

#[test]
fn bounded_shapes_match_the_per_pixel_definition() {
    let (w, h) = (23u32, 17u32);
    let boxes = [
        (2, 3, 9, 7),
        (-4, -5, 12, 9),
        (15, 10, 20, 20),
        (0, 0, 23, 17),
        (-30, 2, 60, 5),
        (5, 5, 1, 1),
        (7, 2, 2, 13),
        (30, 30, 4, 4),
    ];
    for (x, y, bw, bh) in boxes {
        let rect = reference_shape(w, h, |px, py| {
            px >= x as i64 && px < (x + bw) as i64 && py >= y as i64 && py < (y + bh) as i64
        });
        assert_eq!(
            Selection::rect(w, h, x, y, bw, bh),
            rect,
            "rect {x},{y} {bw}x{bh}"
        );
        let (rx, ry) = (bw as f64 / 2.0, bh as f64 / 2.0);
        let (cx, cy) = (x as f64 + rx, y as f64 + ry);
        let ellipse = reference_shape(w, h, |px, py| {
            let dx = (px as f64 + 0.5 - cx) / rx;
            let dy = (py as f64 + 0.5 - cy) / ry;
            dx * dx + dy * dy <= 1.0
        });
        assert_eq!(
            Selection::ellipse(w, h, x, y, bw, bh),
            ellipse,
            "ellipse {x},{y} {bw}x{bh}"
        );
    }
}

#[test]
fn row_copy_translate_matches_the_per_pixel_shift() {
    let src = sel_from(11, 7, |x, y| ((x * 37 + y * 11) % 256) as u8);
    for (dx, dy) in [
        (3, 2),
        (-4, 1),
        (0, -3),
        (10, 6),
        (-11, 0),
        (5, -7),
        (20, 1),
    ] {
        let expected = sel_from(11, 7, |x, y| {
            let (sx, sy) = (x as i64 - dx as i64, y as i64 - dy as i64);
            if (0..11).contains(&sx) && (0..7).contains(&sy) {
                src.data[(sy * 11 + sx) as usize]
            } else {
                0
            }
        });
        assert_eq!(src.translate(dx, dy), expected, "shift {dx},{dy}");
    }
}
