use pictura_core::PixelBuffer;

use crate::common::{
    hermite_eval, linear_to_srgb, luma, map_lut, map_lut_plane, map_rgb, monotone_tangents,
    planes_mut, srgb_to_linear,
};
use crate::types::{
    AdjustError, BrightnessContrastParams, CurvesParams, ExposureParams, GradientMapParams,
    GradientStop, LevelsChannel, LevelsParams, OpacityStop, ShadowsHighlightsParams,
};

// ---------------------------------------------------------------------------
// Tonal adjustments
// ---------------------------------------------------------------------------

pub(crate) fn levels(p: &LevelsParams, buf: &mut PixelBuffer, n: usize) -> Result<(), AdjustError> {
    let composite = p.composite().lut()?;
    let channel = |c: &Option<LevelsChannel>| c.as_ref().map(LevelsChannel::lut).transpose();
    let (red, green, blue) = (channel(&p.red)?, channel(&p.green)?, channel(&p.blue)?);
    {
        let (r, g, b) = planes_mut(buf, n);
        for (plane, lut) in [(r, red), (g, green), (b, blue)] {
            if let Some(lut) = lut {
                map_lut_plane(plane, &lut);
            }
        }
    }
    // ponytail: channel records before the composite mirrors Curves; the
    // reference order is unpublished (no CS6 baseline carries both).
    map_lut(buf, n, &composite);
    Ok(())
}

/// Build a 256-entry monotone-Hermite LUT for `points`, validating the shared
/// curves contract (2..=14 control points, strictly increasing inputs).
fn curve_lut(points: &[(u8, u8)]) -> Result<[u8; 256], AdjustError> {
    if points.len() < 2 {
        return Err(AdjustError::InvalidParams(
            "curves need at least 2 points".into(),
        ));
    }
    if points.len() > 14 {
        return Err(AdjustError::InvalidParams(
            "curves support at most 14 points".into(),
        ));
    }
    let xs: Vec<f64> = points.iter().map(|q| q.0 as f64).collect();
    let ys: Vec<f64> = points.iter().map(|q| q.1 as f64).collect();
    if xs.windows(2).any(|w| w[0] >= w[1]) {
        return Err(AdjustError::InvalidParams(
            "curve inputs must be strictly increasing".into(),
        ));
    }
    let ms = monotone_tangents(&xs, &ys);
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = hermite_eval(&xs, &ys, &ms, i as f64)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    Ok(lut)
}

pub(crate) fn curves(p: &CurvesParams, buf: &mut PixelBuffer, n: usize) -> Result<(), AdjustError> {
    let composite = curve_lut(&p.points)?;
    let red = p.red.as_deref().map(curve_lut).transpose()?;
    let green = p.green.as_deref().map(curve_lut).transpose()?;
    let blue = p.blue.as_deref().map(curve_lut).transpose()?;
    {
        let (r, g, b) = planes_mut(buf, n);
        for (plane, lut) in [(r, red), (g, green), (b, blue)] {
            if let Some(lut) = lut {
                map_lut_plane(plane, &lut);
            }
        }
    }
    // ponytail: per-channel curves then the composite curve is an assumption;
    // The reference's composition order is unpublished. Revisit with a CS6/CC
    // Curves baseline carrying both a composite and a per-channel curve.
    map_lut(buf, n, &composite);
    Ok(())
}

