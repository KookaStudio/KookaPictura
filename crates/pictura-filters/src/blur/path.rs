//! Path Blur: motion blur that follows drawn paths (`FILT-092`; a post-CS6
//! Blur Gallery effect ported on the owner's request, not CS6 parity), with
//! the gallery's Noise pass.
//!
//! Each path is the Catmull-Rom curve through its points. A pixel's motion
//! vector blends every path's nearest-point tangent, scaled by the end-point
//! speeds interpolated along the curve, by inverse-square distance (as Field
//! Blur blends its pins), times the global Speed. The pixel then averages
//! taps along the streamline of that field — half behind and half ahead of
//! it with Centered Blur, all behind it otherwise — weighted by Taper, the
//! strobe flashes, and Rear Sync Flash.
//!
//! ponytail: Adobe's motion field and flash model are closed. The blend, the
//! streamline gather (at most 64 bilinear taps, the field read from a grid of
//! at most 512 cells on the long edge), and the taper / strobe / Rear Sync
//! weights are inferred.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use super::field::{apply_effects, validate_noise, BlurNoise, BokehParams, MAX_FIELD_PINS};
use super::tilt::bilinear;
use crate::{validate, FilterError};

/// The most points one path carries.
pub const MAX_PATH_POINTS: usize = 32;

/// The Speed slider's top, in percent (unverified; read off the owner's
/// screenshots' slider travel).
pub const MAX_PATH_SPEED: f64 = 500.0;

/// The largest End Point Speed, in buffer pixels (unverified).
pub const MAX_END_POINT_SPEED: f64 = 500.0;

/// The most taps one pixel's trail samples.
const MAX_PATH_TAPS: usize = 64;

/// Curve samples per span between two points.
const SPAN_SEGMENTS: usize = 16;

/// The motion field's grid has at most this many cells on the long edge.
const FIELD_CELLS: u32 = 512;

/// The distance floor of the inverse-square weights, in buffer widths.
const PATH_EPSILON: f64 = 1.0e-6;

/// One blur path: its points as fractions of the buffer (outside allowed)
/// and the speed at each end in buffer pixels — the length of that end's
/// blur shape.
#[derive(Debug, Clone, PartialEq)]
pub struct BlurPath {
    pub points: Vec<(f64, f64)>,
    pub start_speed: f64,
    pub end_speed: f64,
}

/// The Path Blur row's and Motion Effects tab's values, shared by every path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathMotion {
    /// 0..=`MAX_PATH_SPEED` percent; scales every end-point speed.
    pub speed: f64,
    /// 0..=100; fades the trail toward its far end.
    pub taper: f64,
    /// Centre the trail on the pixel instead of trailing behind it.
    pub centered: bool,
    /// Rear Sync Flash: the subject frozen sharp at the end of the exposure.
    pub rear_sync: bool,
    /// 0..=100; mixes the continuous trail toward the strobe flashes.
    pub strobe_strength: f64,
    /// 1..=100 flashes spread over the trail.
    pub strobe_flashes: u32,
}

impl Default for PathMotion {
    fn default() -> Self {
        // The owner's screenshots' opening state.
        Self {
            speed: 50.0,
            taper: 0.0,
            centered: true,
            rear_sync: false,
            strobe_strength: 0.0,
            strobe_flashes: 4,
        }
    }
}

/// Path-following motion blur with the Noise pass. An empty path list is a
/// no-op.
pub fn path_blur(
    buf: &mut PixelBuffer,
    paths: &[BlurPath],
    motion: &PathMotion,
    noise: &BlurNoise,
) -> Result<(), FilterError> {
    validate(buf)?;
    validate_paths(paths)?;
    validate_motion(motion)?;
    validate_noise(noise)?;
    if let Some(weights) = path_pass(buf, paths, motion) {
        apply_effects(buf, &weights, &BOKEH_OFF, noise);
    }
    Ok(())
}

pub(super) const BOKEH_OFF: BokehParams = BokehParams {
    enabled: false,
    light_bokeh: 0.0,
    bokeh_color: 0.0,
    light_range: (0, 255),
};

