//! The Custom Shape tool's built-in shapes, in the order its picker lists
//! them. Photoshop ships its shapes as `.csh` artwork; these are generated
//! (CS6's set is Adobe's artwork). Each is designed in its own coordinates and
//! stretched to the dragged box; its designed proportions are what Shift keeps.
//! Ported from photorust's `CUSTOM_SHAPE_NAMES` / `unit_shape` / `heart`.

use super::{closed, fit, point};
use crate::path::Subpath;

pub const CUSTOM_SHAPE_NAMES: [&str; 6] = ["Star", "Heart", "Arrow", "Cross", "Lightning", "Check"];

/// Shape `index` (Star when out of range) in its design coordinates, as a
/// closed polygon of corner anchors.
pub(super) fn unit_shape(index: usize) -> Subpath {
    let points: Vec<(f64, f64)> = match CUSTOM_SHAPE_NAMES.get(index).copied().unwrap_or("Star") {
        "Heart" => heart(),
        "Arrow" => vec![
            (0.0, 0.32),
            (0.55, 0.32),
            (0.55, 0.06),
            (1.0, 0.5),
            (0.55, 0.94),
            (0.55, 0.68),
            (0.0, 0.68),
        ],
        "Cross" => vec![
            (0.34, 0.0),
            (0.66, 0.0),
            (0.66, 0.34),
            (1.0, 0.34),
            (1.0, 0.66),
            (0.66, 0.66),
            (0.66, 1.0),
            (0.34, 1.0),
            (0.34, 0.66),
            (0.0, 0.66),
            (0.0, 0.34),
            (0.34, 0.34),
        ],
        "Lightning" => vec![
            (0.58, 0.0),
            (0.9, 0.0),
            (0.62, 0.4),
            (0.86, 0.4),
            (0.3, 1.0),
            (0.44, 0.56),
            (0.16, 0.56),
            (0.34, 0.0),
        ],
        "Check" => vec![
            (0.9, 0.12),
            (1.0, 0.28),
            (0.42, 0.95),
            (0.0, 0.58),
            (0.12, 0.42),
            (0.44, 0.7),
        ],
        // Ten points alternating between two radii, the first pointing up.
        _ => (0..10)
            .map(|i| {
                let radius = if i % 2 == 0 { 0.5 } else { 0.2 };
                let angle =
                    -std::f64::consts::FRAC_PI_2 + f64::from(i) / 10.0 * std::f64::consts::TAU;
                (0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
            })
            .collect(),
    };
    closed(points.into_iter().map(|a| point(a, None, None)).collect())
}

/// The heart from the usual parametric curve, y flipped for a raster.
fn heart() -> Vec<(f64, f64)> {
    const STEPS: usize = 48;
    (0..STEPS)
        .map(|i| {
            let t = i as f64 / STEPS as f64 * std::f64::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            (x, -y)
        })
        .collect()
}

/// Shape `index`'s designed width over height.
pub(super) fn aspect(index: usize) -> f64 {
    let shape = unit_shape(index);
    let (l, t, r, b) = shape.points.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(l, t, r, b), p| {
            (
                l.min(p.anchor.0),
                t.min(p.anchor.1),
                r.max(p.anchor.0),
                b.max(p.anchor.1),
            )
        },
    );
    if b > t {
        (r - l) / (b - t)
    } else {
        1.0
    }
}

/// Shape `index` as the picker shows it: a `size` square, row-major coverage
/// (0–255) of the shape at its designed proportions, inset a tenth of the side.
/// Empty for an unknown index.
pub fn custom_shape_preview(index: usize, size: u32) -> Vec<u8> {
    centred_coverage(index, size, (f64::from(size) * 0.1).max(1.0))
}

/// Shape `index` as a blur kernel: like [`custom_shape_preview`] but filling
/// the `size` square edge to edge along its longer side.
pub fn custom_shape_kernel(index: usize, size: u32) -> Vec<u8> {
    centred_coverage(index, size, 0.0)
}