pub(crate) fn brightness_contrast(
    p: &BrightnessContrastParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    if !(-150..=150).contains(&p.brightness) {
        return Err(AdjustError::InvalidParams("brightness out of range".into()));
    }
    if !(-50..=100).contains(&p.contrast) {
        return Err(AdjustError::InvalidParams("contrast out of range".into()));
    }
    if p.brightness == 0 && p.contrast == 0 {
        return Ok(());
    }
    let c = p.contrast as f64 / 100.0;
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let out = if p.use_legacy {
            // Approximation: additive shift in levels, then linear contrast
            // about mid-grey (the reference's legacy normalization is closed).
            let v = i as f64 + p.brightness as f64;
            ((v - 127.5) * (1.0 + c) + 127.5) / 255.0
        } else {
            let b = p.brightness as f64 / 150.0;
            let gamma = 2f64.powf(b);
            let y = (i as f64 / 255.0).powf(1.0 / gamma);
            // Monotone S-curve through (0,0),(0.5,0.5),(1,1); central slope 1+c.
            let xs = [0.0, 0.5, 1.0];
            let ys = [0.0, 0.5, 1.0];
            let ms = [1.0, 1.0 + c, 1.0];
            hermite_eval(&xs, &ys, &ms, y)
        };
        *slot = (out.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    map_lut(buf, n, &lut);
    Ok(())
}

pub(crate) fn exposure(
    p: &ExposureParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    if !p.exposure.is_finite() || !p.offset.is_finite() {
        return Err(AdjustError::InvalidParams(
            "exposure/offset must be finite".into(),
        ));
    }
    if !p.gamma.is_finite() || p.gamma <= 0.0 {
        return Err(AdjustError::InvalidParams(
            "exposure gamma must be > 0".into(),
        ));
    }
    if p.exposure == 0.0 && p.offset == 0.0 && p.gamma == 1.0 {
        return Ok(());
    }
    let gain = 2f64.powf(p.exposure);
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        // 8-bit values are sRGB-encoded; decode to linear light, apply, re-encode.
        let lin = (srgb_to_linear(i as f64 / 255.0) * gain + p.offset).max(0.0);
        let lin = lin.powf(p.gamma);
        *slot = (linear_to_srgb(lin).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    map_lut(buf, n, &lut);
    Ok(())
}

/// Shadows/Highlights: a pointwise approximation of CS6's local operator.
///
/// ponytail: no surround blur, tonal width, or advanced options — each pixel is
/// lifted/darkened from its own luma. The 0.35/0.30 gains stand in for Adobe's
/// closed amount→gain mapping. Upgrade with a CS6 grey-ramp baseline.
pub(crate) fn shadows_highlights(
    p: &ShadowsHighlightsParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    let sa = p.shadows_amount;
    let ha = p.highlights_amount;
    if !sa.is_finite()
        || !ha.is_finite()
        || !(0.0..=100.0).contains(&sa)
        || !(0.0..=100.0).contains(&ha)
    {
        return Err(AdjustError::InvalidParams(
            "shadows/highlights amounts must be within 0..=100".into(),
        ));
    }
    if sa == 0.0 && ha == 0.0 {
        return Ok(());
    }
    let mut lut = [0.0f64; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let l = i as f64 / 255.0;
        let st = (1.0 - 2.0 * l).clamp(0.0, 1.0);
        let sw = st * st * (3.0 - 2.0 * st);
        let shadow_delta = sw * (sa / 100.0) * 0.35 * 255.0;
        let ht = ((l - 0.5) * 2.0).clamp(0.0, 1.0);
        let hw = ht * ht * (3.0 - 2.0 * ht);
        let highlight_delta = hw * (ha / 100.0) * 0.30 * 255.0;
        *slot = shadow_delta - highlight_delta;
    }
    map_rgb(buf, n, |rv, gv, bv| {
        let y = luma(rv as f64, gv as f64, bv as f64);
        let delta = lut[y.round().clamp(0.0, 255.0) as usize];
        [
            (rv as f64 + delta).round().clamp(0.0, 255.0) as u8,
            (gv as f64 + delta).round().clamp(0.0, 255.0) as u8,
            (bv as f64 + delta).round().clamp(0.0, 255.0) as u8,
        ]
    });
    Ok(())
}

pub(crate) fn desaturate(buf: &mut PixelBuffer, n: usize) {
    map_rgb(buf, n, |rv, gv, bv| {
        // (min + max) / 2 == HSL with S = 0, matching the CS6 spec.
        let v = (rv.min(gv).min(bv) as u16 + rv.max(gv).max(bv) as u16) as f64 / 2.0;
        let v = v.round() as u8;
        [v, v, v]
    });
}

/// One lookup from the cumulative histogram of every colour sample, applied to
/// each channel, so neutral greys stay neutral and the darkest and lightest
/// levels present map to 0 and 255. A flat image is left alone.
pub(crate) fn equalize(buf: &mut PixelBuffer, n: usize) {
    let mut histogram = [0u64; 256];
    for &v in &buf.data[..3 * n] {
        histogram[v as usize] += 1;
    }
    let total: u64 = histogram.iter().sum();
    let first = histogram.iter().copied().find(|&h| h > 0).unwrap_or(0);
    if total == 0 || first == total {
        return;
    }
    let mut lut = [0u8; 256];
    let mut running = 0u64;
    for (slot, &count) in lut.iter_mut().zip(&histogram) {
        running += count;
        let spread = running.saturating_sub(first) as f64 / (total - first) as f64;
        *slot = (spread * 255.0).round() as u8;
    }
    map_lut(buf, n, &lut);
}

pub(crate) fn posterize(levels: u8, buf: &mut PixelBuffer, n: usize) -> Result<(), AdjustError> {
    if levels < 2 {
        return Err(AdjustError::InvalidParams(
            "posterize levels must be 2..=255".into(),
        ));
    }
    if levels == 255 {
        // 255 target levels coincide with every 8-bit code (spec: "turns it off").
        return Ok(());
    }
    let denom = (levels - 1) as f64;
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let q = (i as f64 * denom / 255.0).round();
        *slot = (q * 255.0 / denom).round().clamp(0.0, 255.0) as u8;
    }
    map_lut(buf, n, &lut);
    Ok(())
}

pub(crate) fn threshold(level: u8, buf: &mut PixelBuffer, n: usize) -> Result<(), AdjustError> {
    if level == 0 {
        return Err(AdjustError::InvalidParams(
            "threshold level must be 1..=255".into(),
        ));
    }
    let t = level as f64;
    map_rgb(buf, n, |rv, gv, bv| {
        let y = luma(rv as f64, gv as f64, bv as f64);
        // Equality falls to black (spec's boundary is unspecified).
        let v = if y > t { 255 } else { 0 };
        [v, v, v]
    });
    Ok(())
}

/// The gradient map's 256-entry LUT over luma: the mapped colour and its
/// opacity (`0.0..=1.0`). Refuses fewer than two colour stops, a location past
/// 4096, colour stops out of order, or an opacity past 100%.
/// ponytail: plain linear interpolation between adjacent stops; the reference's
/// midpoint bias, dither, and interpolation modes are not modelled
/// (gradient-map.md marks them closed/inferred).
pub(crate) fn gradient_lut(p: &GradientMapParams) -> Result<[([u8; 3], f64); 256], AdjustError> {
    if p.stops.len() < 2 {
        return Err(AdjustError::InvalidParams(
            "gradient map needs at least 2 stops".into(),
        ));
    }
    if p.stops.iter().any(|s| s.location > 4096) || p.transparency.iter().any(|s| s.location > 4096)
    {
        return Err(AdjustError::InvalidParams(
            "gradient stop location must be <= 4096".into(),
        ));
    }
    if p.stops.windows(2).any(|w| w[0].location >= w[1].location)
        || p.transparency
            .windows(2)
            .any(|w| w[0].location > w[1].location)
    {
        return Err(AdjustError::InvalidParams(
            "gradient stop locations must be increasing".into(),
        ));
    }
    if p.transparency.iter().any(|s| s.opacity > 100) {
        return Err(AdjustError::InvalidParams(
            "gradient opacity must be <= 100".into(),
        ));
    }
    let mut lut = [([0u8; 3], 1.0); 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let l = i as f64 / 255.0;
        let pos = (if p.reverse { 1.0 - l } else { l } * 4096.0).round();
        *slot = (
            sample_gradient(&p.stops, pos),
            sample_opacity(&p.transparency, pos),
        );
    }
    Ok(lut)
}

/// The opacity (`0.0..=1.0`) at `pos`, lerped between the bracketing stops;
/// opaque with no stops.
fn sample_opacity(stops: &[OpacityStop], pos: f64) -> f64 {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return 1.0;
    };
    let value = if pos <= first.location as f64 {
        first.opacity as f64
    } else if pos >= last.location as f64 {
        last.opacity as f64
    } else {
        let i = stops
            .windows(2)
            .position(|w| pos <= w[1].location as f64)
            .unwrap_or(0);
        let (a, b) = (stops[i], stops[i + 1]);
        let span = (b.location - a.location).max(1) as f64;
        let t = ((pos - a.location as f64) / span).clamp(0.0, 1.0);
        a.opacity as f64 + (b.opacity as f64 - a.opacity as f64) * t
    };
    value / 100.0
}

