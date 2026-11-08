//! Selection edits for a shape known to be zero outside a box: the lassos and
//! marquees know their box, and the box the result is zero outside follows
//! from it.

use pictura_core::PsdRect;

use super::{blur, Selection, FEATHER_MAX};

impl Selection {
    /// [`Selection::feather`] of a mask that is zero outside `bounds`; returns
    /// the feathered mask and the box it is zero outside (`bounds` grown by
    /// the kernel's reach). The tiled feather already skips every tile whose
    /// neighbourhood is empty, so the box costs nothing extra (#1214).
    pub fn feather_within(&self, radius: f64, bounds: PsdRect) -> (Selection, Option<PsdRect>) {
        if !radius.is_finite() || radius <= 0.0 || self.data.is_empty() {
            return (self.clone(), Some(bounds));
        }
        let sigma = radius.clamp(0.0, FEATHER_MAX) / 2.0;
        let pad = i32::try_from(blur::reach(sigma) + 1).unwrap_or(i32::MAX);
        let Some(window) = bounds.grow(pad).clip_to(self.width, self.height) else {
            return (Selection::none(self.width, self.height), None);
        };
        (self.feather(radius), Some(window))
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
}
