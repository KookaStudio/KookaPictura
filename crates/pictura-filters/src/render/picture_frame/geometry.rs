//! The scan-converting rasteriser, the contour geometry helpers, the rounded
//! rectangle motifs are laid along, and the seeded arrangement generator.
//!
//! Ported from perfecto25/photorust (`core/src/filters/frame.rs`), GPL-3.0.

use std::f32::consts::{PI, TAU};

use crate::photorust::pixmap::{source_over, Pixmap, Rgba8};

// ------------------------------------------------------------ rasterising --

pub(super) type Contour = Vec<(f32, f32)>;

/// Sub-scanlines per pixel row: vertical anti-aliasing. Horizontal comes from
/// fractional coverage at the ends of each span.
pub(super) const SUBSAMPLES: usize = 4;

/// Twice the signed area: positive for one winding, negative for the other.
pub(super) fn area(c: &[(f32, f32)]) -> f32 {
    let mut a = 0.0;
    for i in 0..c.len() {
        let (x0, y0) = c[i];
        let (x1, y1) = c[(i + 1) % c.len()];
        a += x0 * y1 - x1 * y0;
    }
    a
}

/// The contour wound the filling way.
pub(super) fn solid(mut c: Contour) -> Contour {
    if area(&c) < 0.0 {
        c.reverse();
    }
    c
}

/// The contour wound the cutting way: under the non-zero rule it takes back out
/// what one solid contour around it put in.
pub(super) fn hole(mut c: Contour) -> Contour {
    if area(&c) > 0.0 {
        c.reverse();
    }
    c
}

/// Add `weight` of coverage between `left` and `right`, with the end pixels
/// taking the fraction of them the span covers.
pub(super) fn add_span(row: &mut [f32], left: f32, right: f32, weight: f32) {
    let n = row.len() as f32;
    let (l, r) = (left.max(0.0), right.min(n));
    if r <= l {
        return;
    }
    let (li, rf) = (l.floor() as usize, r.floor() as usize);
    if li == rf || (li + 1 == rf && r == rf as f32) {
        row[li.min(row.len() - 1)] += (r - l) * weight;
        return;
    }
    row[li] += (li as f32 + 1.0 - l) * weight;
    let end = rf.min(row.len());
    for v in &mut row[li + 1..end] {
        *v += weight;
    }
    if rf < row.len() {
        row[rf] += (r - rf as f32) * weight;
    }
}

/// Fill `contours` under the non-zero rule with `colour`, `alpha` of the way.
pub(super) fn fill(pixmap: &mut Pixmap, contours: &[Contour], colour: Rgba8, alpha: f32) {
    let edges: Vec<(f32, f32, f32, f32)> = contours
        .iter()
        .filter(|c| c.len() >= 3)
        .flat_map(|c| {
            (0..c.len()).map(move |i| {
                let (x0, y0) = c[i];
                let (x1, y1) = c[(i + 1) % c.len()];
                (x0, y0, x1, y1)
            })
        })
        // A point that is not a number would throw the winding count off for
        // the rest of the row and flood it.
        .filter(|&(x0, y0, x1, y1)| {
            x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()
        })
        .collect();
    if edges.is_empty() || alpha <= 0.0 {
        return;
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x0, y0, x1, y1) in &edges {
        min_x = min_x.min(x0.min(x1));
        max_x = max_x.max(x0.max(x1));
        min_y = min_y.min(y0.min(y1));
        max_y = max_y.max(y0.max(y1));
    }
    let left = (min_x.floor() as i32).max(0);
    let top = (min_y.floor() as i32).max(0);
    let right = (max_x.ceil() as i32 + 1).min(pixmap.width() as i32);
    let bottom = (max_y.ceil() as i32 + 1).min(pixmap.height() as i32);
    if right <= left || bottom <= top {
        return;
    }
    let width = (right - left) as usize;
    let mut row = vec![0.0f32; width];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    let weight = 1.0 / SUBSAMPLES as f32;
    for y in top..bottom {
        row.fill(0.0);
        for s in 0..SUBSAMPLES {
            let sy = y as f32 + (s as f32 + 0.5) / SUBSAMPLES as f32;
            crossings.clear();
            for &(x0, y0, x1, y1) in &edges {
                // Half-open in y, so a vertex on the sub-scanline counts once.
                if (y0 <= sy) == (y1 <= sy) {
                    continue;
                }
                let t = (sy - y0) / (y1 - y0);
                crossings.push((
                    x0 + t * (x1 - x0) - left as f32,
                    if y0 <= sy { 1 } else { -1 },
                ));
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                if winding != 0 {
                    add_span(&mut row, pair[0].0, pair[1].0, weight);
                }
            }
        }
        for (i, &c) in row.iter().enumerate() {
            let a = c.min(1.0) * alpha;
            if a <= 0.0 {
                continue;
            }
            let x = left + i as i32;
            let dst = pixmap.get(x, y);
            pixmap.set(x, y, source_over(dst, colour, a));
        }
    }
}

