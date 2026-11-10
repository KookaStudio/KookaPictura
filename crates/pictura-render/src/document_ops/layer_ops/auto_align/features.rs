//! Harris corners and a small rotation-invariant descriptor for Auto-Align.
//!
//! The descriptor is a SIFT-lite 4×4 grid of 8-bin gradient histograms in a
//! dominant-orientation frame. It is deliberately dependency-free: the engine
//! already has the resamplers and blur helpers, and the matcher only needs
//! enough repeatability for a behavioural-parity alignment.

use super::image::WorkImage;

/// One detected feature in work-image coordinates.
pub(super) struct Corner {
    pub x: f64,
    pub y: f64,
    pub desc: [f32; 128],
}

const MAX_CORNERS: usize = 1200;
const BORDER: i32 = 12;

/// Detect corners and build their descriptors.
pub(super) fn detect(img: &WorkImage) -> Vec<Corner> {
    let (w, h) = (img.width as i32, img.height as i32);
    if w < BORDER * 2 + 3 || h < BORDER * 2 + 3 {
        return Vec::new();
    }
    let mut gx = vec![0.0f64; img.luma.len()];
    let mut gy = vec![0.0f64; img.luma.len()];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            gx[i] = f64::from(img.luma[i + 1]) - f64::from(img.luma[i - 1]);
            gy[i] = f64::from(img.luma[i + w as usize]) - f64::from(img.luma[i - w as usize]);
        }
    }
    let mut xx = vec![0.0f64; img.luma.len()];
    let mut yy = vec![0.0f64; img.luma.len()];
    let mut xy = vec![0.0f64; img.luma.len()];
    for i in 0..img.luma.len() {
        xx[i] = gx[i] * gx[i];
        yy[i] = gy[i] * gy[i];
        xy[i] = gx[i] * gy[i];
    }
    let mut response = vec![0.0f64; img.luma.len()];
    let mut peak = 0.0f64;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            let (mut sx, mut sy, mut sxy) = (0.0, 0.0, 0.0);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let j = ((y + dy) * w + x + dx) as usize;
                    sx += xx[j];
                    sy += yy[j];
                    sxy += xy[j];
                }
            }
            let r = sx * sy - sxy * sxy - 0.04 * (sx + sy).powi(2);
            response[i] = r;
            peak = peak.max(r);
        }
    }
    if peak <= 0.0 {
        return Vec::new();
    }
    let threshold = peak * 0.002;
    let mut candidates = Vec::new();
    for y in BORDER..h - BORDER {
        for x in BORDER..w - BORDER {
            let i = (y * w + x) as usize;
            let r = response[i];
            if r < threshold || !img.alpha[i] {
                continue;
            }
            let mut is_peak = true;
            'neighbours: for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if response[((y + dy) * w + x + dx) as usize] > r {
                        is_peak = false;
                        break 'neighbours;
                    }
                }
            }
            if is_peak {
                candidates.push((r, x, y));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    candidates.truncate(MAX_CORNERS);
    candidates
        .into_iter()
        .filter_map(|(_, x, y)| descriptor(img, &gx, &gy, x, y))
        .collect()
}

fn gaussian(dx: f64, dy: f64, sigma: f64) -> f64 {
    (-(dx * dx + dy * dy) / (2.0 * sigma * sigma)).exp()
}

fn descriptor(img: &WorkImage, gx: &[f64], gy: &[f64], x: i32, y: i32) -> Option<Corner> {
    let w = img.width as i32;
    let at = |dx: i32, dy: i32| ((y + dy) * w + x + dx) as usize;
    let mut hist = [0.0f64; 36];
    for dy in -8..=8 {
        for dx in -8..=8 {
            let i = at(dx, dy);
            let mag = gx[i].hypot(gy[i]);
            if mag == 0.0 {
                continue;
            }
            let angle = gy[i].atan2(gx[i]);
            let bin = (((angle + std::f64::consts::PI) / std::f64::consts::TAU) * 36.0)
                .floor()
                .rem_euclid(36.0) as usize;
            hist[bin] += mag * gaussian(f64::from(dx), f64::from(dy), 4.0);
        }
    }
    let (peak_bin, _) = hist.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let theta = (peak_bin as f64 + 0.5) / 36.0 * std::f64::consts::TAU - std::f64::consts::PI;
    let (sin, cos) = theta.sin_cos();
    let mut desc = [0.0f32; 128];
    for dy in -8..=8 {
        for dx in -8..=8 {
            let i = at(dx, dy);
            let mag = gx[i].hypot(gy[i]);
            if mag == 0.0 {
                continue;
            }
            let rx = cos * f64::from(dx) + sin * f64::from(dy);
            let ry = -sin * f64::from(dx) + cos * f64::from(dy);
            if rx < -8.0 || rx >= 8.0 || ry < -8.0 || ry >= 8.0 {
                continue;
            }
            let cell_x = (((rx + 8.0) / 16.0) * 4.0).floor().clamp(0.0, 3.0) as usize;
            let cell_y = (((ry + 8.0) / 16.0) * 4.0).floor().clamp(0.0, 3.0) as usize;
            let angle = (gy[i].atan2(gx[i]) - theta).rem_euclid(std::f64::consts::TAU);
            let bin = ((angle / std::f64::consts::TAU) * 8.0)
                .floor()
                .clamp(0.0, 7.0) as usize;
            let weight = mag * gaussian(rx, ry, 6.0);
            desc[(cell_y * 4 + cell_x) * 8 + bin] += weight as f32;
        }
    }
    let norm = desc.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm < 1e-6 {
        return None;
    }
    for v in &mut desc {
        *v = (*v / norm).min(0.2);
    }
    let norm = desc.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm < 1e-6 {
        return None;
    }
    for v in &mut desc {
        *v /= norm;
    }
    Some(Corner {
        x: f64::from(x),
        y: f64::from(y),
        desc,
    })
}

fn distance(a: &[f32; 128], b: &[f32; 128]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f32>()
}

/// Match `moving` to `reference` with Lowe's ratio test and a mutual-best
/// check. Returns `(moving index, reference index)` pairs.
pub(super) fn match_corners(moving: &[Corner], reference: &[Corner]) -> Vec<(usize, usize)> {
    if moving.is_empty() || reference.is_empty() {
        return Vec::new();
    }
    let mut forward = vec![None; moving.len()];
    for (i, a) in moving.iter().enumerate() {
        let mut best = (f32::INFINITY, 0usize);
        let mut second = f32::INFINITY;
        for (j, b) in reference.iter().enumerate() {
            let d = distance(&a.desc, &b.desc);
            if d < best.0 {
                second = best.0;
                best = (d, j);
            } else if d < second {
                second = d;
            }
        }
        if best.0 < 0.85 * second {
            forward[i] = Some(best.1);
        }
    }
    forward
        .iter()
        .enumerate()
        .filter_map(|(i, j)| j.map(|j| (i, j)))
        .collect()
}