/// The gather alone. Returns each pixel's trail length as a weight (0 sharp,
/// 255 at the longest trail) for the effects passes, or `None` (buffer
/// untouched) when there are no paths or no trail reaches a pixel. Callers
/// validate first.
pub(super) fn path_pass(
    buf: &mut PixelBuffer,
    paths: &[BlurPath],
    motion: &PathMotion,
) -> Option<Vec<u8>> {
    if paths.is_empty() {
        return None;
    }
    let (w, h) = (buf.width as usize, buf.height as usize);
    let field = MotionField::new(paths, motion.speed / 100.0, buf.width, buf.height);
    // A bilinear blend never outgrows its longest corner vector.
    let longest = field.longest();
    if longest < 1.0 {
        return None;
    }
    let n = w * h;
    let planes = (buf.channels as usize).min(4);
    let src = buf.data[..planes * n].to_vec();
    let gathered: Vec<([u8; 4], u8)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            let (vx, vy) = field.at(x, y);
            let length = vx.hypot(vy);
            let mut out = [0u8; 4];
            if length < 1.0 {
                for (c, slot) in out.iter_mut().enumerate().take(planes) {
                    *slot = src[c * n + i];
                }
                return (out, 0);
            }
            let mut taps = [(0.0, 0.0, 0.0); MAX_PATH_TAPS];
            let count = trail(&field, motion, x, y, (vx, vy), length, &mut taps);
            for (c, slot) in out.iter_mut().enumerate().take(planes) {
                let plane = &src[c * n..(c + 1) * n];
                let sum: f64 = taps[..count]
                    .iter()
                    .map(|&(tx, ty, weight)| weight * bilinear(plane, w, h, tx - 0.5, ty - 0.5))
                    .sum();
                *slot = sum.round().clamp(0.0, 255.0) as u8;
            }
            let weight = (length / longest * 255.0).round().clamp(0.0, 255.0) as u8;
            (out, weight)
        })
        .collect();
    for c in 0..planes {
        buf.data[c * n..(c + 1) * n]
            .par_iter_mut()
            .zip(gathered.par_iter())
            .for_each(|(out, (pixel, _))| *out = pixel[c]);
    }
    Some(gathered.into_iter().map(|(_, weight)| weight).collect())
}

/// Fill `taps` with a pixel's trail — `(x, y, weight)` in pixel-centre
/// coordinates, weights summing to 1, back to front — and return how many.
fn trail(
    field: &MotionField,
    motion: &PathMotion,
    x: f64,
    y: f64,
    vector: (f64, f64),
    length: f64,
    taps: &mut [(f64, f64, f64); MAX_PATH_TAPS],
) -> usize {
    let count = (length.ceil() as usize + 1).clamp(2, MAX_PATH_TAPS);
    let step = length / (count - 1) as f64;
    let behind = if motion.centered {
        (count - 1) / 2
    } else {
        count - 1
    };
    let ahead = count - 1 - behind;
    let start = (vector.0 / length, vector.1 / length);
    taps[behind] = (x, y, 0.0);
    // Walk the streamline each way, re-reading the direction at every step
    // so a curved path's trail bends with it.
    for (sign, steps) in [(-1.0, behind), (1.0, ahead)] {
        let (mut px, mut py) = (x, y);
        let mut dir = start;
        for k in 1..=steps {
            px += sign * dir.0 * step;
            py += sign * dir.1 * step;
            let index = if sign < 0.0 { behind - k } else { behind + k };
            taps[index] = (px, py, 0.0);
            let (dx, dy) = field.at(px, py);
            let norm = dx.hypot(dy);
            if norm > 1.0e-9 {
                dir = (dx / norm, dy / norm);
            }
        }
    }
    let reach = behind.max(ahead).max(1) as f64;
    let taper = motion.taper / 100.0;
    let mut base = [0.0f64; MAX_PATH_TAPS];
    for (k, weight) in base.iter_mut().enumerate().take(count) {
        let u = (k as f64 - behind as f64).abs() / reach;
        *weight = 1.0 - taper * u;
    }
    let base_sum: f64 = base[..count].iter().sum();
    let mut weights = [0.0f64; MAX_PATH_TAPS];
    for k in 0..count {
        weights[k] = base[k] / base_sum;
    }
    let strength = motion.strobe_strength / 100.0;
    if strength > 0.0 {
        let mut strobe = [0.0f64; MAX_PATH_TAPS];
        let flashes = motion.strobe_flashes as usize;
        for j in 0..flashes {
            let index = if flashes == 1 {
                behind
            } else {
                ((j * (count - 1)) as f64 / (flashes - 1) as f64).round() as usize
            };
            strobe[index] += base[index];
        }
        let strobe_sum: f64 = strobe[..count].iter().sum();
        if strobe_sum > 0.0 {
            for k in 0..count {
                weights[k] = weights[k] * (1.0 - strength) + strobe[k] / strobe_sum * strength;
            }
        }
    }
    let mut total = 1.0;
    if motion.rear_sync {
        weights[behind] += 1.0;
        total = 2.0;
    }
    for k in 0..count {
        taps[k].2 = weights[k] / total;
    }
    count
}