fn centred_coverage(index: usize, size: u32, inset: f64) -> Vec<u8> {
    if index >= CUSTOM_SHAPE_NAMES.len() || size == 0 {
        return Vec::new();
    }
    let side = f64::from(size);
    let room = (side - inset * 2.0).max(1.0);
    let a = aspect(index);
    let (w, h) = if a >= 1.0 {
        (room, room / a)
    } else {
        (room * a, room)
    };
    let rect = ((side - w) / 2.0, (side - h) / 2.0, w, h);
    fit(&unit_shape(index), rect).map_or_else(Vec::new, |s| super::coverage(&s, size, size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{outline, outline_in_box, ShapeKind, ShapeOptions};

    #[test]
    fn every_custom_shape_fills_its_box() {
        for (index, name) in CUSTOM_SHAPE_NAMES.iter().enumerate() {
            let mut o = ShapeOptions::new(ShapeKind::CustomShape, 0.0, 3);
            o.custom = index;
            let s = outline_in_box(o, (10.0, 20.0, 100.0, 50.0)).unwrap();
            assert!(s.closed && s.points.len() >= 6, "{}", name);
            let xs = s.points.iter().map(|p| p.anchor.0);
            let ys = s.points.iter().map(|p| p.anchor.1);
            let (l, r) = xs.fold((f64::MAX, f64::MIN), |(l, r), x| (l.min(x), r.max(x)));
            let (t, b) = ys.fold((f64::MAX, f64::MIN), |(t, b), y| (t.min(y), b.max(y)));
            assert!(
                (l - 10.0).abs() < 1e-6 && (r - 110.0).abs() < 1e-6,
                "{}",
                name
            );
            assert!(
                (t - 20.0).abs() < 1e-6 && (b - 70.0).abs() < 1e-6,
                "{}",
                name
            );
        }
    }

    #[test]
    fn shift_keeps_the_designed_proportions() {
        let mut o = ShapeOptions::new(ShapeKind::CustomShape, 0.0, 3);
        o.custom = 1;
        let heart = outline(o, (0.0, 0.0), (100.0, 10.0), true, false).unwrap();
        let (r, b) = heart.points.iter().fold((0.0f64, 0.0f64), |(r, b), p| {
            (r.max(p.anchor.0), b.max(p.anchor.1))
        });
        assert!(
            (r / b - aspect(1)).abs() < 1e-3,
            "the heart kept its proportions"
        );
        assert!((r - 100.0).abs() < 1e-6, "the box grew to the pointer");
    }

    #[test]
    fn previews_are_square_silhouettes() {
        let star = custom_shape_preview(0, 32);
        assert_eq!(star.len(), 32 * 32);
        assert_eq!(star[16 * 32 + 16], 255, "the middle is filled");
        assert_eq!(star[0], 0, "the corner is empty");
        assert!(custom_shape_preview(99, 32).is_empty());
    }

    #[test]
    fn a_kernel_reaches_the_square_edge_a_preview_is_inset_from() {
        let size = 41;
        let rows_touched = |c: &[u8]| {
            let first = c.iter().position(|&v| v > 0).unwrap() / size;
            let last = c.iter().rposition(|&v| v > 0).unwrap() / size;
            (first, last)
        };
        // The cross is square, so it fills both axes.
        let kernel = custom_shape_kernel(3, size as u32);
        assert_eq!(rows_touched(&kernel), (0, size - 1));
        assert_eq!(
            kernel[(size / 2) * size],
            255,
            "the bar reaches the left edge"
        );
        let (first, last) = rows_touched(&custom_shape_preview(3, size as u32));
        assert!(
            first >= 3 && last <= size - 4,
            "the preview keeps its inset"
        );
        assert!(custom_shape_kernel(CUSTOM_SHAPE_NAMES.len(), 9).is_empty());
    }
}
