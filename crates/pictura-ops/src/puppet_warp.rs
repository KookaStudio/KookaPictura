//! Bounded, pin-driven triangular raster deformation.

use pictura_core::{PixelBuffer, PsdRect};

pub const MAX_PIXELS: u64 = 4 * 1024 * 1024;
const MAX_VERTICES: usize = 262_144;
const MAX_VISITS: u64 = 64_000_000;
const MAX_PINS: usize = 32;

#[derive(Clone, Copy, Debug)]
pub struct Pin {
    pub rest: [f64; 2],
    pub target: [f64; 2],
    pub angle: Option<f64>,
    pub ordinal: u32,
}

#[derive(Clone, Debug)]
struct Triangle {
    src: [[f64; 2]; 3],
    dst: [[f64; 2]; 3],
    depth: usize,
    index: usize,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    width: u32,
    height: u32,
    triangles: Vec<Triangle>,
    bounds: PsdRect,
    identity: bool,
    pin_angles: Vec<(u32, f64)>,
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn dist2(a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1]
}
fn finite(p: [f64; 2]) -> bool {
    p[0].is_finite() && p[1].is_finite()
}

fn angle(pin: &Pin, pins: &[Pin]) -> f64 {
    if let Some(a) = pin.angle {
        return a;
    }
    let (mut s, mut c) = (0.0, 0.0);
    for other in pins {
        let a = sub(other.rest, pin.rest);
        let b = sub(other.target, pin.target);
        let d = dist2(other.rest, pin.rest);
        if d <= 1e-18 || dist2(other.target, pin.target) <= 1e-18 {
            continue;
        }
        let delta = cross(a, b).atan2(a[0] * b[0] + a[1] * b[1]);
        s += delta.sin() / d;
        c += delta.cos() / d;
    }
    if s.hypot(c) < 1e-15 {
        0.0
    } else {
        s.atan2(c)
    }
}

fn map_vertex(p: [f64; 2], pins: &[Pin], angles: &[f64], exponent: f64) -> [f64; 2] {
    if pins.is_empty() {
        return p;
    }
    for pin in pins {
        if p == pin.rest {
            return pin.target;
        }
    }
    // Relative weights avoid overflow when a point approaches a pin.
    let nearest = pins
        .iter()
        .map(|pin| dist2(p, pin.rest))
        .fold(f64::INFINITY, f64::min);
    let mut result = [0.0, 0.0];
    let mut total = 0.0;
    for (pin, a) in pins.iter().zip(angles) {
        let w = (nearest / dist2(p, pin.rest)).powf(exponent * 0.5);
        let d = sub(p, pin.rest);
        let (s, c) = a.sin_cos();
        result[0] += w * (pin.target[0] + c * d[0] - s * d[1]);
        result[1] += w * (pin.target[1] + s * d[0] + c * d[1]);
        total += w;
    }
    [result[0] / total, result[1] / total]
}

fn axis(size: u32, expansion: i32, segments: usize, pins: &[Pin], component: usize) -> Vec<f64> {
    let lo = -(expansion as f64);
    let hi = f64::from(size) + f64::from(expansion);
    let mut values: Vec<_> = (0..=segments)
        .map(|i| lo + (hi - lo) * i as f64 / segments as f64)
        .collect();
    values.extend(pins.iter().map(|p| p.rest[component]));
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| *a == *b);
    values
}

fn triangle_owner(src: &[[f64; 2]; 3], pins: &[Pin]) -> usize {
    let centroid = [
        (src[0][0] + src[1][0] + src[2][0]) / 3.0,
        (src[0][1] + src[1][1] + src[2][1]) / 3.0,
    ];
    pins.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            dist2(centroid, a.rest)
                .total_cmp(&dist2(centroid, b.rest))
                .then(a.ordinal.cmp(&b.ordinal))
        })
        .map_or(0, |(i, _)| i)
}

fn rounded(v: f64) -> f64 {
    let nearest = v.round();
    if (v - nearest).abs() < 1e-9 {
        nearest
    } else {
        v
    }
}

