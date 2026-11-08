//! Tiled selections against the dense implementations (#1214).

use pictura_core::mask_ops::{self, Border};
use pictura_core::{assert_no_materialization, PixelBuffer, Plane, TileState, TILE};

use crate::{blur, wand, SelectOp, Selection};

fn next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// A soft `w × h` mask of random blocks of `levels`, a speckled quarter, so
/// it has `Empty`, `Uniform` and dense tiles once tiled.
fn soft(w: usize, h: usize, levels: &[u8], seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    let block = 173;
    let (bw, bh) = (w.div_ceil(block), h.div_ceil(block));
    let cells: Vec<u8> = (0..bw * bh)
        .map(|_| levels[(next(&mut state) % levels.len() as u64) as usize])
        .collect();
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            if (x / 120 + y / 90) % 5 == 0 {
                (next(&mut state) & 0xff) as u8
            } else {
                cells[(y / block) * bw + x / block]
            }
        })
        .collect()
}

fn tiled(data: &[u8], w: usize, h: usize) -> Selection {
    let mut plane = Plane::from(data.to_vec());
    assert!(plane.tile(w, h));
    Selection {
        width: w as u32,
        height: h as u32,
        data: plane,
    }
}

fn states(sel: &Selection) -> Vec<TileState<u8>> {
    (0..sel.data.tile_count())
        .map(|t| sel.data.tile_state(t).expect("tiled"))
        .collect()
}

#[test]
fn tiled_mask_marquee_interior_is_uniform() {
    let (w, h) = (4250usize, 4250usize);
    let cols = w.div_ceil(TILE);
    let (x, y, rw, rh) = (300i32, 500i32, 3000i32, 2000i32);
    for (sel, ellipse) in [
        (Selection::rect(w as u32, h as u32, x, y, rw, rh), false),
        (Selection::ellipse(w as u32, h as u32, x, y, rw, rh), true),
    ] {
        assert!(sel.data.is_tiled());
        for (t, state) in states(&sel).into_iter().enumerate() {
            let (tx, ty) = ((t % cols * TILE) as i32, (t / cols * TILE) as i32);
            let (tx1, ty1) = (tx + TILE as i32, ty + TILE as i32);
            let outside = tx1 <= x || ty1 <= y || tx >= x + rw || ty >= y + rh;
            let inside_box = tx >= x && ty >= y && tx1 <= x + rw && ty1 <= y + rh;
            if outside {
                assert_eq!(state, TileState::Empty, "tile {t} outside");
            } else if inside_box && !ellipse {
                assert_eq!(state, TileState::Uniform(255), "tile {t} inside");
            }
        }
        // The ellipse's centre tile is inside the shape whole.
        let centre = ((y + rh / 2) as usize / TILE) * cols + (x + rw / 2) as usize / TILE;
        assert_eq!(sel.data.tile_state(centre), Some(TileState::Uniform(255)));
        let dense = states(&sel)
            .iter()
            .filter(|s| matches!(s, TileState::Dense(_)))
            .count();
        assert!(dense < 64, "only edge tiles are dense: {dense}");
    }
    // The bytes are the per-pixel shapes.
    let (sw, sh) = (700u32, 650u32);
    let rect = Selection::rect(sw, sh, 37, 290, 600, 333);
    let ellipse = Selection::ellipse(sw, sh, -20, 41, 690, 580);
    let (rx, ry) = (690.0f64 / 2.0, 580.0f64 / 2.0);
    let (cx, cy) = (-20.0 + rx, 41.0 + ry);
    for py in 0..sh as usize {
        for px in 0..sw as usize {
            let i = py * sw as usize + px;
            let in_rect = (37..637).contains(&px) && (290..623).contains(&py);
            assert_eq!(rect.data.sample(i), Some(u8::from(in_rect) * 255));
            let dx = (px as f64 + 0.5 - cx) / rx;
            let dy = (py as f64 + 0.5 - cy) / ry;
            let in_ellipse = dx * dx + dy * dy <= 1.0;
            assert_eq!(
                ellipse.data.sample(i),
                Some(u8::from(in_ellipse) * 255),
                "({px},{py})"
            );
        }
    }
}

