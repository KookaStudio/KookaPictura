//! Shared colour-range weighting for the HSL and Camera Calibration stages.

/// ACR's eight HSL hue ranges, in on-disk order (Reds … Magentas).
pub(crate) const HUE_CENTERS: [f64; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 275.0, 300.0];

/// Circular hue distance in degrees.
fn hue_dist(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Triangular window: 1 within `SUSTAIN` degrees of `center`, ramping to 0 at
/// `SUSTAIN + RAMP`. ponytail: the reference's exact band boundaries are closed.
pub(crate) fn hue_weight(h: f64, center: f64) -> f64 {
    const SUSTAIN: f64 = 20.0;
    const RAMP: f64 = 30.0;
    let d = hue_dist(h, center);
    if d <= SUSTAIN {
        1.0
    } else if d < SUSTAIN + RAMP {
        (SUSTAIN + RAMP - d) / RAMP
    } else {
        0.0
    }
}