/// One path resolved to buffer pixels: its sampled curve and the cumulative
/// arc length (a fraction of the whole) at each sample.
struct Curve {
    samples: Vec<(f64, f64)>,
    arc: Vec<f64>,
    start_speed: f64,
    end_speed: f64,
}

impl Curve {
    fn new(path: &BlurPath, w: f64, h: f64) -> Self {
        let points: Vec<(f64, f64)> = path.points.iter().map(|&(x, y)| (x * w, y * h)).collect();
        let samples = catmull_rom(&points);
        let mut arc = Vec::with_capacity(samples.len());
        let mut total = 0.0;
        arc.push(0.0);
        for pair in samples.windows(2) {
            total += (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
            arc.push(total);
        }
        if total > 0.0 {
            for value in &mut arc {
                *value /= total;
            }
        }
        Self {
            samples,
            arc,
            start_speed: path.start_speed,
            end_speed: path.end_speed,
        }
    }

    /// The squared distance to the nearest curve point and the curve's unit
    /// tangent times the interpolated speed there (zero on a degenerate
    /// curve).
    fn nearest(&self, x: f64, y: f64) -> (f64, (f64, f64)) {
        let mut best = (f64::INFINITY, (0.0, 0.0));
        for (i, pair) in self.samples.windows(2).enumerate() {
            let (ax, ay) = pair[0];
            let (dx, dy) = (pair[1].0 - ax, pair[1].1 - ay);
            let length2 = dx * dx + dy * dy;
            if length2 <= 0.0 {
                continue;
            }
            let t = (((x - ax) * dx + (y - ay) * dy) / length2).clamp(0.0, 1.0);
            let (qx, qy) = (ax + dx * t - x, ay + dy * t - y);
            let distance2 = qx * qx + qy * qy;
            if distance2 < best.0 {
                let s = self.arc[i] + (self.arc[i + 1] - self.arc[i]) * t;
                let speed = self.start_speed + (self.end_speed - self.start_speed) * s;
                let length = length2.sqrt();
                best = (distance2, (dx / length * speed, dy / length * speed));
            }
        }
        if best.0.is_infinite() {
            // Every point coincides: the path has no direction.
            let (px, py) = self.samples[0];
            best.0 = (px - x).powi(2) + (py - y).powi(2);
        }
        best
    }
}

/// The uniform Catmull-Rom curve through `points`, the ends' outer control
/// points mirrored, sampled `SPAN_SEGMENTS` times per span.
fn catmull_rom(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let last = points.len() - 1;
    let at = |i: isize| -> (f64, f64) {
        if i < 0 {
            let (a, b) = (points[0], points[1]);
            (2.0 * a.0 - b.0, 2.0 * a.1 - b.1)
        } else if i as usize > last {
            let (a, b) = (points[last], points[last - 1]);
            (2.0 * a.0 - b.0, 2.0 * a.1 - b.1)
        } else {
            points[i as usize]
        }
    };
    let mut samples = Vec::with_capacity(last * SPAN_SEGMENTS + 1);
    for span in 0..last as isize {
        let (p0, p1, p2, p3) = (at(span - 1), at(span), at(span + 1), at(span + 2));
        for k in 0..SPAN_SEGMENTS {
            let t = k as f64 / SPAN_SEGMENTS as f64;
            let (t2, t3) = (t * t, t * t * t);
            let blend = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b
                    + (c - a) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            samples.push((blend(p0.0, p1.0, p2.0, p3.0), blend(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    samples.push(points[last]);
    samples
}

/// The motion vector field, in buffer pixels, on a grid of nodes `cell`
/// pixels apart (node `(i, j)` at pixel-centre coordinates
/// `(i·cell, j·cell)`), read back bilinearly.
pub(super) struct MotionField {
    nodes: Vec<(f64, f64)>,
    columns: usize,
    rows: usize,
    cell: f64,
}

impl MotionField {
    pub(super) fn new(paths: &[BlurPath], scale: f64, width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width), f64::from(height));
        let curves: Vec<Curve> = paths.iter().map(|path| Curve::new(path, w, h)).collect();
        let cell = width.max(height).div_ceil(FIELD_CELLS).max(1);
        let columns = width.div_ceil(cell) as usize + 1;
        let rows = height.div_ceil(cell) as usize + 1;
        let cell = f64::from(cell);
        let nodes = (0..columns * rows)
            .into_par_iter()
            .map(|i| {
                let (x, y) = ((i % columns) as f64 * cell, (i / columns) as f64 * cell);
                let (mut sx, mut sy, mut weight_sum) = (0.0, 0.0, 0.0);
                for curve in &curves {
                    let (distance2, (vx, vy)) = curve.nearest(x, y);
                    let weight = 1.0 / (distance2 / (w * w) + PATH_EPSILON);
                    sx += weight * vx;
                    sy += weight * vy;
                    weight_sum += weight;
                }
                (sx / weight_sum * scale, sy / weight_sum * scale)
            })
            .collect();
        Self {
            nodes,
            columns,
            rows,
            cell,
        }
    }

    fn longest(&self) -> f64 {
        self.nodes
            .iter()
            .map(|&(x, y)| x.hypot(y))
            .fold(0.0, f64::max)
    }

    /// The bilinear field vector at pixel-centre coordinates, clamped to the
    /// grid.
    pub(super) fn at(&self, x: f64, y: f64) -> (f64, f64) {
        let gx = (x / self.cell).clamp(0.0, (self.columns - 1) as f64);
        let gy = (y / self.cell).clamp(0.0, (self.rows - 1) as f64);
        let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.columns - 1), (y0 + 1).min(self.rows - 1));
        let (tx, ty) = (gx - x0 as f64, gy - y0 as f64);
        let node = |x: usize, y: usize| self.nodes[y * self.columns + x];
        let lerp =
            |a: (f64, f64), b: (f64, f64), t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        lerp(
            lerp(node(x0, y0), node(x1, y0), tx),
            lerp(node(x0, y1), node(x1, y1), tx),
            ty,
        )
    }
}

pub(super) fn validate_paths(paths: &[BlurPath]) -> Result<(), FilterError> {
    if paths.len() > MAX_FIELD_PINS {
        return Err(FilterError::InvalidParams(format!(
            "path blur takes at most {MAX_FIELD_PINS} paths, got {}",
            paths.len()
        )));
    }
    for path in paths {
        if !(2..=MAX_PATH_POINTS).contains(&path.points.len()) {
            return Err(FilterError::InvalidParams(format!(
                "a blur path takes 2..={MAX_PATH_POINTS} points, got {}",
                path.points.len()
            )));
        }
        if path
            .points
            .iter()
            .any(|&(x, y)| !x.is_finite() || !y.is_finite())
        {
            return Err(FilterError::InvalidParams(
                "blur path points must be finite".into(),
            ));
        }
        for speed in [path.start_speed, path.end_speed] {
            if !speed.is_finite() || !(0.0..=MAX_END_POINT_SPEED).contains(&speed) {
                return Err(FilterError::InvalidParams(format!(
                    "end point speed {speed} is outside 0..={MAX_END_POINT_SPEED}"
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_motion(motion: &PathMotion) -> Result<(), FilterError> {
    for (name, value, top) in [
        ("speed", motion.speed, MAX_PATH_SPEED),
        ("taper", motion.taper, 100.0),
        ("strobe strength", motion.strobe_strength, 100.0),
    ] {
        if !value.is_finite() || !(0.0..=top).contains(&value) {
            return Err(FilterError::InvalidParams(format!(
                "path blur {name} {value} is outside 0..={top}"
            )));
        }
    }
    if !(1..=100).contains(&motion.strobe_flashes) {
        return Err(FilterError::InvalidParams(format!(
            "strobe flashes {} is outside 1..=100",
            motion.strobe_flashes
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
