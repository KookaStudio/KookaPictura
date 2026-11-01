//! Stack Images: each cell shows the layer scoring best on focus and exposure.
//! The score multiplies Laplacian energy by a Mertens well-exposedness term, so
//! one rule serves focus and exposure stacks. Approximated: Adobe's metric is
//! unpublished.

use pictura_core::{Document, Layer};

use super::sources::{scan, Bits, Grid, UNLABELLED};
use super::{Cancelled, Control};

/// Focus below this is noise, leaving exposure to decide.
const FOCUS_FLOOR: f64 = 4e-3;
/// Focus regions span at least this, and this share of the short side, so a
/// stack blends by area and not by speckle.
const FOCUS_SIGMA: f64 = 4.0;
const FOCUS_SIGMA_SHARE: f64 = 1.0 / 128.0;
/// Luma this far from the ends counts as well exposed; beyond it the score
/// falls to zero, so only blown or crushed areas lose, not bright flat sky.
const EXPOSURE_MARGIN: f32 = 0.06;
/// A higher layer must beat the best score by this factor. Flat or out-of-focus
/// areas score alike, and they then stay with the lowest layer.
const WIN_MARGIN: f64 = 1.3;
/// A higher layer with real detail needs only this share of the best score, so
/// fine texture below it such as spray does not erase its subject.
const DETAIL_FOCUS: f64 = 0.01;
const DETAIL_SHARE: f64 = 0.6;

pub(super) fn label(
    doc: &Document,
    layers: &[&Layer],
    grid: &Grid,
    control: &Control,
) -> Result<Vec<u16>, Cancelled> {
    let cells = grid.w * grid.h;
    let mut ids = vec![UNLABELLED; cells];
    let mut best = vec![f64::NEG_INFINITY; cells];
    let mut covers = Vec::with_capacity(layers.len());
    let sigma = FOCUS_SIGMA.max(grid.w.min(grid.h) as f64 * FOCUS_SIGMA_SHARE);
    for (index, layer) in layers.iter().enumerate() {
        let source = scan(doc, layer, grid, true, control)?;
        let mut focus: Vec<f64> = source.energy.iter().map(|&e| f64::from(e)).collect();
        pictura_core::blur::approx_gaussian(&mut focus, grid.w, grid.h, sigma);
        let mut covered = Bits::new(cells);
        for cell in 0..cells {
            if source.alpha[cell] <= 0.0 {
                continue;
            }
            covered.set(cell);
            let luma = source.luma[cell];
            let exposure = (luma.min(1.0 - luma) / EXPOSURE_MARGIN).clamp(0.0, 1.0);
            let score = (focus[cell] + FOCUS_FLOOR).sqrt() * f64::from(exposure.max(0.01));
            let detailed = focus[cell] > DETAIL_FOCUS && score > best[cell] * DETAIL_SHARE;
            if ids[cell] == UNLABELLED || score > best[cell] * WIN_MARGIN || detailed {
                best[cell] = score;
                ids[cell] = index as u16;
            }
        }
        covers.push(covered);
        control.step()?;
    }
    Ok(despeckle(&ids, &covers, grid))
}

/// A cell almost no neighbour agrees with takes its neighbours' majority.
fn despeckle(ids: &[u16], covers: &[Bits], grid: &Grid) -> Vec<u16> {
    let (w, h) = (grid.w, grid.h);
    let mut out = ids.to_vec();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let cell = y * w + x;
            let around = [
                cell - w - 1,
                cell - w,
                cell - w + 1,
                cell - 1,
                cell + 1,
                cell + w - 1,
                cell + w,
                cell + w + 1,
            ]
            .map(|i| ids[i]);
            if around.iter().filter(|&&id| id == ids[cell]).count() > 1 {
                continue;
            }
            let winner = around
                .iter()
                .copied()
                .filter(|&id| id != UNLABELLED && covers[id as usize].get(cell))
                .max_by_key(|&id| (around.iter().filter(|&&n| n == id).count(), id));
            if let Some(id) = winner {
                out[cell] = id;
            }
        }
    }
    out
}
