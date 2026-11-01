//! Soft label edges. A single mask cannot hold true multiband blending, so this
//! approximates it: a label's Gaussian pyramid is combined by each location's
//! share of band energy, giving wide transitions in smooth areas and tight ones
//! at detail.

use rayon::prelude::*;

const LEVELS: usize = 5;

/// Band energy that always belongs to the coarsest level, so a flat area takes
/// the widest transition.
const SMOOTH_FLOOR: f32 = 0.002;

struct Level {
    w: usize,
    h: usize,
    data: Vec<f32>,
}

impl Level {
    /// A 2×2 box reduction.
    fn reduce(&self) -> Level {
        let (w, h) = (self.w.div_ceil(2), self.h.div_ceil(2));
        let mut data = vec![0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let (x0, y0) = (x * 2, y * 2);
                let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
                data[y * w + x] = (self.data[y0 * self.w + x0]
                    + self.data[y0 * self.w + x1]
                    + self.data[y1 * self.w + x0]
                    + self.data[y1 * self.w + x1])
                    / 4.0;
            }
        }
        Level { w, h, data }
    }

    /// Bilinear sample where `(x, y)` is a cell of the finest level and
    /// `scale` is that level's cells per cell of this one.
    #[inline]
    fn sample(&self, x: usize, y: usize, scale: usize) -> f32 {
        let f = scale as f32;
        let u = ((x as f32 + 0.5) / f - 0.5).clamp(0.0, (self.w - 1) as f32);
        let v = ((y as f32 + 0.5) / f - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (u as usize, v as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (a, b) = (u - x0 as f32, v - y0 as f32);
        let at = |x: usize, y: usize| self.data[y * self.w + x];
        let top = at(x0, y0) * (1.0 - a) + at(x1, y0) * a;
        let bottom = at(x0, y1) * (1.0 - a) + at(x1, y1) * a;
        top * (1.0 - b) + bottom * b
    }
}

fn pyramid(data: Vec<f32>, w: usize, h: usize, count: usize) -> Vec<Level> {
    let mut levels = vec![Level { w, h, data }];
    while levels.len() < count {
        let next = levels[levels.len() - 1].reduce();
        levels.push(next);
    }
    levels
}

/// Local band energy of the blended luma.
pub(super) struct Bands {
    w: usize,
    h: usize,
    energy: Vec<Level>,
}

impl Bands {
    /// `conflict` adds to the finest band, tightening transitions there.
    pub fn new(luma: &[f32], conflict: Option<&[f32]>, w: usize, h: usize) -> Bands {
        let count = LEVELS.min((w.min(h).max(1).ilog2() as usize).max(1));
        let gaussian = pyramid(luma.to_vec(), w, h, count);
        let mut energy: Vec<Level> = gaussian
            .windows(2)
            .map(|pair| {
                let (fine, coarse) = (&pair[0], &pair[1]);
                let data = (0..fine.w * fine.h)
                    .map(|i| (fine.data[i] - coarse.sample(i % fine.w, i / fine.w, 2)).abs())
                    .collect();
                Level {
                    w: fine.w,
                    h: fine.h,
                    data,
                }
            })
            .collect();
        if let (Some(conflict), Some(finest)) = (conflict, energy.first_mut()) {
            for (value, extra) in finest.data.iter_mut().zip(conflict) {
                *value += extra;
            }
        }
        Bands { w, h, energy }
    }

    /// Soften a one-hot label plane.
    pub fn soften(&self, hot: &[f32]) -> Vec<f32> {
        let labels = pyramid(hot.to_vec(), self.w, self.h, self.energy.len() + 1);
        let w = self.w;
        let mut out = vec![0f32; w * self.h];
        out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, value) in row.iter_mut().enumerate() {
                let mut total = SMOOTH_FLOOR;
                let mut sum =
                    SMOOTH_FLOOR * labels[labels.len() - 1].sample(x, y, 1 << (labels.len() - 1));
                for (k, band) in self.energy.iter().enumerate() {
                    let share = band.sample(x, y, 1 << k);
                    total += share;
                    sum += share * labels[k].sample(x, y, 1 << k);
                }
                *value = sum / total;
            }
        });
        out
    }
}
