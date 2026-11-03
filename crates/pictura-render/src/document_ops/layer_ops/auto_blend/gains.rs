//! Seamless Tones And Colors: Brown-Lowe gain compensation per channel, baked
//! into the merged layer so source pixels never change. Approximated: Adobe's
//! tonal model is unpublished. Gains act on unit samples, encoded at 8/16-bit and
//! linear at 32-bit.

use pictura_core::{Document, Layer, PsdRect};

use super::sources::{color_ids, unit_row, Grid};
use super::{Cancelled, Control};

const THUMBNAIL_PIXELS: u64 = 512 * 512;
const GAIN_RANGE: (f32, f32) = (0.5, 2.0);
/// Summed neighbour step below which a thumbnail cell counts as flat.
const FLAT_STEP: f32 = 0.04;
const NOISE_SIGMA: f64 = 0.04;
const GAIN_SIGMA: f64 = 0.1;
/// Samples outside this range clipped, so they say nothing about the gain.
const USABLE: (f32, f32) = (0.02, 0.98);

struct Thumbnail {
    /// Mean colour per cell for each channel.
    channels: Vec<Vec<f32>>,
    opaque: Vec<bool>,
    /// Cells without local detail. Tone is measured there, because detail is
    /// the content a stack keeps, not a difference to correct.
    flat: Vec<bool>,
}

fn thumbnail(
    doc: &Document,
    layer: &Layer,
    grid: &Grid,
    control: &Control,
) -> Result<Thumbnail, Cancelled> {
    let ids = color_ids(doc);
    let cells = grid.w * grid.h;
    let mut totals = vec![vec![0f32; cells]; ids.len()];
    let mut seen = vec![0u32; cells];
    let w = layer.rect.width().max(0) as usize;
    let h = layer.rect.height().max(0) as usize;
    let mut alpha = vec![0f32; w];
    let mut colour = vec![vec![0f32; w]; ids.len()];
    let step = grid.f.max(1);
    for y in (0..h).step_by(step) {
        control.check()?;
        unit_row(doc, layer, -1, y, &mut alpha, 1.0);
        for (&id, row) in ids.iter().zip(colour.iter_mut()) {
            unit_row(doc, layer, id, y, row, 0.0);
        }
        for x in (0..w).step_by(step) {
            let cell_x = (layer.rect.left + x as i32 - grid.left).div_euclid(step as i32);
            let cell_y = (layer.rect.top + y as i32 - grid.top).div_euclid(step as i32);
            let inside = cell_x >= 0
                && cell_y >= 0
                && (cell_x as usize) < grid.w
                && (cell_y as usize) < grid.h;
            if inside && alpha[x] > 0.99 {
                let cell = cell_y as usize * grid.w + cell_x as usize;
                seen[cell] += 1;
                for (total, row) in totals.iter_mut().zip(&colour) {
                    total[cell] += row[x];
                }
            }
        }
    }
    for total in &mut totals {
        for (value, &n) in total.iter_mut().zip(&seen) {
            if n > 0 {
                *value /= n as f32;
            }
        }
    }
    let flat = flat_cells(&totals, grid.w, grid.h);
    Ok(Thumbnail {
        channels: totals,
        opaque: seen.iter().map(|&n| n > 0).collect(),
        flat,
    })
}

fn flat_cells(channels: &[Vec<f32>], w: usize, h: usize) -> Vec<bool> {
    (0..w * h)
        .map(|i| {
            let (right, down) = ((i + 1).min(w * h - 1), (i + w).min(w * h - 1));
            channels
                .iter()
                .all(|c| (c[i] - c[right]).abs() + (c[i] - c[down]).abs() < FLAT_STEP)
        })
        .collect()
}

