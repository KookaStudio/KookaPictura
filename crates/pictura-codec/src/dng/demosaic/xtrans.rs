//! Edge-directed demosaic for 6 × 6 X-Trans patterns (#1473): Pictura's own
//! design, not a published algorithm, so parity with other converters is
//! approximate.
//!
//! 1. Green at a red or blue photosite: along the row, the nearest green on
//!    each side (within `REACH`) interpolated linearly by distance, and the
//!    same along the column. The two estimates blend with weights
//!    `1 / (1 + a)²`, where `a` is the direction's activity: the mean absolute
//!    difference of same-colour photosites one or two apart along that
//!    direction, over the three lines through and beside the pixel and
//!    `REACH` either side. Across an edge the activity is high, so the
//!    estimate along the edge wins.
//! 2. Red and blue: the pixel's green plus the colour difference (sample −
//!    green) of the photosites of that colour in the 5 × 5 window, weighted
//!    by `1 / ((1 + d²)(1 + (Δg / EDGE_SCALE)²))`, with `d` the distance and
//!    `Δg` their green difference to the pixel, so differences from across an
//!    edge count little. Near the image border the window widens until it
//!    holds the colour.
//!
//! Every photosite keeps its own sample. Bands of output rows are processed
//! in parallel, each with its own green plane two rows deeper on each side.

use rayon::prelude::*;

use super::Mosaic;

const BAND: usize = 32;
const GREEN: usize = 1;
/// The furthest green looked for along a line.
const REACH: isize = 3;
/// The colour-difference window's half-width.
const WINDOW: isize = 2;
const EDGE_SCALE: f32 = 2048.0;

fn at(colors: &[u8], x: isize, y: isize) -> usize {
    colors[y.rem_euclid(6) as usize * 6 + x.rem_euclid(6) as usize] as usize
}

/// A 6 × 6 pattern in which every 5 × 5 window holds all three colours and
/// every red or blue photosite has a green within `REACH` along its row or
/// column.
pub(super) fn applies(m: &Mosaic) -> bool {
    let (cfa, colors) = (m.cfa, &m.cfa.colors);
    if cfa.width != 6 || cfa.height != 6 || colors.len() != 36 {
        return false;
    }
    (0..6isize).all(|y| {
        (0..6isize).all(|x| {
            let window_holds = |c| {
                (-WINDOW..=WINDOW)
                    .any(|dy| (-WINDOW..=WINDOW).any(|dx| at(colors, x + dx, y + dy) == c))
            };
            let green_on_line = at(colors, x, y) == GREEN
                || (1..=REACH).any(|k| {
                    [(k, 0), (-k, 0), (0, k), (0, -k)]
                        .iter()
                        .any(|&(dx, dy)| at(colors, x + dx, y + dy) == GREEN)
                });
            (0..3).all(window_holds) && green_on_line
        })
    })
}

pub(super) fn demosaic(m: &Mosaic) -> Vec<u16> {
    let mut rgb = vec![0u16; m.width * m.height * 3];
    rgb.par_chunks_mut(m.width * 3 * BAND)
        .enumerate()
        .for_each(|(b, out)| band(m, b * BAND, out));
    rgb
}

fn sample(m: &Mosaic, x: isize, y: isize) -> Option<(usize, f32)> {
    let inside = x >= 0 && y >= 0 && (x as usize) < m.width && (y as usize) < m.height;
    inside.then(|| {
        let (x, y) = (x as usize, y as usize);
        (m.color(x, y), f32::from(m.samples[y * m.width + x]))
    })
}

/// The green interpolated along direction `(dx, dy)` from the nearest green
/// on each side; the one side's green when the other has none.
fn along(m: &Mosaic, x: isize, y: isize, (dx, dy): (isize, isize)) -> Option<f32> {
    let nearest = |sign: isize| {
        (1..=REACH).find_map(|k| match sample(m, x + sign * k * dx, y + sign * k * dy) {
            Some((GREEN, v)) => Some((k as f32, v)),
            _ => None,
        })
    };
    match (nearest(-1), nearest(1)) {
        (Some((ka, a)), Some((kb, b))) => Some((a * kb + b * ka) / (ka + kb)),
        (Some((_, v)), None) | (None, Some((_, v))) => Some(v),
        (None, None) => None,
    }
}

