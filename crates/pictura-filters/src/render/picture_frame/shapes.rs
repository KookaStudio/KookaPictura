//! The 22 flower shapes and 23 leaf shapes, each about the origin.
//!
//! Ported from perfecto25/photorust (`core/src/filters/frame.rs`), GPL-3.0.

use std::f32::consts::{PI, TAU};

use super::geometry::{
    arc_band, circle, dashed_ring, ellipse, hole, petal, polar, rect, solid, star, stroke, Contour,
};

// ---------------------------------------------------------------- flowers --

/// Flower `index`, 1 to 22, about the origin with a radius of 1.
pub(super) fn flower_shape(index: u32) -> Vec<Contour> {
    let around = |n: usize, f: &dyn Fn(f32) -> Contour| -> Vec<Contour> {
        (0..n)
            .map(|i| f(i as f32 / n as f32 * TAU - PI / 2.0))
            .collect()
    };
    match index {
        1 => vec![circle(0.0, 0.0, 0.35)],
        2 => vec![circle(0.0, 0.0, 1.0)],
        3 => {
            let mut out = vec![circle(0.0, 0.0, 0.3)];
            out.extend(around(5, &|a| petal(a, 0.42, 1.0, 0.62, 0.75)));
            out
        }
        4 => {
            let mut out = vec![circle(0.0, 0.0, 0.2)];
            out.extend(around(8, &|a| petal(a, 0.3, 1.0, 0.26, 1.0)));
            out
        }
        5 => {
            let mut out = vec![circle(0.0, 0.0, 0.6)];
            out.extend(dashed_ring(0.78, 0.95, 8, 0.7));
            out
        }
        6 => {
            // A disc with a hole and eight swirling points.
            let mut out = vec![circle(0.0, 0.0, 0.6), hole(circle(0.0, 0.0, 0.28))];
            out.extend(around(8, &|a| {
                solid(vec![
                    (0.55 * (a - 0.3).cos(), 0.55 * (a - 0.3).sin()),
                    (0.55 * (a + 0.3).cos(), 0.55 * (a + 0.3).sin()),
                    ((a + 0.45).cos(), (a + 0.45).sin()),
                ])
            }));
            out
        }
        7 => around(4, &|a| petal(a, 0.12, 1.0, 0.84, 0.6)),
        8 => {
            let mut out = vec![circle(0.0, 0.0, 0.2)];
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                out.push(circle(0.42 * a.cos(), 0.42 * a.sin(), 0.08));
                out.push(petal(a, 0.55, 1.0, 0.14, 1.0));
                out.push(petal(a + PI / 8.0, 0.55, 0.78, 0.12, 1.0));
            }
            out
        }
        9 => {
            // A rose: five rounded lobes, with the folds of the petals cut out
            // of it in arcs spiralling in.
            let mut out = vec![polar(80, |t| 0.8 + 0.14 * (2.5 * t).cos().abs())];
            for (r, a0, span) in [
                (0.64, 0.3, 2.4),
                (0.64, 3.4, 2.2),
                (0.46, 1.6, 2.6),
                (0.46, 4.6, 2.0),
                (0.27, 0.8, 3.4),
            ] {
                out.push(hole(arc_band(0.0, 0.0, r - 0.04, r + 0.04, a0, a0 + span)));
            }
            out
        }
        10 => {
            let mut out = vec![circle(0.0, 0.0, 0.2), hole(circle(0.0, 0.0, 0.1))];
            out.extend(around(16, &|a| petal(a, 0.16, 1.0, 0.11, 1.0)));
            out
        }
        11 => {
            let mut out = vec![circle(0.0, 0.0, 0.42)];
            out.extend(around(8, &|a| {
                solid(vec![
                    (0.52 * (a - 0.3).cos(), 0.52 * (a - 0.3).sin()),
                    (0.52 * (a + 0.3).cos(), 0.52 * (a + 0.3).sin()),
                    (a.cos(), a.sin()),
                ])
            }));
            out
        }
        12 => {
            let mut out = vec![circle(0.0, 0.0, 0.5)];
            out.extend(dashed_ring(0.68, 0.95, 10, 0.62));
            out
        }
        13 => {
            // A cross with its arms curling back at the ends.
            let mut out = vec![rect(-0.08, -1.0, 0.08, 1.0), rect(-1.0, -0.08, 1.0, 0.08)];
            for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                out.push(arc_band(0.45 * sx, 0.45 * sy, 0.2, 0.3, 0.0, TAU * 0.7));
            }
            out
        }
        14 => vec![polar(96, |t| {
            // Four points, the upright ones longer: an astroid.
            let (c, s) = (t.cos(), t.sin());
            let r = (c.abs().powf(2.0 / 3.0) / 0.7f32.powf(2.0 / 3.0) + s.abs().powf(2.0 / 3.0))
                .powf(-1.5);
            r.max(0.06)
        })],
        15 => {
            let mut out = vec![circle(0.0, 0.0, 0.12)];
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU - PI / 2.0;
                let length = if i % 2 == 0 { 1.0 } else { 0.75 };
                out.push(petal(a, 0.0, length, 0.14, 1.0));
            }
            out
        }
        16 => vec![
            polar(96, |t| 0.86 + 0.14 * (4.0 * t).cos().abs()),
            hole(circle(0.0, 0.0, 0.72)),
            star(5, 0.55, 0.22),
        ],
        17 => around(4, &|a| petal(a, 0.0, 1.0, 0.8, 1.6)),
        18 => vec![rect(-0.25, -1.0, 0.25, 1.0), rect(-1.0, -0.25, 1.0, 0.25)],
        19 => vec![heart()],
        20 => vec![star(5, 1.0, 0.4)],
        21 => {
            let mut out = vec![polar(6, |_| 0.2)];
            for i in 0..6 {
                let a = i as f32 / 6.0 * TAU - PI / 2.0;
                let (c, s) = (a.cos(), a.sin());
                out.extend(stroke(&[(0.0, 0.0), (0.95 * c, 0.95 * s)], 0.1));
                for (at, reach) in [(0.45, 0.28), (0.7, 0.22)] {
                    let (bx, by) = (at * c, at * s);
                    for side in [-0.8f32, 0.8] {
                        let b = a + side;
                        out.extend(stroke(
                            &[(bx, by), (bx + reach * b.cos(), by + reach * b.sin())],
                            0.08,
                        ));
                    }
                }
            }
            out
        }
        22 => vec![
            ellipse(0.0, 0.42, 0.45, 0.38, 0.0),
            ellipse(-0.62, -0.12, 0.15, 0.21, -0.4),
            ellipse(-0.24, -0.55, 0.16, 0.22, -0.12),
            ellipse(0.24, -0.55, 0.16, 0.22, 0.12),
            ellipse(0.62, -0.12, 0.15, 0.21, 0.4),
        ],
        _ => Vec::new(),
    }
}

