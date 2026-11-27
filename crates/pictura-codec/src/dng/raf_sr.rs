//! SuperCCD SR RAFs (#1648): the FinePix S3 Pro, S5 Pro and F700 store a
//! second, low-sensitivity (R) frame beside the primary (S) one. Both are
//! repacked onto the same grid and merged into extended highlight range: S
//! below its clip, R scaled by a fitted S/R ratio above it. An approximation
//! of our own: Fujifilm's and Adobe's merges are closed and LibRaw reads one
//! frame at a time.

use rayon::prelude::*;

use super::levels::Levels;
use super::CfaPattern;

/// An R sample the file does not hold (the F700 ends before its last R
/// row). No word section reaches it: their white is under 16 bits.
pub(super) const MISSING: u16 = u16::MAX;

/// The S range, as a fraction of white less black, whose samples fit the
/// ratio.
const FIT_BAND: (f64, f64) = (0.25, 0.75);
/// R samples below this many counts above black are too noisy to fit.
const FIT_MIN_R: f64 = 16.0;
/// Fewer fitted samples than this fall back to the nominal ratio.
const FIT_MIN_SAMPLES: usize = 256;
/// The fit reads at most about this many samples, at a fixed stride.
const FIT_SAMPLES: usize = 1 << 21;
/// S is kept below the first, R taken from the second, linear between.
const BLEND: (f64, f64) = (0.80, 0.95);
/// The S/R ratio of every SR body when the records do not name one.
pub(super) const NOMINAL_RATIO: f64 = 16.0;

/// The merged grid and the stops of headroom it was scaled down by.
pub(super) struct Merged {
    pub samples: Vec<u16>,
    pub stops: f64,
}

struct Frames<'a> {
    s: &'a [u16],
    r: &'a [u16],
    width: usize,
    s_levels: &'a Levels,
    r_levels: &'a Levels,
    cfa: &'a CfaPattern,
}

fn black(levels: &Levels, x: usize, y: usize) -> f64 {
    levels.black[(y % levels.black_rows) * levels.black_cols + x % levels.black_cols]
}

impl Frames<'_> {
    /// Sample `i`'s CFA colour, black-subtracted S, S range and
    /// black-subtracted R (`None` when missing), in counts.
    fn at(&self, i: usize) -> (usize, f64, f64, Option<f64>) {
        let (x, y) = (i % self.width, i / self.width);
        let colour = self.cfa.colors[(y % self.cfa.height) * self.cfa.width + x % self.cfa.width];
        let s_black = black(self.s_levels, x, y);
        let s = f64::from(self.s[i]) - s_black;
        let r = (self.r[i] != MISSING).then(|| f64::from(self.r[i]) - black(self.r_levels, x, y));
        (usize::from(colour), s, self.s_levels.white - s_black, r)
    }

    /// The median S/R per CFA colour over the fit band.
    fn ratios(&self, nominal: f64) -> [f64; 3] {
        let step = self.s.len().div_ceil(FIT_SAMPLES).max(1);
        let mut by_colour: [Vec<f64>; 3] = Default::default();
        for i in (0..self.s.len()).step_by(step) {
            let (c, s, range, r) = self.at(i);
            let Some(r) = r else { continue };
            if range > 0.0 && r >= FIT_MIN_R && (FIT_BAND.0..=FIT_BAND.1).contains(&(s / range)) {
                by_colour[c.min(2)].push(s / r);
            }
        }
        by_colour.map(|mut v| {
            if v.len() < FIT_MIN_SAMPLES {
                return nominal;
            }
            let mid = v.len() / 2;
            *v.select_nth_unstable_by(mid, f64::total_cmp).1
        })
    }

    /// Sample `i` merged, in S counts above black, and its S range.
    fn merged(&self, i: usize, k: &[f64; 3]) -> (f64, f64) {
        let (c, s, range, r) = self.at(i);
        let Some(r) = r.filter(|_| range > 0.0) else {
            return (s, range);
        };
        let t = ((s / range - BLEND.0) / (BLEND.1 - BLEND.0)).clamp(0.0, 1.0);
        if t == 0.0 {
            return (s, range);
        }
        let m = s + (k[c.min(2)] * r - s) * t;
        // A clipped S sample is a lower bound; the R photosite beside it
        // can read darker.
        (if s >= range { m.max(s) } else { m }, range)
    }
}

/// Merges the S grid `s` with the R grid `r` (same size and CFA) into
/// samples normalized over the scene's headroom. `nominal` is the ratio
/// used for a colour with too few samples to fit.
pub(super) fn merge(
    (s, s_levels): (&[u16], &Levels),
    (r, r_levels): (&[u16], &Levels),
    width: usize,
    cfa: &CfaPattern,
    nominal: f64,
) -> Merged {
    let frames = Frames {
        s,
        r,
        width,
        s_levels,
        r_levels,
        cfa,
    };
    let k = frames.ratios(nominal);
    let headroom = (0..s.len())
        .into_par_iter()
        .map(|i| match frames.merged(i, &k) {
            (m, range) if range > 0.0 => m / range,
            _ => 0.0,
        })
        .reduce(|| 1.0, f64::max);
    let mut samples = vec![0u16; s.len()];
    samples.par_iter_mut().enumerate().for_each(|(i, out)| {
        let (m, range) = frames.merged(i, &k);
        // As `levels::normalize` computes it, so headroom 1 changes nothing.
        let n = if range > 0.0 {
            (m * 65535.0 / range / headroom).round()
        } else if m >= range {
            65535.0
        } else {
            0.0
        };
        *out = n.clamp(0.0, 65535.0) as u16;
    });
    Merged {
        samples,
        stops: headroom.log2(),
    }
}
