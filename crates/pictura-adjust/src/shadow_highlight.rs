use std::borrow::Cow;

use pictura_core::{blur::approx_gaussian, Sample, ShadowsHighlightsParams};

use crate::{common::luma, AdjustError, ColorBalanceMode};

pub(crate) fn apply<T: Sample>(
    params: &ShadowsHighlightsParams,
    data: &mut [T],
    width: usize,
    height: usize,
    mode: ColorBalanceMode,
    bins: usize,
) -> Result<(), AdjustError> {
    let options = params
        .local
        .filter(|_| params.is_valid())
        .ok_or_else(|| AdjustError::InvalidParams("invalid Shadows/Highlights settings".into()))?;
    let grayscale = mode == ColorBalanceMode::Grayscale;
    let brightness = if grayscale { options.brightness / 100.0 } else { 0.0 };
    let contrast = options.midtone_contrast / 100.0;
    let shadow_amount = params.shadows_amount / 100.0;
    let highlight_amount = params.highlights_amount / 100.0;
    if shadow_amount == 0.0
        && highlight_amount == 0.0
        && brightness == 0.0
        && contrast == 0.0
        && options.black_clip == 0.0
        && options.white_clip == 0.0
    {
        return Ok(());
    }

    let n = width * height;
    let (red, rest) = data.split_at_mut(n);
    let (green, rest) = rest.split_at_mut(n);
    let blue = &mut rest[..n];
    let luminance: Vec<f64> = red
        .iter()
        .zip(green.iter())
        .zip(blue.iter())
        .map(|((&r, &g), &b)| luma(r.to_unit(), g.to_unit(), b.to_unit()))
        .collect();
    let surround = |amount, radius| {
        if amount == 0.0 || radius == 0.0 {
            Cow::Borrowed(luminance.as_slice())
        } else {
            let mut plane = luminance.clone();
            approx_gaussian(&mut plane, width, height, radius);
            Cow::Owned(plane)
        }
    };
    let shadows = surround(shadow_amount, options.shadows_radius);
    let highlights = if shadow_amount > 0.0 && options.shadows_radius == options.highlights_radius {
        Cow::Borrowed(shadows.as_ref())
    } else {
        surround(highlight_amount, options.highlights_radius)
    };
    let tonal = |i: usize| {
        let y = luminance[i];
        let lift = shadow_amount * weight(shadows[i], options.shadows_width);
        let pull = highlight_amount * weight(1.0 - highlights[i], options.highlights_width);
        (y + lift * (1.0 - y)) * (1.0 - pull)
    };
    let corrected = |i: usize| {
        let y = (tonal(i) + brightness).clamp(0.0, 1.0);
        y + contrast * y * (1.0 - y) * (2.0 * y - 1.0)
    };
    let (black, white) = clip_endpoints(n, bins, options.black_clip, options.white_clip, corrected);
    for i in 0..n {
        let y = luminance[i];
        let out = ((corrected(i) - black) / (white - black)).clamp(0.0, 1.0);
        if grayscale {
            let value = T::from_unit(out);
            red[i] = value;
            green[i] = value;
            blue[i] = value;
            continue;
        }
        let chroma = [red[i], green[i], blue[i]].map(|sample| sample.to_unit() - y);
        let correction = options.color_correction / 100.0;
        let mut gain = 1.0 + correction * (tonal(i) - y).abs().sqrt();
        // Preserve hue at a gamut boundary and make clipped endpoints neutral.
        for component in chroma {
            if component > 0.0 {
                gain = gain.min((1.0 - out) / component);
            } else if component < 0.0 {
                gain = gain.min(out / -component);
            }
        }
        let [r, g, b] = chroma.map(|component| T::from_unit(out + gain * component));
        red[i] = r;
        green[i] = g;
        blue[i] = b;
    }
    Ok(())
}

fn weight(luma: f64, width: f64) -> f64 {
    if width == 0.0 {
        return f64::from(u8::from(luma <= 0.0));
    }
    let limit = (width / 100.0).min(0.98);
    let t = (1.0 - luma / limit).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn clip_endpoints(
    count: usize,
    bins: usize,
    black: f64,
    white: f64,
    value: impl Fn(usize) -> f64,
) -> (f64, f64) {
    let black_rank = (count as f64 * black / 100.0) as usize;
    let white_rank = (count as f64 * white / 100.0) as usize;
    if black_rank == 0 && white_rank == 0 {
        return (0.0, 1.0);
    }
    let scale = (bins - 1) as f64;
    let mut histogram = vec![0usize; bins];
    for i in 0..count {
        histogram[(value(i) * scale).round().clamp(0.0, scale) as usize] += 1;
    }
    let Some(first) = histogram.iter().position(|&count| count != 0) else {
        return (0.0, 1.0);
    };
    let last = histogram.iter().rposition(|&count| count != 0).unwrap_or(first);
    if first == last {
        return (0.0, 1.0);
    }
    let lo = if black_rank == 0 {
        0.0
    } else {
        let mut total = 0;
        let position = histogram.iter().position(|&bin| {
            total += bin;
            total >= black_rank
        }).unwrap_or(first);
        // Use the upper edge of the endpoint bin to clip every sample in it.
        (position as f64 + 0.5) / scale
    };
    let hi = if white_rank == 0 {
        1.0
    } else {
        let mut total = 0;
        let position = histogram.iter().rev().position(|&bin| {
            total += bin;
            total >= white_rank
        }).map_or(last, |offset| bins - 1 - offset);
        (position as f64 - 0.5) / scale
    };
    if lo < hi { (lo, hi) } else { (0.0, 1.0) }
}