#[test]
fn tiled_polygon_matches_dense() {
    let (w, h) = (900u32, 700u32);
    let mut state = 77u64;
    for _ in 0..6 {
        let points: Vec<(i32, i32)> = (0..9)
            .map(|_| {
                (
                    (next(&mut state) % 1100) as i32 - 100,
                    (next(&mut state) % 900) as i32 - 100,
                )
            })
            .collect();
        let (sel, bounds) = Selection::polygon_bounded(w, h, &points);
        let want = crate::polygon::tests::reference(w, h, &points);
        assert!(sel.data == want, "polygon {points:?}");
        assert_eq!(bounds, Selection::from_dense(w, h, want).bounds());
        let cols = (w as usize).div_ceil(TILE);
        if let Some(b) = bounds {
            for (t, s) in states(&sel).into_iter().enumerate() {
                let (tx, ty) = ((t % cols * TILE) as i32, (t / cols * TILE) as i32);
                let off = tx + TILE as i32 <= b.left
                    || ty + TILE as i32 <= b.top
                    || tx >= b.right
                    || ty >= b.bottom;
                if off {
                    assert_eq!(s, TileState::Empty, "tile {t} off the polygon");
                }
            }
        }
    }
}

impl Selection {
    fn from_dense(w: u32, h: u32, data: Vec<u8>) -> Selection {
        Selection {
            width: w,
            height: h,
            data: data.into(),
        }
    }
}

#[test]
fn tiled_boolean_ops_match_dense() {
    let (w, h) = (700usize, 610usize);
    let a = soft(w, h, &[0, 0, 255, 255, 90], 3);
    let b = soft(w, h, &[0, 255, 255, 40], 5);
    type Op = (SelectOp, fn(u8, u8) -> u8);
    let ops: [Op; 4] = [
        (SelectOp::Replace, |_, b| b),
        (SelectOp::Add, u8::max),
        (SelectOp::Subtract, u8::saturating_sub),
        (SelectOp::Intersect, |a, b| {
            (((a as u16) * (b as u16) + 127) / 255) as u8
        }),
    ];
    for (op, f) in ops {
        let want: Vec<u8> = a.iter().zip(&b).map(|(&x, &y)| f(x, y)).collect();
        let mut got = tiled(&a, w, h);
        got.combine(&tiled(&b, w, h), op).unwrap();
        assert!(got.data.is_tiled());
        assert!(got.data == want, "{op:?}");
        // A flat operand gives the same bytes.
        let mut flat = Selection::from_dense(w as u32, h as u32, a.clone());
        flat.combine(&Selection::from_dense(w as u32, h as u32, b.clone()), op)
            .unwrap();
        assert!(flat.data == want, "{op:?} flat");
    }
    let inverted: Vec<u8> = a.iter().map(|v| 255 - v).collect();
    assert!(tiled(&a, w, h).invert().data == inverted);
    // Uniform tiles stay uniform: Select All minus nothing, inverted.
    let all = Selection::all(w as u32, h as u32);
    let none = all.invert();
    assert!(states(&none).iter().all(|s| *s == TileState::Empty));
    for (dx, dy) in [(0, 0), (37, -300), (-256, 256), (900, 0), (-5, 3)] {
        let mut want = vec![0u8; w * h];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let (sx, sy) = (x - dx, y - dy);
                if sx >= 0 && sy >= 0 && sx < w as i64 && sy < h as i64 {
                    want[(y * w as i64 + x) as usize] = a[(sy * w as i64 + sx) as usize];
                }
            }
        }
        let got = tiled(&a, w, h).translate(dx as i32, dy as i32);
        assert!(got.data == want, "translate {dx},{dy}");
    }
}

