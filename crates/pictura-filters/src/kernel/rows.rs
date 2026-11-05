// SIMD spans pixels only; each call adds the next tap in the original order.
macro_rules! weighted_row {
    ($name:ident, $portable:ident, $avx2:ident, $acc:ty, $sample:ty) => {
        #[inline]
        pub(super) fn $name(acc: &mut [$acc], src: &[$sample], weight: $acc) {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            if std::is_x86_feature_detected!("avx2") {
                // The runtime check covers the only extra precondition of this function.
                unsafe { $avx2(acc, src, weight) };
                return;
            }
            $portable(acc, src, weight);
        }

        fn $portable(acc: &mut [$acc], src: &[$sample], weight: $acc) {
            for (a, &value) in acc.iter_mut().zip(src) {
                *a += weight * <$acc>::from(value);
            }
        }

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[target_feature(enable = "avx2")]
        unsafe fn $avx2(acc: &mut [$acc], src: &[$sample], weight: $acc) {
            for (a, &value) in acc.iter_mut().zip(src) {
                *a += weight * <$acc>::from(value);
            }
        }
    };
}

weighted_row!(add_f64, add_f64_portable, add_f64_avx2, f64, f64);
weighted_row!(add_f32, add_f32_portable, add_f32_avx2, f32, f32);
weighted_row!(
    add_f32_to_f64,
    add_f32_to_f64_portable,
    add_f32_to_f64_avx2,
    f64,
    f32
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_rows_match_scalar() {
        for len in [0, 1, 3, 4, 7, 8, 9, 64, 97, 1024] {
            let src: Vec<f32> = (0..len)
                .map(|i| ((i * 7919) % 4096) as f32 / 1024.0 - 2.0)
                .collect();
            let wide: Vec<f64> = src.iter().map(|&v| f64::from(v)).collect();
            let mut narrow = vec![0.0f32; len];
            let mut double = vec![0.0f64; len];
            let mut mixed = vec![0.0f64; len];
            let mut scalar_narrow = narrow.clone();
            let mut scalar_double = double.clone();
            let mut portable_narrow = narrow.clone();
            let mut portable_double = double.clone();
            let mut portable_mixed = mixed.clone();
            for weight in [0.13, -0.0, 0.79, -0.34] {
                add_f32(&mut narrow, &src, weight as f32);
                add_f64(&mut double, &wide, weight);
                add_f32_to_f64(&mut mixed, &src, weight);
                add_f32_portable(&mut portable_narrow, &src, weight as f32);
                add_f64_portable(&mut portable_double, &wide, weight);
                add_f32_to_f64_portable(&mut portable_mixed, &src, weight);
                for i in 0..len {
                    scalar_narrow[i] += weight as f32 * src[i];
                    scalar_double[i] += weight * wide[i];
                    assert_eq!(narrow[i].to_bits(), scalar_narrow[i].to_bits());
                    assert_eq!(double[i].to_bits(), scalar_double[i].to_bits());
                    assert_eq!(mixed[i].to_bits(), scalar_double[i].to_bits());
                    assert_eq!(portable_narrow[i].to_bits(), scalar_narrow[i].to_bits());
                    assert_eq!(portable_double[i].to_bits(), scalar_double[i].to_bits());
                    assert_eq!(portable_mixed[i].to_bits(), scalar_double[i].to_bits());
                }
            }
        }
    }
}
