//! Geometric distortions — Filter ▸ Distort.
//!
//! These are a different shape of operation from the convolutions next door.
//! A blur asks "what is around this pixel"; a distortion asks "where did this
//! pixel come from", moves the whole picture about, and reads the source at a
//! position that almost never lands on a whole pixel. So they all share one
//! back end, [`remap`], and differ only in the function that answers that
//! question.
//!
//! Working backwards — for each destination pixel, find its source — rather
//! than forwards is what keeps the result free of holes. Pushing pixels
//! outwards leaves gaps between them wherever the distortion stretches.

use crate::photorust::pixmap::bilinear;
use crate::photorust::pixmap::Pixmap;
pub(crate) use crate::{RippleSize, SpherizeMode, WaveType as WaveKind, ZigZagStyle};
use rayon::prelude::*;

/// What to do when a distortion reaches for a pixel that is not there.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EdgeMode {
    /// Carry the edge pixel outwards — CS6's "Repeat Edge Pixels".
    #[default]
    Clamp,
    /// Come back in the other side — CS6's "Wrap Around".
    Wrap,
}

impl RippleSize {
    /// Distance between crests, in pixels.
    fn wavelength(self) -> f32 {
        match self {
            RippleSize::Small => 10.0,
            RippleSize::Medium => 22.0,
            RippleSize::Large => 46.0,
        }
    }
}

/// Rebuild `pixmap` by asking `source_of` where each destination pixel came
/// from, in pixel coordinates.
///
/// Sampled bilinearly, because a distortion lands between pixels nearly
/// everywhere and rounding to the nearest one leaves the jagged staircase
/// along every curve that gives cheap warps away.
pub(super) fn remap<F>(pixmap: &mut Pixmap, edge: EdgeMode, source_of: F)
where
    F: Fn(f32, f32) -> (f32, f32) + Sync,
{
    if pixmap.is_empty() {
        return;
    }
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;

    // Premultiplied, so that a sample straddling a transparent edge does not
    // drag the colour of invisible pixels into visible ones.
    pixmap.premultiply();
    let source = pixmap.clone();
    let src = &source;
    let stride = pixmap.stride();

    pixmap
        .as_bytes_mut()
        .par_chunks_exact_mut(stride)
        .enumerate()
        .for_each(|(row, out)| {
            let y = row as i32;
            for x in 0..width {
                // The centre of the pixel, not its corner: a distortion that
                // works from corners is half a pixel out everywhere.
                let (mut sx, mut sy) = source_of(x as f32 + 0.5, y as f32 + 0.5);
                let i = x as usize * 4;

                match edge {
                    EdgeMode::Wrap => {
                        sx = sx.rem_euclid(width as f32);
                        sy = sy.rem_euclid(height as f32);
                    }
                    // `bilinear` already clamps.
                    EdgeMode::Clamp => {}
                }

                let p = bilinear(src, sx - 0.5, sy - 0.5);
                out[i] = p.r;
                out[i + 1] = p.g;
                out[i + 2] = p.b;
                out[i + 3] = p.a;
            }
        });

    pixmap.unpremultiply();
}

/// Filter ▸ Distort ▸ Pinch.
///
/// `amount` is a percentage from -100 to 100. Positive squeezes the middle in,
/// negative pushes it out like a bubble.
///
/// The effect is confined to the ellipse inscribed in the image, and both the
/// centre and the rim of it stay put — that is what lets it be applied without
/// the corners tearing away from it.
///
/// In between, the displacement follows `r(1 - r²)`. The shape of that falloff
/// is not decoration: the map has to stay **monotonic**, because the moment
/// two destination radii want the same source radius the picture folds over
/// itself and leaves a dark crease. A sine falloff, which is the obvious
/// choice, turns over at the rim once the amount passes about two thirds and
/// creases exactly there. This one's slope is `1 + s(1 - 3r²)`, whose worst
/// value is `1 - 2s` at the rim, so a strength below a half cannot fold
/// whichever way it is pointed.
pub fn pinch(pixmap: &mut Pixmap, amount: f32) {
    let strength = pinch_strength(amount);
    if strength == 0.0 || pixmap.is_empty() {
        return;
    }
    let half_w = pixmap.width() as f32 / 2.0;
    let half_h = pixmap.height() as f32 / 2.0;

    remap(pixmap, EdgeMode::Clamp, move |x, y| {
        let dx = (x - half_w) / half_w;
        let dy = (y - half_h) / half_h;
        let r = (dx * dx + dy * dy).sqrt();
        if r >= 1.0 || r <= 0.0 {
            return (x, y);
        }
        // Sampling further out than we are drawing pulls the picture inwards,
        // which is the pinch; sampling nearer the middle magnifies it.
        let k = pinch_map(r, strength) / r;
        (half_w + dx * k * half_w, half_h + dy * k * half_h)
    });
}

