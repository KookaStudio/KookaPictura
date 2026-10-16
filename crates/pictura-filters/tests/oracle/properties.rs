//! ImageMagick-independent property and known-value tests for the M7
//! no-equivalent rows and the blur / sharpen / noise families.

use pictura_core::PixelBuffer;
use pictura_filters::{apply, Filter, NoiseDistribution, Quality, RadialMethod};

use crate::common::{pixel_row, plane_range, test_image_buffer};

/// M7 no-equivalent rows: guard the contracts directly because no ImageMagick
/// operator is faithful. See the mapping table for the observed deltas.
#[test]
fn stylize_no_equivalent_filters_properties() {
    // A flat color field for the spatial filters.
    let mut flat = PixelBuffer::new(6, 5, 3);
    for v in flat.data.iter_mut() {
        *v = 90;
    }

    // High Pass: a flat field collapses to mid-gray (128).
    let mut hp = flat.clone();
    apply(&Filter::HighPass { radius: 2.0 }, &mut hp).expect("apply");
    assert!(
        hp.data.iter().all(|&v| (v as i32 - 128).abs() <= 1),
        "HighPass flat field must be mid-gray"
    );

    // Find Edges: a flat field is white (Sobel magnitude 0, inverted), and a
    // step edge is dark.
    let mut fe = flat.clone();
    apply(&Filter::FindEdges, &mut fe).expect("apply");
    assert!(
        fe.data.iter().all(|&v| v == 255),
        "FindEdges flat field must stay light"
    );
    let mut step = pixel_row(&[0, 0, 0, 0, 255, 255, 255, 255]);
    apply(&Filter::FindEdges, &mut step).expect("apply");
    assert!(
        step.data[3] < 128 && step.data[4] < 128,
        "FindEdges must darken a step edge"
    );

    // Emboss: a flat field is neutral gray. CS6 traces edges in the original
    // colour, so a gray picture stays gray while a coloured one keeps
    // coloured fringes along its edges.
    let mut emb = flat.clone();
    apply(
        &Filter::Emboss {
            angle: 135.0,
            height: 3.0,
            amount: 100.0,
        },
        &mut emb,
    )
    .expect("apply");
    assert!(
        emb.data.iter().all(|&v| (v as i32 - 128).abs() <= 1),
        "Emboss flat field must be neutral gray"
    );
    let colored = test_image_buffer();
    let n = colored.pixel_count();
    let mut gray = colored.clone();
    for i in 0..n {
        let v = gray.data[i];
        gray.data[n + i] = v;
        gray.data[2 * n + i] = v;
    }
    let emboss = |b: &PixelBuffer| {
        let mut out = b.clone();
        apply(
            &Filter::Emboss {
                angle: 135.0,
                height: 3.0,
                amount: 100.0,
            },
            &mut out,
        )
        .expect("apply");
        out
    };
    let gray_emb = emboss(&gray);
    for i in 0..n {
        assert_eq!(
            gray_emb.data[i],
            gray_emb.data[n + i],
            "gray Emboss R != G at {i}"
        );
        assert_eq!(
            gray_emb.data[i],
            gray_emb.data[2 * n + i],
            "gray Emboss R != B at {i}"
        );
    }
    let colored_emb = emboss(&colored);
    assert!(
        (0..n).any(|i| colored_emb.data[i] != colored_emb.data[2 * n + i]),
        "Emboss keeps coloured edges on a coloured picture"
    );

    // Offset wrap = false: the exposed area takes `background`, the shifted
    // area copies the source.
    let base = test_image_buffer();
    let mut shifted = base.clone();
    apply(
        &Filter::Offset {
            horizontal: 3,
            vertical: 2,
            wrap: false,
            background: [10, 20, 30],
            transparent: false,
        },
        &mut shifted,
    )
    .expect("apply");
    let at = |b: &PixelBuffer, x: usize, y: usize, c: usize| {
        b.data[c * b.pixel_count() + y * b.width as usize + x]
    };
    assert_eq!(
        [
            at(&shifted, 0, 0, 0),
            at(&shifted, 0, 0, 1),
            at(&shifted, 0, 0, 2)
        ],
        [10, 20, 30],
        "exposed pixel must take the background"
    );
    for c in 0..3 {
        assert_eq!(
            at(&base, 5, 5, c),
            at(&shifted, 8, 7, c),
            "shifted pixel must copy the source (channel {c})"
        );
    }
}

