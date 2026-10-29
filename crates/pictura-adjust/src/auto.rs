use pictura_core::PixelBuffer;

use crate::common::{luma, map_lut_plane, map_plane, planes_mut};
use crate::types::{AdjustError, AutoKind};

// ---------------------------------------------------------------------------
// Auto corrections
// ---------------------------------------------------------------------------

pub(crate) fn auto(
    kind: AutoKind,
    buf: &mut PixelBuffer,
    n: usize,
    coverage: Option<&[u8]>,
) -> Result<(), AdjustError> {
    match kind {
        AutoKind::Tone => auto_per_channel(buf, n, 0.001, coverage),
        AutoKind::Contrast => auto_joint(buf, n, 0.005, coverage),
        AutoKind::Color => {
            auto_per_channel(buf, n, 0.005, coverage);
            snap_neutral_midtones(buf, n, coverage);
        }
    }
    Ok(())
}

fn selected(coverage: Option<&[u8]>, i: usize) -> bool {
    coverage.is_none_or(|coverage| coverage.get(i).is_some_and(|&v| v > 0))
}

/// First/last histogram bin that keeps `clip` of the population out.
pub(crate) fn percentile_bounds(hist: &[u64; 256], total: u64, clip: f64) -> Option<(u8, u8)> {
    if total == 0 {
        return None;
    }
    let cut = (total as f64 * clip).floor() as u64;
    let mut lo = 0usize;
    let mut acc = 0u64;
    while lo < 255 {
        if acc + hist[lo] > cut {
            break;
        }
        acc += hist[lo];
        lo += 1;
    }
    let mut hi = 255usize;
    let mut acc = 0u64;
    while hi > 0 {
        if acc + hist[hi] > cut {
            break;
        }
        acc += hist[hi];
        hi -= 1;
    }
    Some((lo as u8, hi as u8))
}

pub(crate) fn stretch_lut(lo: u8, hi: u8) -> [u8; 256] {
    let scale = 255.0 / (hi as f64 - lo as f64);
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        *slot = ((i as f64 - lo as f64) * scale).round().clamp(0.0, 255.0) as u8;
    }
    lut
}

pub(crate) fn auto_per_channel(
    buf: &mut PixelBuffer,
    n: usize,
    clip: f64,
    coverage: Option<&[u8]>,
) {
    let (r, g, b) = planes_mut(buf, n);
    for plane in [r, g, b] {
        let mut hist = [0u64; 256];
        let mut total = 0u64;
        for (i, &v) in plane.iter().enumerate() {
            if selected(coverage, i) {
                hist[v as usize] += 1;
                total += 1;
            }
        }
        if let Some((lo, hi)) = percentile_bounds(&hist, total, clip) {
            if lo < hi {
                let lut = stretch_lut(lo, hi);
                map_lut_plane(plane, &lut);
            }
        }
    }
}

pub(crate) fn auto_joint(buf: &mut PixelBuffer, n: usize, clip: f64, coverage: Option<&[u8]>) {
    let (r, g, b) = planes_mut(buf, n);
    let mut hist = [0u64; 256];
    let mut total = 0u64;
    for plane in [&*r, &*g, &*b] {
        for (i, &v) in plane.iter().enumerate() {
            if selected(coverage, i) {
                hist[v as usize] += 1;
                total += 1;
            }
        }
    }
    if let Some((lo, hi)) = percentile_bounds(&hist, total, clip) {
        if lo < hi {
            let lut = stretch_lut(lo, hi);
            for plane in [r, g, b] {
                map_lut_plane(plane, &lut);
            }
        }
    }
}

/// Approximation of "Find Dark & Light + Snap Neutral Midtones": per-channel
/// gamma brings the average near-neutral midtone to the shared mean.
pub(crate) fn snap_neutral_midtones(buf: &mut PixelBuffer, n: usize, coverage: Option<&[u8]>) {
    let (r, g, b) = planes_mut(buf, n);
    let mut sums = [0.0f64; 3];
    let mut count = 0u64;
    for (i, ((rv, gv), bv)) in r.iter().zip(g.iter()).zip(b.iter()).enumerate() {
        if !selected(coverage, i) {
            continue;
        }
        let y = luma(*rv as f64, *gv as f64, *bv as f64);
        if (64.0..=192.0).contains(&y) {
            sums[0] += *rv as f64;
            sums[1] += *gv as f64;
            sums[2] += *bv as f64;
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    let means = [
        sums[0] / count as f64,
        sums[1] / count as f64,
        sums[2] / count as f64,
    ];
    let target = (means[0] + means[1] + means[2]) / 3.0;
    if target <= 0.0 || target >= 255.0 {
        return;
    }
    for (c, plane) in [r, g, b].into_iter().enumerate() {
        let m = means[c];
        if m <= 0.0 || (m / 255.0).ln() == 0.0 {
            continue;
        }
        let gamma = ((m / 255.0).ln() / (target / 255.0).ln()).clamp(0.1, 9.99);
        map_plane(plane, |v| {
            (255.0 * (v as f64 / 255.0).powf(1.0 / gamma))
                .round()
                .clamp(0.0, 255.0) as u8
        });
    }
}