/// The radius a pinch reads from, for a destination at radius `r`.
///
/// Both the filter and the wireframe it is drawn with go through this, so the
/// diagram in the dialog cannot drift away from what OK will do.
fn pinch_map(r: f32, strength: f32) -> f32 {
    r + strength * r * (1.0 - r * r)
}

fn pinch_strength(amount: f32) -> f32 {
    (amount.clamp(-100.0, 100.0) / 100.0) * 0.4
}

/// Filter ▸ Distort ▸ Polar Coordinates.
///
/// `to_polar` is CS6's "Rectangular to Polar", which wraps the picture into a
/// disc: the top edge becomes the centre, the bottom edge the outer rim, and
/// the left and right edges meet. False runs it the other way and unrolls a
/// disc back into a rectangle.
///
/// Unrolling reaches past the edge of the picture wherever the ray it is
/// following leaves the frame before it has run its full length — everywhere
/// except the four diagonals, since a rectangle is not a disc. Those pixels
/// take the nearest edge pixel, which is what streaks the bottom of an
/// unrolled image vertically; Photoshop does the same. Leaving them empty
/// instead punches scalloped transparent bites out of the result.
pub fn polar_coordinates(pixmap: &mut Pixmap, to_polar: bool) {
    if pixmap.is_empty() {
        return;
    }
    let width = pixmap.width() as f32;
    let height = pixmap.height() as f32;
    let cx = width / 2.0;
    let cy = height / 2.0;
    // Out to the corner, so that the whole frame is accounted for.
    let max_radius = (cx * cx + cy * cy).sqrt();
    let tau = std::f32::consts::TAU;

    if to_polar {
        remap(pixmap, EdgeMode::Clamp, move |x, y| {
            let dx = x - cx;
            let dy = y - cy;
            let radius = (dx * dx + dy * dy).sqrt();
            // Measured from straight up and going clockwise, so that the seam
            // where the left and right edges meet falls at the top.
            let angle = dx.atan2(-dy).rem_euclid(tau);
            (angle / tau * width, radius / max_radius * height)
        });
    } else {
        remap(pixmap, EdgeMode::Clamp, move |x, y| {
            let angle = x / width * tau;
            let radius = y / height * max_radius;
            (cx + radius * angle.sin(), cy - radius * angle.cos())
        });
    }
}

/// Filter ▸ Distort ▸ Shear.
///
/// Each row of the image is pushed sideways by an amount read off a curve —
/// `curve` is that curve as `(position, offset)` control points top to bottom,
/// with `offset` in fractions of half the image's width. All zeroes is a
/// straight line down the middle and does nothing.
///
/// The curve stays `f64` through the interpolation: the caller validates the
/// strictly-increasing positions at that width, and a lossy cast to `f32`
/// could collapse two of them onto one zero-width span.
pub fn shear(pixmap: &mut Pixmap, curve: &[(f64, f64)], wrap: bool) {
    // The all-zero shortcut keeps a straight curve bit-exact on translucent
    // pixels, where the premultiplied round trip in `remap` would not be.
    if pixmap.is_empty() || curve.iter().all(|&(_, offset)| offset == 0.0) {
        return;
    }
    let height = pixmap.height() as f32;
    let half_w = pixmap.width() as f32 / 2.0;
    let edge = if wrap {
        EdgeMode::Wrap
    } else {
        EdgeMode::Clamp
    };

    remap(pixmap, edge, move |x, y| {
        // The pixel centre of the top row reads the curve at -1 and of the
        // bottom row at 1; between them the curve is a smooth line.
        let position = if height > 1.0 {
            f64::from((y - 0.5) / (height - 1.0) * 2.0 - 1.0)
        } else {
            0.0
        };
        (x - curve_at(curve, position) as f32 * half_w, y)
    });
}

/// Smooth `offset` at `position`, clamped outside the control span.
///
/// A cubic Hermite through the control points with Catmull-Rom tangents, so
/// the line bends rather than kinks where the points are, the way CS6's curve
/// does. With two points both tangents are the secant, which makes the curve
/// exactly the straight line between them.
fn curve_at(curve: &[(f64, f64)], position: f64) -> f64 {
    let first = curve[0];
    let last = curve[curve.len() - 1];
    if position <= first.0 {
        return first.1;
    }
    if position >= last.0 {
        return last.1;
    }
    let mut i = 0;
    while i + 2 < curve.len() && position > curve[i + 1].0 {
        i += 1;
    }
    let (p1, o1) = curve[i];
    let (p2, o2) = curve[i + 1];
    let (p0, o0) = curve[i.saturating_sub(1)];
    let (p3, o3) = curve[(i + 2).min(curve.len() - 1)];
    let secant = (o2 - o1) / (p2 - p1);
    // The ends get the adjacent secant, which both clamps the ends and makes a
    // two-point curve a line.
    let slope1 = if i == 0 {
        secant
    } else {
        (o2 - o0) / (p2 - p0)
    };
    let slope2 = if i + 2 >= curve.len() {
        secant
    } else {
        (o3 - o1) / (p3 - p1)
    };
    hermite(o1, slope1, o2, slope2, p2 - p1, (position - p1) / (p2 - p1))
}

