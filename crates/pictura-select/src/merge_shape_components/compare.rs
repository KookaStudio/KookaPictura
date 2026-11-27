//! Conservative comparison of effective polygon fills after normalization.

use super::{FillRule, FloatOverlay, OverlayRule, VectorFillRule, VectorSubpath, Vertex};

const LIMIT: f64 = 1.0 / 256.0;
const MAX_RESOLVED_EDGES: usize = 512;
const MAX_DISTANCE_COMPARISONS: usize = 400_000;

/// Compare rendered mask polygons after a round trip through the mask encoder.
pub fn mask_fills_match(
    before: &[VectorSubpath],
    after: &[VectorSubpath],
    rule: VectorFillRule,
) -> bool {
    let contours = |paths: &[VectorSubpath]| -> Vec<Vec<Vertex>> {
        paths
            .iter()
            .filter(|p| p.closed && p.points.len() >= 3)
            .map(|p| {
                p.points
                    .iter()
                    .map(|p| [f64::from(p[0]) / 256.0, f64::from(p[1]) / 256.0])
                    .collect()
            })
            .collect()
    };
    let rule = match rule {
        VectorFillRule::EvenOdd => FillRule::EvenOdd,
        VectorFillRule::NonZero => FillRule::NonZero,
    };
    fills_match(&contours(before), &contours(after), rule)
}

pub(super) fn fills_match(before: &[Vec<Vertex>], after: &[Vec<Vertex>], rule: FillRule) -> bool {
    if before.is_empty() && after.is_empty() {
        return true;
    }
    // Both resolutions use the same adapter, even when their bounds differ.
    let resolve =
        |which| FloatOverlay::<Vertex, i64>::from_subj_and_clip(before, after).overlay(which, rule);
    let a = resolve(OverlayRule::Subject);
    let b = resolve(OverlayRule::Clip);
    let mut a_holes: Vec<_> = a.iter().map(|shape| shape.len()).collect();
    let mut b_holes: Vec<_> = b.iter().map(|shape| shape.len()).collect();
    a_holes.sort_unstable();
    b_holes.sort_unstable();
    if a_holes != b_holes {
        return false;
    }
    let edges = |shapes: &[Vec<Vec<Vertex>>]| -> Vec<(Vertex, Vertex)> {
        shapes
            .iter()
            .flatten()
            .flat_map(|ring| {
                ring.iter()
                    .copied()
                    .zip(ring.iter().copied().cycle().skip(1))
                    .take(ring.len())
            })
            .collect()
    };
    let a_edges = edges(&a);
    let b_edges = edges(&b);
    if a_edges.len() > MAX_RESOLVED_EDGES || b_edges.len() > MAX_RESOLVED_EDGES {
        return false;
    }
    if a_edges.is_empty() || b_edges.is_empty() {
        return a_edges.is_empty() && b_edges.is_empty();
    }
    if contacts(&b_edges) > contacts(&a_edges)
        || shared_junctions(&b_edges) > shared_junctions(&a_edges)
    {
        return false;
    }
    let mut budget = MAX_DISTANCE_COMPARISONS;
    directed(&a_edges, &b_edges, &mut budget) && directed(&b_edges, &a_edges, &mut budget)
}

fn directed(from: &[(Vertex, Vertex)], to: &[(Vertex, Vertex)], budget: &mut usize) -> bool {
    from.iter().all(|&(a, b)| certified(a, b, to, 0, budget))
}

// Distance to one line segment is convex: certifying both endpoints against
// that same segment certifies the entire subedge.
fn certified(a: Vertex, b: Vertex, to: &[(Vertex, Vertex)], depth: u8, budget: &mut usize) -> bool {
    for &(c, d) in to {
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        if distance(a, c, d) <= LIMIT {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            if distance(b, c, d) <= LIMIT {
                return true;
            }
        }
    }
    if depth == 20 {
        return false;
    }
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    certified(a, mid, to, depth + 1, budget) && certified(mid, b, to, depth + 1, budget)
}

fn distance(p: Vertex, a: Vertex, b: Vertex) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let length = d[0] * d[0] + d[1] * d[1];
    let t = if length == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / length).clamp(0.0, 1.0)
    };
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

fn contacts(edges: &[(Vertex, Vertex)]) -> usize {
    fn cross(a: Vertex, b: Vertex, c: Vertex) -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    fn touches(a: Vertex, b: Vertex, c: Vertex, d: Vertex) -> bool {
        if a[0].min(b[0]) > c[0].max(d[0])
            || c[0].min(d[0]) > a[0].max(b[0])
            || a[1].min(b[1]) > c[1].max(d[1])
            || c[1].min(d[1]) > a[1].max(b[1])
        {
            return false;
        }
        let (x, y, z, w) = (
            cross(a, b, c),
            cross(a, b, d),
            cross(c, d, a),
            cross(c, d, b),
        );
        (x <= 0.0 && y >= 0.0 || y <= 0.0 && x >= 0.0)
            && (z <= 0.0 && w >= 0.0 || w <= 0.0 && z >= 0.0)
    }
    let mut count = 0;
    for (i, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[i + 1..] {
            if a == c || a == d || b == c || b == d {
                continue;
            }
            if touches(a, b, c, d) {
                count += 1;
            }
        }
    }
    count
}

fn shared_junctions(edges: &[(Vertex, Vertex)]) -> usize {
    edges
        .iter()
        .enumerate()
        .filter(|(i, (a, _))| {
            let count = edges.iter().filter(|(b, _)| b == a).count();
            count > 1 && !edges[..*i].iter().any(|(b, _)| b == a)
        })
        .count()
}
