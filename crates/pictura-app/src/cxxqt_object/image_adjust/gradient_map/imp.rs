use super::super::super::helpers::rgba_from_argb;
use super::super::{from_block, to_block};
use cxx_qt_lib::QString;
use pictura_paint::gradient;
use pictura_render::{
    encode_gradient_map, Adjustment, GradientMapParams, GradientStop, OpacityStop,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Stop {
    location: u16,
    /// Red, green, blue, opacity.
    rgba: [u8; 4],
}

pub(super) fn format_stops(stops: &[Stop]) -> String {
    stops
        .iter()
        .map(|s| {
            let [r, g, b, a] = s.rgba;
            if a == 255 {
                format!("{}:{r:02x}{g:02x}{b:02x}", s.location)
            } else {
                format!("{}:{r:02x}{g:02x}{b:02x}{a:02x}", s.location)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn parse_stops(text: &str) -> Option<Vec<Stop>> {
    text.split_whitespace()
        .map(|item| {
            let (location, hex) = item.split_once(':')?;
            let location = location.parse::<u16>().ok().filter(|l| *l <= 4096)?;
            let value = u32::from_str_radix(hex, 16).ok()?;
            let rgba = match hex.len() {
                6 => (value << 8 | 0xff).to_be_bytes(),
                8 => value.to_be_bytes(),
                _ => return None,
            };
            Some(Stop { location, rgba })
        })
        .collect()
}

/// Sort `stops` and move coincident ones apart by one step, so their locations
/// strictly increase within `0..=4096` as `grdm` requires. A ramp's hard edge
/// (two stops at one place) becomes a one-step blend.
pub(super) fn spread(stops: &mut [Stop]) -> bool {
    if stops.len() < 2 || stops.len() > 4097 {
        return false;
    }
    stops.sort_by_key(|s| s.location);
    for i in 1..stops.len() {
        if stops[i].location <= stops[i - 1].location {
            stops[i].location = stops[i - 1].location + 1;
        }
    }
    let last = stops.len() - 1;
    stops[last].location = stops[last].location.min(4096);
    for i in (0..last).rev() {
        if stops[i].location >= stops[i + 1].location {
            stops[i].location = stops[i + 1].location - 1;
        }
    }
    true
}

/// Smoothness blends straight lines between the stops (0 %) toward a spline
/// through them (100 %) that leaves each stop without a kink, sampled every 32
/// steps. Two stops stay a straight line at any smoothness.
/// ponytail: the spline (Catmull-Rom tangents, clamped) is inferred from how
/// CS6's ramps look; the reference's interpolation is closed.
pub(super) fn smoothed(stops: &[Stop], smoothness: i32) -> Vec<Stop> {
    let s = f64::from(smoothness.clamp(0, 100)) / 100.0;
    if s == 0.0 || stops.len() < 3 {
        return stops.to_vec();
    }
    let x: Vec<f64> = stops.iter().map(|p| f64::from(p.location)).collect();
    let n = stops.len();
    let value = |pos: f64, c: usize| -> f64 {
        let y = |i: usize| f64::from(stops[i].rgba[c]);
        if pos <= x[0] {
            return y(0);
        }
        if pos >= x[n - 1] {
            return y(n - 1);
        }
        let i = x.windows(2).position(|w| pos <= w[1]).unwrap_or(0);
        let h = x[i + 1] - x[i];
        let t = (pos - x[i]) / h;
        let linear = y(i) + (y(i + 1) - y(i)) * t;
        let tangent = |k: usize| {
            let (a, b) = (k.saturating_sub(1), (k + 1).min(n - 1));
            (y(b) - y(a)) / (x[b] - x[a])
        };
        let (slope_start, slope_end) = (tangent(i) * h, tangent(i + 1) * h);
        let (t2, t3) = (t * t, t * t * t);
        let spline = (2.0 * t3 - 3.0 * t2 + 1.0) * y(i)
            + (t3 - 2.0 * t2 + t) * slope_start
            + (-2.0 * t3 + 3.0 * t2) * y(i + 1)
            + (t3 - t2) * slope_end;
        linear + (spline - linear) * s
    };
    (0..=128u16)
        .map(|k| {
            let location = k * 32;
            let pos = f64::from(location);
            let rgba = [0, 1, 2, 3].map(|c| value(pos, c).round().clamp(0.0, 255.0) as u8);
            Stop { location, rgba }
        })
        .collect()
}

pub(super) fn smoothed_from_text(text: &str, smoothness: i32) -> Option<Vec<Stop>> {
    let mut stops = parse_stops(text)?;
    spread(&mut stops).then(|| smoothed(&stops, smoothness))
}

pub(super) fn decode_map(block: &[u8]) -> Option<GradientMapParams> {
    match pictura_render::decode_adjustment(&from_block(block)?)? {
        Adjustment::GradientMap(p) => Some(p),
        _ => None,
    }
}

/// `points` (location, value), sorted, linearly interpolated at `x` and held
/// flat past either end.
pub(super) fn interpolate(points: &[(f64, f64)], x: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return 0.0;
    };
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    let i = points.windows(2).position(|w| x <= w[1].0).unwrap_or(0);
    let ((x0, y0), (x1, y1)) = (points[i], points[i + 1]);
    y0 + (y1 - y0) * (x - x0) / (x1 - x0).max(1.0)
}

/// The colour and opacity stops merged into RGBA stops at every location
/// either list names; colour stops only when the map is opaque.
pub(super) fn readback_stops(p: &GradientMapParams) -> Vec<Stop> {
    let mut locations: Vec<u16> = p
        .stops
        .iter()
        .map(|s| s.location)
        .chain(p.transparency.iter().map(|s| s.location))
        .collect();
    locations.sort_unstable();
    locations.dedup();
    let channel = |c: usize| -> Vec<(f64, f64)> {
        p.stops
            .iter()
            .map(|s| (f64::from(s.location), f64::from(s.color[c])))
            .collect()
    };
    let colour = [channel(0), channel(1), channel(2)];
    let opacity: Vec<(f64, f64)> = p
        .transparency
        .iter()
        .map(|s| (f64::from(s.location), f64::from(s.opacity)))
        .collect();
    locations
        .into_iter()
        .map(|location| {
            let x = f64::from(location);
            let [r, g, b] = [0, 1, 2].map(|c| interpolate(&colour[c], x).round() as u8);
            let percent = if opacity.is_empty() {
                100.0
            } else {
                interpolate(&opacity, x)
            };
            let a = (percent * 255.0 / 100.0).round().clamp(0.0, 255.0) as u8;
            Stop {
                location,
                rgba: [r, g, b, a],
            }
        })
        .collect()
}

pub(super) fn gradient_map_stops(block: &[u8]) -> QString {
    let text = decode_map(block)
        .map(|p| format_stops(&readback_stops(&p)))
        .unwrap_or_default();
    QString::from(text.as_str())
}

pub(super) fn gradient_map_reverse(block: &[u8]) -> bool {
    decode_map(block).is_some_and(|p| p.reverse)
}

pub(super) fn gradient_map_dither(block: &[u8]) -> bool {
    decode_map(block).is_some()
        && from_block(block)
            .and_then(|d| pictura_render::gradient_map_dither(&d.data))
            .unwrap_or(false)
}

pub(super) fn gradient_map_block(
    stops: &QString,
    smoothness: i32,
    reverse: bool,
    dither: bool,
) -> Vec<u8> {
    let Some(stops) = smoothed_from_text(&stops.to_string(), smoothness) else {
        return Vec::new();
    };
    let transparency = if stops.iter().all(|s| s.rgba[3] == 255) {
        Vec::new()
    } else {
        stops
            .iter()
            .map(|s| OpacityStop {
                location: s.location,
                opacity: (f64::from(s.rgba[3]) * 100.0 / 255.0).round() as u8,
            })
            .collect()
    };
    let params = GradientMapParams {
        stops: stops
            .iter()
            .map(|s| GradientStop {
                location: s.location,
                color: [s.rgba[0], s.rgba[1], s.rgba[2]],
            })
            .collect(),
        reverse,
        transparency,
    };
    to_block(Some(encode_gradient_map(&params, dither)))
}

pub(super) fn gradient_smoothed_stops(stops: &QString, smoothness: i32) -> QString {
    let text = smoothed_from_text(&stops.to_string(), smoothness)
        .map(|s| format_stops(&s))
        .unwrap_or_default();
    QString::from(text.as_str())
}

/// xorshift32: a fixed sequence per seed, so a Noise gradient is reproducible.
pub(super) struct Rng(u32);

impl Rng {
    fn unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        f64::from(x) / f64::from(u32::MAX)
    }
}

pub(super) fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [f64; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// ponytail: CS6's noise generator is closed. This one places `3 + 0.61 ×
/// roughness` stops (jittered along the ramp) of random colours inside the
/// component ranges; Restrict Colors caps HSB saturation at 80 %.
pub(super) fn noise_stops(
    seed: u32,
    roughness: i32,
    model: i32,
    ranges: [(i32, i32); 3],
    restrict: bool,
    transparency: bool,
) -> Vec<Stop> {
    let mut rng = Rng(seed.max(1));
    let count = 3 + roughness.clamp(0, 100) as usize * 61 / 100;
    let spacing = 4096.0 / (count - 1) as f64;
    let mut stops: Vec<Stop> = (0..count)
        .map(|i| {
            let jitter = if i == 0 || i == count - 1 {
                0.0
            } else {
                (rng.unit() - 0.5) * 0.7 * spacing
            };
            let location = (i as f64 * spacing + jitter).round().clamp(0.0, 4096.0) as u16;
            let u = ranges.map(|(low, high)| {
                let (low, high) = (low.clamp(0, 100), high.clamp(0, 100));
                let (low, high) = (low.min(high), low.max(high));
                (f64::from(low) + rng.unit() * f64::from(high - low)) / 100.0
            });
            let mut rgb = match model {
                1 => hsv_to_rgb(u[0] * 360.0, u[1], u[2]),
                2 => {
                    let lab = pictura_codec::lab_to_rgb(&u.map(|v| (v * 255.0).round() as u8));
                    [0, 1, 2].map(|c| f64::from(lab[c]) / 255.0)
                }
                _ => u,
            };
            if restrict {
                let max = rgb[0].max(rgb[1]).max(rgb[2]);
                let min = rgb[0].min(rgb[1]).min(rgb[2]);
                if max > 0.0 && (max - min) / max > 0.8 {
                    rgb = rgb.map(|c| max - (max - c) * 0.8 * max / (max - min));
                }
            }
            let alpha = if transparency {
                rng.unit() * 255.0
            } else {
                255.0
            };
            let [r, g, b] = rgb.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8);
            Stop {
                location,
                rgba: [r, g, b, alpha.round() as u8],
            }
        })
        .collect();
    spread(&mut stops);
    stops
}

#[allow(clippy::too_many_arguments)]
pub(super) fn gradient_noise_stops(
    seed: u32,
    roughness: i32,
    model: i32,
    low0: i32,
    high0: i32,
    low1: i32,
    high1: i32,
    low2: i32,
    high2: i32,
    restrict: bool,
    transparency: bool,
) -> QString {
    let ranges = [(low0, high0), (low1, high1), (low2, high2)];
    let stops = noise_stops(seed, roughness, model, ranges, restrict, transparency);
    QString::from(format_stops(&stops).as_str())
}

pub(super) fn gradient_preset_map_stops(index: i32, foreground: u32, background: u32) -> QString {
    let preset = usize::try_from(index)
        .ok()
        .and_then(|i| gradient::preset(i, rgba_from_argb(foreground), rgba_from_argb(background)));
    let Some(preset) = preset else {
        return QString::default();
    };
    let mut stops: Vec<Stop> = preset
        .stops
        .iter()
        .map(|s| Stop {
            location: (s.position.clamp(0.0, 1.0) * 4096.0).round() as u16,
            rgba: [s.color.r, s.color.g, s.color.b, s.color.a],
        })
        .collect();
    if !spread(&mut stops) {
        return QString::default();
    }
    QString::from(format_stops(&stops).as_str())
}

#[cfg(test)]
mod tests;
