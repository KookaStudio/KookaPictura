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
}

impl Default for CoverageRows {
    fn default() -> Self {
        Self {
            ident: crate::fresh_stamp(),
            rows: OnceLock::new(),
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

impl VectorMask {
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
}
