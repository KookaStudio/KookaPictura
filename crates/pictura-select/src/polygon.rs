//! Even-odd polygon fill for the lassos, bounded to the polygon's rows.
//!
//! A pixel is inside when its centre is, under the even-odd rule: the parity
//! of edge crossings to the right of the centre. Each row computes its
//! crossings once, sorts them, and fills the spans between pairs, so a row
//! costs its edges plus its inside pixels rather than every pixel times every
//! edge (#1096). The crossing formula is the per-pixel test's, so the spans
//! select exactly the pixels it did.

use pictura_core::mask_ops::{empty_mask, TiledMask};
use pictura_core::PsdRect;

use crate::tiled::rasterize;

struct Edge {
    xi: f64,
    yi: f64,
    xj: f64,
    yj: f64,
    lo: f64,
    hi: f64,
}

impl Edge {
    /// Where the edge crosses row centre `cy`, when it does (half-open in y).
    fn crossing(&self, cy: f64) -> Option<f64> {
        ((self.yi > cy) != (self.yj > cy))
            .then(|| self.xi + (cy - self.yi) / (self.yj - self.yi) * (self.xj - self.xi))
    }
}

/// The first pixel whose centre is at or right of `c`, clamped to `0..=width`.
fn first_at_or_after(c: f64, width: i64) -> i64 {
    let centre = |px: i64| px as f64 + 0.5;
    let mut px = if c.is_finite() {
        ((c - 0.5).ceil() as i64).clamp(0, width)
    } else if c > 0.0 {
        width
    } else {
        0
    };
    while px > 0 && centre(px - 1) >= c {
        px -= 1;
    }
    while px < width && centre(px) < c {
        px += 1;
    }
    px
}

/// `points` filled at 255 into a `width`×`height` tiled mask, and the tight
/// bounds of the filled pixels, `None` when none is inside. Only the tiles
/// along the outline are dense (#1214).
pub(crate) fn fill(width: u32, height: u32, points: &[(i32, i32)]) -> (TiledMask, Option<PsdRect>) {
    let (w, h) = (width as usize, height as usize);
    let n = points.len();
    if n < 3 || w == 0 || h == 0 {
        return (empty_mask(w, h), None);
    }
    let edges: Vec<Edge> = (0..n)
        .map(|i| {
            let j = (i + n - 1) % n;
            let (xi, yi) = (points[i].0 as f64, points[i].1 as f64);
            let (xj, yj) = (points[j].0 as f64, points[j].1 as f64);
            Edge {
                xi,
                yi,
                xj,
                yj,
                lo: yi.min(yj),
                hi: yi.max(yj),
            }
        })
        .filter(|e| e.yi != e.yj)
        .collect();
    let lo = edges.iter().map(|e| e.lo).fold(f64::INFINITY, f64::min);
    let hi = edges.iter().map(|e| e.hi).fold(f64::NEG_INFINITY, f64::max);
    if lo >= hi {
        return (empty_mask(w, h), None);
    }
    // A row centre `py + 0.5` meets an edge only inside `lo..hi`.
    let first_row = ((lo - 0.5).floor().max(0.0) as usize).min(h);
    let last_row = ((hi + 0.5).ceil().max(0.0) as usize).min(h);
    rasterize(w, h, first_row..last_row, |py, runs| {
        let cy = py as f64 + 0.5;
        let mut crossings: Vec<f64> = edges
            .iter()
            .filter(|e| e.lo <= cy && e.hi >= cy)
            .filter_map(|e| e.crossing(cy))
            .collect();
        crossings.sort_by(f64::total_cmp);
        for &[enter, leave] in crossings.as_chunks::<2>().0 {
            let start = first_at_or_after(enter, w as i64) as usize;
            let end = first_at_or_after(leave, w as i64) as usize;
            if start < end {
                runs.push((start, end));
            }
        }
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::Selection;

    /// The per-pixel test the scanline fill replaced, kept as its oracle.
    pub(crate) fn reference(width: u32, height: u32, points: &[(i32, i32)]) -> Vec<u8> {
        let mut data = vec![0u8; width as usize * height as usize];
        let n = points.len();
        if n < 3 {
            return data;
        }
        for py in 0..height {
            let cy = py as f64 + 0.5;
            for px in 0..width {
                let cx = px as f64 + 0.5;
                let mut inside = false;
                let mut j = n - 1;
                for i in 0..n {
                    let (xi, yi) = (points[i].0 as f64, points[i].1 as f64);
                    let (xj, yj) = (points[j].0 as f64, points[j].1 as f64);
                    if (yi > cy) != (yj > cy) {
                        let cross = xi + (cy - yi) / (yj - yi) * (xj - xi);
                        if cx < cross {
                            inside = !inside;
                        }
                    }
                    j = i;
                }
                if inside {
                    data[py as usize * width as usize + px as usize] = 255;
                }
            }
        }
        data
    }

    /// A fixed-seed xorshift, so the polygons are the same on every run.
    fn polygons() -> Vec<Vec<(i32, i32)>> {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move |span: i32| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % span as u64) as i32
        };
        let mut out = vec![
            vec![(0, 0), (4, 0), (4, 4), (0, 4)],
            vec![(-20, -20), (300, -20), (300, 300), (-20, 300)],
            // A bow tie crosses itself; a star overlaps itself under even-odd.
            vec![(5, 5), (150, 120), (150, 5), (5, 120)],
            vec![(80, 0), (100, 130), (0, 50), (160, 50), (60, 130)],
            // Collinear, repeated, and horizontal edges.
            vec![(10, 10), (10, 10), (60, 10), (110, 10), (110, 90), (10, 90)],
        ];
        for _ in 0..40 {
            let count = 3 + next(40) as usize;
            out.push(
                (0..count)
                    .map(|_| (next(260) - 50, next(220) - 40))
                    .collect(),
            );
        }
        out
    }

    #[test]
    fn scanline_fill_matches_the_per_pixel_test() {
        let (w, h) = (167u32, 139u32);
        for points in polygons() {
            let (sel, bounds) = Selection::polygon_bounded(w, h, &points);
            let expected = reference(w, h, &points);
            assert_eq!(sel.data.as_slice(), &expected[..], "polygon {points:?}");
            assert_eq!(bounds, sel.bounds(), "bounds of {points:?}");
        }
    }

    #[test]
    fn spans_past_a_band_and_the_canvas_edge() {
        let (w, h) = (90u32, 200u32);
        let points = [(-30, 3), (120, 70), (40, 260)];
        let (sel, bounds) = Selection::polygon_bounded(w, h, &points);
        assert_eq!(sel.data.as_slice(), &reference(w, h, &points)[..]);
        assert_eq!(bounds, sel.bounds());
    }

    #[test]
    fn too_few_points_or_no_area_is_empty() {
        for points in [vec![(0, 0), (5, 5)], vec![(0, 3), (9, 3), (4, 3)]] {
            let (sel, bounds) = Selection::polygon_bounded(10, 10, &points);
            assert_eq!(sel, Selection::none(10, 10));
            assert_eq!(bounds, None);
        }
    }
}
