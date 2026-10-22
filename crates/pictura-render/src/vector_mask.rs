//! Vector-mask coverage for the compositor.
//!
//! The mask path is document-relative, so sampling happens in document pixels,
//! through the mask's antialiased per-row coverage ([`VectorMask::coverage`]):
//! an edge pixel shows the share of it the path covers, so a slanted or curved
//! shape edge is smooth rather than stair-stepped.

use pictura_core::{PsdRect, VectorMask};

/// Coverage at a canvas pixel: `255` where the layer shows, `0` where the mask
/// hides it. An absent or disabled mask, or one with no closed subpath, is
/// fully permissive. `clip` is the sampled region the row table is bounded to.
pub(crate) fn coverage(mask: Option<&VectorMask>, clip: PsdRect, x: i32, y: i32) -> u8 {
    let Some(mask) = mask else {
        return 255;
    };
    if !mask.has_fill() {
        return 255;
    }
    // D4: union only. Every closed subpath contributes edges to one fill; if
    // any declares non-zero, the whole path is sampled non-zero.
    let c = mask.coverage(x, y, clip);
    if mask.invert {
        255 - c
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{VectorFillRule, VectorSubpath};

    const CLIP: PsdRect = PsdRect {
        top: 0,
        left: 0,
        bottom: 4,
        right: 4,
    };

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> VectorSubpath {
        let p = |x: i32, y: i32| [x * 256, y * 256];
        VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)],
        }
    }

    fn mask(subpaths: Vec<VectorSubpath>) -> VectorMask {
        VectorMask {
            subpaths,
            invert: false,
            disabled: false,
            ..Default::default()
        }
    }

    #[test]
    fn absent_or_disabled_is_permissive() {
        assert_eq!(coverage(None, CLIP, 0, 0), 255);
        let mut m = mask(vec![rect(0, 0, 2, 2)]);
        m.disabled = true;
        assert_eq!(coverage(Some(&m), CLIP, 0, 0), 255);
    }

    #[test]
    fn no_closed_subpath_is_permissive() {
        assert_eq!(coverage(Some(&mask(Vec::new())), CLIP, 0, 0), 255);
        let mut open = rect(0, 0, 2, 2);
        open.closed = false;
        assert_eq!(coverage(Some(&mask(vec![open])), CLIP, 0, 0), 255);
    }

    #[test]
    fn even_odd_clips_inside_the_rectangle() {
        let m = mask(vec![rect(1, 1, 3, 3)]);
        assert_eq!(coverage(Some(&m), CLIP, 1, 1), 255);
        assert_eq!(coverage(Some(&m), CLIP, 2, 2), 255);
        assert_eq!(coverage(Some(&m), CLIP, 0, 0), 0);
        assert_eq!(coverage(Some(&m), CLIP, 3, 3), 0);
    }

    #[test]
    fn invert_flips_the_inside() {
        let mut m = mask(vec![rect(1, 1, 3, 3)]);
        m.invert = true;
        assert_eq!(coverage(Some(&m), CLIP, 1, 1), 0);
        assert_eq!(coverage(Some(&m), CLIP, 0, 0), 255);
    }

    #[test]
    fn an_inner_winding_subpath_cuts_a_hole() {
        // Two same-winding rectangles nest: even-odd leaves the inner one out.
        let m = mask(vec![rect(0, 0, 4, 4), rect(1, 1, 3, 3)]);
        assert_eq!(
            coverage(Some(&m), CLIP, 2, 2),
            0,
            "the inner rectangle is a hole"
        );
        assert_eq!(coverage(Some(&m), CLIP, 0, 0), 255);
    }
}