#[test]
fn tiled_margin_ops_match_dense() {
    let (w, h) = (640usize, 590usize);
    let a = soft(w, h, &[0, 0, 255, 255, 128], 9);
    let sel = tiled(&a, w, h);
    for radius in [1.0, 3.0, 9.0, 60.0] {
        let want = blur::blur(&a, w, h, radius / 2.0);
        assert!(sel.feather(radius).data == want, "feather {radius}");
    }
    for r in [1u32, 4, 40] {
        let ru = r as usize;
        let dilated = mask_ops::dilate(&a, w, h, ru, Border::Zero(0u8));
        let eroded = mask_ops::erode(&a, w, h, ru, Border::Zero(0u8));
        assert!(sel.expand(r).data == dilated, "expand {r}");
        assert!(sel.contract(r).data == eroded, "contract {r}");
        assert!(
            sel.smooth(r).data == mask_ops::median(&a, w, h, ru),
            "smooth {r}"
        );
        let outer = mask_ops::dilate(&a, w, h, r.div_ceil(2) as usize, Border::Zero(0u8));
        let inner = mask_ops::erode(&a, w, h, (r / 2) as usize, Border::Zero(0u8));
        let band: Vec<u8> = if r / 2 == 0 {
            outer.iter().zip(&a).map(|(o, i)| o.abs_diff(*i)).collect()
        } else {
            outer
                .iter()
                .zip(&inner)
                .map(|(o, i)| o.abs_diff(*i))
                .collect()
        };
        assert!(sel.border(r).data == band, "border {r}");
    }
}

#[test]
fn selection_ops_do_not_materialize() {
    let (w, h) = (900usize, 800usize);
    let a = tiled(&soft(w, h, &[0, 255, 255, 60], 13), w, h);
    let b = Selection::ellipse(w as u32, h as u32, 100, 80, 600, 500);
    assert_no_materialization(|| {
        let mut c = a.clone();
        for op in [
            SelectOp::Add,
            SelectOp::Subtract,
            SelectOp::Intersect,
            SelectOp::Replace,
        ] {
            c.combine(&b, op).unwrap();
        }
        let _ = a.invert();
        let _ = a.feather(5.0);
        let _ = a.feather(30.0);
        let _ = a.expand(4);
        let _ = a.contract(4);
        let _ = a.border(6);
        let _ = a.smooth(3);
        let _ = a.translate(40, -70);
        let _ = a.bounds();
        let _ = crate::contour(&b, 128);
    });
}

/// A seeded planar RGB composite of colour blocks, a speckled part, and a
/// stacked tiled plane (as the document's composite is).
fn composite(w: usize, h: usize, seed: u64) -> PixelBuffer {
    let planes: Vec<Vec<u8>> = (0..3)
        .map(|k| soft(w, h, &[10, 12, 200, 90], seed + k))
        .collect();
    let mut img = PixelBuffer::new(w as u32, h as u32, 3);
    let mut data = Vec::with_capacity(3 * w * h);
    for p in &planes {
        data.extend_from_slice(p);
    }
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    img.data = plane;
    img
}

#[test]
fn tiled_wand_matches_dense() {
    let (w, h) = (620usize, 530usize);
    let img = composite(w, h, 21);
    for tol in [0u8, 4, 32, 255] {
        for (x, y) in [(0u32, 0u32), (310, 260), (619, 529)] {
            for contiguous in [true, false] {
                let got = crate::magic_wand(&img, x, y, tol, contiguous).unwrap();
                let want = wand::dense::magic_wand(&img, x, y, tol, contiguous);
                assert!(
                    got.data == want,
                    "tol {tol} at ({x},{y}) contiguous {contiguous}"
                );
            }
        }
    }
}

#[test]
fn tiled_grow_and_similar_match_dense() {
    let (w, h) = (600usize, 560usize);
    let img = composite(w, h, 31);
    let seeds = soft(w, h, &[0, 0, 0, 0, 255, 120], 41);
    let sel = tiled(&seeds, w, h);
    for tol in [0u8, 6, 40] {
        let want = wand::dense::grow(&seeds, &img, tol);
        assert!(
            crate::grow(&sel, &img, tol).unwrap().data == want,
            "grow {tol}"
        );
        let want = wand::dense::similar(&seeds, &img, tol);
        assert!(
            crate::similar(&sel, &img, tol).unwrap().data == want,
            "similar {tol}"
        );
    }
}
