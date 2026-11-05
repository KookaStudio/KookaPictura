use super::*;

#[test]
fn effect_row_replay_matches_scalar() {
    let rect = PsdRect {
        top: 2,
        left: 1,
        bottom: 90,
        right: 44,
    };
    for tracking in [false, true] {
        for scale in [0.0, 0.41, 1.0] {
            for mode in [
                BlendMode::Normal,
                BlendMode::Multiply,
                BlendMode::Screen,
                BlendMode::Dissolve,
                BlendMode::SoftLight,
                BlendMode::Hue,
                BlendMode::ColorBurn,
            ] {
                let mut canvas = Canvas::new_region(5, 9, 31, 57);
                canvas.effect_scale = scale;
                canvas.cover = tracking.then(|| vec![false; canvas.px.len()]);
                for (i, pixel) in canvas.px.iter_mut().enumerate() {
                    pixel.r = (i % 19) as f32 / 19.0;
                    pixel.g = (i % 23) as f32 / 23.0;
                    pixel.b = (i % 29) as f32 / 29.0;
                    pixel.a = (i % 7) as f32 / 7.0;
                }
                let mut rgb = PixelBuffer::new(43, 88, 3);
                for (i, value) in rgb.data.as_mut_slice().iter_mut().enumerate() {
                    *value = (i * 37) as u8;
                }
                let alpha: Vec<_> = (0..43 * 88).map(|i| (i % 11) as f32 / 10.0).collect();
                let passes = [
                    EffectPass {
                        rect,
                        mode,
                        color: EffectColor::Constant([0.13, 0.72, 0.41]),
                        alpha: alpha.clone(),
                    },
                    EffectPass {
                        rect,
                        mode,
                        color: EffectColor::PerPixel(rgb),
                        alpha,
                    },
                ];
                let mut expected = canvas.clone();
                apply_scalar(&mut expected, &passes);
                apply(&mut canvas, &passes);
                assert_eq!(canvas.cover, expected.cover);
                for (got, want) in canvas.px.iter().zip(&expected.px) {
                    assert_eq!(
                        [got.r, got.g, got.b, got.a].map(f32::to_bits),
                        [want.r, want.g, want.b, want.a].map(f32::to_bits),
                        "mode={mode:?} scale={scale} tracking={tracking}"
                    );
                }
            }
        }
    }
}
