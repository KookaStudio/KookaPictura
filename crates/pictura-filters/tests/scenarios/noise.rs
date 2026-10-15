//! Spec scenarios from the replaced `noise.rs` unit tests, run through
//! `apply` against the ported photorust engine.

use super::shims::*;
use pictura_core::PixelBuffer;
use pictura_filters::*;

fn plane(w: u32, h: u32, channels: u8, fill: [u8; 3]) -> PixelBuffer {
    let n = w as usize * h as usize;
    let mut data = vec![0u8; n * channels as usize];
    for p in 0..n {
        for (c, &v) in fill.iter().enumerate() {
            data[c * n + p] = v;
        }
        if channels == 4 {
            data[3 * n + p] = 200;
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels,
        data: data.into(),
    }
}

fn stats(buf: &PixelBuffer, plane: usize) -> (f64, f64) {
    let n = buf.pixel_count();
    let vals = &buf.data[plane * n..plane * n + n];
    let mean = vals.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let var = vals.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    (mean, var.sqrt())
}

#[test]
fn add_noise_keeps_mean_and_grows_std() {
    for distribution in [NoiseDistribution::Uniform, NoiseDistribution::Gaussian] {
        let mut low = plane(64, 64, 3, [128, 128, 128]);
        add(&mut low, 2.0, distribution, false, 1).unwrap();
        let (mean_low, std_low) = stats(&low, 0);
        assert!((mean_low - 128.0).abs() < 2.0, "mean drifted: {mean_low}");
        assert!(std_low > 1.0, "std too small: {std_low}");

        let mut high = plane(64, 64, 3, [128, 128, 128]);
        add(&mut high, 20.0, distribution, false, 1).unwrap();
        let (mean_high, std_high) = stats(&high, 0);
        assert!((mean_high - 128.0).abs() < 4.0, "mean drifted: {mean_high}");
        assert!(
            std_high > std_low * 3.0,
            "{distribution:?}: std should grow with amount ({std_low} -> {std_high})"
        );
    }
}

#[test]
fn add_noise_monochromatic_preserves_hue() {
    let mut mono = plane(32, 32, 3, [100, 150, 200]);
    add(&mut mono, 5.0, NoiseDistribution::Uniform, true, 7).unwrap();
    let n = mono.pixel_count();
    for p in 0..n {
        let r = mono.data[p] as i32;
        let g = mono.data[n + p] as i32;
        let b = mono.data[2 * n + p] as i32;
        assert_eq!(r - g, -50);
        assert_eq!(g - b, -50);
    }

    let mut color = plane(32, 32, 3, [100, 150, 200]);
    add(&mut color, 5.0, NoiseDistribution::Uniform, false, 7).unwrap();
    let varied = (0..n).any(|p| {
        let r = color.data[p] as i32;
        let g = color.data[n + p] as i32;
        let b = color.data[2 * n + p] as i32;
        r - g != -50 || g - b != -50
    });
    assert!(varied, "per-channel noise should shift channel differences");
}

#[test]
fn add_noise_same_seed_is_identical_and_different_seed_differs() {
    let base = plane(16, 16, 4, [128, 128, 128]);
    let mut a = base.clone();
    let mut b = base.clone();
    let mut c = base.clone();
    add(&mut a, 10.0, NoiseDistribution::Gaussian, false, 42).unwrap();
    add(&mut b, 10.0, NoiseDistribution::Gaussian, false, 42).unwrap();
    add(&mut c, 10.0, NoiseDistribution::Gaussian, false, 43).unwrap();
    assert_eq!(a.data, b.data);
    assert_ne!(a.data, c.data);
}

#[test]
fn zero_amount_and_zero_radius_are_noops_bad_amount_errors() {
    let base = plane(4, 4, 3, [128, 128, 128]);
    let mut buf = base.clone();
    add(&mut buf, 0.0, NoiseDistribution::Uniform, false, 1).unwrap();
    assert_eq!(buf.data, base.data);
    median(&mut buf, 0).unwrap();
    assert_eq!(buf.data, base.data);
    assert!(add(&mut buf, -1.0, NoiseDistribution::Uniform, false, 1).is_err());
}

#[test]
fn dust_and_scratches_smooths_speck_at_zero_threshold_and_keeps_it_high() {
    let mut smoothed = plane(8, 8, 3, [100, 100, 100]);
    smoothed.data[3 * 8 + 4] = 255;
    dust_and_scratches(&mut smoothed, 1, 0).unwrap();
    assert_eq!(
        smoothed.data[3 * 8 + 4],
        100,
        "threshold 0 must replace the speck with the median"
    );

    let mut kept = plane(8, 8, 3, [100, 100, 100]);
    kept.data[3 * 8 + 4] = 255;
    dust_and_scratches(&mut kept, 1, 255).unwrap();
    assert_eq!(
        kept.data[3 * 8 + 4],
        255,
        "threshold 255 must leave the speck alone"
    );
}

#[test]
fn dust_and_scratches_rejects_bad_parameters() {
    let base = plane(4, 4, 3, [128, 128, 128]);
    let mut out = base.clone();
    assert!(dust_and_scratches(&mut out, 0, 10).is_err());
    assert!(dust_and_scratches(&mut out, 17, 10).is_err());
    assert!(dust_and_scratches(&mut out, 2, 256).is_err());
    assert_eq!(out, base, "rejected parameters must not modify the buffer");
    assert!(dust_and_scratches(&mut out, 2, 255).is_ok());
}

#[test]
fn dust_and_scratches_is_deterministic_and_preserves_alpha() {
    let base = plane(9, 7, 4, [90, 140, 60]);
    let n = base.pixel_count();
    let alpha = base.data[3 * n..].to_vec();
    let mut a = base.clone();
    dust_and_scratches(&mut a, 2, 30).unwrap();
    let mut b = base.clone();
    dust_and_scratches(&mut b, 2, 30).unwrap();
    assert_eq!(a.data, b.data, "no seed, so re-apply is byte-identical");
    assert_eq!(&a.data[3 * n..], &alpha[..]);
}

#[test]
fn dust_and_scratches_uniform_is_bit_unchanged() {
    let base = plane(8, 8, 3, [120, 120, 120]);
    let mut out = base.clone();
    dust_and_scratches(&mut out, 2, 10).unwrap();
    assert_eq!(
        out.data, base.data,
        "a flat neighbourhood is never replaced"
    );
}

#[test]
fn reduce_noise_routes_through_apply_and_lowers_luminance_variance() {
    let mut buf = plane(32, 32, 3, [128, 128, 128]);
    let n = buf.pixel_count();
    let mut state = 12345u64;
    for i in 0..n {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let v = (128 + ((state >> 33) as i32 % 41) - 20).clamp(0, 255) as u8;
        buf.data[i] = v;
        buf.data[n + i] = v;
        buf.data[2 * n + i] = v;
    }
    let variance = |b: &PixelBuffer| {
        let n = b.pixel_count();
        let vals = &b.data[0..n];
        let mean = vals.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;
        vals.iter()
            .map(|&v| (f64::from(v) - mean).powi(2))
            .sum::<f64>()
            / n as f64
    };
    let before = variance(&buf);
    reduce_noise(
        &mut buf,
        &ReduceNoiseParams {
            channels: [ChannelDetail {
                strength: 8.0,
                preserve_details: 0.0,
            }; 3],
            ..ReduceNoiseParams::default()
        },
    )
    .unwrap();
    assert!(
        variance(&buf) < before,
        "Strength 8 must lower the variance"
    );
}

#[test]
fn alpha_is_untouched_and_tiny_images_do_not_panic() {
    for &(w, h) in &[(1u32, 1u32), (2, 2), (3, 3), (8, 8)] {
        let base = plane(w, h, 4, [100, 100, 100]);
        let n = base.pixel_count();
        let alpha = base.data[3 * n..].to_vec();

        let mut a = base.clone();
        add(&mut a, 10.0, NoiseDistribution::Gaussian, true, 3).unwrap();
        assert_eq!(&a.data[3 * n..], &alpha[..]);

        let mut m = base.clone();
        median(&mut m, 1).unwrap();
        assert_eq!(&m.data[3 * n..], &alpha[..]);

        let mut d = base.clone();
        despeckle(&mut d).unwrap();
        assert_eq!(&d.data[3 * n..], &alpha[..]);

        let mut s = base.clone();
        dust_and_scratches(&mut s, 1, 10).unwrap();
        assert_eq!(&s.data[3 * n..], &alpha[..]);
    }
}