/// The cubic Hermite blend over one segment.
fn hermite(o1: f64, slope1: f64, o2: f64, slope2: f64, span: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    (2.0 * t3 - 3.0 * t2 + 1.0) * o1
        + (t3 - 2.0 * t2 + t) * span * slope1
        + (-2.0 * t3 + 3.0 * t2) * o2
        + (t3 - t2) * span * slope2
}

/// Filter ▸ Distort ▸ Ripple.
///
/// `amount` is a percentage from -999 to 999 and `size` sets how far apart the
/// crests are.
///
/// Two waves of different length are added rather than one, because a single
/// sine gives a regular corrugation that reads as a machine, and CS6's ripple
/// is irregular the way water is.
pub fn ripple(pixmap: &mut Pixmap, amount: f32, size: RippleSize) {
    let wavelength = size.wavelength();
    let amplitude = wavelength * (amount.clamp(-999.0, 999.0) / 1000.0) * 1.2;
    if amplitude == 0.0 || pixmap.is_empty() {
        return;
    }
    let k = std::f32::consts::TAU / wavelength;

    remap(pixmap, EdgeMode::Clamp, move |x, y| {
        // Each axis is displaced by what the other axis is doing, so the
        // ripple runs across the picture rather than along it.
        let dx = amplitude * ((y * k).sin() * 0.7 + (y * k * 2.3 + 1.7).sin() * 0.3);
        let dy = amplitude * ((x * k).sin() * 0.7 + (x * k * 1.9 + 0.6).sin() * 0.3);
        (x + dx, y + dy)
    });
}

/// The radius a spherize reads from, for a destination at radius `r`.
///
/// Wrapping the picture onto a ball means reading it from `asin`, which is
/// what makes the middle swell; pushing it into a bowl reads from `sin`, which
/// is the same curve the other way up. Blended against the identity by the
/// amount, so both ends of the slider are smooth and both fixed points — the
/// centre and the rim — hold whatever it is set to.
fn spherize_map(r: f32, amount: f32) -> f32 {
    let a = amount.clamp(-100.0, 100.0) / 100.0;
    let curved = if a >= 0.0 {
        r.clamp(-1.0, 1.0).asin() * std::f32::consts::FRAC_2_PI
    } else {
        (r * std::f32::consts::FRAC_PI_2).sin()
    };
    r * (1.0 - a.abs()) + curved * a.abs()
}

/// Filter ▸ Distort ▸ Spherize.
///
/// `amount` is a percentage from -100 to 100: positive wraps the picture onto
/// a ball, negative presses it into a bowl. `mode` confines it to one axis, as
/// CS6's Horizontal Only and Vertical Only do, which bulges a column or a band
/// rather than a disc.
pub fn spherize(pixmap: &mut Pixmap, amount: f32, mode: SpherizeMode) {
    if amount == 0.0 || pixmap.is_empty() {
        return;
    }
    let half_w = pixmap.width() as f32 / 2.0;
    let half_h = pixmap.height() as f32 / 2.0;

    remap(pixmap, EdgeMode::Clamp, move |x, y| {
        let dx = (x - half_w) / half_w;
        let dy = (y - half_h) / half_h;
        match mode {
            // One axis only, so the distance that matters is along it alone.
            SpherizeMode::HorizontalOnly => {
                let r = dx.abs();
                if r <= 0.0 || r >= 1.0 {
                    return (x, y);
                }
                (half_w + dx.signum() * spherize_map(r, amount) * half_w, y)
            }
            SpherizeMode::VerticalOnly => {
                let r = dy.abs();
                if r <= 0.0 || r >= 1.0 {
                    return (x, y);
                }
                (x, half_h + dy.signum() * spherize_map(r, amount) * half_h)
            }
            SpherizeMode::Normal => {
                let r = (dx * dx + dy * dy).sqrt();
                if r <= 0.0 || r >= 1.0 {
                    return (x, y);
                }
                let k = spherize_map(r, amount) / r;
                (half_w + dx * k * half_w, half_h + dy * k * half_h)
            }
        }
    });
}

/// How far a twirl turns the picture at radius `r`, in radians.
///
/// The turn dies away as a square rather than a straight line so that the rim
/// is not merely unmoved but unstressed: a linear falloff still has the
/// picture shearing as it reaches the edge, and that shows as a crease around
/// the circle.
fn twirl_turn(r: f32, angle: f32) -> f32 {
    let fade = 1.0 - r;
    angle.to_radians() * fade * fade
}

