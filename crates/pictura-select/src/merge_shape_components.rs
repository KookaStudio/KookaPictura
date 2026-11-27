//! Area-preserving normalization for Path Selection's Merge Shape Components.

use i_overlay::core::{fill_rule::FillRule, overlay_rule::OverlayRule};
use i_overlay::float::overlay::{FloatOverlay, OverlayOptions};
use pictura_core::{
    path::{PathPoint, Subpath},
    VectorFillRule, VectorSubpath,
};

const MAX_CONTOURS: usize = 128;
const MAX_VERTICES: usize = 4096;
const MAX_HANDLES: usize = 128;
const MAX_COORDINATE: f64 = 1e8;
const CURVE_TOLERANCE: f64 = 0.1;
const VERTEX_EPSILON: f64 = 1e-7;

type Vertex = [f64; 2];

mod compare;
pub use compare::mask_fills_match;

struct Contour {
    source: usize,
    vertices: Vec<Vertex>,
    min: Vertex,
    max: Vertex,
}

impl Contour {
    fn new(source: usize, vertices: Vec<Vertex>) -> Self {
        let mut min = vertices[0];
        let mut max = vertices[0];
        for &[x, y] in &vertices[1..] {
            min = [min[0].min(x), min[1].min(y)];
            max = [max[0].max(x), max[1].max(y)];
        }
        Self {
            source,
            vertices,
            min,
            max,
        }
    }

    fn overlaps(&self, other: &Self, rule: FillRule) -> bool {
        (0..2).all(|axis| self.min[axis] < other.max[axis] && other.min[axis] < self.max[axis])
            && !FloatOverlay::<Vertex, i64>::from_subj_and_clip(&self.vertices, &other.vertices)
                .overlay(OverlayRule::Intersect, rule)
                .is_empty()
    }
}

/// Resolve overlapping contour groups using their existing fill rule. A group
/// that contains no interactions retains its original anchors and handles.
/// `None` means unchanged or outside the merge budget, `Some(vec![])` means
/// the input contours cancelled out under the requested fill rule.
pub fn merge_shape_components(
    subpaths: &[Subpath],
    rule: VectorFillRule,
    mask_contours: Option<&[VectorSubpath]>,
) -> Option<Vec<Subpath>> {
    if !(2..=MAX_CONTOURS).contains(&subpaths.len()) {
        return None;
    }
    if mask_contours.is_some_and(|mask| mask.len() != subpaths.len()) {
        return None;
    }
    let mut vertices_left = MAX_VERTICES;
    let mut handles_left = MAX_HANDLES;
    let mut anchors_left = MAX_VERTICES;
    let mut contours = Vec::new();
    for (source, subpath) in subpaths.iter().enumerate() {
        anchors_left = anchors_left.checked_sub(subpath.points.len())?;
        let mut vertices: Vec<Vertex> = if let Some(mask) = mask_contours {
            let contour = &mask[source];
            if !contour.closed {
                continue;
            }
            if contour.points.len() > vertices_left {
                return None;
            }
            contour
                .points
                .iter()
                .map(|&[x, y]| [f64::from(x) / 256.0, f64::from(y) / 256.0])
                .collect()
        } else {
            for point in &subpath.points {
                handles_left = handles_left.checked_sub(
                    usize::from(point.in_handle.is_some())
                        + usize::from(point.out_handle.is_some()),
                )?;
                for (x, y) in [Some(point.anchor), point.in_handle, point.out_handle]
                    .into_iter()
                    .flatten()
                {
                    if !(x.abs() <= MAX_COORDINATE && y.abs() <= MAX_COORDINATE) {
                        return None;
                    }
                }
            }
            subpath
                .flatten(CURVE_TOLERANCE)
                .into_iter()
                .map(|(x, y)| [x, y])
                .collect()
        };
        vertices_left = vertices_left.checked_sub(vertices.len())?;
        vertices.dedup();
        if vertices.first() == vertices.last() {
            vertices.pop();
        }
        if vertices.len() >= 3 {
            contours.push(Contour::new(source, vertices));
        }
    }

    let rule = match rule {
        VectorFillRule::EvenOdd => FillRule::EvenOdd,
        VectorFillRule::NonZero => FillRule::NonZero,
    };
    let mut used = vec![false; contours.len()];
    let mut replacements = vec![None; subpaths.len()];
    let mut remaining_output = MAX_VERTICES;
    for first in 0..contours.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let mut group = vec![first];
        let mut visited = 0;
        while visited < group.len() {
            let current = &contours[group[visited]];
            for (i, was_used) in used.iter_mut().enumerate().skip(first + 1) {
                if !*was_used && current.overlaps(&contours[i], rule) {
                    *was_used = true;
                    group.push(i);
                }
            }
            visited += 1;
        }
        if group.len() == 1 {
            continue;
        }
        group.sort_unstable();
        let input: Vec<_> = group
            .iter()
            .map(|&i| contours[i].vertices.as_slice())
            .collect();
        let mut options = OverlayOptions::default();
        options.preserve_input_collinear = true;
        options.preserve_output_collinear = true;
        options.clean_result = false;
        let normalized =
            FloatOverlay::<Vertex, i64>::from_subj_custom(&input, options, Default::default())
                .overlay(OverlayRule::Subject, rule);
        let rings: Vec<_> = normalized.into_iter().flatten().collect();
        let source: Vec<_> = input.iter().map(|ring| ring.to_vec()).collect();
        if !compare::fills_match(&source, &rings, rule) {
            return None;
        }
        if same_boundaries(&input, &rings) {
            continue;
        }
        let mut replacement = Vec::new();
        for ring in rings {
            remaining_output = remaining_output.checked_sub(ring.len())?;
            let points = ring
                .into_iter()
                .map(|[x, y]| PathPoint {
                    anchor: (x, y),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                })
                .collect();
            replacement.push(Subpath {
                points,
                closed: true,
            });
        }
        replacements[contours[first].source] = Some(replacement);
        for i in group.into_iter().skip(1) {
            replacements[contours[i].source] = Some(Vec::new());
        }
    }

    if replacements.iter().all(Option::is_none) {
        return None;
    }
    Some(
        subpaths
            .iter()
            .zip(replacements)
            .flat_map(|(original, replacement)| {
                replacement.unwrap_or_else(|| vec![original.clone()])
            })
            .collect(),
    )
}

fn close(a: Vertex, b: Vertex) -> bool {
    (a[0] - b[0]).abs() <= VERTEX_EPSILON && (a[1] - b[1]).abs() <= VERTEX_EPSILON
}

fn same_ring(a: &[Vertex], b: &[Vertex]) -> bool {
    let n = a.len();
    n == b.len()
        && b.iter().enumerate().any(|(start, &point)| {
            close(a[0], point)
                && ((0..n).all(|i| close(a[i], b[(start + i) % n]))
                    || (0..n).all(|i| close(a[i], b[(start + n - i) % n])))
        })
}

fn same_boundaries(input: &[&[Vertex]], output: &[Vec<Vertex>]) -> bool {
    if input.len() != output.len() {
        return false;
    }
    let mut matched = vec![false; output.len()];
    for ring in input {
        let Some(i) = output
            .iter()
            .enumerate()
            .position(|(i, out)| !matched[i] && same_ring(ring, out))
        else {
            return false;
        };
        matched[i] = true;
    }
    true
}

#[cfg(test)]
mod tests;
