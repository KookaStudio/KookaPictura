//! Resample kernels for the layer transform family: the CS6 interpolation
//! choice and the pixel-centre samplers the transform ops drive.
//!
//! The cubic methods are separable Mitchell–Netravali convolutions with
//! inferred parameters (Adobe's exact kernels are closed): Bicubic =
//! Catmull–Rom (B = 0, C = 1/2, interpolating), Bicubic Smoother = cubic
//! B-spline (B = 1, C = 0), Bicubic Sharper = B = 0, C = 3/4.

use pictura_core::Sample;

/// A CS6 resample method: the Free Transform options bar's Interpolation menu
/// (also `Image > Image Size`'s method set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    Nearest,
    #[default]
    Bilinear,
    Bicubic,
    BicubicSmoother,
    BicubicSharper,
    BicubicAutomatic,
}

impl Interpolation {
    /// The stable id the bridge and options bar use.
    pub fn id(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::Bilinear => "bilinear",
            Self::Bicubic => "bicubic",
            Self::BicubicSmoother => "bicubic-smoother",
            Self::BicubicSharper => "bicubic-sharper",
            Self::BicubicAutomatic => "bicubic-automatic",
        }
    }

    /// Parse a stable id; `None` for an unknown string.
    pub fn from_id(id: &str) -> Option<Self> {
        [
            Self::Nearest,
            Self::Bilinear,
            Self::Bicubic,
            Self::BicubicSmoother,
            Self::BicubicSharper,
            Self::BicubicAutomatic,
        ]
        .into_iter()
        .find(|method| method.id() == id)
    }

    /// Resolve `BicubicAutomatic` against the source and destination pixel
    /// counts: shrinking picks Sharper, growing Smoother, equal Bicubic. Every
    /// other method is returned unchanged.
    pub fn resolve(self, src_pixels: u64, dest_pixels: u64) -> Self {
        match self {
            Self::BicubicAutomatic if dest_pixels < src_pixels => Self::BicubicSharper,
            Self::BicubicAutomatic if dest_pixels > src_pixels => Self::BicubicSmoother,
            Self::BicubicAutomatic => Self::Bicubic,
            other => other,
        }
    }
}

/// Bilinear sample of a plane at source-local `(lx, ly)` in the pixel-centre
/// convention: taps are edge-clamped. The caller has already checked
/// `0 <= lx < w` and `0 <= ly < h`.
pub(super) fn bilinear<T: Sample>(src: &[T], w: usize, h: usize, lx: f64, ly: f64) -> T {
    let u = lx - 0.5;
    let v = ly - 0.5;
    let x0f = u.floor();
    let y0f = v.floor();
    let fx = u - x0f;
    let fy = v - y0f;
    let max_x = (w - 1) as f64;
    let max_y = (h - 1) as f64;
    let x0 = x0f.clamp(0.0, max_x) as usize;
    let y0 = y0f.clamp(0.0, max_y) as usize;
    let x1 = (x0f + 1.0).clamp(0.0, max_x) as usize;
    let y1 = (y0f + 1.0).clamp(0.0, max_y) as usize;
    let p00 = src[y0 * w + x0].to_unit();
    let p10 = src[y0 * w + x1].to_unit();
    let p01 = src[y1 * w + x0].to_unit();
    let p11 = src[y1 * w + x1].to_unit();
    let top = p00 + (p10 - p00) * fx;
    let bot = p01 + (p11 - p01) * fx;
    T::from_unit(top + (bot - top) * fy)
}

/// Nearest-neighbour sample: replicate the source pixel nearest the point.
fn nearest<T: Sample>(src: &[T], w: usize, h: usize, lx: f64, ly: f64) -> T {
    let x = (lx - 0.5).round().clamp(0.0, (w - 1) as f64) as usize;
    let y = (ly - 0.5).round().clamp(0.0, (h - 1) as f64) as usize;
    src[y * w + x]
}

/// The Mitchell–Netravali cubic convolution kernel at distance `x`.
fn cubic_weight(x: f64, b: f64, c: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x.powi(3)
            + (-18.0 + 12.0 * b + 6.0 * c) * x.powi(2)
            + (6.0 - 2.0 * b))
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x.powi(3)
            + (6.0 * b + 30.0 * c) * x.powi(2)
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

/// Catmull–Rom (the Bicubic method).
const CATMULL_ROM: (f64, f64) = (0.0, 0.5);
/// Cubic B-spline (the Bicubic Smoother method).
const B_SPLINE: (f64, f64) = (1.0, 0.0);
/// The sharper overshoot (the Bicubic Sharper method).
const SHARPER: (f64, f64) = (0.0, 0.75);

