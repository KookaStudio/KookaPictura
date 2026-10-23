//! The constraint solve: coarse-grid output positions by linear least squares
//! over a similarity (conformal) energy per triangle, a weak anchor to the
//! base projection, and a straightness term per constraint sample. Unfixed
//! lines take their direction from the previous pass. The normal equations
//! are assembled row by row in insertion order, so sums are deterministic,
//! and solved by Jacobi-preconditioned conjugate gradients.

use super::{dot, normalize, AwaConstraint, AwaOrientation, Grid, Lens};

const SHAPE_WEIGHT: f64 = 1.0;
/// Inside a polygon the perspective shape is pressed harder.
const POLYGON_WEIGHT: f64 = 4.0;
const ANCHOR_WEIGHT: f64 = 0.1;
/// Vertices past a fisheye's rim stay on it.
const RIM_WEIGHT: f64 = 10.0;
const LINE_WEIGHT: f64 = 10.0;
const UNFIXED_PASSES: usize = 5;
const MAX_ITERATIONS: usize = 4000;
const TOLERANCE: f64 = 1e-10;

/// A trace sample's output, `Σ w·v + k`: bilinear over the coarse vertices
/// plus the exact base curve's departure from that bilinear.
struct Sample {
    weights: [(usize, f64); 4],
    k: [f64; 2],
}

struct LineTerm {
    samples: Vec<Sample>,
    orientation: AwaOrientation,
}

/// A valid polygon: its traced boundary and the direction it faces.
struct Polygon {
    boundary: Vec<[f64; 2]>,
    axis: [f64; 3],
}

pub(super) fn solve(
    lens: &Lens,
    grid: &Grid,
    base: &[[f64; 2]],
    constraints: &[AwaConstraint],
    traces: &[Option<Vec<Vec<[f64; 2]>>>],
) -> Vec<[f64; 2]> {
    let sample = |p: [f64; 2]| {
        let weights = grid.weights(p);
        let b = lens.base(p);
        let mut k = b;
        for (i, w) in weights {
            k[0] -= w * base[i][0];
            k[1] -= w * base[i][1];
        }
        Sample { weights, k }
    };
    let mut lines = Vec::new();
    let mut polygons = Vec::new();
    for (c, trace) in constraints.iter().zip(traces) {
        let Some(edges) = trace else {
            continue;
        };
        let orientation = if c.polygon {
            AwaOrientation::Unfixed
        } else {
            c.orientation
        };
        for edge in edges {
            lines.push(LineTerm {
                samples: edge.iter().map(|&p| sample(p)).collect(),
                orientation,
            });
        }
        if c.polygon {
            let mut axis = [0.0; 3];
            for p in &c.points {
                if let Some(v) = lens.dir([p[0] * lens.w, p[1] * lens.h]) {
                    axis = [axis[0] + v[0], axis[1] + v[1], axis[2] + v[2]];
                }
            }
            if dot(axis, axis) > 1e-12 {
                polygons.push(Polygon {
                    boundary: edges.concat(),
                    axis: normalize(axis),
                });
            }
        }
    }
    if lines.is_empty() {
        return base.to_vec();
    }

    let n = grid.vertices();
    let points: Vec<[f64; 2]> = (0..n)
        .map(|k| grid.at(k % grid.nx(), k / grid.nx()))
        .collect();
    let clipped: Vec<bool> = points.iter().map(|&p| lens.clipped(p)).collect();
    let mut system = Normal::new(2 * n);
    for k in 0..n {
        let w = if clipped[k] {
            RIM_WEIGHT
        } else {
            ANCHOR_WEIGHT
        };
        system.add(&[(2 * k, 1.0)], base[k][0], w);
        system.add(&[(2 * k + 1, 1.0)], base[k][1], w);
    }
    let nx = grid.nx();
    for j in 0..grid.cy {
        for i in 0..grid.cx {
            let k = j * nx + i;
            for tri in [[k, k + 1, k + nx + 1], [k, k + nx + 1, k + nx]] {
                if tri.iter().any(|&v| clipped[v]) {
                    continue;
                }
                let (reference, weight) = reference_shape(lens, &points, base, &polygons, tri);
                add_similarity(&mut system, tri, reference, weight);
            }
        }
    }

    let mut x: Vec<f64> = base.iter().flat_map(|p| *p).collect();
    let passes = if lines
        .iter()
        .any(|l| l.orientation == AwaOrientation::Unfixed)
    {
        UNFIXED_PASSES
    } else {
        1
    };
    for _ in 0..passes {
        let mut pass = system.clone();
        for line in &lines {
            add_line(&mut pass, line, &x);
        }
        x = pass.conjugate_gradients(x);
    }
    x.as_chunks::<2>().0.to_vec()
}