// --------------------------------------------------------------- geometry --

pub(super) fn circle(cx: f32, cy: f32, r: f32) -> Contour {
    ellipse(cx, cy, r, r, 0.0)
}

pub(super) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, turn: f32) -> Contour {
    let n = ((rx.max(ry) * 1.5) as usize).clamp(10, 96);
    let (s, c) = turn.sin_cos();
    solid(
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * TAU;
                let (x, y) = (rx * t.cos(), ry * t.sin());
                (cx + x * c - y * s, cy + x * s + y * c)
            })
            .collect(),
    )
}

pub(super) fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Contour {
    solid(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

/// A closed curve round the origin, `radius` of each angle, in `n` steps.
pub(super) fn polar(n: usize, radius: impl Fn(f32) -> f32) -> Contour {
    solid(
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * TAU;
                let r = radius(t);
                (r * t.cos(), r * t.sin())
            })
            .collect(),
    )
}

/// A star of `points` points between `outer` and `inner`, one pointing up.
pub(super) fn star(points: usize, outer: f32, inner: f32) -> Contour {
    solid(
        (0..points * 2)
            .map(|i| {
                let r = if i % 2 == 0 { outer } else { inner };
                let t = i as f32 / (points * 2) as f32 * TAU - PI / 2.0;
                (r * t.cos(), r * t.sin())
            })
            .collect(),
    )
}

/// A petal along `angle` from `r0` to `r1` from the origin, `width` across at
/// its widest; `bulge` below 1 fattens it towards the tip.
pub(super) fn petal(angle: f32, r0: f32, r1: f32, width: f32, bulge: f32) -> Contour {
    let n = 16;
    let (s, c) = angle.sin_cos();
    let mut side: Vec<(f32, f32)> = Vec::with_capacity(2 * n + 2);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let along = r0 + t * (r1 - r0);
        let half = width * 0.5 * (PI * t.powf(bulge)).sin().max(0.0);
        side.push((along, half));
    }
    let mut out: Vec<(f32, f32)> = side.iter().map(|&(a, h)| (a, h)).collect();
    out.extend(side.iter().rev().skip(1).take(n - 1).map(|&(a, h)| (a, -h)));
    solid(
        out.into_iter()
            .map(|(a, h)| (a * c - h * s, a * s + h * c))
            .collect(),
    )
}

/// A band round the origin from `inner` to `outer`, from angle `a0` to `a1`.
pub(super) fn arc_band(cx: f32, cy: f32, inner: f32, outer: f32, a0: f32, a1: f32) -> Contour {
    let n = (((a1 - a0).abs() * outer * 0.8) as usize).clamp(6, 64);
    let mut out = Vec::with_capacity(2 * n + 2);
    for i in 0..=n {
        let t = a0 + (a1 - a0) * i as f32 / n as f32;
        out.push((cx + outer * t.cos(), cy + outer * t.sin()));
    }
    for i in (0..=n).rev() {
        let t = a0 + (a1 - a0) * i as f32 / n as f32;
        out.push((cx + inner * t.cos(), cy + inner * t.sin()));
    }
    solid(out)
}

/// A ring cut into `dashes` dashes, each `share` of its step long.
pub(super) fn dashed_ring(inner: f32, outer: f32, dashes: usize, share: f32) -> Vec<Contour> {
    let step = TAU / dashes as f32;
    (0..dashes)
        .map(|i| {
            let a = i as f32 * step - PI / 2.0;
            arc_band(
                0.0,
                0.0,
                inner,
                outer,
                a - step * share / 2.0,
                a + step * share / 2.0,
            )
        })
        .collect()
}

/// A line of `width` along `points`, with round joins and ends, as outlines
/// that union under the non-zero rule.
pub(super) fn stroke(points: &[(f32, f32)], width: f32) -> Vec<Contour> {
    let half = width * 0.5;
    let mut out = Vec::with_capacity(points.len() * 2);
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 1e-4 {
            continue;
        }
        let (nx, ny) = (-dy / len * half, dx / len * half);
        out.push(solid(vec![
            (x0 + nx, y0 + ny),
            (x1 + nx, y1 + ny),
            (x1 - nx, y1 - ny),
            (x0 - nx, y0 - ny),
        ]));
    }
    // Round joins, which also round the two ends. Only worth it once the line
    // is wide enough for a corner to show.
    if half > 0.6 {
        for &(x, y) in points {
            out.push(circle(x, y, half));
        }
    }
    out
}