/// Separable 4×4 cubic convolution with the given `(B, C)` parameters; taps
/// are edge-clamped.
fn bicubic<T: Sample>(src: &[T], w: usize, h: usize, lx: f64, ly: f64, bc: (f64, f64)) -> T {
    let (b, c) = bc;
    let u = lx - 0.5;
    let v = ly - 0.5;
    let x0 = u.floor();
    let y0 = v.floor();
    let tx = u - x0;
    let ty = v - y0;
    let wx = [
        cubic_weight(1.0 + tx, b, c),
        cubic_weight(tx, b, c),
        cubic_weight(1.0 - tx, b, c),
        cubic_weight(2.0 - tx, b, c),
    ];
    let wy = [
        cubic_weight(1.0 + ty, b, c),
        cubic_weight(ty, b, c),
        cubic_weight(1.0 - ty, b, c),
        cubic_weight(2.0 - ty, b, c),
    ];
    let max_x = (w - 1) as f64;
    let max_y = (h - 1) as f64;
    let mut sum = 0.0;
    for (j, wj) in wy.iter().enumerate() {
        let yy = (y0 + j as f64 - 1.0).clamp(0.0, max_y) as usize;
        let mut row = 0.0;
        for (i, wi) in wx.iter().enumerate() {
            let xx = (x0 + i as f64 - 1.0).clamp(0.0, max_x) as usize;
            row += wi * src[yy * w + xx].to_unit();
        }
        sum += wj * row;
    }
    T::from_unit(sum)
}

/// Sample a plane at source-local `(lx, ly)` with `interpolation`, in the
/// pixel-centre convention. `BicubicAutomatic` must be resolved by the caller
/// ([`Interpolation::resolve`]); it falls back to bilinear here.
pub(super) fn sample<T: Sample>(
    src: &[T],
    w: usize,
    h: usize,
    lx: f64,
    ly: f64,
    interpolation: Interpolation,
) -> T {
    match interpolation {
        Interpolation::Nearest => nearest(src, w, h, lx, ly),
        Interpolation::Bicubic => bicubic(src, w, h, lx, ly, CATMULL_ROM),
        Interpolation::BicubicSmoother => bicubic(src, w, h, lx, ly, B_SPLINE),
        Interpolation::BicubicSharper => bicubic(src, w, h, lx, ly, SHARPER),
        Interpolation::Bilinear | Interpolation::BicubicAutomatic => bilinear(src, w, h, lx, ly),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Vec<u8> {
        let mut src = vec![0u8; 4 * 4];
        for y in 0..4 {
            for x in 0..4 {
                src[y * 4 + x] = if (x + y) % 2 == 0 { 0 } else { 255 };
            }
        }
        src
    }

    #[test]
    fn ids_round_trip() {
        for method in [
            Interpolation::Nearest,
            Interpolation::Bilinear,
            Interpolation::Bicubic,
            Interpolation::BicubicSmoother,
            Interpolation::BicubicSharper,
            Interpolation::BicubicAutomatic,
        ] {
            assert_eq!(Interpolation::from_id(method.id()), Some(method));
        }
        assert_eq!(Interpolation::from_id("lanczos"), None);
    }

    #[test]
    fn automatic_resolves_by_area() {
        let auto = Interpolation::BicubicAutomatic;
        assert_eq!(auto.resolve(100, 400), Interpolation::BicubicSmoother);
        assert_eq!(auto.resolve(400, 100), Interpolation::BicubicSharper);
        assert_eq!(auto.resolve(100, 100), Interpolation::Bicubic);
        assert_eq!(
            Interpolation::Nearest.resolve(400, 100),
            Interpolation::Nearest
        );
    }

    #[test]
    fn nearest_replicates_a_checkerboard() {
        let src = checker();
        for oy in 0..8 {
            for ox in 0..8 {
                let v = sample(
                    &src,
                    4,
                    4,
                    ox as f64 * 0.5 + 0.25,
                    oy as f64 * 0.5 + 0.25,
                    Interpolation::Nearest,
                );
                assert!(v == 0 || v == 255, "no intermediate value: {v}");
            }
        }
    }

    #[test]
    fn integer_samples_are_exact() {
        let src: Vec<u8> = (0..16).map(|i| (i * 13) as u8).collect();
        for interpolation in [
            Interpolation::Nearest,
            Interpolation::Bilinear,
            Interpolation::Bicubic,
        ] {
            for y in 0..4 {
                for x in 0..4 {
                    let v = sample(&src, 4, 4, x as f64 + 0.5, y as f64 + 0.5, interpolation);
                    assert_eq!(v, src[y * 4 + x], "{interpolation:?} at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn bicubic_differs_from_bilinear_on_a_ramp() {
        let src: Vec<u8> = (0..16).map(|i| (i * 16) as u8).collect();
        let bilinear = sample(&src, 4, 4, 1.25, 1.0, Interpolation::Bilinear);
        let bicubic = sample(&src, 4, 4, 1.25, 1.0, Interpolation::Bicubic);
        assert_ne!(bilinear, bicubic);
    }

    #[test]
    fn smoother_and_sharper_are_distinct() {
        let src: Vec<u8> = (0..16).map(|i| if i % 4 == 0 { 0 } else { 255 }).collect();
        let smoother = sample(&src, 4, 4, 1.5, 1.0, Interpolation::BicubicSmoother);
        let sharper = sample(&src, 4, 4, 1.5, 1.0, Interpolation::BicubicSharper);
        assert!(smoother < sharper, "smoother {smoother} sharper {sharper}");
    }

    #[test]
    fn typed_planes_sample_in_the_unit_domain() {
        let src: Vec<u16> = (0..16).map(|i| (i * 4096) as u16).collect();
        assert_eq!(sample(&src, 4, 4, 1.5, 1.5, Interpolation::Nearest), src[5]);
        let mid = sample(&src, 4, 4, 1.5, 1.5, Interpolation::Bilinear);
        assert_eq!(mid, src[5]);
    }
}
