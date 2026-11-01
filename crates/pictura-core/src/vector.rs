//! A derived view of a layer's `vmsk` vector mask.
//!
//! The raw `vmsk` tagged block stays in [`crate::Layer::extra_blocks`] and is
//! re-emitted verbatim; this view is what the compositor can sample.

use std::sync::OnceLock;

use crate::PsdRect;

/// How a subpath's interior is determined.
///
/// Even-odd is the documented PSD default; non-zero is read only from the
/// ag-psd fill-rule marker `2` and is not documented by the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VectorFillRule {
    #[default]
    EvenOdd,
    NonZero,
}

/// One flattened subpath of a vector mask.
///
/// `operation` is the subpath's boolean operation (1 union, 2 not-or, 3
/// intersect, 0 xor, -1 continuation); it is recorded but not applied.
/// `points` are `[x, y]` document-pixel polyline vertices in 1/256-pixel units,
/// stored as integers so [`crate::Layer`] keeps its `Eq` derive.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VectorSubpath {
    pub closed: bool,
    pub operation: i16,
    pub fill_rule: VectorFillRule,
    pub points: Vec<[i32; 2]>,
}

/// A layer's decoded vector mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorMask {
    pub subpaths: Vec<VectorSubpath>,
    pub invert: bool,
    pub disabled: bool,
    /// PSD vector-mask density, 0-255; 255 is the path as drawn.
    pub density: u8,
    /// [`f64::to_bits`] of the feather radius in pixels, as on
    /// [`crate::LayerMask`].
    pub feather_bits: u64,
    /// The sampling cache [`VectorMask::inside`] builds; leave it `Default`.
    pub rows: CoverageRows,
}

impl Default for VectorMask {
    fn default() -> Self {
        Self {
            subpaths: Vec::new(),
            invert: false,
            disabled: false,
            density: 255,
            feather_bits: 0,
            rows: CoverageRows::default(),
        }
    }
}

/// A mask's closed subpaths as per-row edge crossings, built on the first
/// [`VectorMask::inside`] query so each later sample is a scan of that row's
/// few crossings rather than of every edge. A cache, not data: it compares
/// equal whatever it holds, and it is not rebuilt (except when the feather
/// radius changes, which grows the sampled rows), so set `subpaths` before
/// sampling.
#[derive(Clone)]
pub struct CoverageRows {
    /// A nonzero id shared by this mask's clones and by no mask built from
    /// other subpaths; see [`VectorMask::ident`].
    ident: u64,
    rows: OnceLock<Rows>,
    /// The antialiased coverage [`VectorMask::coverage`] samples, built on its
    /// first query like `rows`.
    smooth: OnceLock<SmoothRows>,
}

impl Default for CoverageRows {
    fn default() -> Self {
        Self {
            ident: crate::fresh_stamp(),
            rows: OnceLock::new(),
            smooth: OnceLock::new(),
        }
    }
}

impl PartialEq for CoverageRows {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for CoverageRows {}

impl std::fmt::Debug for CoverageRows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CoverageRows")
    }
}

#[derive(Clone)]
struct Rows {
    /// The pixel row of `crossings[0]`.
    top: i32,
    /// The rows any edge can cross, `[first, end)`: a row outside them has no
    /// crossing, whatever the table covers.
    span: (i32, i32),
    non_zero: bool,
    /// Per pixel row, each edge crossing the row's centre line: its x (1/256
    /// px) and direction (+1 downward, -1 upward).
    crossings: Vec<Vec<(f64, i8)>>,
}

/// The pixel row whose centre line a point at `v` (1/256 px) belongs to.
fn row_of(v: f64) -> i32 {
    ((v - 128.0) / 256.0).ceil() as i32
}