pub(crate) fn gradient_map(
    p: &GradientMapParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    let lut = gradient_lut(p)?;
    map_rgb(buf, n, |rv, gv, bv| {
        let y = luma(rv as f64, gv as f64, bv as f64);
        let (c, a) = lut[y.round().clamp(0.0, 255.0) as usize];
        let blend =
            |orig: u8, mapped: u8| (orig as f64 + (mapped as f64 - orig as f64) * a).round() as u8;
        [blend(rv, c[0]), blend(gv, c[1]), blend(bv, c[2])]
    });
    Ok(())
}

/// Sample the gradient at integer position `pos` (`0..=4096`), clamping outside
/// the stop range and lerping between the bracketing stops.
pub(crate) fn sample_gradient(stops: &[GradientStop], pos: f64) -> [u8; 3] {
    let first = stops[0];
    let last = stops[stops.len() - 1];
    if pos <= first.location as f64 {
        return first.color;
    }
    if pos >= last.location as f64 {
        return last.color;
    }
    let mut i = 0;
    while i + 1 < stops.len() && (stops[i + 1].location as f64) < pos {
        i += 1;
    }
    let a = stops[i];
    let b = stops[i + 1];
    let span = (b.location - a.location) as f64;
    let t = ((pos - a.location as f64) / span).clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    [
        mix(a.color[0], b.color[0]),
        mix(a.color[1], b.color[1]),
        mix(a.color[2], b.color[2]),
    ]
}
