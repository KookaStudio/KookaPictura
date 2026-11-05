use super::*;
use pictura_core::{LayerMask, VectorMask, VectorSubpath};

#[test]
fn prepared_effect_mattes_match_scalar() {
    let (w, h) = (37usize, 29usize);
    let src: Vec<_> = (0..w * h).map(|i| (i % 23) as f32 / 22.0).collect();
    for radius in [0.0, 0.4, 3.0, 17.5] {
        let pixels: Vec<u8> = src
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let mut scalar = PixelBuffer::new(w as u32, h as u32, 1);
        scalar.data = pixels.into();
        if radius > 0.0 {
            pictura_filters::kernel::gaussian_blur_planes(
                &mut scalar,
                pictura_filters::kernel::sigma_from_radius(radius),
            );
        }
        let expected: Vec<_> = if radius > 0.0 {
            scalar.data.iter().map(|&v| v as f32 / 255.0).collect()
        } else {
            src.clone()
        };
        assert_eq!(blur_matte(&src, w, h, radius), expected);
    }
    let clip = PsdRect {
        top: 0,
        left: 0,
        bottom: 29,
        right: 37,
    };
    for feather in [0.0, 1.5] {
        for density in [0, 123, 255] {
            for disabled in [false, true] {
                let mut layer = Layer {
                    mask: Some(LayerMask {
                        rect: PsdRect {
                            top: 3,
                            left: 4,
                            bottom: 23,
                            right: 29,
                        },
                        density,
                        disabled,
                        default_color: 73,
                        data: Some((0..500).map(|i| (i * 13) as u8).collect::<Vec<_>>().into()),
                        ..Default::default()
                    }),
                    vector_mask: Some(VectorMask {
                        subpaths: vec![VectorSubpath {
                            closed: true,
                            operation: 1,
                            points: vec![[256, 256], [30 * 256, 4 * 256], [8 * 256, 27 * 256]],
                            ..Default::default()
                        }],
                        density,
                        disabled,
                        invert: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                layer.mask.as_mut().unwrap().set_feather(feather);
                layer.vector_mask.as_mut().unwrap().set_feather(feather);
                let expected: Vec<_> = src
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        v * (crate::composite::mask_alpha(
                            &layer,
                            clip,
                            (i % w) as i32,
                            (i / w) as i32,
                        ) as f32
                            / 255.0)
                    })
                    .collect();
                let mut got = src.clone();
                apply_mask(&mut got, w, 0, 0, &layer, clip);
                assert_eq!(
                    got.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
            }
        }
    }
}