pub(super) fn heart() -> Contour {
    solid(
        (0..64)
            .map(|i| {
                let t = i as f32 / 64.0 * TAU;
                let x = 16.0 * t.sin().powi(3);
                let y = 13.0 * t.cos()
                    - 5.0 * (2.0 * t).cos()
                    - 2.0 * (3.0 * t).cos()
                    - (4.0 * t).cos();
                (x / 17.0, -y / 17.0 - 0.1)
            })
            .collect(),
    )
}

// ----------------------------------------------------------------- leaves --

/// A closed curve from the top point round to the bottom and back, with
/// `half(t)` the half-width at each height, `t` from 0 at the top to 1.
pub(super) fn profile(half: impl Fn(f32) -> f32) -> Contour {
    let n = 40;
    let mut out = Vec::with_capacity(2 * n);
    // `max` also turns the NaN a fractional power of sin(π)'s rounding error
    // gives back into the zero it should be.
    let half = |t: f32| half(t).max(0.0);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        out.push((half(t), -1.0 + 2.0 * t));
    }
    for i in (1..n).rev() {
        let t = i as f32 / n as f32;
        out.push((-half(t), -1.0 + 2.0 * t));
    }
    solid(out)
}

/// Leaf `index`, 1 to 23, as CS6's list draws it: hanging from its stalk at
/// (0, -1) and reaching down to y = 1.
pub(super) fn leaf_shape(index: u32) -> Vec<Contour> {
    let mirrored = |half: &[(f32, f32)]| -> Contour {
        let mut out: Vec<(f32, f32)> = half.to_vec();
        out.extend(
            half.iter()
                .rev()
                .filter(|p| p.0 != 0.0)
                .map(|&(x, y)| (-x, y)),
        );
        solid(out)
    };
    match index {
        1 => vec![circle(0.0, 0.0, 0.95)],
        2 => vec![rect(-0.9, -0.9, 0.9, 0.9)],
        3 => vec![profile(|t| 0.8 * (PI * t).sin() * (PI * t / 2.0).sin())],
        4 => vec![profile(|t| 0.5 * (PI * t).sin().powf(1.4))],
        5 => vec![profile(|t| {
            0.7 * (PI * t).sin() * (PI * (1.0 - t) / 2.0).sin()
        })],
        6 => vec![profile(|t| {
            0.62 * (PI * t).sin() * (PI * t / 2.0).sin().powf(1.3)
        })],
        7 => vec![mirrored(&[
            (0.0, -1.0),
            (0.3, -0.35),
            (0.75, 0.55),
            (0.25, 0.35),
            (0.0, 1.0),
        ])],
        8 => vec![
            circle(0.0, -0.25, 0.55),
            circle(-0.55, -0.15, 0.42),
            circle(0.55, -0.15, 0.42),
            circle(0.0, 0.55, 0.3),
        ],
        9 => vec![mirrored(&[
            (0.0, -1.0),
            (0.8, -0.3),
            (0.22, -0.35),
            (0.16, 0.2),
            (0.0, 1.0),
        ])],
        10 => {
            let mut half: Vec<(f32, f32)> = vec![(0.0, -0.9)];
            for i in 1..=10 {
                let t = i as f32 / 10.0;
                half.push((0.9 * t * t, -0.9 + 1.4 * t));
            }
            for i in 1..=8 {
                let t = i as f32 / 8.0;
                half.push((0.9 * (1.0 - t), 0.5 + 0.3 * (PI * t / 2.0).sin()));
            }
            half.pop();
            half.push((0.0, 0.8));
            vec![mirrored(&half)]
        }
        11 => vec![solid(vec![(0.0, -0.8), (0.85, 0.8), (-0.85, 0.8)])],
        12 => vec![mirrored(&[
            (0.0, -1.0),
            (0.85, -0.05),
            (0.3, 0.1),
            (0.0, 0.9),
        ])],
        13 => vec![polar(96, |t| {
            let (c, s) = (t.cos(), t.sin());
            (c.abs().powf(2.0 / 3.0) / 0.55f32.powf(2.0 / 3.0) + s.abs().powf(2.0 / 3.0))
                .powf(-1.5)
                .max(0.05)
        })],
        14 => vec![ellipse(0.0, 0.0, 0.42, 0.95, 0.0)],
        15 => vec![solid(vec![
            (-0.35, -0.6),
            (0.35, -0.6),
            (0.95, 0.6),
            (-0.95, 0.6),
        ])],
        16 => vec![rect(-0.08, -1.0, 0.08, 0.3), circle(0.0, 0.55, 0.45)],
        17 => vec![profile(|t| {
            0.75 * (PI * t).sin().powf(0.7) * (1.0 - 0.3 * (1.0 - t))
        })],
        18 => vec![polar(96, |t| {
            let (c, s) = (t.cos(), t.sin());
            (c.abs().powf(2.0 / 3.0) + s.abs().powf(2.0 / 3.0))
                .powf(-1.5)
                .max(0.08)
                * 0.95
        })],
        19 => vec![
            rect(-0.12, -1.0, 0.12, -0.2),
            solid(vec![
                (-0.12, -0.35),
                (0.12, -0.35),
                (0.42, 0.25),
                (-0.42, 0.25),
            ]),
            circle(0.0, 0.45, 0.5),
        ],
        20 => vec![rect(-0.2, -0.9, 0.2, 0.9), rect(-0.9, -0.2, 0.9, 0.2)],
        21 => vec![heart()],
        22 => vec![rect(-0.18, -1.0, 0.18, 0.55), rect(-0.9, 0.5, 0.9, 0.9)],
        23 => vec![star(5, 1.0, 0.4)],
        _ => Vec::new(),
    }
}
