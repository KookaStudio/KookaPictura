//! The Refine Edge brush-dab field, split from `refine.rs` to keep each file
//! under the size cap.

/// One refinement-brush dab, in the pixel space of the coverage being refined.
///
/// The app records dabs in document pixels; the preview scales them into the
/// proxy's space before calling the engine. A refine dab widens the edge band
/// locally; an `erase` dab restores the input coverage under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefineDab {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub erase: bool,
}

/// Whether a dab can affect a `w × h` coverage. A dab with a non-finite
/// value, a non-positive radius, a centre outside the coverage, or a radius
/// larger than the coverage is ignored.
fn usable_dab(dab: &RefineDab, w: usize, h: usize) -> bool {
    dab.x.is_finite()
        && dab.y.is_finite()
        && dab.radius.is_finite()
        && dab.radius > 0.0
        && dab.x >= 0.0
        && dab.y >= 0.0
        && dab.x < w as f64
        && dab.y < h as f64
        && dab.radius <= w.max(h) as f64
}

pub(super) fn usable_dabs(dabs: &[RefineDab], w: usize, h: usize) -> Vec<RefineDab> {
    dabs.iter()
        .copied()
        .filter(|d| usable_dab(d, w, h))
        .collect()
}

/// Rasterize ordered dabs into a signed refine/erase field and the local
/// refinement radius. Each dab overwrites the pixels it covers, so the last
/// dab wins (a later smaller refine dab replaces an earlier larger radius).
pub(super) fn brush_field(dabs: &[RefineDab], w: usize, h: usize) -> (Vec<i8>, Vec<u32>) {
    let mut sign = vec![0i8; w * h];
    let mut radius = vec![0u32; w * h];
    for dab in dabs {
        let r = dab.radius;
        let x_lo = ((dab.x - r).floor() as isize).max(0);
        let x_hi = ((dab.x + r).ceil() as isize).min(w as isize - 1);
        let y_lo = ((dab.y - r).floor() as isize).max(0);
        let y_hi = ((dab.y + r).ceil() as isize).min(h as isize - 1);
        if x_lo > x_hi || y_lo > y_hi {
            continue;
        }
        let local = r.round().max(1.0) as u32;
        for y in y_lo..=y_hi {
            for x in x_lo..=x_hi {
                let dx = x as f64 + 0.5 - dab.x;
                let dy = y as f64 + 0.5 - dab.y;
                if dx * dx + dy * dy <= r * r {
                    let i = y as usize * w + x as usize;
                    sign[i] = if dab.erase { -1 } else { 1 };
                    radius[i] = local;
                }
            }
        }
    }
    (sign, radius)
}
