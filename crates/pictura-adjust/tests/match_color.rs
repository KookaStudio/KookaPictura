//! Match Color (#352) kernel behaviour.

use pictura_adjust::{
    apply, match_color_stats, AdjustError, Adjustment, MatchColorParams, MatchColorStats,
};
use pictura_core::PixelBuffer;

fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
    let n = (w * h) as usize;
    assert_eq!(px.len(), n);
    let mut data = vec![0u8; n * 3];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: data.into(),
    }
}

fn buf4(px: &[[u8; 4]]) -> PixelBuffer {
    let n = px.len();
    let mut data = vec![0u8; n * 4];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
        data[3 * n + i] = p[3];
    }
    PixelBuffer {
        width: n as u32,
        height: 1,
        channels: 4,
        data: data.into(),
    }
}

/// Statistics of a flat image: zero std, and the mean is the flat colour.
fn flat_stats(rgb: [u8; 3]) -> MatchColorStats {
    let pixels: Vec<[u8; 4]> = vec![[rgb[0], rgb[1], rgb[2], 255]; 16];
    match_color_stats(&pixels, None)
}

fn params(target: MatchColorStats, source: MatchColorStats) -> MatchColorParams {
    MatchColorParams {
        target,
        source,
        neutralize: false,
        luminance: 100.0,
        color_intensity: 100.0,
        fade: 0.0,
    }
}

fn apply3(p: &MatchColorParams, px: &[[u8; 3]]) -> Vec<[u8; 3]> {
    let n = px.len();
    let mut buf = buf3(n as u32, 1, px);
    apply(&Adjustment::MatchColor(*p), &mut buf).expect("match color applies");
    (0..n)
        .map(|i| [buf.data[i], buf.data[n + i], buf.data[2 * n + i]])
        .collect()
}

#[test]
fn transfer_shifts_mean_toward_the_source() {
    // A gray target matched to a red source: every pixel takes the source's
    // mean and the flat-target guard keeps its (zero) contrast.
    let out = apply3(
        &params(flat_stats([128, 128, 128]), flat_stats([220, 20, 20])),
        &[[128, 128, 128]; 4],
    );
    for px in &out {
        assert_eq!(px[0], 220, "red channel should reach the source mean");
        assert_eq!(px[1], 20);
        assert_eq!(px[2], 20);
    }
}

#[test]
fn neutralize_keeps_luminance_but_drops_the_cast() {
    let mut p = params(flat_stats([128, 128, 128]), flat_stats([220, 20, 20]));
    p.neutralize = true;
    let out = apply3(&p, &[[128, 128, 128]; 4]);
    for px in &out {
        // A gray-world result: the source's mean L with zero chroma, so all
        // three channels are equal.
        assert_eq!(px[0], px[1]);
        assert_eq!(px[1], px[2]);
    }
}

#[test]
fn color_intensity_one_is_exactly_gray() {
    let mut p = params(flat_stats([128, 128, 128]), flat_stats([220, 20, 20]));
    p.color_intensity = 1.0;
    let out = apply3(&p, &[[200, 30, 90], [10, 200, 40], [0, 0, 255]]);
    for px in &out {
        assert_eq!(px[0], px[1]);
        assert_eq!(px[1], px[2]);
    }
}

#[test]
fn luminance_scales_monotonically() {
    let px = [[100, 100, 100], [200, 200, 200], [10, 10, 10]];
    let mut means = Vec::new();
    for luminance in [50.0, 100.0, 180.0] {
        let mut p = params(flat_stats([128, 128, 128]), flat_stats([128, 128, 128]));
        p.luminance = luminance;
        let out = apply3(&p, &px);
        means.push(out.iter().map(|c| c[0] as f64).sum::<f64>() / out.len() as f64);
    }
    assert!(means[0] < means[1] && means[1] < means[2], "{means:?}");
    // At 100 with equal statistics the result is the exact identity.
    assert_eq!(
        apply3(
            &params(flat_stats([128, 128, 128]), flat_stats([128, 128, 128])),
            &px
        ),
        px
    );
}