/// ImageMagick has no faithful MotionBlur equivalent: `-motion-blur` builds a
/// one-sided Gaussian line kernel while Pictura averages symmetric uniform taps
/// (observed max delta 86). Guard the tap geometry directly instead.
#[test]
fn motion_blur_known_values() {
    let mut b = PixelBuffer::new(9, 9, 3);
    b.data[4 * 9 + 4] = 255;
    apply(
        &Filter::MotionBlur {
            angle: 0.0,
            distance: 5,
        },
        &mut b,
    )
    .expect("apply");
    let px = |x: usize, y: usize| b.data[y * 9 + x];
    assert_eq!(px(4, 4), 51);
    assert_eq!(px(2, 4), 51);
    assert_eq!(px(6, 4), 51);
    assert_eq!(px(4, 3), 0);
    assert_eq!(px(4, 5), 0);

    // distance <= 1 is a no-op.
    let mut base = PixelBuffer::new(4, 4, 3);
    base.data[5] = 200;
    let mut same = base.clone();
    apply(
        &Filter::MotionBlur {
            angle: 30.0,
            distance: 1,
        },
        &mut same,
    )
    .expect("apply");
    assert_eq!(same.data, base.data);
}

/// Sharpen Edges has no faithful ImageMagick equivalent; Sharpen and Sharpen
/// More are diffed too. Guard the PS contract directly for all three.
#[test]
fn sharpen_family_known_values() {
    let flat = pixel_row(&[77, 77, 77, 77, 77, 77]);
    for filter in [Filter::Sharpen, Filter::SharpenMore, Filter::SharpenEdges] {
        let mut out = flat.clone();
        apply(&filter, &mut out).expect("apply");
        assert_eq!(
            out.data, flat.data,
            "{filter:?} must not change a flat field"
        );
    }

    let base = pixel_row(&[100, 100, 100, 100, 140, 140, 140, 140]);
    for filter in [Filter::Sharpen, Filter::SharpenMore, Filter::SharpenEdges] {
        let mut out = base.clone();
        apply(&filter, &mut out).expect("apply");
        assert!(
            plane_range(&out) > plane_range(&base),
            "{filter:?} must increase edge contrast"
        );
    }
}

/// ImageMagick has no faithful Average / Radial / Surface equivalent; Blur and
/// Blur More are diffed too.
/// Guard their contracts (identity at zero, or a trivial known value).
#[test]
fn no_equivalent_filters_properties() {
    let original = test_image_buffer();

    // Average is the exact global mean, replicated across every pixel.
    let mut averaged = original.clone();
    apply(&Filter::Average, &mut averaged).expect("apply");
    let n = original.pixel_count();
    for c in 0..3usize {
        let plane = &original.data[c * n..c * n + n];
        let mean = (plane.iter().map(|&v| v as u64).sum::<u64>() as f64 / n as f64).round() as u8;
        assert!(
            averaged.data[c * n..c * n + n].iter().all(|&v| v == mean),
            "Average plane {c} is not the constant region mean {mean}"
        );
    }

    // Radial spin/zoom are identity at zero amount (Surface rejects radius 0).
    for filter in [
        Filter::RadialBlur {
            method: RadialMethod::Spin,
            amount: 0.0,
            center: (0.5, 0.5),
            quality: Quality::Best,
        },
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 0.0,
            center: (0.5, 0.5),
            quality: Quality::Best,
        },
    ] {
        let mut out = original.clone();
        apply(&filter, &mut out).expect("apply");
        assert_eq!(out.data, original.data, "{filter:?} must be identity");
    }

    // Blur / Blur More must be near-identity on a solid field.
    for filter in [Filter::Blur, Filter::BlurMore] {
        let mut solid = PixelBuffer::new(6, 5, 3);
        for v in solid.data.iter_mut() {
            *v = 90;
        }
        let before = solid.clone();
        apply(&filter, &mut solid).expect("apply");
        for (a, b) in solid.data.iter().zip(before.data.iter()) {
            assert!(
                (*a as i32 - *b as i32).abs() <= 1,
                "{filter:?} drifted on solid"
            );
        }
    }
}

/// Add Noise is randomized, so it is not diffed against ImageMagick. The
/// contract is same-seed determinism (and that different seeds differ).
#[test]
fn add_noise_same_seed_is_deterministic() {
    for distribution in [NoiseDistribution::Uniform, NoiseDistribution::Gaussian] {
        for monochromatic in [false, true] {
            let base = test_image_buffer();
            let mut a = base.clone();
            let mut b = base.clone();
            let mut c = base.clone();
            let filter = Filter::AddNoise {
                amount: 10.0,
                distribution,
                monochromatic,
                seed: 42,
            };
            apply(&filter, &mut a).expect("apply");
            apply(&filter, &mut b).expect("apply");
            apply(
                &Filter::AddNoise {
                    amount: 10.0,
                    distribution,
                    monochromatic,
                    seed: 43,
                },
                &mut c,
            )
            .expect("apply");
            assert_eq!(a.data, b.data, "{filter:?} must be seed-deterministic");
            assert_ne!(a.data, c.data, "{filter:?} must vary with the seed");
        }
    }
}