/// Points along a quadratic Bézier curve.
pub(super) fn bezier(a: (f32, f32), b: (f32, f32), c: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * b.0 + t * t * c.0,
                u * u * a.1 + 2.0 * u * t * b.1 + t * t * c.1,
            )
        })
        .collect()
}

/// A spiral curling in from `(x, y)`: it starts heading along `heading`, turns
/// `turns` times round with its radius shrinking from `radius`, the way a
/// tendril does. `sense` is 1 to curl clockwise and -1 against.
pub(super) fn curl(
    x: f32,
    y: f32,
    heading: f32,
    radius: f32,
    turns: f32,
    sense: f32,
) -> Vec<(f32, f32)> {
    let n = ((turns * radius * 2.0) as usize).clamp(12, 120);
    // The centre is a radius to the side of the start, so the curve sets off
    // along `heading`.
    let side = heading + sense * PI / 2.0;
    let (cx, cy) = (x + radius * side.cos(), y + radius * side.sin());
    let start = side + PI;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let r = radius * (1.0 - 0.75 * t);
            let a = start + sense * t * turns * TAU;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// Scale by `scale`, turn by `turn` and move to `(x, y)`.
pub(super) fn place(contours: &[Contour], x: f32, y: f32, scale: f32, turn: f32) -> Vec<Contour> {
    let (s, c) = turn.sin_cos();
    contours
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|&(px, py)| {
                    let (px, py) = (px * scale, py * scale);
                    (x + px * c - py * s, y + px * s + py * c)
                })
                .collect()
        })
        .collect()
}

// ------------------------------------------------------------------ track --

/// The rounded rectangle the motifs are laid along, measured clockwise from the
/// top left.
#[derive(Clone, Copy)]
pub(super) struct Track {
    pub(super) x0: f32,
    pub(super) y0: f32,
    pub(super) x1: f32,
    pub(super) y1: f32,
    pub(super) radius: f32,
}

/// One side of a track, then the corner after it: start, direction, length,
/// corner centre, and the corner's start angle.
pub(super) type Piece = ((f32, f32), (f32, f32), f32, (f32, f32), f32);

impl Track {
    pub(super) fn sides(&self) -> (f32, f32) {
        (
            (self.x1 - self.x0 - 2.0 * self.radius).max(0.0),
            (self.y1 - self.y0 - 2.0 * self.radius).max(0.0),
        )
    }

    pub(super) fn length(&self) -> f32 {
        let (w, h) = self.sides();
        2.0 * (w + h) + TAU * self.radius
    }

    /// Where `s` along the track is, and which way it runs there.
    pub(super) fn at(&self, s: f32) -> ((f32, f32), (f32, f32)) {
        let (w, h) = self.sides();
        let r = self.radius;
        let quarter = PI / 2.0 * r;
        let mut s = s.rem_euclid(self.length());
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1, self.y1);
        // Each side, then the corner after it.
        let pieces: [Piece; 4] = [
            ((x0 + r, y0), (1.0, 0.0), w, (x1 - r, y0 + r), -PI / 2.0),
            ((x1, y0 + r), (0.0, 1.0), h, (x1 - r, y1 - r), 0.0),
            ((x1 - r, y1), (-1.0, 0.0), w, (x0 + r, y1 - r), PI / 2.0),
            ((x0, y1 - r), (0.0, -1.0), h, (x0 + r, y0 + r), PI),
        ];
        for (start, dir, len, centre, a0) in pieces {
            if s <= len {
                return ((start.0 + dir.0 * s, start.1 + dir.1 * s), dir);
            }
            s -= len;
            if s <= quarter && r > 0.0 {
                let a = a0 + s / r;
                return (
                    (centre.0 + r * a.cos(), centre.1 + r * a.sin()),
                    (-a.sin(), a.cos()),
                );
            }
            s -= quarter;
        }
        ((x0 + r, y0), (1.0, 0.0))
    }

    /// Positions spread evenly round the track, about `spacing` apart, the
    /// first half a step in so the corners come out alike.
    pub(super) fn spread(&self, spacing: f32) -> Vec<f32> {
        let n = (self.length() / spacing.max(1.0)).round().max(4.0) as usize;
        let step = self.length() / n as f32;
        (0..n).map(|i| (i as f32 + 0.5) * step).collect()
    }
}

/// A small, fast, seeded generator: the arrangement is a seed, and the same
/// seed has to give the same frame every time or undo could not replay it.
pub(super) struct Rng(u64);

impl Rng {
    pub(super) fn new(seed: u32, frame: u32) -> Rng {
        Rng(0x9E37_79B9_7F4A_7C15
            ^ ((seed as u64) << 32 | frame as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
    }

    pub(super) fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(super) fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}
