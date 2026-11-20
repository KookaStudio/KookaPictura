//! Adaptive homogeneity-directed demosaic for 2×2 Bayer patterns, after
//! Hirakawa & Parks, "Adaptive homogeneity-directed demosaicing algorithm",
//! IEEE Trans. Image Processing 14(3), 2005.
//!
//! Each band of output rows is rebuilt in a padded window mirrored at the image
//! edge (a mirror about a photosite keeps the Bayer phase), interpolated
//! horizontally and vertically, and each pixel takes the direction whose
//! CIELab neighbourhood is more homogeneous.

use rayon::prelude::*;

use super::Mosaic;

const BAND: usize = 32;
/// Green reads ±2, red/blue ±1 more, homogeneity ±1, its 3×3 sum ±1.
const MARGIN: usize = 6;
const MIN_SIZE: usize = 8;

pub(super) fn applies(m: &Mosaic) -> bool {
    let c = &m.cfa.colors;
    let bayer = m.cfa.width == 2
        && m.cfa.height == 2
        && ((c[0] == 1 && c[3] == 1 && c[1] != 1 && c[2] != 1 && c[1] != c[2])
            || (c[1] == 1 && c[2] == 1 && c[0] != 1 && c[3] != 1 && c[0] != c[3]));
    bayer && m.width >= MIN_SIZE && m.height >= MIN_SIZE
}

pub(super) fn demosaic(m: &Mosaic) -> Vec<u16> {
    let mut rgb = vec![0u16; m.width * m.height * 3];
    rgb.par_chunks_mut(m.width * 3 * BAND)
        .enumerate()
        .for_each(|(b, out)| band(m, b * BAND, out));
    rgb
}

fn mirror(i: isize, n: usize) -> usize {
    let last = n as isize - 1;
    let i = if i < 0 { -i } else { i };
    let i = if i > last { 2 * last - i } else { i };
    i.clamp(0, last) as usize
}

/// Linear RGB on the 0–65535 scale to CIELab, treating camera RGB as linear
/// sRGB under D65. ponytail: the camera matrix belongs to the colour stage
/// (#398); here Lab only ranks the two candidates.
fn lab(rgb: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = rgb.map(|v| v / 65535.0);
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |t: f32| {
        const E: f32 = 216.0 / 24389.0;
        if t > E {
            t.cbrt()
        } else {
            t * (841.0 / 108.0) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn chroma2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn band(m: &Mosaic, y0: usize, out: &mut [u16]) {
    let rows = out.len() / (m.width * 3);
    let pw = m.width + 2 * MARGIN;
    let ph = rows + 2 * MARGIN;
    let n = pw * ph;

    let mut raw = vec![0f32; n];
    let mut color = vec![0u8; n];
    for py in 0..ph {
        let sy = mirror((y0 + py) as isize - MARGIN as isize, m.height);
        for px in 0..pw {
            let sx = mirror(px as isize - MARGIN as isize, m.width);
            raw[py * pw + px] = f32::from(m.samples[sy * m.width + sx]);
            color[py * pw + px] = m.color(sx, sy) as u8;
        }
    }
    let inset = |k: usize| (k..ph - k).flat_map(move |y| (k..pw - k).map(move |x| y * pw + x));

    // Per direction (0 horizontal, 1 vertical): green, the RGB candidate, Lab.
    let steps = [1, pw];
    let mut green = [vec![0f32; n], vec![0f32; n]];
    let mut cand = [vec![[0f32; 3]; n], vec![[0f32; 3]; n]];
    let mut labs = [vec![[0f32; 3]; n], vec![[0f32; 3]; n]];
    for d in 0..2 {
        let s = steps[d];
        let g = &mut green[d];
        for i in inset(2) {
            g[i] = if color[i] == 1 {
                raw[i]
            } else {
                let (a, b) = (raw[i - s], raw[i + s]);
                let v = (a + b) / 2.0 + (2.0 * raw[i] - raw[i - 2 * s] - raw[i + 2 * s]) / 4.0;
                // ponytail: the clamp to the adjacent greens is a common guard
                // against overshoot, not part of the paper's filter.
                v.clamp(a.min(b), a.max(b))
            };
        }
        let g = &green[d];
        let diff = |j: usize| raw[j] - g[j];
        for i in inset(3) {
            let mut px = [g[i]; 3];
            let own = color[i] as usize;
            if own == 1 {
                let (h, v) = (
                    (diff(i - 1) + diff(i + 1)) / 2.0,
                    (diff(i - pw) + diff(i + pw)) / 2.0,
                );
                let h_color = color[i - 1] as usize;
                px[h_color] = g[i] + h;
                px[2 - h_color] = g[i] + v;
            } else {
                px[own] = raw[i];
                let diag =
                    diff(i - pw - 1) + diff(i - pw + 1) + diff(i + pw - 1) + diff(i + pw + 1);
                px[2 - own] = g[i] + diag / 4.0;
            }
            cand[d][i] = px;
            labs[d][i] = lab(px);
        }
    }

    let mut homog = [vec![0u8; n], vec![0u8; n]];
    for i in inset(4) {
        let (lh, lv) = (&labs[0], &labs[1]);
        let el = (lh[i][0] - lh[i - 1][0])
            .abs()
            .max((lh[i][0] - lh[i + 1][0]).abs())
            .min(
                (lv[i][0] - lv[i - pw][0])
                    .abs()
                    .max((lv[i][0] - lv[i + pw][0]).abs()),
            );
        let ec = chroma2(lh[i], lh[i - 1])
            .max(chroma2(lh[i], lh[i + 1]))
            .min(chroma2(lv[i], lv[i - pw]).max(chroma2(lv[i], lv[i + pw])));
        for d in 0..2 {
            let l = &labs[d];
            homog[d][i] = [i - 1, i + 1, i - pw, i + pw]
                .into_iter()
                .filter(|&q| (l[i][0] - l[q][0]).abs() <= el && chroma2(l[i], l[q]) <= ec)
                .count() as u8;
        }
    }

    for y in 0..rows {
        for x in 0..m.width {
            let i = (y + MARGIN) * pw + x + MARGIN;
            let sum = |h: &[u8]| -> u32 {
                [i - pw, i, i + pw]
                    .into_iter()
                    .flat_map(|r| [r - 1, r, r + 1])
                    .map(|j| u32::from(h[j]))
                    .sum()
            };
            let (sh, sv) = (sum(&homog[0]), sum(&homog[1]));
            let px = match sh.cmp(&sv) {
                std::cmp::Ordering::Greater => cand[0][i],
                std::cmp::Ordering::Less => cand[1][i],
                std::cmp::Ordering::Equal => {
                    let (a, b) = (cand[0][i], cand[1][i]);
                    [
                        (a[0] + b[0]) / 2.0,
                        (a[1] + b[1]) / 2.0,
                        (a[2] + b[2]) / 2.0,
                    ]
                }
            };
            let o = (y * m.width + x) * 3;
            for c in 0..3 {
                out[o + c] = px[c].round().clamp(0.0, 65535.0) as u16;
            }
        }
    }
}
