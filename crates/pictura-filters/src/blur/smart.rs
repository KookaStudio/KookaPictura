//! Smart Blur: a selective Gaussian (`FILT-002`). Each pixel averages the
//! neighbours within Radius whose colour lies within Threshold of its own, so
//! flat areas smooth while edges stay put. Adobe's kernel is closed; this is
//! the edge-detection + threshold-limited averaging family the spec names.
//!
//! - A neighbour counts when every colour channel is within Threshold levels
//!   of the centre (one joint test, so channels never fringe apart).
//! - Taps are Gaussian-weighted (σ = radius / 3, `FILT-010`) over the disc of
//!   Radius. Quality sets how densely the disc is sampled: Low, Medium and High
//!   step through it so a side carries at most 3, 6 or 12 taps each way of the
//!   centre; up to that radius every quality samples every pixel.
//! - An edge is a pixel with a 4-neighbour more than Threshold away. Edge Only
//!   paints edges white on black; Overlay Edge paints them white over the blur.
//!
//! ponytail: the sampling budget, the joint test and the edge rule are
//! inferred from CS6's output, not Adobe's numbers.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::kernel::sigma_from_radius;
use crate::{validate, FilterError, Quality};

/// Smart Blur's output (`Mode` in the dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartBlurMode {
    Normal,
    EdgeOnly,
    OverlayEdge,
}

/// One sample of the search disc: offset and Gaussian weight.
struct Tap {
    dx: isize,
    dy: isize,
    weight: f32,
}

