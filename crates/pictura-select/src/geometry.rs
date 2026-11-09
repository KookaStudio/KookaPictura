//! Marquee rect math and the lasso close-radius test.
//!
//! The Qt shell keeps the event, modifier, style, and cursor policy; this
//! module owns the geometry those decisions produce. The marquee box reuses
//! [`pictura_core::shape::drag_rect`] for Normal / Shift / mirror; Fixed Ratio
//! keeps the old branch so its rounding stays exact.

use pictura_core::shape::drag_rect;

/// The Rectangular and Elliptical Marquee Style option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarqueeStyle {
    Normal,
    FixedRatio,
    FixedSize,
}

impl MarqueeStyle {
    /// The style from its C++ `MarqueeStyle` value (0 Normal, 1 Fixed Ratio,
    /// 2 Fixed Size); an unknown value is `None`.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::FixedRatio),
            2 => Some(Self::FixedSize),
            _ => None,
        }
    }
}

/// Qt's `qRound` rounds half away from zero, as `f64::round` does.
fn round(value: f64) -> i32 {
    value.round() as i32
}

/// The integer rect a marquee drag marks out.
///
/// `from` is the press point and `to` the current pointer. `shift` squares the
/// Normal drag and `mirror` centres it on the press point. Fixed Ratio keeps
/// the entered `(width, height)` ratio inside the non-degenerate drag box;
/// Fixed Size puts the entered integer size's top-left corner at the pointer.
pub fn marquee_rect(
    from: (f64, f64),
    to: (f64, f64),
    shift: bool,
    mirror: bool,
    style: MarqueeStyle,
    ratio: (f64, f64),
    fixed: (i32, i32),
) -> (i32, i32, i32, i32) {
    match style {
        MarqueeStyle::FixedSize => {
            let (width, height) = fixed;
            (round(to.0), round(to.1), width, height)
        }
        MarqueeStyle::FixedRatio => {
            let ratio = ratio.0 / ratio.1;
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            let mut width = dx.abs();
            let mut height = dy.abs();
            if ratio > 0.0 {
                if width <= 0.0 {
                    width = height * ratio;
                } else if height <= 0.0 || width / ratio <= height {
                    height = width / ratio;
                } else {
                    width = height * ratio;
                }
            }
            let left = if dx >= 0.0 {
                round(from.0)
            } else {
                round(from.0 - width)
            };
            let top = if dy >= 0.0 {
                round(from.1)
            } else {
                round(from.1 - height)
            };
            (left, top, round(width), round(height))
        }
        MarqueeStyle::Normal => {
            let (x, y, width, height) = drag_rect(from, to, shift.then_some(1.0), mirror);
            (round(x), round(y), round(width), round(height))
        }
    }
}

