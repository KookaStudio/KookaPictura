//! Shape Blur: a convolution whose kernel is a Custom Shape preset, `2r+1`
//! pixels across, weighted by the shape's antialiased coverage (`FILT-002`).
//! Adobe's kernel is closed; this is the family the spec names.
//!
//! Each kernel row breaks into runs of equal weight, and a run costs one
//! difference of per-row prefix sums, so a pixel costs the kernel's run count
//! (about its height times its edge crossings) rather than its area.

use pictura_core::shape::{custom_shape_kernel, CUSTOM_SHAPE_NAMES};
use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::{validate, FilterError};

/// The largest radius Shape Blur accepts.
pub const SHAPE_BLUR_MAX_RADIUS: u32 = 1000;

/// Custom shape `shape`'s kernel at `radius`: a `2r+1` square of coverage
/// weights (0–255), row-major, the shape at its designed proportions filling
/// it. Empty for an unknown shape.
pub fn shape_kernel(shape: usize, radius: u32) -> Vec<u8> {
    custom_shape_kernel(shape, 2 * radius + 1)
}

/// One run of equal kernel weight: source row `y + dy`, columns
/// `x + a ..= x + b`.
struct Run {
    dy: isize,
    a: isize,
    b: isize,
    weight: i64,
}

pub fn shape(buf: &mut PixelBuffer, shape: usize, radius: u32) -> Result<(), FilterError> {
    validate(buf)?;
    if shape >= CUSTOM_SHAPE_NAMES.len() {
        return Err(FilterError::InvalidParams(format!(
            "shape blur shape {shape} is not one of the {} custom shapes",
            CUSTOM_SHAPE_NAMES.len()
        )));
    }
    if radius > SHAPE_BLUR_MAX_RADIUS {
        return Err(FilterError::InvalidParams(format!(
            "shape blur radius {radius} is above {SHAPE_BLUR_MAX_RADIUS}"
        )));
    }
    if radius == 0 {
        return Ok(());
    }
    let runs = runs(&shape_kernel(shape, radius), radius as isize);
    let total: i64 = runs.iter().map(|r| r.weight * (r.b - r.a + 1) as i64).sum();
    if total == 0 {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    for c in 0..buf.channels as usize {
        let plane = &mut buf.data[c * n..(c + 1) * n];
        let pre = row_prefixes(plane, w);
        plane.par_chunks_mut(w).enumerate().for_each_init(
            || vec![0i64; w],
            |acc, (y, out)| {
                acc.fill(0);
                for run in &runs {
                    let sy = (y as isize + run.dy).clamp(0, h as isize - 1) as usize;
                    let row = &pre[sy * (w + 1)..(sy + 1) * (w + 1)];
                    // Where the run stays inside the row it is one plain
                    // difference; only the ends need the edge repeat.
                    let lo = (1 - run.a).clamp(0, w as isize) as usize;
                    let hi = (w as isize - run.b).clamp(lo as isize, w as isize) as usize;
                    let edge = |x: usize| {
                        let x = x as isize;
                        span_sum(row, x + run.b) - span_sum(row, x + run.a - 1)
                    };
                    for (x, acc) in acc.iter_mut().enumerate().take(lo) {
                        *acc += run.weight * edge(x);
                    }
                    if hi > lo {
                        let a = (lo as isize + run.a) as usize;
                        let b = (lo as isize + run.b + 1) as usize;
                        for ((acc, &end), &start) in acc[lo..hi]
                            .iter_mut()
                            .zip(&row[b..b + (hi - lo)])
                            .zip(&row[a..a + (hi - lo)])
                        {
                            *acc += run.weight * (i64::from(end) - i64::from(start));
                        }
                    }
                    for (x, acc) in acc.iter_mut().enumerate().skip(hi) {
                        *acc += run.weight * edge(x);
                    }
                }
                for (out, &acc) in out.iter_mut().zip(acc.iter()) {
                    *out = ((acc + total / 2) / total).clamp(0, 255) as u8;
                }
            },
        );
    }
    Ok(())
}

/// The kernel's weighted runs, flipped through its centre so the shape is
/// the image of a single bright pixel (a true convolution).
fn runs(kernel: &[u8], r: isize) -> Vec<Run> {
    let side = (2 * r + 1) as usize;
    let mut runs = Vec::new();
    for (j, row) in kernel.chunks(side).enumerate() {
        let mut i = 0;
        while i < side {
            let weight = row[i];
            let start = i;
            while i < side && row[i] == weight {
                i += 1;
            }
            if weight > 0 {
                runs.push(Run {
                    dy: r - j as isize,
                    a: r - (i - 1) as isize,
                    b: r - start as isize,
                    weight: i64::from(weight),
                });
            }
        }
    }
    runs
}

/// Each row's `w + 1` prefix sums, `pre[k] = Σ_{i<k} v[i]`.
fn row_prefixes(plane: &[u8], w: usize) -> Vec<u32> {
    let mut pre = vec![0u32; plane.len() / w * (w + 1)];
    pre.par_chunks_mut(w + 1)
        .zip(plane.par_chunks(w))
        .for_each(|(pre, line)| {
            for (k, &v) in line.iter().enumerate() {
                pre[k + 1] = pre[k] + u32::from(v);
            }
        });
    pre
}

/// `Σ_{i=0..=t} v[clamp(i)]` over one row's prefix sums: the edge sample
/// repeats past either end (`FILT-010`), and `t = -1` is 0.
fn span_sum(pre: &[u32], t: isize) -> i64 {
    let w = pre.len() - 1;
    if t < 0 {
        // The virtual columns before 0 repeat v[0]; below -1 they count negative.
        return (t as i64 + 1) * i64::from(pre[1]);
    }
    let t = t as usize;
    if t >= w {
        let last = i64::from(pre[w] - pre[w - 1]);
        return i64::from(pre[w]) + (t - w + 1) as i64 * last;
    }
    i64::from(pre[t + 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> PixelBuffer {
        let n = (w * h) as usize;
        let mut data = vec![0u8; n * 3];
        for c in 0..3 {
            for y in 0..h {
                for x in 0..w {
                    data[c * n + (y * w + x) as usize] = f(x, y);
                }
            }
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        }
    }

    /// The direct `Σ k·v / Σ k` over the flipped kernel, clamp-to-edge.
    fn reference(src: &PixelBuffer, shape: usize, radius: u32) -> Vec<u8> {
        let (w, h) = (src.width as isize, src.height as isize);
        let r = radius as isize;
        let side = 2 * r + 1;
        let k = shape_kernel(shape, radius);
        let total: u64 = k.iter().map(|&v| u64::from(v)).sum();
        let mut out = vec![0u8; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0u64;
                for j in 0..side {
                    for i in 0..side {
                        let sx = (x - (i - r)).clamp(0, w - 1);
                        let sy = (y - (j - r)).clamp(0, h - 1);
                        acc += u64::from(k[(j * side + i) as usize])
                            * u64::from(src.data[(sy * w + sx) as usize]);
                    }
                }
                out[(y * w + x) as usize] = ((acc + total / 2) / total) as u8;
            }
        }
        out
    }

    #[test]
    fn runs_match_the_direct_convolution_on_every_shape() {
        let src = buffer(23, 17, |x, y| ((x * 37 + y * 91) ^ (x * y)) as u8);
        for (shape_index, name) in CUSTOM_SHAPE_NAMES.iter().enumerate() {
            for radius in [1, 4, 9, 14] {
                let mut buf = src.clone();
                shape(&mut buf, shape_index, radius).unwrap();
                let n = 23 * 17;
                assert_eq!(
                    &buf.data[..n],
                    &reference(&src, shape_index, radius)[..],
                    "{name} radius {radius}"
                );
                assert_eq!(buf.data[..n], buf.data[n..2 * n], "planes blur alike");
            }
        }
    }

    #[test]
    fn a_bright_pixel_spreads_into_the_shape_unflipped() {
        // The arrow points right: its head sits right of the centre, so a lone
        // bright pixel lights more to its right than to its left.
        let mut buf = buffer(41, 41, |x, y| if (x, y) == (20, 20) { 255 } else { 0 });
        shape(&mut buf, 2, 12).unwrap();
        let above = |x: usize| -> u32 { (8..15).map(|y| u32::from(buf.data[y * 41 + x])).sum() };
        assert!(buf.data[20 * 41 + 30] > 0, "the shaft reaches the tip");
        assert!(above(23) > 0, "the head is wide right of the centre");
        assert_eq!(above(9), 0, "the tail is a thin shaft left of it");
    }

    #[test]
    fn flat_input_is_kept_alpha_blurs_like_colour_and_radius_zero_is_a_no_op() {
        let mut flat = buffer(12, 9, |_, _| 173);
        shape(&mut flat, 1, 30).unwrap();
        assert!(flat.data.iter().all(|&v| v == 173));

        let plane: Vec<u8> = (0..64).map(|i| (i * 7) as u8).collect();
        let mut rgba = PixelBuffer {
            width: 8,
            height: 8,
            channels: 4,
            data: plane.repeat(4).into(),
        };
        shape(&mut rgba, 0, 3).unwrap();
        assert_ne!(&rgba.data[192..], &plane[..], "alpha is blurred");
        assert_eq!(&rgba.data[192..], &rgba.data[..64], "as the colour is");

        let src = buffer(9, 9, |x, y| (x * 20 + y) as u8);
        let mut same = src.clone();
        shape(&mut same, 4, 0).unwrap();
        assert_eq!(same.data, src.data);
    }

    #[test]
    fn unknown_shapes_and_oversized_radii_are_refused() {
        let mut buf = buffer(4, 4, |_, _| 0);
        assert!(shape(&mut buf, CUSTOM_SHAPE_NAMES.len(), 5).is_err());
        assert!(shape(&mut buf, 0, SHAPE_BLUR_MAX_RADIUS + 1).is_err());
    }
}