/// Whether a row's crossings to the right of `cx` mark `(x, y)` inside:
/// non-zero when any subpath declares it, else even-odd.
fn scan(crossings: impl Iterator<Item = (f64, i8)>, cx: f64, non_zero: bool) -> bool {
    let right = crossings.filter(|(px, _)| *px > cx);
    if non_zero {
        right.map(|(_, d)| i32::from(d)).sum::<i32>() != 0
    } else {
        right.count() % 2 == 1
    }
}

impl Rows {
    /// Index `subpaths` into per-row crossings over `clip`: only the rows a
    /// caller samples are built, so a path whose points span the whole `i32`
    /// range (a crafted `vmsk` reaches about ±128× the document height) cannot
    /// size the table. Edges that span past `clip` contribute crossings to the
    /// in-range rows only; [`VectorMask::inside`] scans the edges directly for
    /// a row outside the table, so the clip bounds the cache, not correctness.
    fn build(subpaths: &[VectorSubpath], clip: PsdRect) -> Rows {
        let closed: Vec<&VectorSubpath> = subpaths
            .iter()
            .filter(|s| s.closed && s.points.len() >= 3)
            .collect();
        let non_zero = closed
            .iter()
            .any(|s| s.fill_rule == VectorFillRule::NonZero);
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for s in &closed {
            for p in &s.points {
                top = top.min(row_of(p[1] as f64));
                bottom = bottom.max(row_of(p[1] as f64));
            }
        }
        // The clip plus one row each side, clamped to the path's own span, in
        // `i64` so neither `i32::MIN - 1` nor the path's extremes overflow.
        let lo = (i64::from(top).max(i64::from(clip.top) - 1)).max(i64::from(i32::MIN)) as i32;
        let hi =
            (i64::from(bottom).min(i64::from(clip.bottom) + 1)).min(i64::from(i32::MAX)) as i32;
        if lo >= hi {
            return Rows {
                top: 0,
                span: (top, bottom),
                non_zero,
                crossings: Vec::new(),
            };
        }
        let mut crossings = vec![Vec::new(); (hi - lo) as usize];
        for s in &closed {
            for i in 0..s.points.len() {
                let [ax, ay] = s.points[i].map(f64::from);
                let [bx, by] = s.points[(i + 1) % s.points.len()].map(f64::from);
                if ay == by {
                    continue;
                }
                let dir: i8 = if by > ay { 1 } else { -1 };
                for y in row_of(ay.min(by)).max(lo)..row_of(ay.max(by)).min(hi) {
                    let cy = f64::from(y) * 256.0 + 128.0;
                    let x = ax + (cy - ay) * (bx - ax) / (by - ay);
                    crossings[(y - lo) as usize].push((x, dir));
                }
            }
        }
        Rows {
            top: lo,
            span: (top, bottom),
            non_zero,
            crossings,
        }
    }
}

/// Sample lines per pixel row for [`VectorMask::coverage`]. Along each line the
/// covered length is exact, so a near-vertical edge is as smooth as the
/// line count lets a near-horizontal one be.
const SAMPLE_LINES: i32 = 16;

/// One pixel row's antialiased coverage, in sample lines (`0..=SAMPLE_LINES`):
/// a step function of whole-pixel coverage plus the partial pixels at the
/// edges, so a lookup is two binary searches.
#[derive(Clone, Default)]
struct SmoothRow {
    /// `(x, level)`: from pixel `x` up to the next entry, each pixel is
    /// covered by `level` whole sample lines.
    steps: Vec<(i32, f32)>,
    /// `(x, extra)`: the partly covered length of pixel `x`, summed over the
    /// sample lines; sorted by `x`, one entry per pixel.
    partial: Vec<(i32, f32)>,
}

