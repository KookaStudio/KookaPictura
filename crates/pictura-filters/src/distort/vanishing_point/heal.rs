//! Heal: how dropped selections and strokes blend into the layer. Off
//! copies; On solves `Δf = Δsrc` inside the footprint with the destination
//! fixed around it (a Poisson blend: the source's texture, the destination's
//! colour and lighting, no seam); Luminance runs the same solve on
//! luminance only and keeps the source's chroma. *(approximation of CS6's
//! closed blend)*

use super::canvas::{Canvas, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HealMode {
    #[default]
    Off,
    Luminance,
    On,
}

impl HealMode {
    pub fn from_i32(v: i32) -> HealMode {
        match v {
            1 => HealMode::Luminance,
            2 => HealMode::On,
            _ => HealMode::Off,
        }
    }
}

/// ponytail: a fixed Gauss-Seidel budget, started from the source shifted to
/// the border's mean, so very large footprints converge only near the border.
const SWEEPS: usize = 300;

fn luma(c: [f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

/// Blend `src` (straight colour `0..=255`, alpha `0..=1`, one per pixel of
/// `rect`) into `canvas` by `mask` (`0..=1`, same layout).
pub fn blend(canvas: &mut Canvas, rect: Rect, src: &[[f32; 4]], mask: &[f32], mode: HealMode) {
    let (w, h) = (rect.width(), rect.height());
    if w == 0 || h == 0 || src.len() != w * h || mask.len() != w * h {
        return;
    }
    let cover: Vec<f32> = (0..w * h)
        .map(|i| (mask[i] * src[i][3]).clamp(0.0, 1.0))
        .collect();
    let dest: Vec<[f32; 4]> = (0..w * h)
        .map(|i| {
            let px = canvas.get(rect.x0 + (i % w) as i32, rect.y0 + (i / w) as i32);
            [
                px[0] as f32,
                px[1] as f32,
                px[2] as f32,
                px[3] as f32 / 255.0,
            ]
        })
        .collect();
    let colour: Vec<[f32; 3]> = match mode {
        HealMode::Off => src.iter().map(|s| [s[0], s[1], s[2]]).collect(),
        HealMode::On => {
            let mut out = vec![[0f32; 3]; w * h];
            for c in 0..3 {
                let s: Vec<f32> = src.iter().map(|p| p[c]).collect();
                let d: Vec<f32> = dest.iter().map(|p| p[c]).collect();
                let f = poisson(&s, &d, &cover, w, h);
                for i in 0..w * h {
                    out[i][c] = f[i];
                }
            }
            out
        }
        HealMode::Luminance => {
            let s: Vec<f32> = src.iter().map(|p| luma([p[0], p[1], p[2]])).collect();
            let d: Vec<f32> = dest.iter().map(|p| luma([p[0], p[1], p[2]])).collect();
            let f = poisson(&s, &d, &cover, w, h);
            (0..w * h)
                .map(|i| {
                    let shift = f[i] - s[i];
                    [src[i][0] + shift, src[i][1] + shift, src[i][2] + shift]
                })
                .collect()
        }
    };
    for i in 0..w * h {
        let m = cover[i];
        if m <= 0.0 {
            continue;
        }
        let d = dest[i];
        let a = d[3] + m * (1.0 - d[3]);
        let mut px = [0u8; 4];
        for c in 0..3 {
            let v = if a > 0.0 {
                (d[c] * d[3] * (1.0 - m) + colour[i][c] * m) / a
            } else {
                colour[i][c]
            };
            px[c] = v.round().clamp(0.0, 255.0) as u8;
        }
        px[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        canvas.set(rect.x0 + (i % w) as i32, rect.y0 + (i / w) as i32, px);
    }
}

/// Solve `Δf = Δs` where `cover > 0`, with `f = d` elsewhere and beyond the
/// rect's edge.
fn poisson(s: &[f32], d: &[f32], cover: &[f32], w: usize, h: usize) -> Vec<f32> {
    let unknown: Vec<bool> = cover.iter().map(|&m| m > 0.0).collect();
    // Start from the source moved to the border's mean level.
    let (mut sum, mut n) = (0f64, 0usize);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !unknown[i] {
                continue;
            }
            let border = x == 0
                || y == 0
                || x + 1 == w
                || y + 1 == h
                || !unknown[i - 1]
                || !unknown[i + 1]
                || !unknown[i - w]
                || !unknown[i + w];
            if border {
                sum += (d[i] - s[i]) as f64;
                n += 1;
            }
        }
    }
    let shift = if n > 0 { (sum / n as f64) as f32 } else { 0.0 };
    let mut f: Vec<f32> = (0..w * h)
        .map(|i| if unknown[i] { s[i] + shift } else { d[i] })
        .collect();
    for _ in 0..SWEEPS {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !unknown[i] {
                    continue;
                }
                let mut acc = 0f32;
                for (nx, ny) in [
                    (x as isize - 1, y as isize),
                    (x as isize + 1, y as isize),
                    (x as isize, y as isize - 1),
                    (x as isize, y as isize + 1),
                ] {
                    // The source only guides between unknowns; a fixed
                    // neighbour (or the rect's edge) is pure boundary.
                    acc += if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        d[i]
                    } else {
                        let j = ny as usize * w + nx as usize;
                        if unknown[j] {
                            f[j] + s[i] - s[j]
                        } else {
                            d[j]
                        }
                    };
                }
                f[i] = acc / 4.0;
            }
        }
    }
    f
}
