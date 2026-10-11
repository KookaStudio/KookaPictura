//! Sharpen family: Sharpen and Sharpen More as fixed-strength Unsharp Masks,
//! and thresholded blur-difference sharpening (Unsharp Mask). Sharpen Edges
//! runs on the photorust engine. Color planes only; alpha is untouched.

use pictura_core::PixelBuffer;

use crate::kernel::{gaussian_blur_planes, sigma_from_radius};
use crate::{validate, FilterError};
use rayon::prelude::*;

/// Sharpen's and Sharpen More's Unsharp Mask: 50 % and 100 % at σ 1 (radius 3
/// under the 3σ convention), photorust's model (#222). A 3×3 high-pass kernel
/// has so much gain at the finest detail that repeated passes destroy a
/// photograph; CS6's Sharpen can be applied again and again, each pass adding
/// a little.
pub const SHARPEN_AMOUNT: f64 = 50.0;
pub const SHARPEN_MORE_AMOUNT: f64 = 100.0;
pub const SHARPEN_RADIUS: f64 = 3.0;

/*** public filters ***/

pub fn sharpen(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    unsharp_mask(buf, SHARPEN_AMOUNT, SHARPEN_RADIUS, 0)
}

pub fn sharpen_more(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    unsharp_mask(buf, SHARPEN_MORE_AMOUNT, SHARPEN_RADIUS, 0)
}

pub fn unsharp_mask(
    buf: &mut PixelBuffer,
    amount: f64,
    radius: f64,
    threshold: u8,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !(1.0..=500.0).contains(&amount) {
        return Err(FilterError::InvalidParams(format!(
            "unsharp amount {amount} is outside 1..=500"
        )));
    }
    if radius <= 0.0 || !radius.is_finite() {
        return Err(FilterError::InvalidParams(format!(
            "unsharp radius {radius} must be finite and positive"
        )));
    }

    let n = buf.pixel_count();
    let planes = (buf.channels as usize).min(3);
    let orig = buf.data.clone();
    let mut blurred = buf.clone();
    gaussian_blur_planes(&mut blurred, sigma_from_radius(radius));

    let gain = amount / 100.0;
    let thr = threshold as f64;
    let w = (buf.width as usize).max(1);
    buf.data[..planes * n]
        .par_chunks_mut(w)
        .zip(orig[..planes * n].par_chunks(w))
        .zip(blurred.data[..planes * n].par_chunks(w))
        .for_each(|((out, orig), blurred)| {
            for ((out, &o), &b) in out.iter_mut().zip(orig).zip(blurred) {
                let o = o as f64;
                let diff = o - b as f64;
                if diff.abs() > thr {
                    *out = (o + diff * gain).round().clamp(0.0, 255.0) as u8;
                }
            }
        });
    Ok(())
}

/*** internals ***/

mod smart;
pub use smart::smart_sharpen;

#[cfg(test)]
mod tests {
    use super::*;

    /// Sharpen Edges runs on the photorust engine, so it is reached through `apply`.
    fn edges(buf: &mut PixelBuffer) -> Result<(), FilterError> {
        crate::apply(&crate::Filter::SharpenEdges, buf)
    }

    pub(super) fn planar(width: u32, height: u32, channels: u8, planes: &[Vec<u8>]) -> PixelBuffer {
        let mut data = Vec::new();
        for p in planes {
            data.extend_from_slice(p);
        }
        PixelBuffer {
            width,
            height,
            channels,
            data: data.into(),
        }
    }

    pub(super) fn gray_row(values: &[u8]) -> PixelBuffer {
        planar(
            values.len() as u32,
            1,
            3,
            &[values.to_vec(), values.to_vec(), values.to_vec()],
        )
    }

    pub(super) fn total_deviation(a: &PixelBuffer, b: &PixelBuffer) -> i64 {
        a.data
            .iter()
            .zip(&b.data)
            .map(|(x, y)| (*x as i64 - *y as i64).abs())
            .sum()
    }

    pub(super) fn plane_range(buf: &PixelBuffer) -> u8 {
        let plane = &buf.data[..buf.pixel_count()];
        *plane.iter().max().unwrap() - *plane.iter().min().unwrap()
    }

