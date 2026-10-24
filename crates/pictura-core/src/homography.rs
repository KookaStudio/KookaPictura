//! Planar projective maps: the four-point homography solve shared by the
//! perspective transforms, Perspective Crop and Vanishing Point.

/// Determinant and homogeneous-divisor floor: a projective map whose scale
/// collapses below this is treated as singular.
pub const HOMOGRAPHY_EPSILON: f64 = 1e-12;
/// Gaussian-elimination pivot floor; a zero (or vanishing) pivot is singular.
const PIVOT_EPSILON: f64 = 1e-12;

/// A row-major 3×3 homography acting on `(x, y, 1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography(pub [f64; 9]);

impl Homography {
    /// The homography sending `src[i]` to `dst[i]`, normalised so the last
    /// coefficient is 1; `None` for a non-finite corner or a singular system.
    pub fn from_quads(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<Self> {
        if src
            .iter()
            .chain(dst.iter())
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return None;
        }
        let mut a = [[0.0f64; 8]; 8];
        let mut b = [0.0f64; 8];
        for i in 0..4 {
            let (x, y) = src[i];
            let (u, v) = dst[i];
            a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u];
            b[2 * i] = u;
            a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v];
            b[2 * i + 1] = v;
        }
        let s = gaussian_solve(a, b)?;
        Some(Homography([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7], 1.0,
        ]))
    }

    /// The image of `(x, y)`; `None` when the homogeneous divisor is
    /// non-finite or ~0 (a point on the horizon).
    pub fn apply(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let m = &self.0;
        let w = m[6] * x + m[7] * y + m[8];
        if !w.is_finite() || w.abs() < HOMOGRAPHY_EPSILON {
            return None;
        }
        let px = (m[0] * x + m[1] * y + m[2]) / w;
        let py = (m[3] * x + m[4] * y + m[5]) / w;
        (px.is_finite() && py.is_finite()).then_some((px, py))
    }

    pub fn det(&self) -> f64 {
        let m = &self.0;
        m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6])
    }

    /// The inverse map, or `None` when the matrix is singular.
    pub fn inverse(&self) -> Option<Self> {
        let det = self.det();
        if !det.is_finite() || det.abs() < HOMOGRAPHY_EPSILON {
            return None;
        }
        let m = &self.0;
        let id = 1.0 / det;
        Some(Homography([
            (m[4] * m[8] - m[5] * m[7]) * id,
            (m[2] * m[7] - m[1] * m[8]) * id,
            (m[1] * m[5] - m[2] * m[4]) * id,
            (m[5] * m[6] - m[3] * m[8]) * id,
            (m[0] * m[8] - m[2] * m[6]) * id,
            (m[2] * m[3] - m[0] * m[5]) * id,
            (m[3] * m[7] - m[4] * m[6]) * id,
            (m[1] * m[6] - m[0] * m[7]) * id,
            (m[0] * m[4] - m[1] * m[3]) * id,
        ]))
    }
}

/// Solve the 8×8 system `a·x = b` by Gaussian elimination with partial
/// pivoting; deterministic, or `None` when the matrix is singular.
fn gaussian_solve(mut a: [[f64; 8]; 8], mut b: [f64; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for (offset, r) in a[(col + 1)..].iter().enumerate() {
            let v = r[col].abs();
            if v > best {
                best = v;
                pivot = col + 1 + offset;
            }
        }
        if !best.is_finite() || best < PIVOT_EPSILON {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let prow = a[col];
        let p = prow[col];
        for row in (col + 1)..8 {
            let factor = a[row][col] / p;
            if factor == 0.0 {
                continue;
            }
            for k in col..8 {
                a[row][k] -= factor * prow[k];
            }
            b[row] -= factor * b[col];
        }
    }
    let mut x = [0.0f64; 8];
    for i in (0..8).rev() {
        let row = a[i];
        let mut s = b[i];
        for k in (i + 1)..8 {
            s -= row[k] * x[k];
        }
        x[i] = s / row[i];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::Homography;

    #[test]
    fn homography_sends_each_corner_and_inverts() {
        let src = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let dst = [(2.0, 1.0), (14.0, 3.0), (12.0, 15.0), (1.0, 9.0)];
        let h = Homography::from_quads(src, dst).unwrap();
        let inv = h.inverse().unwrap();
        for (s, d) in src.iter().zip(dst.iter()) {
            let (x, y) = h.apply(s.0, s.1).unwrap();
            assert!((x - d.0).abs() < 1e-9 && (y - d.1).abs() < 1e-9);
            let (u, v) = inv.apply(d.0, d.1).unwrap();
            assert!((u - s.0).abs() < 1e-9 && (v - s.1).abs() < 1e-9);
        }
    }

    #[test]
    fn collinear_corners_have_no_homography() {
        let src = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let line = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        assert!(Homography::from_quads(src, line).is_none());
        assert!(Homography::from_quads(src, [(f64::NAN, 0.0); 4]).is_none());
    }
}