/// The mean absolute difference of same-colour photosites one or two apart
/// along `(dx, dy)` near the pixel.
fn activity(m: &Mosaic, x: isize, y: isize, (dx, dy): (isize, isize)) -> f32 {
    let (mut sum, mut n) = (0f32, 0u32);
    for across in -1..=1 {
        let (bx, by) = (x + across * dy, y + across * dx);
        for t in -REACH..REACH {
            let Some((ca, a)) = sample(m, bx + t * dx, by + t * dy) else {
                continue;
            };
            for step in 1..=2 {
                if t + step > REACH {
                    break;
                }
                if let Some((cb, b)) = sample(m, bx + (t + step) * dx, by + (t + step) * dy) {
                    if ca == cb {
                        sum += (a - b).abs();
                        n += 1;
                    }
                }
            }
        }
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

/// The mean of the greens in the smallest square around the pixel holding
/// one.
fn nearest_greens(m: &Mosaic, x: isize, y: isize) -> f32 {
    let limit = m.width.max(m.height) as isize;
    for r in 1..=limit {
        let (mut sum, mut n) = (0f32, 0u32);
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if let Some((GREEN, v)) = sample(m, xx, yy) {
                    sum += v;
                    n += 1;
                }
            }
        }
        if n > 0 {
            return sum / n as f32;
        }
    }
    0.0
}

fn green_at(m: &Mosaic, x: isize, y: isize) -> f32 {
    let Some((own, v)) = sample(m, x, y) else {
        return 0.0;
    };
    if own == GREEN {
        return v;
    }
    let weight = |dir| {
        let a = 1.0 + activity(m, x, y, dir);
        1.0 / (a * a)
    };
    match (along(m, x, y, (1, 0)), along(m, x, y, (0, 1))) {
        (Some(h), Some(v)) => {
            let (wh, wv) = (weight((1, 0)), weight((0, 1)));
            (wh * h + wv * v) / (wh + wv)
        }
        (Some(e), None) | (None, Some(e)) => e,
        (None, None) => nearest_greens(m, x, y),
    }
}

/// The band's green plane, rows `top..top + rows`.
struct Greens {
    top: usize,
    rows: usize,
    plane: Vec<f32>,
}

impl Greens {
    fn get(&self, m: &Mosaic, x: isize, y: isize) -> f32 {
        let row = y as usize;
        if y >= 0 && row >= self.top && row < self.top + self.rows {
            self.plane[(row - self.top) * m.width + x as usize]
        } else {
            green_at(m, x, y)
        }
    }
}

/// The weighted colour difference of colour `c` around the pixel whose
/// green is `g`.
fn difference(m: &Mosaic, greens: &Greens, x: isize, y: isize, c: usize, g: f32) -> f32 {
    let limit = m.width.max(m.height) as isize;
    let mut r = WINDOW;
    loop {
        let (mut sum, mut total) = (0f32, 0f32);
        for dy in -r..=r {
            for dx in -r..=r {
                let Some((own, v)) = sample(m, x + dx, y + dy) else {
                    continue;
                };
                if own != c {
                    continue;
                }
                let gq = greens.get(m, x + dx, y + dy);
                let edge = (gq - g) / EDGE_SCALE;
                let w = 1.0 / ((1.0 + (dx * dx + dy * dy) as f32) * (1.0 + edge * edge));
                sum += w * (v - gq);
                total += w;
            }
        }
        if total > 0.0 {
            return sum / total;
        }
        if r >= limit {
            return 0.0;
        }
        r += 1;
    }
}

fn to_u16(v: f32) -> u16 {
    v.round().clamp(0.0, 65535.0) as u16
}

fn band(m: &Mosaic, y0: usize, out: &mut [u16]) {
    let w = m.width;
    let rows = out.len() / (w * 3);
    let top = y0.saturating_sub(WINDOW as usize);
    let bottom = (y0 + rows + WINDOW as usize).min(m.height);
    let mut plane = vec![0f32; (bottom - top) * w];
    for (i, g) in plane.iter_mut().enumerate() {
        *g = green_at(m, (i % w) as isize, (top + i / w) as isize);
    }
    let greens = Greens {
        top,
        rows: bottom - top,
        plane,
    };
    for r in 0..rows {
        let y = y0 + r;
        for x in 0..w {
            let own = m.color(x, y);
            let g = greens.plane[(y - top) * w + x];
            let px = &mut out[(r * w + x) * 3..][..3];
            for (c, v) in px.iter_mut().enumerate() {
                *v = if c == own {
                    m.samples[y * w + x]
                } else if c == GREEN {
                    to_u16(g)
                } else {
                    to_u16(g + difference(m, &greens, x as isize, y as isize, c, g))
                };
            }
        }
    }
}