impl Mesh {
    pub fn solve(
        width: u32,
        height: u32,
        pins: &[Pin],
        mode: i32,
        density: i32,
        expansion: i32,
    ) -> Option<Self> {
        if width == 0
            || height == 0
            || u64::from(width) * u64::from(height) > MAX_PIXELS
            || expansion < 0
            || pins.len() > MAX_PINS
        {
            return None;
        }
        let exponent = match mode {
            0 => 1.0,
            1 => 2.0,
            2 => 4.0,
            _ => return None,
        };
        let segments = match density {
            0 => 8,
            1 => 16,
            2 => 32,
            _ => return None,
        };
        let lo = -f64::from(expansion);
        let right = f64::from(width) + f64::from(expansion);
        let bottom = f64::from(height) + f64::from(expansion);
        if (right - lo) * (bottom - lo) > MAX_PIXELS as f64 {
            return None;
        }
        for (i, pin) in pins.iter().enumerate() {
            if !finite(pin.rest)
                || !finite(pin.target)
                || pin.angle.is_some_and(|a| !a.is_finite())
                || pin.rest[0] < lo
                || pin.rest[0] > right
                || pin.rest[1] < lo
                || pin.rest[1] > bottom
                || pins[..i]
                    .iter()
                    .any(|other| other.rest == pin.rest || other.ordinal == pin.ordinal)
            {
                return None;
            }
        }
        let xs = axis(width, expansion, segments, pins, 0);
        let ys = axis(height, expansion, segments, pins, 1);
        if xs.len().checked_mul(ys.len())? > MAX_VERTICES {
            return None;
        }
        let angles: Vec<_> = pins.iter().map(|pin| angle(pin, pins)).collect();
        let mut vertices = Vec::with_capacity(xs.len() * ys.len());
        let mut identity = true;
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &y in &ys {
            for &x in &xs {
                let rest = [x, y];
                let point = map_vertex(rest, pins, &angles, exponent);
                if !finite(point) {
                    return None;
                }
                identity &= dist2(rest, point) < 1e-18;
                min_x = min_x.min(point[0]);
                min_y = min_y.min(point[1]);
                max_x = max_x.max(point[0]);
                max_y = max_y.max(point[1]);
                vertices.push(point);
            }
        }
        let edges = [
            rounded(min_x).floor(),
            rounded(min_y).floor(),
            rounded(max_x).ceil(),
            rounded(max_y).ceil(),
        ];
        if edges
            .iter()
            .any(|edge| *edge < i32::MIN as f64 || *edge > i32::MAX as f64)
        {
            return None;
        }
        let bounds = PsdRect {
            left: edges[0] as i32,
            top: edges[1] as i32,
            right: edges[2] as i32,
            bottom: edges[3] as i32,
        };
        if bounds.is_empty()
            || u64::try_from(bounds.width())
                .ok()?
                .checked_mul(u64::try_from(bounds.height()).ok()?)?
                > MAX_PIXELS
        {
            return None;
        }
        let mut triangles = Vec::with_capacity((xs.len() - 1) * (ys.len() - 1) * 2);
        for y in 0..ys.len() - 1 {
            for x in 0..xs.len() - 1 {
                let ids = [
                    y * xs.len() + x,
                    y * xs.len() + x + 1,
                    (y + 1) * xs.len() + x,
                    (y + 1) * xs.len() + x + 1,
                ];
                let points = [
                    [xs[x], ys[y]],
                    [xs[x + 1], ys[y]],
                    [xs[x], ys[y + 1]],
                    [xs[x + 1], ys[y + 1]],
                ];
                for indices in [[0, 1, 3], [0, 3, 2]] {
                    let src = indices.map(|i| points[i]);
                    let dst = indices.map(|i| vertices[ids[i]]);
                    let index = triangles.len();
                    triangles.push(Triangle {
                        depth: triangle_owner(&src, pins),
                        src,
                        dst,
                        index,
                    });
                }
            }
        }
        // Back to front; for equal owners the lowest original index wins.
        triangles.sort_by(|a, b| a.depth.cmp(&b.depth).then(b.index.cmp(&a.index)));
        let pin_angles = pins
            .iter()
            .zip(angles)
            .map(|(pin, angle)| (pin.ordinal, angle))
            .collect();
        Some(Self {
            width,
            height,
            triangles,
            bounds,
            identity,
            pin_angles,
        })
    }

    pub fn pin_angle(&self, ordinal: u32) -> Option<f64> {
        self.pin_angles
            .iter()
            .find(|(id, _)| *id == ordinal)
            .map(|(_, angle)| *angle)
    }

    pub fn bounds(&self) -> PsdRect {
        self.bounds
    }
    pub fn is_identity(&self) -> bool {
        self.identity
    }
    pub fn triangles(&self) -> impl Iterator<Item = [[f64; 2]; 3]> + '_ {
        self.triangles.iter().map(|t| t.dst)
    }

    pub fn source_at(&self, point: [f64; 2]) -> Option<[f64; 2]> {
        if !finite(point) {
            return None;
        }
        self.triangles.iter().rev().find_map(|t| inverse(t, point))
    }

    pub fn can_raster(&self, origin: [f64; 2], scale: f64, width: u32, height: u32) -> bool {
        if !finite(origin)
            || !scale.is_finite()
            || scale <= 0.0
            || width == 0
            || height == 0
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return false;
        }
        let mut visits = 0u64;
        for triangle in &self.triangles {
            let Some((left, top, right, bottom)) =
                clip_bounds(triangle, origin, scale, width, height)
            else {
                return false;
            };
            let area = u64::from(right - left) * u64::from(bottom - top);
            visits = match visits.checked_add(area) {
                Some(value) if value <= MAX_VISITS => value,
                _ => return false,
            };
        }
        true
    }

    pub fn raster(
        &self,
        src: &PixelBuffer,
        mask_default: Option<u8>,
        origin: [f64; 2],
        scale: f64,
        width: u32,
        height: u32,
    ) -> Option<PixelBuffer> {
        let expected = (src.width as usize)
            .checked_mul(src.height as usize)?
            .checked_mul(src.channels as usize)?;
        if src.width != self.width
            || src.height != self.height
            || !matches!(src.channels, 1 | 2 | 4)
            || (src.channels == 1) != mask_default.is_some()
            || src.data.len() != expected
            || !self.can_raster(origin, scale, width, height)
        {
            return None;
        }
        let mut out = PixelBuffer::new(width, height, src.channels);
        if let Some(default) = mask_default {
            out.data.fill(default);
        }
        let plane = width as usize * height as usize;
        for triangle in &self.triangles {
            let (left, top, right, bottom) = clip_bounds(triangle, origin, scale, width, height)?;
            for y in top..bottom {
                for x in left..right {
                    let point = [
                        origin[0] + (f64::from(x) + 0.5) / scale,
                        origin[1] + (f64::from(y) + 0.5) / scale,
                    ];
                    if !covered(triangle, point) {
                        continue;
                    }
                    let Some(source) = inverse(triangle, point) else {
                        continue;
                    };
                    let samples = sample(src, source, mask_default);
                    let index = y as usize * width as usize + x as usize;
                    for channel in 0..src.channels as usize {
                        out.data[channel * plane + index] = samples[channel];
                    }
                }
            }
        }
        Some(out)
    }
}