/// `quality` reads Draft / Good / Best as CS6's Low / Medium / High.
pub fn smart(
    buf: &mut PixelBuffer,
    radius: f64,
    threshold: f64,
    quality: Quality,
    mode: SmartBlurMode,
) -> Result<(), FilterError> {
    validate(buf)?;
    for (name, value) in [("radius", radius), ("threshold", threshold)] {
        if !(0.1..=100.0).contains(&value) {
            return Err(FilterError::InvalidParams(format!(
                "smart blur {name} {value} is outside 0.1..=100"
            )));
        }
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    // Channel differences are whole levels, so `d <= threshold` is `d <= limit`.
    let limit = threshold.floor() as i32;
    let taps = taps(radius, quality);
    // Normal mode averages alpha over the taps the colour admits, so the
    // layer edge softens with it; the edge modes paint colour only.
    let planes = if buf.channels == 4 && mode == SmartBlurMode::Normal {
        4
    } else {
        3
    };
    let src = buf.data[..planes * n].to_vec();
    let (rgb, alpha) = buf.data[..planes * n].split_at_mut(3 * n);
    let (r, rest) = rgb.split_at_mut(n);
    let (g, b) = rest.split_at_mut(n);
    let alpha_rows: Vec<Option<&mut [u8]>> = if alpha.is_empty() {
        (0..h).map(|_| None).collect()
    } else {
        alpha.chunks_mut(w).map(Some).collect()
    };
    let at = |x: isize, y: isize| {
        let i = y.clamp(0, h as isize - 1) as usize * w + x.clamp(0, w as isize - 1) as usize;
        let a = if planes == 4 { src[3 * n + i] } else { 0 };
        [src[i], src[n + i], src[2 * n + i], a]
    };
    let far =
        |p: [u8; 4], q: [u8; 4]| (0..3).any(|c| (i32::from(p[c]) - i32::from(q[c])).abs() > limit);
    r.par_chunks_mut(w)
        .zip(g.par_chunks_mut(w))
        .zip(b.par_chunks_mut(w))
        .zip(alpha_rows)
        .enumerate()
        .for_each(|(y, (((ro, go), bo), mut ao))| {
            let y = y as isize;
            for x in 0..w {
                let xi = x as isize;
                let p = at(xi, y);
                let edge = mode != SmartBlurMode::Normal
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .any(|&(dx, dy)| far(p, at(xi + dx, y + dy)));
                let out = match mode {
                    SmartBlurMode::EdgeOnly => [if edge { 255 } else { 0 }; 4],
                    _ if edge => [255; 4],
                    _ => {
                        let mut acc = [0f32; 4];
                        let mut total = 0f32;
                        for tap in &taps {
                            let q = at(xi + tap.dx, y + tap.dy);
                            if !far(p, q) {
                                for c in 0..4 {
                                    acc[c] += tap.weight * f32::from(q[c]);
                                }
                                total += tap.weight;
                            }
                        }
                        // The centre tap always passes, so `total` is at least 1.
                        acc.map(|v| (v / total).round().clamp(0.0, 255.0) as u8)
                    }
                };
                ro[x] = out[0];
                go[x] = out[1];
                bo[x] = out[2];
                if let Some(ao) = ao.as_deref_mut() {
                    ao[x] = out[3];
                }
            }
        });
    Ok(())
}

/// The search disc's taps at `quality`'s density, centre first.
fn taps(radius: f64, quality: Quality) -> Vec<Tap> {
    let reach = radius.ceil() as isize;
    let per_side = match quality {
        Quality::Draft => 3,
        Quality::Good => 6,
        Quality::Best => 12,
    };
    let step = (reach + per_side - 1) / per_side;
    let sigma = sigma_from_radius(radius);
    let two_var = (2.0 * sigma * sigma) as f32;
    let span = reach / step;
    let mut taps = vec![Tap {
        dx: 0,
        dy: 0,
        weight: 1.0,
    }];
    for j in -span..=span {
        for i in -span..=span {
            let (dx, dy) = (i * step, j * step);
            let d2 = (dx * dx + dy * dy) as f32;
            if (dx, dy) != (0, 0) && f64::from(d2) <= radius * radius {
                taps.push(Tap {
                    dx,
                    dy,
                    weight: (-d2 / two_var).exp(),
                });
            }
        }
    }
    taps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> PixelBuffer {
        let n = (w * h) as usize;
        let mut b = PixelBuffer::new(w, h, 3);
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                for (c, v) in f(x, y).into_iter().enumerate() {
                    b.data[c * n + i] = v;
                }
            }
        }
        b
    }

    fn run(
        src: &PixelBuffer,
        radius: f64,
        threshold: f64,
        q: Quality,
        m: SmartBlurMode,
    ) -> Vec<u8> {
        let mut b = src.clone();
        smart(&mut b, radius, threshold, q, m).unwrap();
        b.data.to_vec()
    }

    /// A two-tone step at x = 10 with ±`amp` noise riding on each side.
    fn noisy_step(amp: u8) -> PixelBuffer {
        rgb(20, 12, |x, y| {
            let base = if x < 10 { 60 } else { 190 };
            let jitter = ((x * 7 + y * 13) % 5) as u8 * amp / 4;
            [base + jitter, base / 2 + jitter, 255 - base - jitter]
        })
    }

    fn spread(v: &[u8]) -> u8 {
        v.iter().max().unwrap() - v.iter().min().unwrap()
    }

    #[test]
    fn noise_below_threshold_smooths_and_the_step_survives() {
        let src = noisy_step(8);
        let out = run(&src, 4.0, 20.0, Quality::Best, SmartBlurMode::Normal);
        let left: Vec<u8> = (0..12)
            .flat_map(|y| (0..10).map(move |x| y * 20 + x))
            .map(|i| out[i])
            .collect();
        let right: Vec<u8> = (0..12)
            .flat_map(|y| (10..20).map(move |x| y * 20 + x))
            .map(|i| out[i])
            .collect();
        let src_left: Vec<u8> = (0..12)
            .flat_map(|y| (0..10).map(move |x| y * 20 + x))
            .map(|i| src.data[i])
            .collect();
        assert!(spread(&left) < spread(&src_left), "the jitter flattens");
        assert!(
            *left.iter().max().unwrap() <= 68 && *right.iter().min().unwrap() >= 190,
            "nothing crosses the step"
        );
    }

    #[test]
    fn a_step_within_the_threshold_blurs_across() {
        let step = rgb(20, 4, |x, _| if x < 10 { [100; 3] } else { [150; 3] });
        let kept = run(&step, 4.0, 49.9, Quality::Best, SmartBlurMode::Normal);
        assert_eq!((kept[9], kept[10]), (100, 150), "50 levels is past 49.9");
        let blurred = run(&step, 4.0, 50.0, Quality::Best, SmartBlurMode::Normal);
        assert!(
            blurred[9] > 100 && blurred[10] < 150,
            "50 levels is within 50"
        );
    }

    #[test]
    fn edge_only_marks_both_sides_of_the_step_and_nothing_else() {
        let src = noisy_step(8);
        let out = run(&src, 3.0, 20.0, Quality::Draft, SmartBlurMode::EdgeOnly);
        let n = 20 * 12;
        for c in 0..3 {
            for y in 0..12 {
                for x in 0..20 {
                    let want = if x == 9 || x == 10 { 255 } else { 0 };
                    assert_eq!(out[c * n + y * 20 + x], want, "plane {c} ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn overlay_edge_is_the_blur_with_white_edges() {
        let src = noisy_step(8);
        let normal = run(&src, 3.0, 20.0, Quality::Good, SmartBlurMode::Normal);
        let edges = run(&src, 3.0, 20.0, Quality::Good, SmartBlurMode::EdgeOnly);
        let overlay = run(&src, 3.0, 20.0, Quality::Good, SmartBlurMode::OverlayEdge);
        for i in 0..normal.len() {
            let want = if edges[i] == 255 { 255 } else { normal[i] };
            assert_eq!(overlay[i], want, "sample {i}");
        }
    }

    #[test]
    fn quality_only_thins_the_disc_past_its_tap_budget() {
        let src = rgb(40, 30, |x, y| [((x * 37 + y * 91) % 23 + 100) as u8; 3]);
        let low = run(&src, 3.0, 30.0, Quality::Draft, SmartBlurMode::Normal);
        let high = run(&src, 3.0, 30.0, Quality::Best, SmartBlurMode::Normal);
        assert_eq!(low, high, "radius 3 is inside every budget");
        assert!(
            taps(100.0, Quality::Best).len() <= 25 * 25,
            "a side holds 12 taps"
        );
        assert!(taps(40.0, Quality::Draft).len() < taps(40.0, Quality::Good).len());
        assert!(taps(40.0, Quality::Good).len() < taps(40.0, Quality::Best).len());
    }

    #[test]
    fn normal_mode_blurs_alpha_only_across_the_taps_the_colour_admits() {
        let (w, h) = (12usize, 12usize);
        let n = w * h;
        let mut rgba = PixelBuffer::new(w as u32, h as u32, 4);
        for i in 0..n {
            let left = i % w < 6;
            for c in 0..3 {
                rgba.data[c * n + i] = if left { 40 } else { 220 };
            }
            rgba.data[3 * n + i] = if left { 0 } else { 255 };
        }
        rgba.data[3 * n + 5 * w + 2] = 255;
        smart(&mut rgba, 4.0, 50.0, Quality::Best, SmartBlurMode::Normal).unwrap();
        let alpha = &rgba.data[3 * n..];
        assert!(alpha[5 * w + 2] < 255, "the speck spreads");
        assert!(alpha[5 * w + 3] > 0, "into its neighbours");
        for y in 0..h {
            assert_eq!(alpha[y * w + 6..(y + 1) * w], [255; 6], "row {y}");
        }
        assert_eq!(alpha[11 * w + 5], 0, "the colour edge holds the alpha edge");
    }

    #[test]
    fn flat_input_alpha_and_bad_params() {
        let flat = rgb(9, 7, |_, _| [40, 140, 220]);
        assert_eq!(
            run(&flat, 50.0, 0.1, Quality::Best, SmartBlurMode::Normal),
            flat.data.to_vec()
        );
        assert!(
            run(&flat, 50.0, 0.1, Quality::Best, SmartBlurMode::EdgeOnly)
                .iter()
                .all(|&v| v == 0)
        );

        let mut rgba = PixelBuffer::new(6, 6, 4);
        for (i, v) in rgba.data.as_mut_slice().iter_mut().enumerate() {
            *v = (i * 29) as u8;
        }
        let alpha = rgba.data[108..].to_vec();
        smart(
            &mut rgba,
            2.0,
            50.0,
            Quality::Good,
            SmartBlurMode::OverlayEdge,
        )
        .unwrap();
        assert_eq!(&rgba.data[108..], &alpha[..]);

        let mut b = flat.clone();
        for (r, t) in [
            (0.0, 10.0),
            (100.1, 10.0),
            (5.0, 0.0),
            (5.0, 100.5),
            (f64::NAN, 5.0),
        ] {
            assert!(
                smart(&mut b, r, t, Quality::Good, SmartBlurMode::Normal).is_err(),
                "{r} {t}"
            );
        }
    }
}