impl SmoothRow {
    /// The row's coverage from `edges`, each `[ax, ay, bx, by]` in 1/256 px.
    fn build(edges: &[[f64; 4]], non_zero: bool, y: i32) -> SmoothRow {
        let mut deltas: Vec<(i32, f32)> = Vec::new();
        let mut partial: Vec<(i32, f32)> = Vec::new();
        let mut crossings: Vec<(f64, i8)> = Vec::new();
        // Interval ends in whole pixels, bounded so a crafted path cannot
        // overflow the cast.
        let pixel = |v: f64| (v / 256.0).clamp(-1.0e9, 1.0e9);
        for line in 0..SAMPLE_LINES {
            let cy = (f64::from(y) + (f64::from(line) + 0.5) / f64::from(SAMPLE_LINES)) * 256.0;
            crossings.clear();
            for &[ax, ay, bx, by] in edges {
                if (ay < cy) != (by < cy) {
                    let dir: i8 = if by > ay { 1 } else { -1 };
                    crossings.push((ax + (cy - ay) * (bx - ax) / (by - ay), dir));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (mut winding, mut count) = (0i32, 0usize);
            for i in 0..crossings.len() {
                winding += i32::from(crossings[i].1);
                count += 1;
                let inside = if non_zero {
                    winding != 0
                } else {
                    count % 2 == 1
                };
                let Some(next) = crossings.get(i + 1) else {
                    break;
                };
                if !inside {
                    continue;
                }
                let (a, b) = (pixel(crossings[i].0), pixel(next.0));
                if b <= a {
                    continue;
                }
                let (ia, ib) = (a.floor(), b.floor());
                if ia == ib {
                    partial.push((ia as i32, (b - a) as f32));
                    continue;
                }
                partial.push((ia as i32, (ia + 1.0 - a) as f32));
                if ib > ia + 1.0 {
                    deltas.push((ia as i32 + 1, 1.0));
                    deltas.push((ib as i32, -1.0));
                }
                if b > ib {
                    partial.push((ib as i32, (b - ib) as f32));
                }
            }
        }
        deltas.sort_by_key(|d| d.0);
        let mut steps: Vec<(i32, f32)> = Vec::new();
        let mut level = 0.0f32;
        for (x, d) in deltas {
            level += d;
            match steps.last_mut() {
                Some(last) if last.0 == x => last.1 = level,
                _ => steps.push((x, level)),
            }
        }
        partial.sort_by_key(|p| p.0);
        let mut merged: Vec<(i32, f32)> = Vec::new();
        for (x, v) in partial {
            match merged.last_mut() {
                Some(last) if last.0 == x => last.1 += v,
                _ => merged.push((x, v)),
            }
        }
        SmoothRow {
            steps,
            partial: merged,
        }
    }

    /// Pixel `x`'s coverage in sample lines.
    fn level(&self, x: i32) -> f32 {
        let i = self.steps.partition_point(|s| s.0 <= x);
        let whole = if i == 0 { 0.0 } else { self.steps[i - 1].1 };
        let part = self
            .partial
            .binary_search_by_key(&x, |p| p.0)
            .map_or(0.0, |j| self.partial[j].1);
        whole + part
    }
}

/// [`SmoothRow`]s over the first query's clip; a row outside it is built on
/// demand from `edges`, so the clip bounds the cache, not correctness.
#[derive(Clone)]
struct SmoothRows {
    top: i32,
    /// The pixel rows any edge can touch, `[first, end)`.
    span: (i32, i32),
    non_zero: bool,
    /// Every closed subpath's edges, `[ax, ay, bx, by]` in 1/256 px.
    edges: Vec<[f64; 4]>,
    rows: Vec<SmoothRow>,
}

impl SmoothRows {
    fn build(subpaths: &[VectorSubpath], clip: PsdRect) -> SmoothRows {
        let closed: Vec<&VectorSubpath> = subpaths
            .iter()
            .filter(|s| s.closed && s.points.len() >= 3)
            .collect();
        let non_zero = closed
            .iter()
            .any(|s| s.fill_rule == VectorFillRule::NonZero);
        let mut edges = Vec::new();
        let (mut top, mut bottom) = (f64::MAX, f64::MIN);
        for s in &closed {
            for i in 0..s.points.len() {
                let [ax, ay] = s.points[i].map(f64::from);
                let [bx, by] = s.points[(i + 1) % s.points.len()].map(f64::from);
                top = top.min(ay);
                bottom = bottom.max(ay);
                if ay != by {
                    edges.push([ax, ay, bx, by]);
                }
            }
        }
        let span = if edges.is_empty() {
            (0, 0)
        } else {
            (
                (top / 256.0).floor().max(f64::from(i32::MIN)) as i32,
                (bottom / 256.0).ceil().min(f64::from(i32::MAX)) as i32,
            )
        };
        let lo = span.0.max(clip.top.saturating_sub(1));
        let hi = span.1.min(clip.bottom.saturating_add(1));
        let rows = (lo..hi.max(lo))
            .map(|y| SmoothRow::build(&Self::edges_near(&edges, y), non_zero, y))
            .collect();
        SmoothRows {
            top: lo,
            span,
            non_zero,
            edges,
            rows,
        }
    }

    /// The edges that reach into pixel row `y`.
    fn edges_near(edges: &[[f64; 4]], y: i32) -> Vec<[f64; 4]> {
        let (top, bottom) = (f64::from(y) * 256.0, f64::from(y + 1) * 256.0);
        edges
            .iter()
            .filter(|e| e[1].min(e[3]) < bottom && e[1].max(e[3]) > top)
            .copied()
            .collect()
    }

    fn level(&self, x: i32, y: i32) -> f32 {
        if y < self.span.0 || y >= self.span.1 {
            return 0.0;
        }
        match usize::try_from(i64::from(y) - i64::from(self.top))
            .ok()
            .and_then(|i| self.rows.get(i))
        {
            Some(row) => row.level(x),
            None => SmoothRow::build(&Self::edges_near(&self.edges, y), self.non_zero, y).level(x),
        }
    }
}

/// A resolved vector-mask coverage table for one clip, sampled per pixel.
/// Obtained from [`VectorMask::coverage_sampler`].
#[derive(Clone, Copy)]
pub struct VectorCoverage<'a> {
    rows: &'a SmoothRows,
}

impl VectorCoverage<'_> {
    /// The antialiased coverage (0–255) of pixel `(x, y)`, ignoring `invert`.
    pub fn level(&self, x: i32, y: i32) -> u8 {
        let share = self.rows.level(x, y) / SAMPLE_LINES as f32;
        (share * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

impl VectorMask {
    /// The antialiased coverage (0–255) of pixel `(x, y)` by the mask's closed
    /// subpaths, under the same fill rule as [`VectorMask::inside`]: the share
    /// of the pixel the path covers, measured along [`SAMPLE_LINES`] lines.
    /// Ignores `invert` and `disabled`; `clip` bounds the cached rows as it
    /// does for `inside`.
    pub fn coverage(&self, x: i32, y: i32, clip: PsdRect) -> u8 {
        self.coverage_sampler(clip).level(x, y)
    }

    /// A resolved coverage table for `clip`, so a compositor can sample the
    /// same mask over a whole layer without rebuilding the row table per pixel.
    /// Ignores `invert` and `disabled`, like [`VectorMask::coverage`].
    pub fn coverage_sampler(&self, clip: PsdRect) -> VectorCoverage<'_> {
        let rows = self
            .rows
            .smooth
            .get_or_init(|| SmoothRows::build(&self.subpaths, clip));
        VectorCoverage { rows }
    }

    /// The feather radius in pixels.
    pub fn feather(&self) -> f64 {
        f64::from_bits(self.feather_bits)
    }

    pub fn set_feather(&mut self, radius: f64) {
        self.feather_bits = radius.to_bits();
        // The feather path samples its blurred plane grown past the clip, so a
        // feather change grows the rows the row table must cover; retire it.
        self.rows = CoverageRows::default();
    }

    /// A nonzero id shared by this mask's clones and by no mask built from
    /// other subpaths, for caches of its coverage. It is assigned when the mask
    /// is built, so set `subpaths` before cloning.
    pub fn ident(&self) -> u64 {
        self.rows.ident
    }

    /// Whether the centre of pixel `(x, y)` lies inside the mask's closed
    /// subpaths: even-odd, or non-zero when any subpath declares it. Ignores
    /// `invert` and `disabled`.
    ///
    /// `clip` bounds the rows the row table caches; a crafted path cannot size
    /// it. A row outside the table is answered by scanning the edges directly,
    /// so the same mask samples correctly over a later, different clip (a
    /// partial-region composite, or the feather plane grown past the canvas).
    pub fn inside(&self, x: i32, y: i32, clip: PsdRect) -> bool {
        let rows = self
            .rows
            .rows
            .get_or_init(|| Rows::build(&self.subpaths, clip));
        let cx = f64::from(x) * 256.0 + 128.0;
        if let Some(row) = usize::try_from(i64::from(y) - i64::from(rows.top))
            .ok()
            .and_then(|i| rows.crossings.get(i))
        {
            return scan(row.iter().copied(), cx, rows.non_zero);
        }
        // Above or below every edge: nothing to scan. Without this, each pixel
        // of such a row walked the whole flattened path.
        if y < rows.span.0 || y >= rows.span.1 {
            return false;
        }
        let mut crossings = Vec::new();
        for s in &self.subpaths {
            if !(s.closed && s.points.len() >= 3) {
                continue;
            }
            for i in 0..s.points.len() {
                let [ax, ay] = s.points[i].map(f64::from);
                let [bx, by] = s.points[(i + 1) % s.points.len()].map(f64::from);
                if ay == by || row_of(ay.min(by)) > y || row_of(ay.max(by)) <= y {
                    continue;
                }
                let cy = f64::from(y) * 256.0 + 128.0;
                let dir: i8 = if by > ay { 1 } else { -1 };
                crossings.push((ax + (cy - ay) * (bx - ax) / (by - ay), dir));
            }
        }
        scan(crossings.into_iter(), cx, rows.non_zero)
    }

    /// Whether the mask has a live closed subpath to sample. An absent,
    /// disabled, empty, or all-open mask is constant coverage (`255`).
    pub fn has_fill(&self) -> bool {
        !self.disabled
            && self
                .subpaths
                .iter()
                .any(|s| s.closed && s.points.len() >= 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(height: i32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: height,
            right: 256,
        }
    }

    fn rectangle(points: Vec<[i32; 2]>) -> VectorMask {
        VectorMask {
            subpaths: vec![VectorSubpath {
                closed: true,
                operation: 1,
                fill_rule: VectorFillRule::EvenOdd,
                points,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_crafted_path_bounds_the_row_table_to_the_clip() {
        // A vmsk's 8.24 points are fractions of the canvas, so a crafted path
        // spans about ±128× the document height: unbounded, the empty row
        // headers alone are hundreds of MB. The table is cut to the clip.
        let mask = rectangle(vec![
            [0, i32::MIN],
            [512, i32::MIN],
            [512, i32::MAX],
            [0, i32::MAX],
        ]);
        assert!(mask.inside(1, 100, clip(256)), "the path covers the canvas");
        let rows = mask.rows.rows.get().expect("the query built the table");
        assert_eq!(
            rows.crossings.len(),
            258,
            "256 rows plus one each side, not the path's millions"
        );
    }

    #[test]
    fn a_row_beyond_the_path_is_outside_whatever_the_cached_clip() {
        // Rows above and below every edge answer without walking the edges;
        // under either fill rule they are outside, inverted or not later.
        for rule in [VectorFillRule::EvenOdd, VectorFillRule::NonZero] {
            let mut mask = rectangle(vec![[0, 2560], [512, 2560], [512, 5120], [0, 5120]]);
            mask.subpaths[0].fill_rule = rule;
            let narrow = PsdRect {
                top: 12,
                left: 0,
                bottom: 14,
                right: 256,
            };
            assert!(
                mask.inside(1, 13, narrow),
                "{rule:?}: a cached row inside the path"
            );
            assert!(
                mask.inside(1, 17, narrow),
                "{rule:?}: an uncached row inside the path"
            );
            for y in [-5, 0, 9, 20, 21, 400] {
                assert!(
                    !mask.inside(1, y, narrow),
                    "{rule:?}: row {y} is beyond the path"
                );
            }
        }
    }

    #[test]
    fn a_row_outside_the_cached_clip_falls_back_to_the_edges() {
        // A later caller with a different clip (a partial-region composite)
        // must stay correct: the table only bounds the cache.
        let mask = rectangle(vec![[0, 0], [512, 0], [512, 5120], [0, 5120]]);
        let narrow = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 256,
        };
        assert!(mask.inside(1, 1, narrow), "inside the cached rows");
        assert_eq!(
            mask.rows.rows.get().unwrap().crossings.len(),
            3,
            "bounded to the clip"
        );
        assert!(
            mask.inside(1, 10, narrow),
            "row 10 is outside the cache but inside the path"
        );
    }

    #[test]
    fn coverage_is_exact_on_whole_pixels_and_partial_on_a_slanted_edge() {
        // A 2×2 square at (1, 1): whole pixels are 0 or 255.
        let square = rectangle(vec![[256, 256], [768, 256], [768, 768], [256, 768]]);
        assert_eq!(square.coverage(1, 1, clip(4)), 255);
        assert_eq!(square.coverage(2, 2, clip(4)), 255);
        assert_eq!(square.coverage(0, 1, clip(4)), 0);
        assert_eq!(square.coverage(3, 3, clip(4)), 0);

        // Half a pixel wide: x from 1.5 to 3.
        let half = rectangle(vec![[384, 0], [768, 0], [768, 1024], [384, 1024]]);
        assert_eq!(half.coverage(1, 2, clip(4)), 128);
        assert_eq!(half.coverage(2, 2, clip(4)), 255);

        // The triangle below the diagonal y = x covers half of each pixel the
        // diagonal crosses, and all of those under it.
        let triangle = rectangle(vec![[0, 0], [2048, 2048], [0, 2048]]);
        assert_eq!(triangle.coverage(3, 3, clip(8)), 128);
        assert_eq!(triangle.coverage(2, 5, clip(8)), 255);
        assert_eq!(triangle.coverage(5, 2, clip(8)), 0);
        // A row outside the cached clip answers the same.
        let fresh = rectangle(vec![[0, 0], [2048, 2048], [0, 2048]]);
        assert_eq!(fresh.coverage(0, 0, clip(2)), 128);
        assert_eq!(fresh.coverage(6, 6, clip(2)), 128);
    }

    #[test]
    fn smooth_coverage_bounds_its_rows_to_the_clip() {
        let mask = rectangle(vec![
            [0, i32::MIN],
            [512, i32::MIN],
            [512, i32::MAX],
            [0, i32::MAX],
        ]);
        assert_eq!(mask.coverage(1, 100, clip(256)), 255);
        let rows = mask.rows.smooth.get().expect("the query built the rows");
        assert_eq!(rows.rows.len(), 258, "the clip plus one row each side");
        assert_eq!(mask.coverage(1, 100_000, clip(256)), 255, "an uncached row");
    }

    #[test]
    fn a_coverage_sampler_matches_the_per_call_coverage() {
        let mask = rectangle(vec![[0, 0], [2048, 2048], [0, 2048]]);
        let sampler = mask.coverage_sampler(clip(8));
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(
                    sampler.level(x, y),
                    mask.coverage(x, y, clip(8)),
                    "sample ({x}, {y})"
                );
            }
        }
    }
}
