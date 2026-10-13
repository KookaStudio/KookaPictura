//! The Tone Curve tab: four parametric regions and point curves (RGB and
//! per-channel), baked to 256-entry lookup tables applied to each plane.

use pictura_core::ToneCurve;

use crate::common::{hermite_eval, monotone_tangents};

use super::Planes;

pub(crate) fn apply(p: &mut Planes, t: &ToneCurve) {
    let has_parametric = [
        t.parametric_shadows,
        t.parametric_darks,
        t.parametric_lights,
        t.parametric_highlights,
    ]
    .iter()
    .any(|v| v.is_some_and(|x| x != 0));
    let identity = |c: &[i32]| c.is_empty() || c == [0, 0, 255, 255];
    if !has_parametric
        && identity(&t.curve_rgb)
        && identity(&t.curve_red)
        && identity(&t.curve_green)
        && identity(&t.curve_blue)
    {
        return;
    }

    let base = base_lut(t, has_parametric);
    let red = channel_lut(&base, &t.curve_red);
    let green = channel_lut(&base, &t.curve_green);
    let blue = channel_lut(&base, &t.curve_blue);

    for (plane, lut) in [(&mut p.r, &red), (&mut p.g, &green), (&mut p.b, &blue)] {
        for x in plane.iter_mut() {
            let i = (*x * 255.0).round().clamp(0.0, 255.0) as usize;
            *x = lut[i];
        }
    }
}

/// Parametric regions composed with the RGB point curve.
fn base_lut(t: &ToneCurve, has_parametric: bool) -> [f32; 256] {
    let regions = if has_parametric {
        Some([
            (t.parametric_shadows, t.shadow_split),
            (t.parametric_darks, t.midtone_split),
            (t.parametric_lights, t.highlight_split),
            (t.parametric_highlights, None),
        ])
    } else {
        None
    };
    let splits = [t.shadow_split, t.midtone_split, t.highlight_split];
    let mut lut = [0f32; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let mut x = i as f64 / 255.0;
        if let Some(regions) = regions {
            x = parametric(x, &regions, &splits);
        }
        if !t.curve_rgb.is_empty() {
            x = eval_curve(&t.curve_rgb, x);
        }
        *slot = x.clamp(0.0, 1.0) as f32;
    }
    lut
}

/// A single channel's point curve composed on top of `base`.
fn channel_lut(base: &[f32; 256], curve: &[i32]) -> [f32; 256] {
    if curve.is_empty() {
        return *base;
    }
    let mut lut = [0f32; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = eval_curve(curve, base[i] as f64).clamp(0.0, 1.0) as f32;
    }
    lut
}

/// Region sliders add a raised-cosine bump anchored between their split points.
/// ponytail: the reference's region curve is closed; this is a monotone-in-the-
/// control approximation, not PV2012's target curve.
fn parametric(x: f64, regions: &[(Option<i32>, Option<i32>); 4], splits: &[Option<i32>; 3]) -> f64 {
    let s: [f64; 3] = [
        splits[0].unwrap_or(25) as f64 / 100.0,
        splits[1].unwrap_or(50) as f64 / 100.0,
        splits[2].unwrap_or(75) as f64 / 100.0,
    ];
    let centers = [
        s[0] / 2.0,
        (s[0] + s[1]) / 2.0,
        (s[1] + s[2]) / 2.0,
        (s[2] + 1.0) / 2.0,
    ];
    let mut y = x;
    for ((value, _), center) in regions.iter().zip(centers) {
        let v = value.unwrap_or(0) as f64 / 100.0;
        if v == 0.0 {
            continue;
        }
        let hw = 0.25;
        let d = (x - center).abs();
        if d < hw {
            let bump = 0.5 * (1.0 + (std::f64::consts::PI * d / hw).cos());
            y += v * 0.15 * bump;
        }
    }
    y
}

/// Evaluate a flattened `[in, out, ...]` point curve through a monotone spline;
/// missing or malformed input leaves `x` unchanged.
fn eval_curve(points: &[i32], x: f64) -> f64 {
    let mut pts: Vec<(f64, f64)> = points
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| (p[0] as f64 / 255.0, p[1] as f64 / 255.0))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
    if pts.len() < 2 {
        return x;
    }
    let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
    let ms = monotone_tangents(&xs, &ys);
    hermite_eval(&xs, &ys, &ms, x.clamp(0.0, 1.0))
}