/// Filter ▸ Distort ▸ Twirl.
///
/// `angle` is in degrees, -999 to 999. The picture is rotated about the middle
/// by more the nearer the middle it is, which winds it into a spiral.
pub fn twirl(pixmap: &mut Pixmap, angle: f32) {
    if angle == 0.0 || pixmap.is_empty() {
        return;
    }
    let half_w = pixmap.width() as f32 / 2.0;
    let half_h = pixmap.height() as f32 / 2.0;

    remap(pixmap, EdgeMode::Clamp, move |x, y| {
        let dx = (x - half_w) / half_w;
        let dy = (y - half_h) / half_h;
        let r = (dx * dx + dy * dy).sqrt();
        if r >= 1.0 || r <= 0.0 {
            return (x, y);
        }
        // Turned the other way, because this is asking where the pixel came
        // from rather than where it is going.
        let turn = -twirl_turn(r, angle);
        let (sin, cos) = turn.sin_cos();
        let rx = dx * cos - dy * sin;
        let ry = dx * sin + dy * cos;
        (half_w + rx * half_w, half_h + ry * half_h)
    });
}

impl WaveKind {
    /// One cycle over a phase in turns, giving -1 to 1.
    fn at(self, turns: f32) -> f32 {
        let t = turns.rem_euclid(1.0);
        match self {
            WaveKind::Sine => (t * std::f32::consts::TAU).sin(),
            // Up for the first half, down for the second.
            WaveKind::Triangle => {
                if t < 0.5 {
                    t * 4.0 - 1.0
                } else {
                    3.0 - t * 4.0
                }
            }
            WaveKind::Square => {
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

/// A hash in 0..1, so that a seed reproduces the same wave every time.
///
/// Wave is the one filter with a Randomize button, and a button that re-rolls
/// has to be able to roll the same thing twice — otherwise the preview would
/// show one wave and OK would apply another, and undo and redo would each
/// bring back a different picture.
fn seeded(seed: u32, salt: u32) -> f32 {
    let mut h = seed.wrapping_mul(0x9E3779B1) ^ salt.wrapping_mul(0x85EBCA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545F491);
    h ^= h >> 13;
    (h % 10007) as f32 / 10007.0
}

/// Filter ▸ Distort ▸ Wave.
///
/// A sum of `generators` waves, each with its own wavelength, amplitude and
/// phase drawn from the given ranges. `scale` is a percentage per axis, and
/// `wrap` is CS6's Undefined Areas choice.
///
/// The generators are averaged rather than added, so that turning the number
/// of them up changes the character of the wave — from a clean swell to
/// something choppy — instead of simply making it bigger, which is what the
/// Amplitude slider is for.
#[allow(clippy::too_many_arguments)]
pub fn wave(
    pixmap: &mut Pixmap,
    generators: u32,
    wavelength: (f32, f32),
    amplitude: (f32, f32),
    scale: (f32, f32),
    kind: WaveKind,
    wrap: bool,
    seed: u32,
) {
    let count = generators.clamp(1, 999);
    if pixmap.is_empty() || (amplitude.0 == 0.0 && amplitude.1 == 0.0) {
        return;
    }
    let (wave_low, wave_high) = (
        wavelength.0.min(wavelength.1).max(1.0),
        wavelength.0.max(wavelength.1).max(1.0),
    );
    let (amp_low, amp_high) = (amplitude.0.min(amplitude.1), amplitude.0.max(amplitude.1));

    // Each generator's wavelength, amplitude and phase, for both axes, worked
    // out once rather than per pixel.
    let mut gens = Vec::with_capacity(count as usize);
    for i in 0..count {
        let pick = |salt: u32, low: f32, high: f32| low + seeded(seed, i * 8 + salt) * (high - low);
        gens.push((
            pick(0, wave_low, wave_high),
            pick(1, amp_low, amp_high),
            seeded(seed, i * 8 + 2),
            pick(3, wave_low, wave_high),
            pick(4, amp_low, amp_high),
            seeded(seed, i * 8 + 5),
        ));
    }

    let n = count as f32;
    let (scale_x, scale_y) = (scale.0 / 100.0, scale.1 / 100.0);
    let edge = if wrap {
        EdgeMode::Wrap
    } else {
        EdgeMode::Clamp
    };

    remap(pixmap, edge, move |x, y| {
        let mut dx = 0.0;
        let mut dy = 0.0;
        for &(lx, ax, px, ly, ay, py) in &gens {
            // Sideways displacement varies down the picture and vertical
            // displacement across it, which is what makes a wave travel
            // rather than shimmer in place.
            dx += ax * kind.at(y / lx + px);
            dy += ay * kind.at(x / ly + py);
        }
        (x + dx / n * scale_x, y + dy / n * scale_y)
    });
}

/// How far a zigzag pushes at normalized radius `r`, as a fraction of the
/// radius. Shared with the wireframe.
fn zigzag_push(r: f32, amount: f32, ridges: u32) -> f32 {
    let a = amount.clamp(-100.0, 100.0) / 100.0;
    let waves = (ridges.max(1)) as f32 * std::f32::consts::PI;
    // A third of the radius at the top of the slider. That is a lot, and it
    // is meant to be: CS6's ZigZag at a middling Amount already breaks an
    // outline into visible saw teeth, and a gentler number leaves the filter
    // looking like it has not done anything.
    //
    // Fading to nothing at the rim keeps the corners out of it.
    a * 0.35 * (waves * r).sin() * (1.0 - r)
}

/// Filter ▸ Distort ▸ ZigZag.
///
/// `amount` is a percentage from -100 to 100 and `ridges` how many rings the
/// disturbance is broken into.
///
/// Unlike the other radial distortions here this one is allowed to fold: at a
/// high ridge count the rings interleave, which is the churn the filter is
/// named for and what CS6 does at the same settings.
pub fn zigzag(pixmap: &mut Pixmap, amount: f32, ridges: u32, style: ZigZagStyle) {
    if amount == 0.0 || pixmap.is_empty() {
        return;
    }
    let half_w = pixmap.width() as f32 / 2.0;
    let half_h = pixmap.height() as f32 / 2.0;

    remap(pixmap, EdgeMode::Clamp, move |x, y| {
        let dx = (x - half_w) / half_w;
        let dy = (y - half_h) / half_h;
        let r = (dx * dx + dy * dy).sqrt();
        if r >= 1.0 || r <= 0.0 {
            return (x, y);
        }
        let push = zigzag_push(r, amount, ridges);

        let (rx, ry) = match style {
            ZigZagStyle::OutFromCenter => {
                // Never past the middle: at this strength a trough near the
                // centre could otherwise carry a pixel through it and out the
                // far side, which reads as a hole rather than a ripple.
                let k = (r + push).max(0.0) / r;
                (dx * k, dy * k)
            }
            ZigZagStyle::AroundCenter => {
                let turn = push * std::f32::consts::PI;
                let (sin, cos) = turn.sin_cos();
                (dx * cos - dy * sin, dx * sin + dy * cos)
            }
            ZigZagStyle::PondRipples => {
                let k = (r + push).max(0.0) / r;
                // Only a touch of turn on top of the radial push. Pond
                // Ripples is mostly a set of rings moving in and out — the
                // swirl is what stops them looking machined, not the effect
                // itself, and at any more than this the teeth smear round the
                // circle instead of pointing out of it.
                let turn = push * std::f32::consts::PI * 0.25;
                let (sin, cos) = turn.sin_cos();
                ((dx * cos - dy * sin) * k, (dx * sin + dy * cos) * k)
            }
        };
        (half_w + rx * half_w, half_h + ry * half_h)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photorust::pixmap::Rgba8;

    /// A grid, so that a distortion has something whose shape can be checked
    /// rather than only its presence.
    fn grid(size: u32, spacing: u32) -> Pixmap {
        let mut px = Pixmap::filled(size, size, Rgba8::WHITE);
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                if (x as u32).is_multiple_of(spacing) || (y as u32).is_multiple_of(spacing) {
                    px.set(x, y, Rgba8::BLACK);
                }
            }
        }
        px
    }

    #[test]
    fn pinch_leaves_the_centre_and_the_rim_where_they_are() {
        // Both ends of the falloff are fixed points. If either moved, the
        // filter would tear away from the unaffected corners.
        let before = grid(65, 8);
        let mut after = before.clone();
        pinch(&mut after, 100.0);

        assert_eq!(after.get(32, 32), before.get(32, 32), "the centre moved");
        assert_eq!(after.get(0, 32), before.get(0, 32), "the rim moved");
    }

    /// A filled disc on white, whose area says how much the distortion grew
    /// or shrank what is in the middle.
    fn disc(size: u32, radius: f32) -> Pixmap {
        let mut px = Pixmap::filled(size, size, Rgba8::WHITE);
        let c = size as f32 / 2.0;
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                let dx = x as f32 + 0.5 - c;
                let dy = y as f32 + 0.5 - c;
                if (dx * dx + dy * dy).sqrt() < radius {
                    px.set(x, y, Rgba8::BLACK);
                }
            }
        }
        px
    }

    fn dark_pixels(px: &Pixmap) -> usize {
        (0..px.height() as i32)
            .flat_map(|y| (0..px.width() as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| px.get(x, y).r < 128)
            .count()
    }

    #[test]
    fn pinch_shrinks_the_middle_and_a_bulge_swells_it() {
        // The sign of the amount is the whole interface, and reversed the
        // filter still looks like a perfectly good distortion. Measured on
        // what it does rather than by repeating the formula.
        let source = disc(129, 30.0);
        let before = dark_pixels(&source);

        let mut pinched = source.clone();
        pinch(&mut pinched, 100.0);
        let mut bulged = source.clone();
        pinch(&mut bulged, -100.0);

        assert!(
            dark_pixels(&pinched) < before,
            "a pinch made the disc bigger: {} then {}",
            before,
            dark_pixels(&pinched)
        );
        assert!(
            dark_pixels(&bulged) > before,
            "a bulge made the disc smaller: {} then {}",
            before,
            dark_pixels(&bulged)
        );
    }

    #[test]
    fn pinch_never_folds_the_picture_over_itself() {
        // The failure a smooth-looking distortion hides: two destination
        // radii reading the same source radius, which creases the image into
        // a dark ring. The mapping has to keep going outwards the whole way.
        for amount in [-100.0f32, -60.0, 60.0, 100.0] {
            let strength = pinch_strength(amount);
            let mut last = 0.0f32;
            for step in 1..=1000 {
                let r = step as f32 / 1000.0;
                let mapped = pinch_map(r, strength);
                assert!(
                    mapped > last,
                    "at {}% the map turns back on itself around r = {:.3}",
                    amount,
                    r
                );
                last = mapped;
            }
        }
    }

    #[test]
    fn spherize_modes_act_on_the_axes_they_name() {
        // Horizontal Only and Vertical Only are one dropdown apart and easy
        // to wire the same way round twice.
        //
        // A spacing that does not divide the middle row, or that row would be
        // one solid line and moving it sideways would change nothing.
        let source = grid(65, 7);
        let run = |mode| {
            let mut px = source.clone();
            spherize(&mut px, 100.0, mode);
            px
        };
        let across = run(SpherizeMode::HorizontalOnly);
        let down = run(SpherizeMode::VerticalOnly);

        // Along the middle row, a horizontal-only spherize moves things and a
        // vertical-only one leaves them exactly where they were, because
        // nothing on that row is off the centre line vertically.
        let middle_row_moved = |px: &Pixmap| (0..65).any(|x| px.get(x, 32) != source.get(x, 32));
        assert!(
            middle_row_moved(&across),
            "Horizontal Only left the middle row alone"
        );
        assert!(
            !middle_row_moved(&down),
            "Vertical Only disturbed the middle row"
        );

        let middle_column_moved = |px: &Pixmap| (0..65).any(|y| px.get(32, y) != source.get(32, y));
        assert!(
            middle_column_moved(&down),
            "Vertical Only left the middle column alone"
        );
        assert!(
            !middle_column_moved(&across),
            "Horizontal Only disturbed the middle column"
        );
    }

    #[test]
    fn twirl_turns_the_middle_and_leaves_the_rim() {
        let source = grid(65, 8);
        let mut after = source.clone();
        twirl(&mut after, 200.0);

        assert_eq!(
            after.get(0, 32),
            source.get(0, 32),
            "the rim of the twirl moved"
        );
        let disturbed = (24..40)
            .flat_map(|y| (24..40).map(move |x| (x, y)))
            .filter(|&(x, y)| after.get(x, y) != source.get(x, y))
            .count();
        assert!(
            disturbed > 20,
            "the middle of the twirl barely moved: {} pixels",
            disturbed
        );

        // ...and the two directions are not the same picture.
        let mut back = source.clone();
        twirl(&mut back, -200.0);
        assert_ne!(
            after.as_bytes(),
            back.as_bytes(),
            "the sign of the angle did nothing"
        );
    }

    #[test]
    fn a_wave_reproduces_itself_from_its_seed() {
        // Randomize re-rolls a seed rather than calling a random number
        // generator as it goes. It has to: the preview and the applied filter
        // are two separate runs, and undo and redo are two more.
        let source = grid(64, 8);
        let run = |seed| {
            let mut px = source.clone();
            wave(
                &mut px,
                6,
                (10.0, 60.0),
                (4.0, 20.0),
                (100.0, 100.0),
                WaveKind::Sine,
                false,
                seed,
            );
            px
        };
        assert_eq!(
            run(11).as_bytes(),
            run(11).as_bytes(),
            "the same seed gave two waves"
        );
        assert_ne!(
            run(11).as_bytes(),
            run(12).as_bytes(),
            "the seed made no difference"
        );
    }

    #[test]
    fn the_wave_type_changes_the_shape_of_the_wave() {
        // Sine, Triangle and Square are three different displacements, not
        // three labels on one.
        let source = grid(64, 8);
        let run = |kind| {
            let mut px = source.clone();
            wave(
                &mut px,
                4,
                (20.0, 40.0),
                (8.0, 16.0),
                (100.0, 100.0),
                kind,
                false,
                5,
            );
            px
        };
        let sine = run(WaveKind::Sine);
        let triangle = run(WaveKind::Triangle);
        let square = run(WaveKind::Square);
        assert_ne!(sine.as_bytes(), triangle.as_bytes());
        assert_ne!(sine.as_bytes(), square.as_bytes());
        assert_ne!(triangle.as_bytes(), square.as_bytes());
    }

    #[test]
    fn a_middling_zigzag_pushes_hard_enough_to_see() {
        // The failure this pins is not a wrong picture but a timid one: the
        // filter ran, nothing crashed, and at the settings a user would
        // actually reach for the image looked untouched. That is how it first
        // shipped, at about a sixth of this.
        //
        // Measured on the mapping rather than on a rendered shape, because
        // the ripples are rings: a circle centred on the middle is pushed the
        // same amount all the way round and comes back a slightly larger
        // circle, so nothing about its outline would show the strength.
        let peak = (0..1000)
            .map(|i| zigzag_push(i as f32 / 1000.0, 28.0, 11).abs())
            .fold(0.0f32, f32::max);
        assert!(
            peak > 0.05,
            "at Amount 28 the strongest push is {:.1}% of the radius, which is not a zigzag \
             anybody would see",
            peak * 100.0
        );
        // ...and not so hard that a middling setting tears the picture up.
        assert!(
            peak < 0.2,
            "Amount 28 moves the picture by {:.1}% of the radius",
            peak * 100.0
        );
    }

    #[test]
    fn zigzag_styles_push_different_ways() {
        // Around Center turns, Out From Center pushes along the radius, and
        // Pond Ripples does both. Wiring all three to the same branch would
        // still produce a zigzag.
        let source = grid(65, 8);
        let run = |style| {
            let mut px = source.clone();
            zigzag(&mut px, 100.0, 6, style);
            px
        };
        let around = run(ZigZagStyle::AroundCenter);
        let out = run(ZigZagStyle::OutFromCenter);
        let pond = run(ZigZagStyle::PondRipples);
        assert_ne!(around.as_bytes(), out.as_bytes());
        assert_ne!(around.as_bytes(), pond.as_bytes());
        assert_ne!(out.as_bytes(), pond.as_bytes());

        // Whichever style, the rim is untouched.
        assert_eq!(
            pond.get(0, 32),
            source.get(0, 32),
            "a zigzag reached the rim"
        );
    }

    #[test]
    fn rectangular_to_polar_puts_the_top_edge_at_the_centre() {
        // Half red at the top and half blue at the bottom: wrapped into a
        // disc, the red must end up in the middle and the blue around it.
        let size = 64u32;
        let mut px = Pixmap::new(size, size);
        for y in 0..size as i32 {
            for x in 0..size as i32 {
                let colour = if y < size as i32 / 2 {
                    Rgba8::new(255, 0, 0, 255)
                } else {
                    Rgba8::new(0, 0, 255, 255)
                };
                px.set(x, y, colour);
            }
        }
        polar_coordinates(&mut px, true);

        let middle = px.get(32, 32);
        assert!(
            middle.r > middle.b,
            "the top of the image did not land in the centre"
        );
        // Straight up from the centre and a long way out, which is past
        // halfway to the corner, so it comes from the bottom half.
        let out = px.get(32, 2);
        assert!(
            out.b > out.r,
            "the bottom of the image did not land at the rim"
        );
    }

    #[test]
    fn polar_to_rectangular_undoes_rectangular_to_polar() {
        // Not exactly — resampling twice softens everything — but the picture
        // has to come back, which it will not if the two directions disagree
        // about where the seam is or which way the angle runs.
        let source = grid(64, 8);
        let mut round_trip = source.clone();
        polar_coordinates(&mut round_trip, true);
        polar_coordinates(&mut round_trip, false);

        // Compared over the middle, away from the corners neither direction
        // can account for.
        let mut close = 0;
        let mut total = 0;
        for y in 20..44 {
            for x in 20..44 {
                let a = round_trip.get(x, y);
                let b = source.get(x, y);
                if a.a > 0 && (a.r as i32 - b.r as i32).abs() < 96 {
                    close += 1;
                }
                total += 1;
            }
        }
        assert!(
            close * 3 > total * 2,
            "only {}/{} of the middle survived the round trip",
            close,
            total
        );
    }

    #[test]
    fn ripple_size_sets_how_far_apart_the_crests_are() {
        // Small and Large at the same amount are different filters, not the
        // same one twice. Counted as how often the displacement changes sign
        // along a row, which is what a wavelength is.
        let crossings = |size: RippleSize| -> usize {
            let mut px = Pixmap::filled(200, 8, Rgba8::WHITE);
            for y in 0..8 {
                for x in (0..200).step_by(2) {
                    px.set(x, y, Rgba8::BLACK);
                }
            }
            let mut rippled = px.clone();
            ripple(&mut rippled, 999.0, size);
            let mut runs = 0;
            let mut last = rippled.get(0, 4).r > 128;
            for x in 1..200 {
                let now = rippled.get(x, 4).r > 128;
                if now != last {
                    runs += 1;
                    last = now;
                }
            }
            runs
        };
        // The finer ripple has to disturb a striped row more often than the
        // coarse one does.
        assert!(
            crossings(RippleSize::Small) != crossings(RippleSize::Large),
            "Small and Large rippled identically"
        );
    }

    #[test]
    fn a_two_point_shear_curve_is_a_straight_line() {
        // Two points make both Hermite tangents the secant, so the shift has
        // to grow evenly down the image rather than easing.
        let curve = [(-1.0f64, -0.5), (1.0, 0.5)];
        for (position, expected) in [
            (-1.0, -0.5),
            (-0.5, -0.25),
            (0.0, 0.0),
            (0.5, 0.25),
            (1.0, 0.5),
        ] {
            assert!(
                (curve_at(&curve, position) - expected).abs() < 1e-6,
                "at {position} the straight curve read {} not {expected}",
                curve_at(&curve, position)
            );
        }
    }

    #[test]
    fn an_interior_shear_point_bends_the_line_without_kinking() {
        // The slope either side of a control point has to agree, or the line
        // is pointy exactly where CS6's is rounded.
        let curve = [(-1.0f64, 0.0), (0.0, 0.5), (1.0, 0.0)];
        let eps = 1e-3f64;
        let before = (curve_at(&curve, -eps) - curve_at(&curve, -3.0 * eps)) / (2.0 * eps);
        let after = (curve_at(&curve, 3.0 * eps) - curve_at(&curve, eps)) / (2.0 * eps);
        assert!(
            (before - after).abs() < 0.05,
            "the curve kinks at the point: {before} then {after}"
        );
        // ...and it bows above the straight chord between the points.
        assert!(curve_at(&curve, -0.5) > 0.25);
        assert!(curve_at(&curve, 0.5) > 0.25);
    }

    #[test]
    fn a_straight_shear_curve_does_nothing() {
        // All zeroes is a straight line down the middle of the box, which is
        // where the curve starts. Opening the dialog and pressing OK must
        // leave the picture alone.
        let source = grid(64, 8);
        let mut after = source.clone();
        shear(&mut after, &[(-1.0, 0.0), (1.0, 0.0)], false);
        assert_eq!(after.as_bytes(), source.as_bytes());
    }

    #[test]
    fn shear_pushes_each_row_by_what_the_curve_says() {
        // A curve that is zero at the top and bends to the right lower down
        // has to move the bottom rows and leave the top ones.
        let mut source = Pixmap::filled(64, 64, Rgba8::WHITE);
        for y in 0..64 {
            source.set(20, y, Rgba8::BLACK);
        }

        let mut after = source.clone();
        shear(&mut after, &[(-1.0, 0.0), (1.0, 0.5)], false);

        // The dark column, found on the top row and the bottom row.
        let column_on =
            |px: &Pixmap, y: i32| -> Option<i32> { (0..64).find(|&x| px.get(x, y).r < 128) };
        assert_eq!(
            column_on(&after, 0),
            Some(20),
            "the top row should not have moved"
        );
        let bottom = column_on(&after, 63).expect("the line vanished from the bottom row");
        assert!(bottom > 20, "the bottom row went the wrong way: {bottom}");
    }

    #[test]
    fn shear_wraps_or_repeats_the_edge_it_shifted_off() {
        // A constant offset that pushes every row half a width to the right:
        // the columns that fall off the left edge come back from the right
        // under Wrap, and repeat the left edge under Clamp.
        let mut source = Pixmap::filled(32, 32, Rgba8::WHITE);
        for y in 0..32 {
            source.set(0, y, Rgba8::new(255, 0, 0, 255));
            source.set(31, y, Rgba8::new(0, 0, 255, 255));
        }
        let curve = [(-1.0, 1.0), (1.0, 1.0)];
        let mut clamp = source.clone();
        shear(&mut clamp, &curve, false);
        let mut wrap = source.clone();
        shear(&mut wrap, &curve, true);

        assert_ne!(clamp.as_bytes(), wrap.as_bytes());
        assert_eq!(clamp.get(0, 16), Rgba8::new(255, 0, 0, 255));
        assert_eq!(wrap.get(0, 16), Rgba8::new(255, 255, 255, 255));
    }
}