    #[test]
    fn sharpen_and_sharpen_more_are_fixed_unsharp_masks() {
        let base = gray_row(&[100, 100, 100, 100, 140, 140, 140, 140]);
        let mut got = base.clone();
        sharpen(&mut got).unwrap();
        let mut want = base.clone();
        unsharp_mask(&mut want, SHARPEN_AMOUNT, SHARPEN_RADIUS, 0).unwrap();
        assert_eq!(got, want);
        let mut got = base.clone();
        sharpen_more(&mut got).unwrap();
        let mut want = base.clone();
        unsharp_mask(&mut want, SHARPEN_MORE_AMOUNT, SHARPEN_RADIUS, 0).unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn repeated_sharpen_stays_bounded() {
        // A 3×3 high-pass multiplies fine detail several times over per pass;
        // the fixed Unsharp Mask adds a little each time.
        let base = gray_row(&[120, 130, 120, 130, 120, 130, 120, 130, 120, 130]);
        let mut out = base.clone();
        for _ in 0..3 {
            sharpen(&mut out).unwrap();
        }
        assert!(plane_range(&out) > plane_range(&base));
        assert!(plane_range(&out) < 80, "range {}", plane_range(&out));
    }

    #[test]
    fn sharpen_more_is_stronger_than_sharpen() {
        let base = gray_row(&[100, 100, 100, 140, 140, 140]);
        let mut sharp = base.clone();
        sharpen(&mut sharp).unwrap();
        let mut more = base.clone();
        sharpen_more(&mut more).unwrap();
        assert!(total_deviation(&base, &more) > total_deviation(&base, &sharp));
    }

    #[test]
    fn sharpen_increases_edge_contrast() {
        let base = gray_row(&[100, 100, 100, 100, 140, 140, 140, 140]);
        let mut out = base.clone();
        sharpen(&mut out).unwrap();
        assert!(plane_range(&out) > plane_range(&base));
    }

    #[test]
    fn sharpen_leaves_uniform_unchanged() {
        let base = gray_row(&[77, 77, 77, 77]);
        let mut a = base.clone();
        sharpen(&mut a).unwrap();
        assert_eq!(a, base);
        let mut b = base.clone();
        sharpen_more(&mut b).unwrap();
        assert_eq!(b, base);
    }

    #[test]
    fn edges_increases_edge_contrast() {
        let base = gray_row(&[50, 50, 50, 50, 100, 100, 100, 100]);
        let mut out = base.clone();
        edges(&mut out).unwrap();
        assert!(plane_range(&out) > plane_range(&base));
    }

    #[test]
    fn edges_leaves_flat_region_unchanged() {
        // The edge is found on a σ 1 blur, so it reaches four samples out;
        // the flat runs are checked beyond that.
        let values = [
            50u8, 50, 50, 50, 50, 50, 50, 50, 100, 100, 100, 100, 100, 100, 100, 100,
        ];
        let base = gray_row(&values);
        let mut out = base.clone();
        edges(&mut out).unwrap();
        let n = base.pixel_count();
        for (x, &v) in values.iter().enumerate().take(3) {
            assert_eq!(out.data[x], v, "left flat region at {x}");
            assert_eq!(out.data[n + x], v, "left flat region at {x}");
            assert_eq!(out.data[2 * n + x], v, "left flat region at {x}");
        }
        for (x, &v) in values.iter().enumerate().skip(13) {
            assert_eq!(out.data[x], v, "right flat region at {x}");
        }
    }

    #[test]
    fn usm_uniform_is_noop() {
        let base = gray_row(&[123; 16]);
        let mut out = base.clone();
        unsharp_mask(&mut out, 100.0, 3.0, 0).unwrap();
        for (a, b) in out.data.iter().zip(&base.data) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
    }

    #[test]
    fn usm_overshoot_grows_with_amount() {
        let base = gray_row(&[100, 100, 100, 100, 100, 160, 160, 160, 160, 160]);
        let mut low = base.clone();
        unsharp_mask(&mut low, 50.0, 3.0, 0).unwrap();
        let mut high = base.clone();
        unsharp_mask(&mut high, 300.0, 3.0, 0).unwrap();
        assert!(total_deviation(&base, &high) > total_deviation(&base, &low));
    }

    #[test]
    fn usm_threshold_gates_low_contrast() {
        let values = [100u8, 100, 100, 100, 100, 104, 104, 104, 104, 104];
        let base = gray_row(&values);
        let mut gated = base.clone();
        unsharp_mask(&mut gated, 200.0, 3.0, 4).unwrap();
        assert_eq!(gated, base, "differences below threshold are untouched");
        let mut open = base.clone();
        unsharp_mask(&mut open, 200.0, 3.0, 0).unwrap();
        assert_ne!(open, base, "threshold 0 sharpens the sub-threshold edge");
    }

    #[test]
    fn usm_rejects_invalid_params() {
        let base = gray_row(&[10, 20, 30, 40]);
        for (amount, radius) in [(0.0, 1.0), (600.0, 1.0), (100.0, 0.0)] {
            let mut out = base.clone();
            assert!(unsharp_mask(&mut out, amount, radius, 0).is_err());
            assert_eq!(out, base, "rejected parameters leave the buffer unchanged");
        }
    }

    #[test]
    fn sharpen_variants_preserve_alpha() {
        let r = vec![10, 60, 120, 200, 250, 30];
        let g = vec![20, 70, 130, 210, 240, 40];
        let b = vec![30, 80, 140, 220, 230, 50];
        let a = vec![5, 95, 128, 170, 200, 255];
        let base = planar(6, 1, 4, &[r, g, b, a.clone()]);
        let n = base.pixel_count();

        let mut s = base.clone();
        sharpen(&mut s).unwrap();
        assert_eq!(&s.data[3 * n..4 * n], &a[..]);

        let mut m = base.clone();
        sharpen_more(&mut m).unwrap();
        assert_eq!(&m.data[3 * n..4 * n], &a[..]);

        let mut e = base.clone();
        edges(&mut e).unwrap();
        assert_eq!(&e.data[3 * n..4 * n], &a[..]);

        let mut u = base.clone();
        unsharp_mask(&mut u, 150.0, 3.0, 0).unwrap();
        assert_eq!(&u.data[3 * n..4 * n], &a[..]);
    }

    #[test]
    fn tiny_images_do_not_panic() {
        for (w, h) in [(1u32, 1u32), (1, 5), (5, 1)] {
            let n = (w * h) as usize;
            let base = planar(
                w,
                h,
                4,
                &[vec![10; n], vec![20; n], vec![30; n], vec![40; n]],
            );
            let mut s = base.clone();
            assert!(sharpen(&mut s).is_ok());
            let mut m = base.clone();
            assert!(sharpen_more(&mut m).is_ok());
            let mut e = base.clone();
            assert!(edges(&mut e).is_ok());
            let mut u = base.clone();
            assert!(unsharp_mask(&mut u, 100.0, 3.0, 0).is_ok());
        }
    }
}
