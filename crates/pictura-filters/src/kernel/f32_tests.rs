use super::*;

// The former bevel convolution, retaining its radius input and intermediate cast.
fn former_blur(src: &[f32], w: usize, h: usize, radius: f64) -> Vec<f32> {
    let kernel = gaussian_kernel(sigma_from_radius(radius));
    let r = (kernel.len() / 2) as isize;
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0f32; src.len()];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0f64;
                for (i, k) in kernel.iter().enumerate() {
                    let o = i as isize - r;
                    let (sx, sy) = if horizontal {
                        ((x as isize + o).clamp(0, w as isize - 1) as usize, y)
                    } else {
                        (x, (y as isize + o).clamp(0, h as isize - 1) as usize)
                    };
                    acc += k * src[sy * w + sx] as f64;
                }
                out[y * w + x] = acc as f32;
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

fn compare_fields(shapes: &[(usize, usize)]) {
    for workers in [1, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap();
        for &(w, h) in shapes {
            let varied: Vec<f32> = (0..w * h)
                .map(|i| (((i * 7919) ^ (i >> 3)) % 4096) as f32 / 1024.0 - 2.0)
                .collect();
            for src in [varied, vec![0.37; w * h], vec![-0.0; w * h]] {
                for radius in [0.0, 0.25, 1.5, 5.0, 250.0 / 3.0, 250.0] {
                    let expected = former_blur(&src, w, h, radius);
                    let actual =
                        pool.install(|| gaussian_blur_f32(&src, w, h, sigma_from_radius(radius)));
                    assert_eq!(actual.len(), expected.len());
                    for (i, (&a, &b)) in actual.iter().zip(&expected).enumerate() {
                        assert_eq!(
                            a.to_bits(),
                            b.to_bits(),
                            "{w}x{h} radius={radius} workers={workers} sample={i}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn gaussian_blur_f32_matches_serial() {
    compare_fields(&[(37, 23), (97, 61), (3, 41), (64, 2)]);
}

#[test]
fn gaussian_blur_f32_handles_degenerate_fields() {
    compare_fields(&[(0, 0), (0, 7), (7, 0), (1, 1), (1, 17), (17, 1)]);
}
