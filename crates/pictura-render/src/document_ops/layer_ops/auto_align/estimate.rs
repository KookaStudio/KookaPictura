//! Robust transform estimation for Auto-Align: translation, similarity,
//! homography, and an overlap score. The estimators are deterministic: the
//! RANSAC sampler is a fixed-seed xorshift and every candidate set is visited
//! in index order.

use pictura_core::Homography;

use super::image::WorkImage;

/// One correspondence, moving/source `a` to reference/destination `b`, in
/// document coordinates.
#[derive(Debug, Clone, Copy)]
pub(super) struct Pair {
    pub a: (f64, f64),
    pub b: (f64, f64),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Model {
    Translation { dx: f64, dy: f64 },
    Similarity { a: f64, b: f64, tx: f64, ty: f64 },
    Homography { h: [f64; 9] },
}

impl Model {
    pub fn forward(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        match self {
            Model::Translation { dx, dy } => Some((x + dx, y + dy)),
            Model::Similarity { a, b, tx, ty } => Some((a * x - b * y + tx, b * x + a * y + ty)),
            Model::Homography { h } => apply_h(h, x, y),
        }
    }

    pub fn homography(&self) -> Option<[f64; 9]> {
        match self {
            Model::Translation { dx, dy } => Some([1.0, 0.0, *dx, 0.0, 1.0, *dy, 0.0, 0.0, 1.0]),
            Model::Similarity { a, b, tx, ty } => Some([*a, -*b, *tx, *b, *a, *ty, 0.0, 0.0, 1.0]),
            Model::Homography { h } => Some(*h),
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn index(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn inliers(pairs: &[Pair], model: &Model, threshold: f64) -> Vec<usize> {
    pairs
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let q = model.forward(p.a.0, p.a.1)?;
            ((q.0 - p.b.0).hypot(q.1 - p.b.1) <= threshold).then_some(i)
        })
        .collect()
}

pub(super) fn ransac_translation(pairs: &[Pair], threshold: f64) -> Option<Model> {
    if pairs.is_empty() {
        return None;
    }
    let mut rng = Rng::new();
    let mut best: Option<(usize, Model)> = None;
    for _ in 0..200 {
        let p = pairs[rng.index(pairs.len())];
        let model = Model::Translation {
            dx: p.b.0 - p.a.0,
            dy: p.b.1 - p.a.1,
        };
        let count = inliers(pairs, &model, threshold).len();
        if best.as_ref().is_none_or(|(n, _)| count > *n) {
            best = Some((count, model));
        }
    }
    let (count, model) = best?;
    if count < 2 {
        return None;
    }
    let ids = inliers(pairs, &model, threshold);
    let (mut dx, mut dy) = (0.0, 0.0);
    for &i in &ids {
        dx += pairs[i].b.0 - pairs[i].a.0;
        dy += pairs[i].b.1 - pairs[i].a.1;
    }
    let n = ids.len() as f64;
    Some(Model::Translation {
        dx: dx / n,
        dy: dy / n,
    })
}

pub(super) fn ransac_similarity(pairs: &[Pair], threshold: f64) -> Option<Model> {
    if pairs.len() < 2 {
        return None;
    }
    let mut rng = Rng::new();
    let mut best: Option<(usize, Model)> = None;
    for _ in 0..500 {
        let i = rng.index(pairs.len());
        let j = rng.index(pairs.len());
        if i == j {
            continue;
        }
        let (p, q) = (pairs[i], pairs[j]);
        let (px, py) = (q.a.0 - p.a.0, q.a.1 - p.a.1);
        let (qx, qy) = (q.b.0 - p.b.0, q.b.1 - p.b.1);
        let len = px * px + py * py;
        if len < 1e-9 {
            continue;
        }
        let a = (qx * px + qy * py) / len;
        let b = (px * qy - py * qx) / len;
        if (a * a + b * b).sqrt() < 1e-6 {
            continue;
        }
        let model = Model::Similarity {
            a,
            b,
            tx: p.b.0 - (a * p.a.0 - b * p.a.1),
            ty: p.b.1 - (b * p.a.0 + a * p.a.1),
        };
        let count = inliers(pairs, &model, threshold).len();
        if best.as_ref().is_none_or(|(n, _)| count > *n) {
            best = Some((count, model));
        }
    }
    let (count, model) = best?;
    if count < 2 {
        return None;
    }
    let ids = inliers(pairs, &model, threshold);
    let fitted = fit_similarity(&ids.iter().map(|&i| pairs[i]).collect::<Vec<_>>())?;
    Some(Model::Similarity {
        a: fitted[0],
        b: fitted[1],
        tx: fitted[2],
        ty: fitted[3],
    })
}

fn fit_similarity(pairs: &[Pair]) -> Option<[f64; 4]> {
    let mut ata = [[0.0f64; 4]; 4];
    let mut atb = [0.0f64; 4];
    for p in pairs {
        let rows = [
            ([p.a.0, -p.a.1, 1.0, 0.0], p.b.0),
            ([p.a.1, p.a.0, 0.0, 1.0], p.b.1),
        ];
        for (row, rhs) in rows {
            for i in 0..4 {
                for j in 0..4 {
                    ata[i][j] += row[i] * row[j];
                }
                atb[i] += row[i] * rhs;
            }
        }
    }
    let v = solve(ata, atb)?;
    Some([v[0], v[1], v[2], v[3]])
}

pub(super) fn ransac_homography(pairs: &[Pair], threshold: f64) -> Option<Model> {
    if pairs.len() < 4 {
        return None;
    }
    let mut rng = Rng::new();
    let mut best: Option<(usize, Model)> = None;
    for _ in 0..1000 {
        let mut ids = [0usize; 4];
        for id in &mut ids {
            *id = rng.index(pairs.len());
        }
        if ids[0] == ids[1]
            || ids[0] == ids[2]
            || ids[0] == ids[3]
            || ids[1] == ids[2]
            || ids[1] == ids[3]
            || ids[2] == ids[3]
        {
            continue;
        }
        let src = ids.map(|i| pairs[i].a);
        let dst = ids.map(|i| pairs[i].b);
        let Some(h) = Homography::from_quads(src, dst) else {
            continue;
        };
        let model = Model::Homography { h: h.0 };
        let count = inliers(pairs, &model, threshold).len();
        if best.as_ref().is_none_or(|(n, _)| count > *n) {
            best = Some((count, model));
        }
    }
    let (count, model) = best?;
    if count < 4 {
        return None;
    }
    let ids = inliers(pairs, &model, threshold);
    let selected: Vec<Pair> = ids.iter().map(|&i| pairs[i]).collect();
    let h = dlt(&selected)?;
    Some(Model::Homography { h })
}

/// Normalised direct linear transform: a least-squares homography over every
/// correspondence.
pub(super) fn dlt(pairs: &[Pair]) -> Option<[f64; 9]> {
    if pairs.len() < 4 {
        return None;
    }
    let (sc, ss) = normalisation(pairs.iter().map(|p| p.a))?;
    let (dc, ds) = normalisation(pairs.iter().map(|p| p.b))?;
    let mut ata = [[0.0f64; 8]; 8];
    let mut atb = [0.0f64; 8];
    for p in pairs {
        let x = (p.a.0 - sc.0) * ss;
        let y = (p.a.1 - sc.1) * ss;
        let u = (p.b.0 - dc.0) * ds;
        let v = (p.b.1 - dc.1) * ds;
        let rows = [
            ([x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u], u),
            ([0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v], v),
        ];
        for (row, rhs) in rows {
            for i in 0..8 {
                for j in 0..8 {
                    ata[i][j] += row[i] * row[j];
                }
                atb[i] += row[i] * rhs;
            }
        }
    }
    let v = solve(ata, atb)?;
    let hn = [v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], 1.0];
    let t_src = [ss, 0.0, -ss * sc.0, 0.0, ss, -ss * sc.1, 0.0, 0.0, 1.0];
    let t_dst_inv = [1.0 / ds, 0.0, dc.0, 0.0, 1.0 / ds, dc.1, 0.0, 0.0, 1.0];
    let h = mul3(&mul3(&t_dst_inv, &hn), &t_src);
    (h[8].abs() > 1e-12).then(|| h.map(|v| v / h[8]))
}

fn normalisation(points: impl Iterator<Item = (f64, f64)>) -> Option<((f64, f64), f64)> {
    let points: Vec<(f64, f64)> = points.collect();
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    let cx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let cy = points.iter().map(|p| p.1).sum::<f64>() / n;
    let mean = points
        .iter()
        .map(|p| (p.0 - cx).hypot(p.1 - cy))
        .sum::<f64>()
        / n;
    (mean > 1e-9).then_some(((cx, cy), std::f64::consts::SQRT_2 / mean))
}

fn mul3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 3 + c] = (0..3).map(|k| a[r * 3 + k] * b[k * 3 + c]).sum();
        }
    }
    out
}

