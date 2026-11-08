//! 50 %-coverage outline of a selection mask (marching ants).
//!
//! The overlay needs the committed edge, not pixel data. One pass reduces each
//! row to its inside runs (an identical row reuses the previous runs), and the
//! boundary falls out of the runs as directed edges on the integer pixel-corner
//! lattice, with the inside always on the same side, so every vertex has as
//! many edges leaving as entering and each walk closes. Chaining then drops
//! collinear vertices: a rectangle is four corners. The drawn union of edges is
//! the exact 4-connected outline; a saddle vertex may split a loop into more
//! than one polyline, which is fine for drawing. Derived and view-only: never
//! stored.

use crate::Selection;
use pictura_core::PsdRect;

/// Closed polylines (integer pixel-corner lattice, `[0, width] x [0, height]`)
/// tracing the boundary of the pixels whose coverage is `>= threshold`.
///
/// The first vertex is not repeated at the end; draw each loop closed.
pub fn contour(selection: &Selection, threshold: u8) -> Vec<Vec<[i32; 2]>> {
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: selection.height as i32,
        right: selection.width as i32,
    };
    contour_within(selection, threshold, whole)
}

/// [`contour`] read only inside `window` (clipped to the canvas), in document
/// coordinates. Every pixel at or above `threshold` must lie in the window, as
/// it does in [`Selection::bounds`]; then no edge lies outside it.
pub fn contour_within(selection: &Selection, threshold: u8, window: PsdRect) -> Vec<Vec<[i32; 2]>> {
    let (w, h) = (selection.width as usize, selection.height as usize);
    if w == 0 || h == 0 || selection.data.len() != w * h {
        return Vec::new();
    }
    let clip = |lo: i32, hi: i32, max: usize| {
        let lo = lo.clamp(0, max as i32);
        (lo as usize, hi.clamp(lo, max as i32) as usize)
    };
    let (x0, x1) = clip(window.left, window.right, w);
    let (y0, y1) = clip(window.top, window.bottom, h);
    if x0 == x1 || y0 == y1 {
        return Vec::new();
    }
    let vw = w as i64 + 1;
    let key = |x: usize, y: usize| y as i64 * vw + x as i64;
    // Rows are copied out of a tiled mask one at a time (#1214).
    let row_at = |y: usize| {
        selection
            .data
            .row_span(y, x0..x1, w)
            .unwrap_or(std::borrow::Cow::Borrowed(&[]))
    };
    let mut previous: Option<std::borrow::Cow<'_, [u8]>> = None;
    let mut edges: Vec<[i64; 2]> = Vec::new();
    let mut above: Vec<(usize, usize)> = Vec::new();
    let mut below: Vec<(usize, usize)> = Vec::new();
    let mut full_rows = 0;

    // Row boundary `y` separates row `y - 1` (above) from row `y` (below).
    // Edges run with the inside on their right in pixel coordinates: left sides
    // down, bottoms right, right sides up, tops left.
    for y in y0..=y1 {
        let row = (y < y1).then(|| row_at(y));
        let same = match &row {
            Some(r) => y > y0 && previous.as_deref() == Some(&r[..]),
            None => above.is_empty(),
        };
        if !same {
            below.clear();
            if let Some(r) = &row {
                inside_runs(r, threshold, x0, &mut below);
            }
            for (a, b) in difference(&below, &above) {
                edges.push([key(b, y), key(a, y)]);
            }
            for (a, b) in difference(&above, &below) {
                edges.push([key(a, y), key(b, y)]);
            }
            std::mem::swap(&mut above, &mut below);
        }
        if y < y1 {
            for &(a, b) in &above {
                edges.push([key(a, y), key(a, y + 1)]);
                edges.push([key(b, y + 1), key(b, y)]);
            }
            if above.as_slice() == [(0, w)] {
                full_rows += 1;
            }
        }
        previous = row;
    }
    if full_rows == h {
        // Select All: the outline is the document rectangle.
        let (w, h) = (w as i32, h as i32);
        return vec![vec![[0, 0], [0, h], [w, h], [w, 0]]];
    }
    chain(&edges, vw)
}

/// Append the half-open `[x0, x1)` runs of `row` at or above `threshold`,
/// shifted right by `offset`.
fn inside_runs(row: &[u8], threshold: u8, offset: usize, out: &mut Vec<(usize, usize)>) {
    let mut x = 0;
    while let Some(start) = row[x..].iter().position(|&v| v >= threshold) {
        let x0 = x + start;
        let x1 = row[x0..]
            .iter()
            .position(|&v| v < threshold)
            .map_or(row.len(), |n| x0 + n);
        out.push((offset + x0, offset + x1));
        x = x1;
    }
}

