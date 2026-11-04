//! Selection edits for a shape known to be zero outside a box: the lassos and
//! marquees know their box, so a document-sized mask is feathered and combined
//! only where the shape can reach.

use pictura_core::PsdRect;
use rayon::prelude::*;

use super::{blur, SelectError, SelectOp, Selection, FEATHER_MAX};

impl Selection {
    /// [`Selection::feather`] of a mask that is zero outside `bounds`, blurring
    /// only `bounds` grown by the kernel's reach; returns the feathered mask
    /// and the box it is zero outside. The result equals the whole-mask
    /// feather: a pixel past the reach reads only zeros, and the clamped
    /// canvas edges stay where they were.
    pub fn feather_within(&self, radius: f64, bounds: PsdRect) -> (Selection, Option<PsdRect>) {
        if !radius.is_finite() || radius <= 0.0 || self.data.is_empty() {
            return (self.clone(), Some(bounds));
        }
        let sigma = radius.clamp(0.0, FEATHER_MAX) / 2.0;
        // One pixel more than the reach, so the window's own clamped edge
        // repeats a zero exactly as the neighbour it stands in for.
        let pad = i32::try_from(blur::reach(sigma) + 1).unwrap_or(i32::MAX);
        let Some(window) = bounds.grow(pad).clip_to(self.width, self.height) else {
            return (Selection::none(self.width, self.height), None);
        };
        let (w, x0) = (self.width as usize, window.left as usize);
        let (ww, wh) = (window.width() as usize, window.height() as usize);
        let rows = window.top as usize..window.bottom as usize;
        let mut src = Vec::with_capacity(ww * wh);
        for y in rows.clone() {
            src.extend_from_slice(&self.data.as_slice()[y * w + x0..y * w + x0 + ww]);
        }
        let blurred = blur::blur(&src, ww, wh, sigma);
        let mut out = Selection::none(self.width, self.height);
        let data = out.data.as_mut_slice();
        for (y, row) in rows.zip(blurred.chunks_exact(ww)) {
            data[y * w + x0..y * w + x0 + ww].copy_from_slice(row);
        }
        (out, Some(window))
    }

    /// [`Selection::combine`] with an `other` that is zero outside `bounds`, so
    /// Add and Subtract visit only `bounds`; Intersect clears the rest, a
    /// fill rather than a per-pixel product.
    pub fn combine_within(
        &mut self,
        other: &Selection,
        op: SelectOp,
        bounds: PsdRect,
    ) -> Result<(), SelectError> {
        if self.width != other.width
            || self.height != other.height
            || self.data.len() != other.data.len()
        {
            return self.combine(other, op);
        }
        if matches!(op, SelectOp::Replace) {
            return self.combine(other, op);
        }
        let w = self.width as usize;
        let window = bounds.clip_to(self.width, self.height);
        let (x0, x1, y0, y1) = window.map_or((0, 0, 0, 0), |r| {
            (
                r.left as usize,
                r.right as usize,
                r.top as usize,
                r.bottom as usize,
            )
        });
        let src = other.data.as_slice();
        let dst = self.data.as_mut_slice();
        let cleared = matches!(op, SelectOp::Intersect);
        dst.par_chunks_exact_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                if !(y0..y1).contains(&y) {
                    if cleared {
                        row.fill(0);
                    }
                    return;
                }
                let theirs = &src[y * w + x0..y * w + x1];
                if cleared {
                    row[..x0].fill(0);
                    row[x1..].fill(0);
                }
                let ours = &mut row[x0..x1];
                match op {
                    SelectOp::Add => {
                        for (a, b) in ours.iter_mut().zip(theirs) {
                            *a = (*a).max(*b);
                        }
                    }
                    SelectOp::Subtract => {
                        for (a, b) in ours.iter_mut().zip(theirs) {
                            *a = a.saturating_sub(*b);
                        }
                    }
                    SelectOp::Intersect => {
                        for (a, b) in ours.iter_mut().zip(theirs) {
                            *a = (((*a as u16) * (*b as u16) + 127) / 255) as u8;
                        }
                    }
                    SelectOp::Replace => unreachable!("replace copies above"),
                }
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shapes() -> Vec<(Selection, PsdRect)> {
        let (w, h) = (220u32, 180u32);
        [
            vec![(60, 50), (150, 70), (110, 140)],
            // Touches the canvas edges, where the blur clamps.
            vec![(-10, -10), (90, 0), (40, 200)],
            vec![(200, 100), (230, 190), (150, 170)],
        ]
        .into_iter()
        .map(|points| {
            let (sel, bounds) = Selection::polygon_bounded(w, h, &points);
            (sel, bounds.expect("each shape covers pixels"))
        })
        .collect()
    }

    #[test]
    fn bounded_feather_equals_the_whole_mask_feather() {
        for (shape, bounds) in shapes() {
            // Both blurs: the direct kernel below radius 4, the box stack above.
            for radius in [0.5, 1.5, 3.0, 4.0, 9.0, 30.0] {
                let (fast, window) = shape.feather_within(radius, bounds);
                assert_eq!(fast, shape.feather(radius), "radius {radius} at {bounds:?}");
                let window = window.expect("a covered shape has a window");
                let outside = fast.bounds().is_none_or(|b| {
                    b.left >= window.left
                        && b.top >= window.top
                        && b.right <= window.right
                        && b.bottom <= window.bottom
                });
                assert!(outside, "radius {radius}: coverage leaves the window");
            }
        }
    }

    #[test]
    fn bounded_combine_equals_the_whole_mask_combine() {
        let (w, h) = (220u32, 180u32);
        let mut base = Selection::rect(w, h, 30, 20, 120, 90);
        base.data[5 * w as usize + 200] = 77;
        for (shape, bounds) in shapes() {
            for op in [
                SelectOp::Replace,
                SelectOp::Add,
                SelectOp::Subtract,
                SelectOp::Intersect,
            ] {
                let mut whole = base.clone();
                whole.combine(&shape, op).unwrap();
                let mut fast = base.clone();
                fast.combine_within(&shape, op, bounds).unwrap();
                assert_eq!(fast, whole, "{op:?} at {bounds:?}");
            }
        }
    }
}
