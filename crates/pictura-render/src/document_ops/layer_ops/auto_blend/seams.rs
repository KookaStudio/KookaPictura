//! Panorama: a seeded watershed over where the layers disagree. Fronts start at
//! pixels only one layer covers and flood the highest disagreement first, so
//! they cross a difference quickly and meet in a low-difference valley.
//! Approximated: Adobe's seam method is unpublished.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use pictura_core::{Document, Layer};

use super::sources::{scan, Bits, Grid, UNLABELLED};
use super::{Cancelled, Control};

const GRADIENT_WEIGHT: f32 = 0.25;
const CONFLICT_SIGMA: f64 = 6.0;

/// The label map and where a transition must stay tight.
pub(super) struct Seams {
    pub ids: Vec<u16>,
    pub conflict: Vec<f32>,
}

pub(super) fn label(
    doc: &Document,
    layers: &[&Layer],
    grid: &Grid,
    luma_gains: &[f32],
    control: &Control,
) -> Result<Seams, Cancelled> {
    let cells = grid.w * grid.h;
    let mut low = vec![f32::INFINITY; cells];
    let mut high = vec![f32::NEG_INFINITY; cells];
    let mut count = vec![0u16; cells];
    let mut covers = Vec::with_capacity(layers.len());
    for (layer, gain) in layers.iter().zip(luma_gains) {
        let source = scan(doc, layer, grid, false, control)?;
        let mut covered = Bits::new(cells);
        for cell in 0..cells {
            if source.alpha[cell] > 0.0 {
                let value = source.luma[cell] * gain;
                covered.set(cell);
                low[cell] = low[cell].min(value);
                high[cell] = high[cell].max(value);
                count[cell] += 1;
            }
        }
        covers.push(covered);
        control.step()?;
    }
    let cost = disagreement(&low, &high, &count, grid);
    let ids = flood(&cost, &count, &covers, grid, control)?;
    let conflict = conflict(&low, &high, &count, grid);
    Ok(Seams { ids, conflict })
}

/// Where layers disagree, spread over the widest transition's reach. A soft
/// edge there would ghost, and the disagreeing content may not be in the merged
/// luma to narrow it.
fn conflict(low: &[f32], high: &[f32], count: &[u16], grid: &Grid) -> Vec<f32> {
    let mut plane: Vec<f64> = (0..count.len())
        .map(|i| {
            if count[i] > 1 {
                f64::from(high[i] - low[i])
            } else {
                0.0
            }
        })
        .collect();
    pictura_core::blur::approx_gaussian(&mut plane, grid.w, grid.h, CONFLICT_SIGMA);
    plane.into_iter().map(|v| v as f32).collect()
}

/// Luma range across the covering layers plus local gradient, quantized for the
/// queue.
fn disagreement(low: &[f32], high: &[f32], count: &[u16], grid: &Grid) -> Vec<u16> {
    let (w, h) = (grid.w, grid.h);
    let mut cost = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            let cell = y * w + x;
            if count[cell] == 0 {
                continue;
            }
            let value = high[cell];
            let step = |other: usize| {
                if count[other] > 0 {
                    (high[other] - value).abs()
                } else {
                    0.0
                }
            };
            let dx = if x + 1 < w { step(cell + 1) } else { 0.0 };
            let dy = if y + 1 < h { step(cell + w) } else { 0.0 };
            let raw = high[cell] - low[cell] + GRADIENT_WEIGHT * (dx + dy);
            cost[cell] = (raw.clamp(0.0, 1.0) * f32::from(u16::MAX)) as u16;
        }
    }
    cost
}

fn flood(
    cost: &[u16],
    count: &[u16],
    covers: &[Bits],
    grid: &Grid,
    control: &Control,
) -> Result<Vec<u16>, Cancelled> {
    let (w, h) = (grid.w, grid.h);
    let mut ids = vec![UNLABELLED; w * h];
    let mut queue: BinaryHeap<(u16, Reverse<u32>, u32, u16)> = BinaryHeap::new();
    let mut order = 0u32;
    let mut grow = |ids: &mut [u16],
                    queue: &mut BinaryHeap<(u16, Reverse<u32>, u32, u16)>,
                    cell: usize,
                    id: u16| {
        let (x, y) = (cell % w, cell / w);
        let neighbours = [
            (x > 0).then(|| cell - 1),
            (x + 1 < w).then(|| cell + 1),
            (y > 0).then(|| cell - w),
            (y + 1 < h).then(|| cell + w),
        ];
        for next in neighbours.into_iter().flatten() {
            if ids[next] == UNLABELLED && covers[id as usize].get(next) {
                order = order.wrapping_add(1);
                queue.push((cost[next], Reverse(order), next as u32, id));
            }
        }
    };
    let top_cover = |cell: usize| (0..covers.len()).rev().find(|&i| covers[i].get(cell));

    for cell in 0..w * h {
        if count[cell] == 1 {
            if let Some(id) = top_cover(cell) {
                ids[cell] = id as u16;
            }
        }
    }
    for cell in 0..w * h {
        let id = ids[cell];
        if count[cell] == 1 && id != UNLABELLED {
            grow(&mut ids, &mut queue, cell, id);
        }
    }
    let mut steps = 0u32;
    // A region no single layer owns has no seed; its topmost layer starts it.
    for start in 0..=w * h {
        while let Some((_, _, cell, id)) = queue.pop() {
            let cell = cell as usize;
            steps = steps.wrapping_add(1);
            if steps.is_multiple_of(1 << 20) {
                control.check()?;
            }
            if ids[cell] == UNLABELLED {
                ids[cell] = id;
                grow(&mut ids, &mut queue, cell, id);
            }
        }
        if start < w * h && ids[start] == UNLABELLED && count[start] > 0 {
            if let Some(id) = top_cover(start) {
                ids[start] = id as u16;
                grow(&mut ids, &mut queue, start, id as u16);
            }
        }
    }
    Ok(ids)
}