/// The parts of the sorted, disjoint runs `a` not covered by `b`, as maximal runs.
fn difference(a: &[(usize, usize)], b: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut j = 0;
    for &(start, end) in a {
        while j < b.len() && b[j].1 <= start {
            j += 1;
        }
        let mut x = start;
        let mut k = j;
        while x < end {
            match b.get(k) {
                Some(&(b0, b1)) if b0 < end => {
                    if b0 > x {
                        out.push((x, b0));
                    }
                    x = x.max(b1);
                    k += 1;
                }
                _ => {
                    out.push((x, end));
                    x = end;
                }
            }
        }
    }
    out
}

/// Walk the directed edges into closed loops, dropping collinear vertices.
fn chain(edges: &[[i64; 2]], vw: i64) -> Vec<Vec<[i32; 2]>> {
    let mut by_from: Vec<(i64, usize)> = edges.iter().enumerate().map(|(i, e)| (e[0], i)).collect();
    by_from.sort_unstable();
    let mut used = vec![false; edges.len()];
    let next_from = |used: &[bool], k: i64| {
        let start = by_from.partition_point(|&(f, _)| f < k);
        by_from[start..]
            .iter()
            .take_while(|&&(f, _)| f == k)
            .map(|&(_, e)| e)
            .find(|&e| !used[e])
    };
    let decode = |k: i64| [(k % vw) as i32, (k / vw) as i32];

    let mut loops = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let first = edges[start][0];
        let mut poly = vec![decode(first)];
        let mut cur = edges[start][1];
        while cur != first {
            poly.push(decode(cur));
            let Some(e) = next_from(&used, cur) else {
                break;
            };
            used[e] = true;
            cur = edges[e][1];
        }
        loops.push(drop_collinear(poly));
    }
    loops
}

