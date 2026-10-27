//! Clamped-window median over an 8-bit plane.
//!
//! A per-column 256-bin histogram (a "band") of the clamped `2r+1`-tall segment
//! is kept for every column in the strip. Advancing a row updates each band by
//! one sample in and one out and carries the strip's `x0` window histogram with
//! the same deltas; sliding a pixel right replaces one whole band in the window
//! histogram with its neighbour. The cost per sample is therefore independent of
//! the radius: two contiguous 256-bin passes instead of `(2r+1)²` samples.

use rayon::prelude::*;

const BINS: usize = 256;
type Band = [u32; BINS];

/// Replace each sample with the median of its clamped `(2r+1)²` window.
///
/// `radius == 0` is the identity. No parameter cap is applied here: callers keep
/// their own documented bounds.
pub fn median(src: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let n = w.saturating_mul(h);
    if radius == 0 || n == 0 || src.len() != n {
        return src.to_vec();
    }
    let threads = rayon::current_num_threads().clamp(1, 16);
    let strip = w.div_ceil(threads).max(1);
    let strips: Vec<(usize, usize, Vec<u8>)> = (0..w)
        .step_by(strip)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|x0| {
            let x1 = (x0 + strip).min(w);
            (x0, x1, median_strip(src, w, h, x0, x1, radius))
        })
        .collect();
    let mut out = vec![0u8; n];
    for (x0, x1, part) in strips {
        let bw = x1 - x0;
        for y in 0..h {
            out[y * w + x0..y * w + x1].copy_from_slice(&part[y * bw..(y + 1) * bw]);
        }
    }
    out
}

fn median_strip(src: &[u8], w: usize, h: usize, x0: usize, x1: usize, r: usize) -> Vec<u8> {
    let window = 2 * r + 1;
    let target = (window * window) / 2;
    let total = window * window;
    let bw = x1 - x0;
    let lo = x0.saturating_sub(r);
    let hi = (x1 + r).min(w);
    let mut bands = seed_bands(src, w, h, lo, hi, r);
    let mults: Vec<usize> = (lo..hi)
        .map(|col| window_multiplicity(col, x0, w, r))
        .collect();
    let mut x0_hist = [0u32; BINS];
    for (offset, &m) in mults.iter().enumerate() {
        if m == 0 {
            continue;
        }
        let m = m as u32;
        for (bin, &count) in bands[offset].iter().enumerate() {
            x0_hist[bin] += count * m;
        }
    }
    let mut out = vec![0u8; h * bw];
    for y in 0..h {
        let mut hist = x0_hist;
        for (j, slot) in out[y * bw..(y + 1) * bw].iter_mut().enumerate() {
            *slot = pick(&hist, total, target);
            let x = x0 + j;
            if x + 1 < x1 {
                let leave = (x as isize - r as isize).clamp(0, w as isize - 1) as usize;
                let enter = (x as isize + r as isize + 1).clamp(0, w as isize - 1) as usize;
                let leave_band = &bands[leave - lo];
                let enter_band = &bands[enter - lo];
                for (bin, slot) in hist.iter_mut().enumerate() {
                    *slot = *slot - leave_band[bin] + enter_band[bin];
                }
            }
        }
        if y + 1 < h {
            advance_row(&mut bands, &mut x0_hist, &mults, src, w, h, y, r, lo);
        }
    }
    out
}

/// The band histograms for row 0: one per column, over the clamped rows
/// `-r ..= r`.
fn seed_bands(src: &[u8], w: usize, h: usize, lo: usize, hi: usize, r: usize) -> Vec<Band> {
    let mut bands = vec![[0u32; BINS]; hi - lo];
    for yy in 0..h {
        let multiplicity = row_multiplicity(yy, 0, h, r);
        if multiplicity == 0 {
            continue;
        }
        let base = yy * w;
        for (offset, band) in bands.iter_mut().enumerate() {
            band[src[base + lo + offset] as usize] += multiplicity as u32;
        }
    }
    bands
}

/// How many of the `2r+1` clamped row taps land on row `yy` for a window
/// centred at `y`.
fn row_multiplicity(yy: usize, y: usize, h: usize, r: usize) -> usize {
    if h == 1 {
        return 2 * r + 1;
    }
    let y = y as isize;
    let r = r as isize;
    let yy = yy as isize;
    let h = h as isize;
    if yy == 0 {
        if y <= r {
            (r - y + 1) as usize
        } else {
            0
        }
    } else if yy == h - 1 {
        if y >= h - 1 - r {
            (r + y - h + 2) as usize
        } else {
            0
        }
    } else if (yy - y).abs() <= r {
        1
    } else {
        0
    }
}

/// Move the bands and the strip's `x0` window from row `y` to `y+1`.
#[allow(clippy::too_many_arguments)]
fn advance_row(
    bands: &mut [Band],
    x0_hist: &mut [u32; BINS],
    mults: &[usize],
    src: &[u8],
    w: usize,
    h: usize,
    y: usize,
    r: usize,
    lo: usize,
) {
    let leave = (y as isize - r as isize).clamp(0, h as isize - 1) as usize * w;
    let enter = (y as isize + r as isize + 1).clamp(0, h as isize - 1) as usize * w;
    for (offset, band) in bands.iter_mut().enumerate() {
        let out_bin = src[leave + lo + offset] as usize;
        let in_bin = src[enter + lo + offset] as usize;
        band[out_bin] -= 1;
        band[in_bin] += 1;
        let m = mults[offset] as u32;
        x0_hist[out_bin] -= m;
        x0_hist[in_bin] += m;
    }
}

/// How many of the `2r+1` clamped column taps land on `col` for a window
/// centred at `center`.
fn window_multiplicity(col: usize, center: usize, w: usize, r: usize) -> usize {
    if w == 1 {
        return if col == 0 { 2 * r + 1 } else { 0 };
    }
    if col == 0 {
        if center > r {
            0
        } else {
            r - center + 1
        }
    } else if col == w - 1 {
        if center + r < w - 1 {
            0
        } else {
            r + center + 2 - w
        }
    } else if col.abs_diff(center) <= r {
        1
    } else {
        0
    }
}

#[inline]
fn pick(hist: &[u32; BINS], total: usize, target: usize) -> u8 {
    let black = hist[0] as usize;
    let white = hist[255] as usize;
    if black + white == total {
        return if white > target { 255 } else { 0 };
    }
    let mut seen = 0usize;
    for (value, &count) in hist.iter().enumerate() {
        seen += count as usize;
        if seen > target {
            return value as u8;
        }
    }
    255
}