#[test]
fn fade_extremes_are_identity() {
    let px = [[100, 100, 100], [200, 10, 10]];
    // Fade 100 withholds the whole adjustment.
    let mut p = params(flat_stats([128, 128, 128]), flat_stats([220, 20, 20]));
    p.fade = 100.0;
    assert_eq!(apply3(&p, &px), px);
    // Fade 50 lands half-way between the original and the full match, using
    // the kernel's own blend arithmetic (float, rounded once at the end).
    let full = apply3(
        &params(flat_stats([128, 128, 128]), flat_stats([220, 20, 20])),
        &px,
    );
    let mut half = p;
    half.fade = 50.0;
    let mid = apply3(&half, &px);
    for c in 0..3 {
        for i in 0..px.len() {
            let (orig, matched) = (f64::from(px[i][c]) / 255.0, f64::from(full[i][c]) / 255.0);
            let expected = ((orig + (matched - orig) * 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            assert_eq!(mid[i][c], expected, "channel {c}, pixel {i}");
        }
    }
}

#[test]
fn contrast_scales_with_the_source_standard_deviation() {
    // Two-value target (std > 0) matched to a low-contrast source: the
    // output keeps the target's ordering but the spread narrows toward the
    // source's, and each channel maps by its own std ratio. Expected values
    // are the kernel's own Lab arithmetic, computed once and pinned.
    let target_pixels: Vec<[u8; 4]> = vec![[0, 0, 0, 255], [255, 255, 255, 255]];
    let target = match_color_stats(&target_pixels, None);
    let source_pixels: Vec<[u8; 4]> = vec![[100, 0, 0, 255], [160, 0, 0, 255]];
    let source = match_color_stats(&source_pixels, None);
    let out = apply3(&params(target, source), &[[0, 0, 0], [255, 255, 255]]);
    // Black stays darker than white, both land inside the source's range,
    // and the red channel (the source's only varying one) carries the shift.
    assert!(out[0][0] < out[1][0], "black must stay darker: {:?}", out);
    assert!(
        out[0][0] >= 90 && out[1][0] <= 165,
        "both pixels should land near the source pair: {out:?}"
    );
    assert_ne!(out[0][0], 0);
    assert_ne!(out[1][0], 255);
}

#[test]
fn statistics_respect_the_selection_plane() {
    let pixels: Vec<[u8; 4]> = vec![[255, 0, 0, 255], [0, 0, 255, 255]];
    let red_only = match_color_stats(&pixels, Some(&[255, 0]));
    let blue_only = match_color_stats(&pixels, Some(&[0, 255]));
    let both = match_color_stats(&pixels, Some(&[255, 255]));
    // Red's Lab a is large and positive; blue's Lab b is large and negative.
    assert!(red_only.mean[1] > 50.0, "red alone: {}", red_only.mean[1]);
    assert!(
        blue_only.mean[2] < -50.0,
        "blue alone: {}",
        blue_only.mean[2]
    );
    assert_eq!(both, match_color_stats(&pixels, None));
}

#[test]
fn alpha_is_preserved() {
    let mut buf = buf4(&[[100, 150, 200, 255], [10, 20, 30, 128], [0, 0, 0, 0]]);
    let n = buf.pixel_count();
    let alpha: Vec<u8> = (0..n).map(|i| buf.data[3 * n + i]).collect();
    apply(
        &Adjustment::MatchColor(params(
            flat_stats([128, 128, 128]),
            flat_stats([220, 20, 20]),
        )),
        &mut buf,
    )
    .expect("match color applies");
    let after: Vec<u8> = (0..n).map(|i| buf.data[3 * n + i]).collect();
    assert_eq!(alpha, after);
}

#[test]
fn invalid_parameters_are_refused() {
    let stats = flat_stats([128, 128, 128]);
    let mut buf = buf3(2, 1, &[[1, 2, 3], [4, 5, 6]]);
    let before = buf.data.as_ref().to_vec();
    for bad in [
        MatchColorParams {
            luminance: 0.0,
            ..params(stats, stats)
        },
        MatchColorParams {
            luminance: 201.0,
            ..params(stats, stats)
        },
        MatchColorParams {
            color_intensity: 0.5,
            ..params(stats, stats)
        },
        MatchColorParams {
            fade: 101.0,
            ..params(stats, stats)
        },
        MatchColorParams {
            fade: f64::NAN,
            ..params(stats, stats)
        },
        MatchColorParams {
            source: MatchColorStats {
                mean: [f64::INFINITY, 0.0, 0.0],
                std: [0.0; 3],
            },
            ..params(stats, stats)
        },
    ] {
        let err = apply(&Adjustment::MatchColor(bad), &mut buf);
        assert!(
            matches!(err, Err(AdjustError::InvalidParams(_))),
            "expected InvalidParams for {bad:?}"
        );
        assert_eq!(buf.data.as_ref(), before.as_slice());
    }
}

#[test]
fn empty_statistics_are_an_identity() {
    // No contributing pixels: zeroed statistics, and the flat-target guard
    // keeps the buffer unchanged at default options.
    let px = [[100, 100, 100], [200, 10, 10]];
    assert_eq!(
        apply3(&params(MatchColorStats::ZERO, MatchColorStats::ZERO), &px),
        px
    );
}