/// Remove vertices whose neighbours lie on the same axis line (the loop is
/// closed, so the first and last vertices are neighbours).
fn drop_collinear(poly: Vec<[i32; 2]>) -> Vec<[i32; 2]> {
    let n = poly.len();
    if n < 3 {
        return poly;
    }
    (0..n)
        .filter(|&i| {
            let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            !((p[0] == c[0] && c[0] == q[0]) || (p[1] == c[1] && c[1] == q[1]))
        })
        .map(|i| poly[i])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(w: u32, h: u32, x0: i64, y0: i64, x1: i64, y1: i64) -> Selection {
        let mut s = Selection::none(w, h);
        for y in y0.max(0)..y1.min(h as i64) {
            for x in x0.max(0)..x1.min(w as i64) {
                s.data[(y * w as i64 + x) as usize] = 255;
            }
        }
        s
    }

    fn has_vertex(loop_: &[[i32; 2]], p: [i32; 2]) -> bool {
        loop_.contains(&p)
    }

    #[test]
    fn rectangle_is_one_loop_with_corners() {
        let sel = filled(6, 6, 1, 1, 5, 4);
        let loops = contour(&sel, 128);
        assert_eq!(loops.len(), 1);
        let l = &loops[0];
        assert!(has_vertex(l, [1, 1]), "top-left corner");
        assert!(has_vertex(l, [5, 1]), "top-right corner");
        assert!(has_vertex(l, [5, 4]), "bottom-right corner");
        assert!(has_vertex(l, [1, 4]), "bottom-left corner");
    }

    #[test]
    fn interior_hole_yields_two_loops() {
        let mut sel = filled(8, 8, 0, 0, 8, 8);
        for y in 3..5 {
            for x in 3..5 {
                sel.data[y * 8 + x] = 0;
            }
        }
        let loops = contour(&sel, 128);
        assert_eq!(loops.len(), 2);
        let outer = loops
            .iter()
            .any(|l| has_vertex(l, [0, 0]) && has_vertex(l, [8, 8]));
        let hole = loops
            .iter()
            .any(|l| has_vertex(l, [3, 3]) && has_vertex(l, [5, 5]));
        assert!(outer, "outer document boundary");
        assert!(hole, "inner hole boundary");
    }

    #[test]
    fn empty_selection_yields_none() {
        assert!(contour(&Selection::none(4, 4), 128).is_empty());
        assert!(contour(&filled(4, 4, 9, 9, 11, 11), 128).is_empty());
    }

    #[test]
    fn threshold_respects_partial_coverage() {
        let mut sel = Selection::none(4, 4);
        sel.data[0] = 100;
        assert!(contour(&sel, 128).is_empty(), "100 < 128 is outside");
        let loops = contour(&sel, 50);
        assert_eq!(loops.len(), 1);
        assert!(has_vertex(&loops[0], [0, 0]));
        assert!(has_vertex(&loops[0], [1, 1]));
    }

    #[test]
    fn select_all_is_the_document_rectangle() {
        let loops = contour(&Selection::all(4, 5), 128);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0], vec![[0, 0], [0, 5], [4, 5], [4, 0]]);
    }

    #[test]
    fn rectangle_keeps_only_its_corners() {
        let loops = contour(&filled(6, 6, 1, 1, 5, 4), 128);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
    }

    /// Unit lattice edges (undirected) a set of loops draws.
    fn drawn_unit_edges(loops: &[Vec<[i32; 2]>]) -> std::collections::BTreeSet<[[i32; 2]; 2]> {
        let mut out = std::collections::BTreeSet::new();
        for l in loops {
            for i in 0..l.len() {
                let (a, b) = (l[i], l[(i + 1) % l.len()]);
                assert!(a[0] == b[0] || a[1] == b[1], "axis-aligned segment");
                let (dx, dy) = ((b[0] - a[0]).signum(), (b[1] - a[1]).signum());
                let mut p = a;
                while p != b {
                    let q = [p[0] + dx, p[1] + dy];
                    out.insert(if p < q { [p, q] } else { [q, p] });
                    p = q;
                }
            }
        }
        out
    }

    #[test]
    fn random_masks_trace_the_exact_pixel_boundary() {
        let mut seed = 0x2545_f491_u32;
        for _ in 0..40 {
            let (w, h) = (13u32, 9u32);
            let mut sel = Selection::none(w, h);
            for v in sel.data.iter_mut() {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = if seed >> 31 == 1 { 255 } else { 0 };
            }
            let inside = |x: i32, y: i32| {
                x >= 0
                    && y >= 0
                    && x < w as i32
                    && y < h as i32
                    && sel.data[(y * w as i32 + x) as usize] >= 128
            };
            let mut expected = std::collections::BTreeSet::new();
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    if !inside(x, y) {
                        continue;
                    }
                    if !inside(x, y - 1) {
                        expected.insert([[x, y], [x + 1, y]]);
                    }
                    if !inside(x, y + 1) {
                        expected.insert([[x, y + 1], [x + 1, y + 1]]);
                    }
                    if !inside(x - 1, y) {
                        expected.insert([[x, y], [x, y + 1]]);
                    }
                    if !inside(x + 1, y) {
                        expected.insert([[x + 1, y], [x + 1, y + 1]]);
                    }
                }
            }
            assert_eq!(drawn_unit_edges(&contour(&sel, 128)), expected);
        }
    }

    #[test]
    fn tracing_within_the_bounds_matches_the_whole_mask() {
        let mut seed = 0x9e37_79b9_u32;
        let mut next = |n: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % n
        };
        let (w, h) = (23u32, 17u32);
        let mut masks = Vec::new();
        for _ in 0..60 {
            // A random patch somewhere on the canvas, with coverage both under
            // and over the threshold so the bounds are wider than the outline.
            let (px, py) = (next(w), next(h));
            let (pw, ph) = (1 + next(w - px), 1 + next(h - py));
            let mut sel = Selection::none(w, h);
            for y in py..py + ph {
                for x in px..px + pw {
                    sel.data[(y * w + x) as usize] = [0, 60, 200, 255][next(4) as usize];
                }
            }
            masks.push(sel);
        }
        masks.push(filled(w, h, 0, 0, w as i64, 3));
        masks.push(filled(w, h, w as i64 - 2, 0, w as i64, h as i64));
        masks.push(filled(w, h, 0, h as i64 - 1, w as i64, h as i64));
        masks.push(Selection::all(w, h));
        for sel in &masks {
            let whole = contour(sel, 128);
            match sel.bounds() {
                Some(bounds) => assert_eq!(
                    drawn_unit_edges(&contour_within(sel, 128, bounds)),
                    drawn_unit_edges(&whole)
                ),
                None => assert!(whole.is_empty()),
            }
        }
        assert_eq!(
            contour_within(
                &Selection::all(w, h),
                128,
                Selection::all(w, h).bounds().unwrap()
            ),
            contour(&Selection::all(w, h), 128),
            "Select All stays the document rectangle"
        );
        let empty = PsdRect {
            top: 4,
            left: 4,
            bottom: 4,
            right: 9,
        };
        assert!(contour_within(&Selection::all(w, h), 128, empty).is_empty());
    }
}
