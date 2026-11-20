//! Chromaticity math shared by the raw colour stage (#398): 3×3 matrices, the
//! RGB-to-XYZ matrix of a set of primaries, Bradford adaptation, and colour
//! temperature with tint (`color/colour-temperature`).

/// A row-major 3×3 matrix.
pub type Mat3 = [[f64; 3]; 3];

pub const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The ICC D50 white as CIE xy, the one `Profile::pro_photo` uses.
pub const D50_XY: [f64; 2] = [0.34567, 0.35850];

/// ROMM (ProPhoto) red, green and blue primaries as CIE xy.
pub const PRO_PHOTO_PRIMARIES: [[f64; 2]; 3] =
    [[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]];

/// Lam's Bradford cone-response matrix.
const BRADFORD: Mat3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

/// The range Krystek's locus approximation is stated for.
pub const MIN_TEMPERATURE: f64 = 1000.0;
pub const MAX_TEMPERATURE: f64 = 15000.0;

/// Tint per unit of signed distance from the Planckian locus in CIE 1960 uv.
/// Negative so that below the locus (magenta) is positive. ponytail: chosen to
/// give Camera Raw's ±150 slider about the same reach; not measured.
const TINT_SCALE: f64 = -3000.0;

pub fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

pub fn mul_vec(a: &Mat3, v: [f64; 3]) -> [f64; 3] {
    a.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

pub fn diag(v: [f64; 3]) -> Mat3 {
    [[v[0], 0.0, 0.0], [0.0, v[1], 0.0], [0.0, 0.0, v[2]]]
}

pub fn scale(a: &Mat3, s: f64) -> Mat3 {
    a.map(|row| row.map(|v| v * s))
}

/// `g · a + (1 − g) · b`.
pub fn lerp(a: &Mat3, b: &Mat3, g: f64) -> Mat3 {
    std::array::from_fn(|r| std::array::from_fn(|c| g * a[r][c] + (1.0 - g) * b[r][c]))
}

/// The inverse, or `None` when the matrix is singular relative to its scale.
pub fn invert(m: &Mat3) -> Option<Mat3> {
    let cof =
        |r0: usize, r1: usize, c0: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    let adj = [
        [cof(1, 2, 1, 2), -cof(0, 2, 1, 2), cof(0, 1, 1, 2)],
        [-cof(1, 2, 0, 2), cof(0, 2, 0, 2), -cof(0, 1, 0, 2)],
        [cof(1, 2, 0, 1), -cof(0, 2, 0, 1), cof(0, 1, 0, 1)],
    ];
    let det = m[0][0] * adj[0][0] + m[0][1] * adj[1][0] + m[0][2] * adj[2][0];
    let norm = m.iter().flatten().fold(0.0f64, |a, v| a.max(v.abs()));
    if !det.is_finite() || norm == 0.0 || det.abs() <= 1e-12 * norm.powi(3) {
        return None;
    }
    Some(scale(&adj, 1.0 / det))
}

/// XYZ with `Y = 1` for a chromaticity.
pub fn xy_to_xyz(xy: [f64; 2]) -> [f64; 3] {
    let [x, y] = xy;
    [x / y, 1.0, (1.0 - x - y) / y]
}

pub fn xyz_to_xy(xyz: [f64; 3]) -> [f64; 2] {
    let sum = xyz[0] + xyz[1] + xyz[2];
    [xyz[0] / sum, xyz[1] / sum]
}

/// The linear RGB → XYZ matrix of `primaries` (red, green, blue xy) whose
/// (1, 1, 1) is `white` at `Y = 1`.
pub fn rgb_to_xyz(primaries: [[f64; 2]; 3], white: [f64; 2]) -> Mat3 {
    let p: Mat3 = std::array::from_fn(|r| std::array::from_fn(|c| xy_to_xyz(primaries[c])[r]));
    let s = mul_vec(
        &invert(&p).expect("primaries are independent"),
        xy_to_xyz(white),
    );
    mul(&p, &diag(s))
}

/// XYZ (D50) → linear ProPhoto.
pub fn xyz_d50_to_pro_photo() -> Mat3 {
    invert(&rgb_to_xyz(PRO_PHOTO_PRIMARIES, D50_XY)).expect("ProPhoto primaries are independent")
}

/// The Bradford adaptation taking XYZ under white `from` to XYZ under `to`.
pub fn bradford(from: [f64; 2], to: [f64; 2]) -> Mat3 {
    let inv = invert(&BRADFORD).expect("Bradford is invertible");
    let (src, dst) = (
        mul_vec(&BRADFORD, xy_to_xyz(from)),
        mul_vec(&BRADFORD, xy_to_xyz(to)),
    );
    let gain = diag(std::array::from_fn(|i| dst[i] / src[i]));
    mul(&inv, &mul(&gain, &BRADFORD))
}

fn xy_to_uv([x, y]: [f64; 2]) -> [f64; 2] {
    let d = -2.0 * x + 12.0 * y + 3.0;
    [4.0 * x / d, 6.0 * y / d]
}

fn uv_to_xy([u, v]: [f64; 2]) -> [f64; 2] {
    let d = 2.0 * u - 8.0 * v + 4.0;
    [3.0 * u / d, 2.0 * v / d]
}

/// The Planckian locus in CIE 1960 uv (Krystek, 1985).
fn locus(t: f64) -> [f64; 2] {
    let u = (0.860117757 + 1.54118254e-4 * t + 1.28641212e-7 * t * t)
        / (1.0 + 8.42420235e-4 * t + 7.08145163e-7 * t * t);
    let v = (0.317398726 + 4.22806245e-5 * t + 4.20481691e-8 * t * t)
        / (1.0 - 2.89741816e-5 * t + 1.61456053e-7 * t * t);
    [u, v]
}

/// The unit normal to the locus at `t`, pointing to the green side.
fn normal(t: f64) -> [f64; 2] {
    let h = (t * 1e-4).max(0.01);
    let (a, b) = (locus(t - h), locus(t + h));
    let (du, dv) = (b[0] - a[0], b[1] - a[1]);
    let len = du.hypot(dv);
    [dv / len, -du / len]
}

/// A chromaticity's correlated colour temperature in kelvin and its tint.
pub fn xy_to_temperature_tint(xy: [f64; 2]) -> (f64, f64) {
    let p = xy_to_uv(xy);
    let dist2 = |mired: f64| {
        let l = locus(1e6 / mired);
        (p[0] - l[0]).powi(2) + (p[1] - l[1]).powi(2)
    };
    // Golden-section search over reciprocal temperature.
    let ratio = (5f64.sqrt() - 1.0) / 2.0;
    let (mut lo, mut hi) = (1e6 / MAX_TEMPERATURE, 1e6 / MIN_TEMPERATURE);
    let mut a = hi - ratio * (hi - lo);
    let mut b = lo + ratio * (hi - lo);
    let (mut fa, mut fb) = (dist2(a), dist2(b));
    while hi - lo > 1e-7 {
        if fa < fb {
            hi = b;
            (b, fb) = (a, fa);
            a = hi - ratio * (hi - lo);
            fa = dist2(a);
        } else {
            lo = a;
            (a, fa) = (b, fb);
            b = lo + ratio * (hi - lo);
            fb = dist2(b);
        }
    }
    let t = (2e6 / (lo + hi)).clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    let (l, n) = (locus(t), normal(t));
    let duv = (p[0] - l[0]) * n[0] + (p[1] - l[1]) * n[1];
    (t, duv * TINT_SCALE)
}

/// The chromaticity of a colour temperature in kelvin (clamped to the locus
/// range) and a tint.
pub fn temperature_tint_to_xy(temperature: f64, tint: f64) -> [f64; 2] {
    let t = temperature.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE);
    let (l, n) = (locus(t), normal(t));
    let duv = tint / TINT_SCALE;
    uv_to_xy([l[0] + duv * n[0], l[1] + duv * n[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Mat3, b: &Mat3, tol: f64) -> bool {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(x, y)| (x - y).abs() <= tol)
    }

    #[test]
    fn standard_illuminants() {
        let (t, tint) = xy_to_temperature_tint([0.44757, 0.40745]);
        assert!((t - 2856.0).abs() <= 10.0, "A: {t}");
        assert!(tint.abs() <= 1.0, "A tint: {tint}");
        let (t, tint) = xy_to_temperature_tint([0.31271, 0.32902]);
        assert!((t - 6504.0).abs() <= 15.0, "D65: {t}");
        assert!((-11.0..=-8.0).contains(&tint), "D65 tint: {tint}");
    }

    #[test]
    fn temperature_tint_round_trip() {
        for t in (2000..=15000).step_by(500).map(f64::from) {
            for tint in [-50.0, 0.0, 50.0] {
                let (t2, tint2) = xy_to_temperature_tint(temperature_tint_to_xy(t, tint));
                assert!((t2 - t).abs() <= t * 1e-3, "{t} {tint}: {t2}");
                assert!((tint2 - tint).abs() <= 0.05, "{t} {tint}: {tint2}");
            }
        }
    }

    #[test]
    fn srgb_and_pro_photo_matrices() {
        // Lindbloom's published Bradford-adapted sRGB → XYZ (D50).
        let published = [
            [0.4360747, 0.3850649, 0.1430804],
            [0.2225045, 0.7168786, 0.0606169],
            [0.0139322, 0.0971045, 0.7141733],
        ];
        let d65 = [0.3127, 0.3290];
        let srgb = rgb_to_xyz([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]], d65);
        let adapted = mul(&bradford(d65, D50_XY), &srgb);
        assert!(close(&adapted, &published, 1e-4), "{adapted:?}");

        let white = mul_vec(&rgb_to_xyz(PRO_PHOTO_PRIMARIES, D50_XY), [1.0; 3]);
        let d50 = xy_to_xyz(D50_XY);
        assert!(white.iter().zip(d50).all(|(a, b)| (a - b).abs() < 1e-12));
        let round = mul(
            &xyz_d50_to_pro_photo(),
            &rgb_to_xyz(PRO_PHOTO_PRIMARIES, D50_XY),
        );
        assert!(close(&round, &IDENTITY, 1e-12));
    }

    #[test]
    fn singular_matrices_do_not_invert() {
        assert!(invert(&[[0.0; 3]; 3]).is_none());
        assert!(invert(&[[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 1.0, 1.0]]).is_none());
        let m = [[2.0, 0.5, 0.0], [0.1, 1.0, 0.3], [0.0, 0.2, 3.0]];
        assert!(close(&mul(&m, &invert(&m).unwrap()), &IDENTITY, 1e-12));
    }
}
