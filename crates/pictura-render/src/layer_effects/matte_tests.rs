mod matte_differential {
    use super::super::{dilate_matte, erode_matte};

    fn axis(
        src: &[f32],
        w: usize,
        h: usize,
        r: isize,
        horizontal: bool,
        want_max: bool,
    ) -> Vec<f32> {
        let mut out = vec![0.0f32; src.len()];
        if horizontal {
            for y in 0..h {
                for x in 0..w {
                    let lo = (x as isize - r).max(0) as usize;
                    let hi = (x as isize + r).min(w as isize - 1) as usize;
                    let mut v = if want_max {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    };
                    for k in lo..=hi {
                        v = if want_max {
                            v.max(src[y * w + k])
                        } else {
                            v.min(src[y * w + k])
                        };
                    }
                    out[y * w + x] = v;
                }
            }
        } else {
            for x in 0..w {
                for y in 0..h {
                    let lo = (y as isize - r).max(0) as usize;
                    let hi = (y as isize + r).min(h as isize - 1) as usize;
                    let mut v = if want_max {
                        f32::NEG_INFINITY
                    } else {
                        f32::INFINITY
                    };
                    for k in lo..=hi {
                        v = if want_max {
                            v.max(src[k * w + x])
                        } else {
                            v.min(src[k * w + x])
                        };
                    }
                    out[y * w + x] = v;
                }
            }
        }
        out
    }

    fn naive(src: &[f32], w: usize, h: usize, r: isize, want_max: bool) -> Vec<f32> {
        let horizontal = axis(src, w, h, r, true, want_max);
        axis(&horizontal, w, h, r, false, want_max)
    }

    #[test]
    fn spread_and_choke_mattes_match_the_pre_migration_scan() {
        let (w, h) = (37usize, 29usize);
        let mut state = 0x5eed_1234u64;
        let mut src = vec![0.0f32; w * h];
        for value in &mut src {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *value = (state % 1000) as f32 / 1000.0;
        }
        for r in [0isize, 1, 3, 10, 50] {
            assert_eq!(
                dilate_matte(&src, w, h, r as i64),
                naive(&src, w, h, r, true),
                "dilate r={r}"
            );
            assert_eq!(
                erode_matte(&src, w, h, r as i64),
                naive(&src, w, h, r, false),
                "erode r={r}"
            );
        }
    }
}

mod matte_parallel {
    use super::super::blur_matte;
    use pictura_core::PixelBuffer;

    /// The pre-change blur: the quantized matte in three identical planes.
    fn blur_three(src: &[f32], w: usize, h: usize, size: f64) -> Vec<f32> {
        let n = w * h;
        let mut buf = PixelBuffer::new(w as u32, h as u32, 3);
        for (i, &v) in src.iter().enumerate() {
            let q = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            buf.data[i] = q;
            buf.data[n + i] = q;
            buf.data[2 * n + i] = q;
        }
        pictura_filters::blur::gaussian(&mut buf, size).expect("valid radius");
        buf.data[..n].iter().map(|&b| b as f32 / 255.0).collect()
    }

    #[test]
    fn one_plane_matte_blur_matches_three_planes() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 40) as f32 / 16_777_216.0
        };
        for &(w, h) in &[(1usize, 1usize), (3, 5), (17, 9), (64, 33)] {
            let src: Vec<f32> = (0..w * h).map(|_| next()).collect();
            for &size in &[0.5f64, 2.0, 7.0] {
                assert_eq!(
                    blur_matte(&src, w, h, size),
                    blur_three(&src, w, h, size),
                    "{w}x{h} size {size}"
                );
            }
        }
    }

    #[test]
    #[ignore = "profiling: times the one-plane matte blur against the three-plane blur"]
    fn matte_blur_speedup_profile() {
        use std::time::Instant;

        let (w, h) = (1800usize, 1800usize);
        let src: Vec<f32> = (0..w * h)
            .map(|i| ((i as u64).wrapping_mul(2_654_435_761) % 1000) as f32 / 1000.0)
            .collect();

        let start = Instant::now();
        let three = blur_three(&src, w, h, 30.0);
        let three_time = start.elapsed();

        let start = Instant::now();
        let one = blur_matte(&src, w, h, 30.0);
        let one_time = start.elapsed();
        assert_eq!(one, three, "one plane must match the three-plane blur");

        eprintln!(
            "pictura perf: matte blur 1800x1800 three-plane={three_time:?} one-plane={one_time:?} speedup={:.2}x",
            three_time.as_secs_f64() / one_time.as_secs_f64()
        );
    }
}