/// Whether `b` lies within `radius` of `a` (Euclidean distance, inclusive).
///
/// Squared distances, so the close test costs no square root.
pub fn within_radius(a: (f64, f64), b: (f64, f64), radius: f64) -> bool {
    if radius < 0.0 {
        return false;
    }
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    dx * dx + dy * dy <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    const NORMAL: MarqueeStyle = MarqueeStyle::Normal;

    #[test]
    fn marquee_style_from_i32() {
        assert_eq!(MarqueeStyle::from_i32(0), Some(MarqueeStyle::Normal));
        assert_eq!(MarqueeStyle::from_i32(1), Some(MarqueeStyle::FixedRatio));
        assert_eq!(MarqueeStyle::from_i32(2), Some(MarqueeStyle::FixedSize));
        assert_eq!(MarqueeStyle::from_i32(9), None);
    }

    #[test]
    fn marquee_rect_normal() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 20.0),
                false,
                false,
                NORMAL,
                (0.0, 0.0),
                (0, 0)
            ),
            (10, 10, 20, 10)
        );
        assert_eq!(
            marquee_rect(
                (30.0, 20.0),
                (10.0, 10.0),
                false,
                false,
                NORMAL,
                (0.0, 0.0),
                (0, 0)
            ),
            (10, 10, 20, 10)
        );
    }

    #[test]
    fn marquee_rect_shift_squares() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 20.0),
                true,
                false,
                NORMAL,
                (0.0, 0.0),
                (0, 0)
            ),
            (10, 10, 20, 20)
        );
    }

    #[test]
    fn marquee_rect_mirror() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 20.0),
                false,
                true,
                NORMAL,
                (0.0, 0.0),
                (0, 0)
            ),
            (-10, 0, 40, 20)
        );
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 20.0),
                true,
                true,
                NORMAL,
                (0.0, 0.0),
                (0, 0)
            ),
            (-10, -10, 40, 40)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_portrait() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 50.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (1.0, 2.0),
                (0, 0),
            ),
            (10, 10, 20, 40)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_landscape() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (50.0, 20.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (10, 10, 20, 10)
        );
        assert_eq!(
            marquee_rect(
                (50.0, 50.0),
                (10.0, 10.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (10, 30, 40, 20)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_fits_drag() {
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (40.0, 10.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (0, 0, 20, 10)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_narrow() {
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (10.0, 40.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (0, 0, 10, 5)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_one_axis() {
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (0.0, 40.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (0, 0, 80, 40)
        );
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (40.0, 0.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (0, 0, 40, 20)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_reverse() {
        assert_eq!(
            marquee_rect(
                (50.0, 50.0),
                (10.0, 10.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (10, 30, 40, 20)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_half_pixel() {
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (7.5, 1.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (5.5, 1.0),
                (0, 0),
            ),
            (0, 0, 6, 1)
        );
        assert_eq!(
            marquee_rect(
                (20.0, 0.0),
                (7.5, 1.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (5.5, 1.0),
                (0, 0),
            ),
            (15, 0, 6, 1)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_without_a_ratio() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (30.0, 20.0),
                false,
                false,
                MarqueeStyle::FixedRatio,
                (0.0, 0.0),
                (0, 0),
            ),
            (10, 10, 20, 10)
        );
    }

    #[test]
    fn marquee_rect_fixed_ratio_ignores_modifiers() {
        assert_eq!(
            marquee_rect(
                (0.0, 0.0),
                (40.0, 10.0),
                true,
                true,
                MarqueeStyle::FixedRatio,
                (2.0, 1.0),
                (0, 0),
            ),
            (0, 0, 20, 10)
        );
    }

    #[test]
    fn marquee_rect_fixed_size_follows_the_pointer() {
        assert_eq!(
            marquee_rect(
                (10.4, 20.6),
                (90.0, 90.0),
                false,
                false,
                MarqueeStyle::FixedSize,
                (0.0, 0.0),
                (101, 51),
            ),
            (90, 90, 101, 51)
        );
    }

    #[test]
    fn marquee_rect_fixed_size_ignores_modifiers() {
        assert_eq!(
            marquee_rect(
                (10.0, 10.0),
                (90.0, 90.0),
                true,
                true,
                MarqueeStyle::FixedSize,
                (0.0, 0.0),
                (100, 50),
            ),
            (90, 90, 100, 50)
        );
    }

    #[test]
    fn marquee_rect_rounds_half_away_from_zero() {
        assert_eq!(
            marquee_rect(
                (10.6, 10.4),
                (30.2, 20.5),
                false,
                false,
                NORMAL,
                (0.0, 0.0),
                (0, 0),
            ),
            (11, 10, 20, 10)
        );
        assert_eq!(
            marquee_rect(
                (10.5, 10.5),
                (30.5, 20.5),
                false,
                false,
                NORMAL,
                (0.0, 0.0),
                (0, 0),
            ),
            (11, 11, 20, 10)
        );
    }

    #[test]
    fn within_radius_includes_the_boundary() {
        assert!(within_radius((0.0, 0.0), (3.0, 4.0), 5.0));
        assert!(within_radius((0.0, 0.0), (5.0, 0.0), 5.0));
        assert!(!within_radius((0.0, 0.0), (3.0, 4.0), 4.999));
        assert!(!within_radius((0.0, 0.0), (0.0, 6.0), 5.0));
    }

    #[test]
    fn within_radius_rejects_a_negative_radius() {
        assert!(!within_radius((0.0, 0.0), (0.0, 0.0), -1.0));
    }
}