/// Per-channel gains for each of `layers`; Grayscale fills only the first.
pub(super) fn solve(
    doc: &Document,
    layers: &[&Layer],
    domain: PsdRect,
    keep_brightness: bool,
    control: &Control,
) -> Result<Vec<[f32; 3]>, Cancelled> {
    let grid = Grid::new(domain, THUMBNAIL_PIXELS);
    let thumbnails = layers
        .iter()
        .map(|layer| thumbnail(doc, layer, &grid, control))
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = vec![[1.0f32; 3]; layers.len()];
    for channel in 0..color_ids(doc).len() {
        let solved = channel_gains(&thumbnails, channel);
        for (gains, value) in out.iter_mut().zip(solved) {
            gains[channel] = value.clamp(GAIN_RANGE.0, GAIN_RANGE.1);
        }
    }
    control.step()?;
    if keep_brightness && color_ids(doc).len() > 1 {
        out.iter_mut().for_each(keep_luma);
    }
    Ok(out)
}

/// Correct colour, not brightness: a stack's layers may be different scenes, so
/// their brightness difference says little about exposure. A panorama keeps it.
fn keep_luma(gain: &mut [f32; 3]) {
    let luma = luma_gain(gain).max(f32::EPSILON);
    gain.iter_mut().for_each(|g| *g /= luma);
}

/// Mean of two layers over their usable overlap, with its sample count.
fn overlap(a: &Thumbnail, b: &Thumbnail, channel: usize) -> (f64, f64, f64) {
    let (mut mean_a, mut mean_b, mut n) = (0f64, 0f64, 0f64);
    let usable = |v: f32| v > USABLE.0 && v < USABLE.1;
    for cell in 0..a.opaque.len() {
        let (x, y) = (a.channels[channel][cell], b.channels[channel][cell]);
        let flat = a.flat[cell] && b.flat[cell];
        if flat && a.opaque[cell] && b.opaque[cell] && usable(x) && usable(y) {
            mean_a += f64::from(x);
            mean_b += f64::from(y);
            n += 1.0;
        }
    }
    if n > 0.0 {
        (mean_a / n, mean_b / n, n)
    } else {
        (0.0, 0.0, 0.0)
    }
}

/// Minimize `Σ N_ij [(g_i μ_ij − g_j μ_ji)²/σn² + ((1−g_i)² + (1−g_j)²)/σg²]`.
fn channel_gains(thumbnails: &[Thumbnail], channel: usize) -> Vec<f32> {
    let n = thumbnails.len();
    let mut system = vec![vec![0f64; n + 1]; n];
    let noise = NOISE_SIGMA * NOISE_SIGMA;
    let prior = GAIN_SIGMA * GAIN_SIGMA;
    for i in 0..n {
        for j in i + 1..n {
            let (a, b, count) = overlap(&thumbnails[i], &thumbnails[j], channel);
            system[i][i] += count * (a * a / noise + 1.0 / prior);
            system[j][j] += count * (b * b / noise + 1.0 / prior);
            system[i][j] -= count * a * b / noise;
            system[j][i] -= count * a * b / noise;
            system[i][n] += count / prior;
            system[j][n] += count / prior;
        }
    }
    for (i, row) in system.iter_mut().enumerate() {
        if row[i] == 0.0 {
            row[i] = 1.0;
            row[n] = 1.0;
        }
    }
    eliminate(system)
}

/// Gaussian elimination with partial pivoting on an augmented matrix.
fn eliminate(mut m: Vec<Vec<f64>>) -> Vec<f32> {
    let n = m.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))
            .unwrap_or(col);
        m.swap(col, pivot);
        for row in col + 1..n {
            let factor = m[row][col] / m[col][col];
            let (upper, lower) = m.split_at_mut(row);
            for (value, above) in lower[0][col..].iter_mut().zip(&upper[col][col..]) {
                *value -= factor * above;
            }
        }
    }
    let mut out = vec![1f64; n];
    for row in (0..n).rev() {
        let known: f64 = (row + 1..n).map(|k| m[row][k] * out[k]).sum();
        out[row] = (m[row][n] - known) / m[row][row];
    }
    out.into_iter().map(|g| g as f32).collect()
}

/// The luma scale of a colour gain.
pub(super) fn luma_gain(gain: &[f32; 3]) -> f32 {
    0.299 * gain[0] + 0.587 * gain[1] + 0.114 * gain[2]
}
