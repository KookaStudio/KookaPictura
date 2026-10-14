//! A derived view of a layer's `vmsk` vector mask.
//!
//! The raw `vmsk` tagged block stays in [`crate::Layer::extra_blocks`] and is
//! re-emitted verbatim; this view is what the compositor can sample.

use std::sync::OnceLock;

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
/// equal whatever it holds, and it is not rebuilt, so set `subpaths` before
/// sampling.
#[derive(Clone, Default)]
pub struct CoverageRows(OnceLock<Rows>);

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
    /// Names these subpaths for caches derived from them; see
    /// [`VectorMask::ident`].
    ident: u64,
    /// The pixel row of `crossings[0]`.
    top: i32,
    non_zero: bool,
    /// Per pixel row, each edge crossing the row's centre line: its x (1/256
    /// px) and direction (+1 downward, -1 upward).
    crossings: Vec<Vec<(f64, i8)>>,
}

impl Rows {
    fn build(subpaths: &[VectorSubpath]) -> Rows {
        let closed: Vec<&VectorSubpath> = subpaths
            .iter()
            .filter(|s| s.closed && s.points.len() >= 3)
            .collect();
        let non_zero = closed
            .iter()
            .any(|s| s.fill_rule == VectorFillRule::NonZero);
        // Pixel row `y` samples the centre line `y * 256 + 128`; an edge from
        // `lo` to `hi` (1/256 px) crosses rows `ceil((lo - 128) / 256)` up to,
        // not including, `ceil((hi - 128) / 256)`.
        let row_of = |v: f64| ((v - 128.0) / 256.0).ceil() as i32;
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for s in &closed {
            for p in &s.points {
                top = top.min(row_of(p[1] as f64));
                bottom = bottom.max(row_of(p[1] as f64));
            }
        }
        if top >= bottom {
            return Rows {
                ident: crate::fresh_stamp(),
                top: 0,
                non_zero,
                crossings: Vec::new(),
            };
        }
        let mut crossings = vec![Vec::new(); (bottom - top) as usize];
        for s in &closed {
            for i in 0..s.points.len() {
                let [ax, ay] = s.points[i].map(f64::from);
                let [bx, by] = s.points[(i + 1) % s.points.len()].map(f64::from);
                if ay == by {
                    continue;
                }
                let dir: i8 = if by > ay { 1 } else { -1 };
                for y in row_of(ay.min(by))..row_of(ay.max(by)) {
                    let cy = f64::from(y) * 256.0 + 128.0;
                    let x = ax + (cy - ay) * (bx - ax) / (by - ay);
                    crossings[(y - top) as usize].push((x, dir));
                }
            }
        }
        Rows {
            ident: crate::fresh_stamp(),
            top,
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
    }

    /// A nonzero id shared by this mask's clones and by no mask built from
    /// other subpaths, for caches of its coverage. Like the row index it
    /// rides on, it assumes `subpaths` is set before the first query.
    pub fn ident(&self) -> u64 {
        self.rows
            .0
            .get_or_init(|| Rows::build(&self.subpaths))
            .ident
    }

    /// Whether the centre of pixel `(x, y)` lies inside the mask's closed
    /// subpaths: even-odd, or non-zero when any subpath declares it. Ignores
    /// `invert` and `disabled`.
    pub fn inside(&self, x: i32, y: i32) -> bool {
        let rows = self.rows.0.get_or_init(|| Rows::build(&self.subpaths));
        let Some(row) = usize::try_from(y - rows.top)
            .ok()
            .and_then(|i| rows.crossings.get(i))
        else {
            return false;
        };
        let cx = f64::from(x) * 256.0 + 128.0;
        let right = row.iter().filter(|(px, _)| *px > cx);
        if rows.non_zero {
            right.map(|(_, d)| i32::from(*d)).sum::<i32>() != 0
        } else {
            right.count() % 2 == 1
        }
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