pub(super) fn apply_h(h: &[f64; 9], x: f64, y: f64) -> Option<(f64, f64)> {
    let w = h[6] * x + h[7] * y + h[8];
    if !w.is_finite() || w.abs() < 1e-9 {
        return None;
    }
    let q = (
        (h[0] * x + h[1] * y + h[2]) / w,
        (h[3] * x + h[4] * y + h[5]) / w,
    );
    (q.0.is_finite() && q.1.is_finite()).then_some(q)
}

pub(super) fn invert_h(h: &[f64; 9]) -> Option<[f64; 9]> {
    let det = h[0] * (h[4] * h[8] - h[5] * h[7]) - h[1] * (h[3] * h[8] - h[5] * h[6])
        + h[2] * (h[3] * h[7] - h[4] * h[6]);
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let id = 1.0 / det;
    Some([
        (h[4] * h[8] - h[5] * h[7]) * id,
        (h[2] * h[7] - h[1] * h[8]) * id,
        (h[1] * h[5] - h[2] * h[4]) * id,
        (h[5] * h[6] - h[3] * h[8]) * id,
        (h[0] * h[8] - h[2] * h[6]) * id,
        (h[2] * h[3] - h[0] * h[5]) * id,
        (h[3] * h[7] - h[4] * h[6]) * id,
        (h[1] * h[6] - h[0] * h[7]) * id,
        (h[0] * h[4] - h[1] * h[3]) * id,
    ])
}

