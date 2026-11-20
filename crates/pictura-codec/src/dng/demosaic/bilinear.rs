//! Bilinear demosaic: each missing channel is the mean of the nearest
//! same-colour photosites. On an interior Bayer pixel the radius-1 window is
//! classic bilinear; other repeat patterns widen until the colour appears.

use rayon::prelude::*;

use super::Mosaic;

pub(super) fn demosaic(m: &Mosaic) -> Vec<u16> {
    let max_radius = m.cfa.width.max(m.cfa.height).max(1);
    let mut rgb = vec![0u16; m.width * m.height * 3];
    rgb.par_chunks_mut(m.width * 3)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..m.width {
                let own = m.color(x, y);
                let px = &mut row[x * 3..x * 3 + 3];
                px[own] = m.samples[y * m.width + x];
                for c in (0..3).filter(|&c| c != own) {
                    px[c] = mean_of(m, x, y, c, max_radius);
                }
            }
        });
    rgb
}

/// The rounded mean of colour `c` in the smallest window around `(x, y)`
/// holding one; 0 when the clipped image holds none.
fn mean_of(m: &Mosaic, x: usize, y: usize, c: usize, max_radius: usize) -> u16 {
    for r in 1..=max_radius {
        let (x0, x1) = (x.saturating_sub(r), (x + r).min(m.width - 1));
        let (y0, y1) = (y.saturating_sub(r), (y + r).min(m.height - 1));
        let (mut sum, mut n) = (0u64, 0u64);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                if m.color(xx, yy) == c {
                    sum += u64::from(m.samples[yy * m.width + xx]);
                    n += 1;
                }
            }
        }
        if let Some(mean) = (sum + n / 2).checked_div(n) {
            return mean as u16;
        }
        if x0 == 0 && y0 == 0 && x1 == m.width - 1 && y1 == m.height - 1 {
            break;
        }
    }
    0
}