fn inverse(t: &Triangle, p: [f64; 2]) -> Option<[f64; 2]> {
    let a = sub(t.dst[1], t.dst[0]);
    let b = sub(t.dst[2], t.dst[0]);
    let d = cross(a, b);
    if d.abs() < 1e-12 || !d.is_finite() {
        return None;
    }
    let q = sub(p, t.dst[0]);
    let u = cross(q, b) / d;
    let v = cross(a, q) / d;
    if u < -1e-10 || v < -1e-10 || u + v > 1.0 + 1e-10 {
        return None;
    }
    Some([
        t.src[0][0] + u * (t.src[1][0] - t.src[0][0]) + v * (t.src[2][0] - t.src[0][0]),
        t.src[0][1] + u * (t.src[1][1] - t.src[0][1]) + v * (t.src[2][1] - t.src[0][1]),
    ])
}

fn covered(t: &Triangle, p: [f64; 2]) -> bool {
    let area = cross(sub(t.dst[1], t.dst[0]), sub(t.dst[2], t.dst[0]));
    if !area.is_finite() || area.abs() < 1e-12 {
        return false;
    }
    for i in 0..3 {
        let mut a = t.dst[i];
        let mut b = t.dst[(i + 1) % 3];
        if area < 0.0 {
            std::mem::swap(&mut a, &mut b);
        }
        let edge = sub(b, a);
        let side = cross(edge, sub(p, a));
        let inclusive = edge[1] < 0.0 || (edge[1] == 0.0 && edge[0] > 0.0);
        if side < 0.0 || (side == 0.0 && !inclusive) {
            return false;
        }
    }
    true
}

fn clip_bounds(
    t: &Triangle,
    origin: [f64; 2],
    scale: f64,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in t.dst {
        for c in 0..2 {
            let v = (p[c] - origin[c]) * scale;
            if !v.is_finite() {
                return None;
            }
            lo[c] = lo[c].min(v);
            hi[c] = hi[c].max(v);
        }
    }
    let edge = |v: f64, limit: u32| v.max(0.0).min(f64::from(limit)) as u32;
    Some((
        edge(lo[0].floor(), width),
        edge(lo[1].floor(), height),
        edge(hi[0].ceil(), width),
        edge(hi[1].ceil(), height),
    ))
}

fn sample(src: &PixelBuffer, p: [f64; 2], default: Option<u8>) -> [u8; 4] {
    let x = p[0] - 0.5;
    let y = p[1] - 0.5;
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let channels = src.channels as usize;
    let n = src.pixel_count();
    let mut values = [0.0; 4];
    for dy in 0..2 {
        for dx in 0..2 {
            let px = x0 + dx;
            let py = y0 + dy;
            let w = if dx == 0 { 1.0 - fx } else { fx } * if dy == 0 { 1.0 - fy } else { fy };
            let inside =
                px >= 0 && py >= 0 && px < i64::from(src.width) && py < i64::from(src.height);
            let read = |c: usize| -> f64 {
                if inside {
                    f64::from(src.data[c * n + py as usize * src.width as usize + px as usize])
                } else {
                    f64::from(default.unwrap_or(0))
                }
            };
            let alpha = if channels == 1 {
                255.0
            } else {
                read(channels - 1)
            };
            for (c, v) in values.iter_mut().enumerate().take(channels) {
                *v += w * if channels > 1 && c < channels - 1 {
                    read(c) * alpha / 255.0
                } else {
                    read(c)
                };
            }
        }
    }
    if channels > 1 {
        let alpha = values[channels - 1];
        for v in values.iter_mut().take(channels - 1) {
            *v = if alpha > 0.0 { *v * 255.0 / alpha } else { 0.0 };
        }
    }
    values.map(|v| v.round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
#[path = "puppet_warp_tests.rs"]
mod tests;
