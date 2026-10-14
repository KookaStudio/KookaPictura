//! Gaussian blur of an 8-bit coverage plane for `Selection::feather`.

/// Sigma at or above which the direct convolution is replaced by the
/// variance-matched box stack.
const BOX_STACK_SIGMA: f64 = 2.0;

/// Gaussian blur of an 8-bit plane with clamp-to-edge sampling.
///
/// The direct convolution is exact and cheap for a narrow kernel; a wide one
/// (feather radius >= 4 px) would dominate a large document's commit time, so
/// it switches to [`box_stack`].
pub(crate) fn blur(src: &[u8], w: usize, h: usize, sigma: f64) -> Vec<u8> {
    if sigma < BOX_STACK_SIGMA {
        gaussian_blur(src, w, h, sigma)
    } else {
        box_stack(src, w, h, sigma)
    }
}

/// Direct separable truncated Gaussian, `O(radius)` per pixel.
fn gaussian_blur(src: &[u8], w: usize, h: usize, sigma: f64) -> Vec<u8> {
    let radius = (sigma * 3.0).ceil().max(1.0) as isize;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0f64;
    for i in -radius..=radius {
        let v = (-(i * i) as f64 / (2.0 * sigma * sigma)).exp();
        kernel.push(v);
        sum += v;
    }
    for k in &mut kernel {
        *k /= sum;
    }
    let mut tmp = vec![0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f64;
            for (ki, i) in (-radius..=radius).enumerate() {
                let xx = (x as isize + i).clamp(0, w as isize - 1) as usize;
                acc += src[y * w + xx] as f64 * kernel[ki];
            }
            tmp[y * w + x] = acc;
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f64;
            for (ki, i) in (-radius..=radius).enumerate() {
                let yy = (y as isize + i).clamp(0, h as isize - 1) as usize;
                acc += tmp[yy * w + x] * kernel[ki];
            }
            out[y * w + x] = acc.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Variance-matched three-box approximation of the Gaussian.
///
/// Each axis runs three moving averages whose widths `w_i` satisfy
/// `sum (w_i^2 - 1) / 12 = sigma^2`, so the ramp keeps the Gaussian's width
/// and 50 % crossing while a pixel costs `O(1)` instead of `O(sigma)`. The
/// stacked boxes stay within a few coverage levels of the truncated Gaussian
/// on a hard edge (`box_stack_tracks_the_gaussian_on_a_hard_edge`); the
/// deviation can grow for high-frequency masks. Inferred approximation.
// ponytail: three boxes; a recursive Gaussian would track the tails closer.
fn box_stack(src: &[u8], w: usize, h: usize, sigma: f64) -> Vec<u8> {
    let widths = box_widths(sigma);
    let mut a = src.to_vec();
    let mut b = vec![0u8; a.len()];
    for &width in &widths {
        box_horizontal(&a, &mut b, w, h, width);
        box_vertical(&b, &mut a, w, h, width);
    }
    a
}

/// Three odd box widths -- the first `m` narrow, the rest two wider -- whose
/// combined variance is closest to `sigma^2`.
fn box_widths(sigma: f64) -> [usize; 3] {
    const N: f64 = 3.0;
    let ideal = (12.0 * sigma * sigma / N + 1.0).sqrt();
    let mut narrow = ideal.floor() as i64;
    if narrow % 2 == 0 {
        narrow -= 1;
    }
    let narrow = narrow.max(1);
    let wide = narrow + 2;
    let m =
        ((12.0 * sigma * sigma - N * (narrow * narrow) as f64 - 4.0 * N * narrow as f64 - 3.0 * N)
            / (-4.0 * narrow as f64 - 4.0))
            .round()
            .clamp(0.0, N) as usize;
    [0, 1, 2].map(|i| {
        if i < m {
            narrow as usize
        } else {
            wide as usize
        }
    })
}

/// One horizontal moving average of `width` taps, clamp-to-edge.
fn box_horizontal(src: &[u8], dst: &mut [u8], w: usize, h: usize, width: usize) {
    let r = (width / 2) as isize;
    let inv = 1.0 / width as f32;
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut dst[y * w..(y + 1) * w];
        let mut sum = 0.0f32;
        for i in -r..=r {
            sum += f32::from(row[i.clamp(0, w as isize - 1) as usize]);
        }
        out[0] = (sum * inv).round() as u8;
        for (x, o) in out.iter_mut().enumerate().skip(1) {
            let add = (x as isize + r).min(w as isize - 1) as usize;
            let sub = (x as isize - r - 1).max(0) as usize;
            sum += f32::from(row[add]) - f32::from(row[sub]);
            *o = (sum * inv).round() as u8;
        }
    }
}

/// One vertical moving average of `width` taps, clamp-to-edge. The running sum
/// is kept per column so rows are read sequentially.
fn box_vertical(src: &[u8], dst: &mut [u8], w: usize, h: usize, width: usize) {
    let r = (width / 2) as isize;
    let inv = 1.0 / width as f32;
    let mut sum = vec![0f32; w];
    for i in -r..=r {
        let y = i.clamp(0, h as isize - 1) as usize;
        for (s, &v) in sum.iter_mut().zip(&src[y * w..y * w + w]) {
            *s += f32::from(v);
        }
    }
    for (o, &s) in dst[..w].iter_mut().zip(&sum) {
        *o = (s * inv).round() as u8;
    }
    for y in 1..h {
        let add = (y as isize + r).min(h as isize - 1) as usize;
        let sub = (y as isize - r - 1).max(0) as usize;
        let arow = &src[add * w..add * w + w];
        let srow = &src[sub * w..sub * w + w];
        let out = &mut dst[y * w..(y + 1) * w];
        for (s, (&a, &b)) in sum.iter_mut().zip(arow.iter().zip(srow)) {
            *s += f32::from(a) - f32::from(b);
        }
        for (o, &s) in out.iter_mut().zip(&sum) {
            *o = (s * inv).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selection;

    #[test]
    fn box_stack_tracks_the_gaussian_on_a_hard_edge() {
        let (w, h) = (1024u32, 8u32);
        let mut step = Selection::none(w, h);
        for y in 0..h {
            for x in 0..w / 2 {
                step.data[(y * w + x) as usize] = 255;
            }
        }
        let (dw, dh) = (256u32, 256u32);
        let (cx, cy, dr) = (128.0f64, 128.0, 60.0);
        let mut disk = Selection::none(dw, dh);
        for y in 0..dh {
            for x in 0..dw {
                let (dx, dy) = (f64::from(x) - cx, f64::from(y) - cy);
                if dx * dx + dy * dy <= dr * dr {
                    disk.data[(y * dw + x) as usize] = 255;
                }
            }
        }
        let exact_of = |shape: &Selection, radius: f64| {
            gaussian_blur(
                &shape.data,
                shape.width as usize,
                shape.height as usize,
                radius / 2.0,
            )
        };
        for radius in [4.0, 8.0, 20.0, 100.0, 250.0] {
            let exact = exact_of(&step, radius);
            let fast = step.feather(radius);
            let max = max_delta(&exact, &fast.data);
            assert!(max <= 4, "step radius {radius}: max delta {max}");
            let moved = (crossing(&exact, w as usize) - crossing(&fast.data, w as usize)).abs();
            assert!(moved < 1.0, "step radius {radius}: crossing moved {moved}");
        }
        for radius in [4.0, 8.0, 20.0] {
            let exact = exact_of(&disk, radius);
            let fast = disk.feather(radius);
            let max = max_delta(&exact, &fast.data);
            assert!(max <= 6, "disk radius {radius}: max delta {max}");
        }
    }

    fn max_delta(exact: &[u8], fast: &[u8]) -> u8 {
        exact
            .iter()
            .zip(fast)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    }

    /// The x where the first row's coverage crosses 50 %, the line the marching
    /// ants trace.
    fn crossing(data: &[u8], w: usize) -> f64 {
        let row = &data[..w];
        (0..w - 1)
            .find_map(|x| {
                let (a, b) = (f64::from(row[x]), f64::from(row[x + 1]));
                ((a >= 127.5) != (b >= 127.5)).then(|| x as f64 + (127.5 - a) / (b - a))
            })
            .expect("step has a 50 % crossing")
    }

    #[test]
    fn box_stack_keeps_solid_and_empty_planes() {
        for radius in [4.0, 50.0, 250.0] {
            assert_eq!(
                Selection::all(37, 21).feather(radius),
                Selection::all(37, 21)
            );
            assert_eq!(
                Selection::none(37, 21).feather(radius),
                Selection::none(37, 21)
            );
        }
    }
}