/// Normalised cross-correlation over the overlap of `reference` and `moving`.
/// `inverse` maps reference work coordinates to moving work coordinates.
/// Returns `None` with too few valid samples.
pub(super) fn score_overlap(
    reference: &WorkImage,
    moving: &WorkImage,
    inverse: impl Fn(f64, f64) -> Option<(f64, f64)>,
) -> Option<f64> {
    let (mut n, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut sampled: f64 = 0.0;
    let mut y = 0usize;
    while y < reference.height {
        let mut x = 0usize;
        while x < reference.width {
            let i = y * reference.width + x;
            if reference.alpha[i] {
                sampled += 1.0;
                if let Some((mx, my)) = inverse(x as f64, y as f64) {
                    if let Some((m, true)) = moving.sample(mx, my) {
                        let r = f64::from(reference.luma[i]);
                        let m = f64::from(m);
                        n += 1.0;
                        sx += r;
                        sy += m;
                        sxx += r * r;
                        syy += m * m;
                        sxy += r * m;
                    }
                }
            }
            x += 2;
        }
        y += 2;
    }
    if n < 100.0 {
        return None;
    }
    let cov = sxy / n - (sx / n) * (sy / n);
    let vx = sxx / n - (sx / n).powi(2);
    let vy = syy / n - (sy / n).powi(2);
    if vx <= 1e-12 || vy <= 1e-12 {
        return None;
    }
    let ncc = cov / (vx * vy).sqrt();
    let overlap = (n / sampled.max(1.0)).sqrt();
    Some(ncc * overlap)
}

fn solve<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let mut pivot = col;
        for row in col + 1..N {
            if a[row][col].abs() > a[pivot][col].abs() {
                pivot = row;
            }
        }
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        if pivot != col {
            a.swap(pivot, col);
            b.swap(pivot, col);
        }
        let d = a[col][col];
        for value in a[col].iter_mut().skip(col) {
            *value /= d;
        }
        b[col] /= d;
        let pivot_row = a[col];
        for row in 0..N {
            if row == col {
                continue;
            }
            let f = a[row][col];
            if f == 0.0 {
                continue;
            }
            for (j, value) in a[row].iter_mut().enumerate().skip(col) {
                *value -= f * pivot_row[j];
            }
            b[row] -= f * b[col];
        }
    }
    Some(b)
}