/// The shape triangle `tri` should keep: the base projection's, or inside a
/// polygon the perspective (gnomonic) projection about its axis.
fn reference_shape(
    lens: &Lens,
    points: &[[f64; 2]],
    base: &[[f64; 2]],
    polygons: &[Polygon],
    tri: [usize; 3],
) -> ([[f64; 2]; 3], f64) {
    let plain = (tri.map(|k| base[k]), SHAPE_WEIGHT);
    let centroid = [
        (points[tri[0]][0] + points[tri[1]][0] + points[tri[2]][0]) / 3.0,
        (points[tri[0]][1] + points[tri[1]][1] + points[tri[2]][1]) / 3.0,
    ];
    let Some(polygon) = polygons.iter().find(|p| inside(&p.boundary, centroid)) else {
        return plain;
    };
    let a = polygon.axis;
    let helper = if a[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = normalize(cross(helper, a));
    let v = cross(a, u);
    let mut shape = [[0.0; 2]; 3];
    for (out, &k) in shape.iter_mut().zip(&tri) {
        let Some(d) = lens.dir(points[k]) else {
            return plain;
        };
        let z = dot(d, a);
        if z < 0.05 {
            return plain;
        }
        *out = [dot(d, u) / z, dot(d, v) / z];
    }
    (shape, POLYGON_WEIGHT)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Even-odd point-in-polygon.
fn inside(boundary: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut odd = false;
    let n = boundary.len();
    for i in 0..n {
        let (a, b) = (boundary[i], boundary[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                odd = !odd;
            }
        }
    }
    odd
}

/// Each vertex of `tri` must sit where a similarity of `reference` puts it
/// relative to the other two (all three rotations, for symmetry).
fn add_similarity(system: &mut Normal, tri: [usize; 3], reference: [[f64; 2]; 3], weight: f64) {
    for r in 0..3 {
        let (a, b, c) = (r, (r + 1) % 3, (r + 2) % 3);
        let d = [
            reference[b][0] - reference[a][0],
            reference[b][1] - reference[a][1],
        ];
        let e = [
            reference[c][0] - reference[a][0],
            reference[c][1] - reference[a][1],
        ];
        let dd = d[0] * d[0] + d[1] * d[1];
        if dd < 1e-12 {
            continue;
        }
        let alpha = (e[0] * d[0] + e[1] * d[1]) / dd;
        let beta = (e[1] * d[0] - e[0] * d[1]) / dd;
        let (ax, ay) = (2 * tri[a], 2 * tri[a] + 1);
        let (bx, by) = (2 * tri[b], 2 * tri[b] + 1);
        let (cx, cy) = (2 * tri[c], 2 * tri[c] + 1);
        system.add(
            &[
                (cx, 1.0),
                (ax, alpha - 1.0),
                (bx, -alpha),
                (by, beta),
                (ay, -beta),
            ],
            0.0,
            weight,
        );
        system.add(
            &[
                (cy, 1.0),
                (ay, alpha - 1.0),
                (by, -alpha),
                (bx, -beta),
                (ax, beta),
            ],
            0.0,
            weight,
        );
    }
}

fn output(sample: &Sample, x: &[f64]) -> [f64; 2] {
    let mut p = sample.k;
    for &(i, w) in &sample.weights {
        p[0] += w * x[2 * i];
        p[1] += w * x[2 * i + 1];
    }
    p
}

/// Every sample of `line` on one line through its first sample, across the
/// normal its orientation (or, unfixed, the current solution) gives.
fn add_line(system: &mut Normal, line: &LineTerm, x: &[f64]) {
    let (Some(first), Some(last)) = (line.samples.first(), line.samples.last()) else {
        return;
    };
    let normal = match line.orientation {
        AwaOrientation::Horizontal => [0.0, 1.0],
        AwaOrientation::Vertical => [1.0, 0.0],
        AwaOrientation::Fixed(angle) => [-angle.sin(), angle.cos()],
        AwaOrientation::Unfixed => {
            let (a, b) = (output(first, x), output(last, x));
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx.hypot(dy);
            if len < 1e-9 {
                return;
            }
            [-dy / len, dx / len]
        }
    };
    for s in &line.samples[1..] {
        let mut entries = Vec::with_capacity(16);
        let anchor = first.weights.iter().map(|&(i, w)| (i, -w));
        for (i, w) in s.weights.iter().copied().chain(anchor) {
            entries.push((2 * i, w * normal[0]));
            entries.push((2 * i + 1, w * normal[1]));
        }
        let target = -(normal[0] * (s.k[0] - first.k[0]) + normal[1] * (s.k[1] - first.k[1]));
        system.add(&entries, target, LINE_WEIGHT);
    }
}

/// `AᵀA x = Aᵀb`, one sparse row per unknown.
#[derive(Clone)]
struct Normal {
    rows: Vec<Vec<(u32, f64)>>,
    rhs: Vec<f64>,
}

impl Normal {
    fn new(n: usize) -> Self {
        Self {
            rows: vec![Vec::new(); n],
            rhs: vec![0.0; n],
        }
    }

    /// Add the residual `Σ a_i x_i − target`, weighted.
    fn add(&mut self, entries: &[(usize, f64)], target: f64, weight: f64) {
        let w2 = weight * weight;
        for &(i, a) in entries {
            if a == 0.0 {
                continue;
            }
            self.rhs[i] += w2 * a * target;
            let row = &mut self.rows[i];
            for &(j, b) in entries {
                if b == 0.0 {
                    continue;
                }
                let v = w2 * a * b;
                match row.iter_mut().find(|e| e.0 == j as u32) {
                    Some(e) => e.1 += v,
                    None => row.push((j as u32, v)),
                }
            }
        }
    }

    fn multiply(&self, x: &[f64], out: &mut [f64]) {
        for (o, row) in out.iter_mut().zip(&self.rows) {
            *o = row.iter().map(|&(j, v)| v * x[j as usize]).sum();
        }
    }

    fn conjugate_gradients(&self, mut x: Vec<f64>) -> Vec<f64> {
        let n = x.len();
        let diag: Vec<f64> = (0..n)
            .map(|i| {
                self.rows[i]
                    .iter()
                    .find(|e| e.0 as usize == i)
                    .map_or(1.0, |e| e.1)
            })
            .collect();
        let mut ax = vec![0.0; n];
        self.multiply(&x, &mut ax);
        let mut r: Vec<f64> = self.rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
        let norm_b = self
            .rhs
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt()
            .max(1e-30);
        let mut z: Vec<f64> = r.iter().zip(&diag).map(|(r, d)| r / d).collect();
        let mut p = z.clone();
        let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let mut ap = vec![0.0; n];
        for _ in 0..MAX_ITERATIONS {
            if r.iter().map(|v| v * v).sum::<f64>().sqrt() <= TOLERANCE * norm_b {
                break;
            }
            self.multiply(&p, &mut ap);
            let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
            if pap.abs() < 1e-300 {
                break;
            }
            let alpha = rz / pap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            for i in 0..n {
                z[i] = r[i] / diag[i];
            }
            let next: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
            let beta = next / rz;
            rz = next;
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
        }
        x
    }
}
