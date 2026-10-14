//! The per-colour layers a motif frame collects, and the 35 motif-frame
//! recipes.
//!
//! Ported from perfecto25/photorust (`core/src/filters/frame.rs`), GPL-3.0.

use std::f32::consts::{PI, TAU};

use super::geometry::{bezier, curl, fill, place, stroke, Contour, Rng, Track};
use super::moulding::moulding;
use super::scale::scale_for;
use super::shapes::{flower_shape, leaf_shape};
use super::{uses, FrameOptions, ANGLE, FRAMES, LINES};
use crate::photorust::pixmap::{Pixmap, Rgba8};

// ----------------------------------------------------------------- frames --
/// What a motif frame draws, collected by colour so each colour goes down in
/// one pass: vines under leaves under flowers, as CS6 layers them.
#[derive(Default)]
struct Layers {
    vine: Vec<Contour>,
    leaves: Vec<Contour>,
    flowers: Vec<Contour>,
    /// Flowers laid down half see-through, for the frames CS6 draws soft.
    soft: Vec<Contour>,
    /// The Advanced tab's Angle, added to every flower and leaf.
    turn: f32,
}

impl Layers {
    fn flower(&mut self, shape: &[Contour], x: f32, y: f32, radius: f32, turn: f32) {
        self.flowers
            .extend(place(shape, x, y, radius, turn + self.turn));
    }

    fn soft_flower(&mut self, shape: &[Contour], x: f32, y: f32, radius: f32, turn: f32) {
        self.soft
            .extend(place(shape, x, y, radius, turn + self.turn));
    }

    /// A leaf with its stalk at `(x, y)`, reaching `length` towards `angle`.
    fn leaf(&mut self, shape: &[Contour], x: f32, y: f32, length: f32, angle: f32) {
        // The shape hangs from (0, -1) down to (0, 1): turn +y onto the angle,
        // and put the middle half a length out so the stalk lands on the point.
        let half = length * 0.5;
        let angle = angle + self.turn;
        let (cx, cy) = (x + half * angle.cos(), y + half * angle.sin());
        self.leaves
            .extend(place(shape, cx, cy, half, angle - PI / 2.0));
    }

    fn line(&mut self, points: &[(f32, f32)], width: f32) {
        // Never finer than a pixel: thinner, a stem breaks up into dots.
        self.vine.extend(stroke(points, width.max(1.0)));
    }
}

/// Filter ▸ Render ▸ Picture Frame.
///
/// The frame is drawn on a clear sheet first and laid over the picture after,
/// so that Fade can thin it as a whole: faded shape by shape, two flowers
/// overlapping would show darker where they cross.
pub(super) fn draw_frame(pixmap: &mut Pixmap, options: &FrameOptions) {
    let frame = options.frame.clamp(*FRAMES.start(), *FRAMES.end());
    let (width, height) = (pixmap.width(), pixmap.height());
    let scale = scale_for(options, width, height);
    if frame >= 36 {
        moulding(
            pixmap,
            frame,
            Rgba8::rgb(options.vine),
            &scale,
            options.invert,
        );
        return;
    }

    let flower = flower_shape(options.flower);
    let leaf = leaf_shape(options.leaf);
    let mut rng = Rng::new(options.arrangement, frame);
    let mut layers = Layers {
        turn: options.angle.min(*ANGLE.end()) as f32 * PI / 180.0,
        ..Layers::default()
    };
    let (w, h) = (width as f32, height as f32);
    let inset = scale.inset;
    let track = |radius: f32| Track {
        x0: inset,
        y0: inset,
        x1: w - inset,
        y1: h - inset,
        radius: radius.min((w.min(h) * 0.5 - inset).max(0.0)),
    };
    let centre = (w / 2.0, h / 2.0);
    let (u, f, l, v) = (scale.unit, scale.flower, scale.leaf, scale.vine);
    // The inward normal of a track running clockwise — or, inverted, the
    // outward one, which is all Invert does to a motif frame: whatever leant in
    // towards the picture leans out towards the edge.
    let flip = if options.invert { -1.0 } else { 1.0 };
    let inward = |t: (f32, f32)| (-t.1 * flip, t.0 * flip);
    // Number of Lines as a density against CS6's default of 15.
    let density = options.lines.clamp(*LINES.start(), *LINES.end()) as f32 / 15.0;
    // Angle as a lean, for the slanted line frames.
    let lean = layers.turn;
    // A vine waving `amplitude` either side of the track, a full wave every
    // `wave` along it: the points, and where each crest falls.
    let wavy = |path: &Track, wave: f32, amplitude: f32, phase: f32| -> Vec<(f32, f32)> {
        let n = (path.length() / wave).round().max(4.0);
        let wave = path.length() / n;
        let steps = (path.length() / 1.5) as usize;
        (0..=steps)
            .map(|i| {
                let s = i as f32 / steps as f32 * path.length();
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = amplitude * (TAU * s / wave + phase).sin();
                (p.0 + nx * off, p.1 + ny * off)
            })
            .collect()
    };
    // The same wave's crests, with which way the vine runs there.
    let crests = |path: &Track, wave: f32, phase: f32| -> Vec<(f32, bool)> {
        let n = (path.length() / wave).round().max(4.0) as usize;
        let wave = path.length() / n as f32;
        let quarter = (0.25 - phase / TAU).rem_euclid(1.0);
        (0..n * 2)
            .map(|k| ((k as f32 * 0.5 + quarter) * wave, k % 2 == 0))
            .collect()
    };

    match frame {
        // Happy Vine: a thin vine waving along the edge, a tendril curling off
        // every swing, and flowers on the crests.
        1 => {
            let path = track(u * 0.6);
            let (wave, amp) = (u * 1.6, u * 0.14);
            layers.line(&wavy(&path, wave, amp, 0.0), v * 0.7);
            for (s, out) in crests(&path, wave, 0.0) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let sign = if out { 1.0 } else { -1.0 };
                let (x, y) = (p.0 + nx * amp * sign, p.1 + ny * amp * sign);
                let heading = t.1.atan2(t.0) + sign * 0.9;
                layers.line(&curl(x, y, heading, u * 0.09, 1.2, -sign), v * 0.5);
                layers.flower(&flower, x, y, f * rng.range(0.6, 1.2), rng.range(0.0, TAU));
                if rng.next() < 0.5 {
                    let off = rng.range(0.25, 0.4) * u * sign;
                    layers.flower(&flower, p.0 + nx * off, p.1 + ny * off, f * 0.45, 0.0);
                }
            }
        }
        // Pretty Vine: a faint vine under loose clusters of flowers, some solid
        // and some see-through, of every size.
        2 => {
            let path = track(u * 0.6);
            layers.line(&wavy(&path, u * 2.0, u * 0.08, 0.0), v * 0.45);
            for s in path.spread(u * 0.8) {
                for _ in 0..(2 + (rng.next() * 3.0) as usize) {
                    let (p, t) = path.at(s + rng.range(-0.35, 0.35) * u);
                    let (nx, ny) = inward(t);
                    let off = rng.range(-0.25, 0.25) * u;
                    let (x, y) = (p.0 + nx * off, p.1 + ny * off);
                    let size = f * rng.range(0.4, 1.3);
                    if rng.next() < 0.45 {
                        layers.soft_flower(&flower, x, y, size, rng.range(0.0, TAU));
                    } else {
                        layers.flower(&flower, x, y, size, rng.range(0.0, TAU));
                    }
                }
            }
        }
        // Smoke Signals: flowers along the edge, each sending up a wisp that
        // winds away and ends in a curl.
        3 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.1) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let rise = u * rng.range(0.6, 0.9);
                let top = (
                    p.0 + nx * rise + t.0 * rise * 0.3,
                    p.1 + ny * rise + t.1 * rise * 0.3,
                );
                let bend = (
                    p.0 + nx * rise * 0.5 - t.0 * rise * 0.4,
                    p.1 + ny * rise * 0.5 - t.1 * rise * 0.4,
                );
                let mut wisp = bezier(p, bend, top, 14);
                let heading = (top.1 - bend.1).atan2(top.0 - bend.0);
                wisp.extend(
                    curl(top.0, top.1, heading, u * 0.08, 1.1, 1.0)
                        .into_iter()
                        .skip(1),
                );
                layers.line(&wisp, v * 0.5);
                layers.flower(
                    &flower,
                    p.0,
                    p.1,
                    f * rng.range(1.0, 1.4),
                    rng.range(0.0, TAU),
                );
                layers.flower(&flower, bend.0, bend.1, f * 0.45, 0.0);
            }
        }
        // Party: bunches of flowers with curled streamers flying off them.
        4 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.4) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                for _ in 0..3 {
                    let a = rng.range(0.0, TAU);
                    let reach = u * rng.range(0.2, 0.4);
                    let end = (p.0 + reach * a.cos(), p.1 + reach * a.sin());
                    let mut streamer = vec![p, end];
                    streamer.extend(
                        curl(
                            end.0,
                            end.1,
                            a,
                            u * 0.07,
                            1.0,
                            if rng.next() < 0.5 { 1.0 } else { -1.0 },
                        )
                        .into_iter()
                        .skip(1),
                    );
                    layers.line(&streamer, v * 0.45);
                }
                for _ in 0..(3 + (rng.next() * 3.0) as usize) {
                    let (ox, oy) = (rng.range(-0.3, 0.3) * u, rng.range(-0.25, 0.25) * u);
                    layers.flower(
                        &flower,
                        p.0 + t.0 * ox + nx * oy,
                        p.1 + t.1 * ox + ny * oy,
                        f * rng.range(0.5, 1.2),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Big Curls: a vine swinging wide, looping at every crest, with leaves
        // down its length and a flower in each loop.
        5 => {
            let path = track(u * 0.6);
            let (wave, amp) = (u * 1.8, u * 0.2);
            layers.line(&wavy(&path, wave, amp, 0.0), v);
            for (s, out) in crests(&path, wave, 0.0) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let sign = if out { 1.0 } else { -1.0 };
                let (x, y) = (p.0 + nx * amp * sign, p.1 + ny * amp * sign);
                let heading = t.1.atan2(t.0);
                let loop_ = curl(x, y, heading, u * 0.16, 0.9, -sign);
                let centre = loop_[loop_.len() / 2];
                layers.line(&loop_, v * 0.8);
                layers.flower(&flower, centre.0, centre.1, f, rng.range(0.0, TAU));
            }
            for (k, s) in path.spread(u * 0.45).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = amp * (TAU * s / wave).sin();
                let along = t.1.atan2(t.0);
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                layers.leaf(&leaf, p.0 + nx * off, p.1 + ny * off, l, along + side * 1.0);
            }
        }
        // Tilde: little "~" strokes one after another, each with a flower on
        // its end.
        6 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.2) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let length = u * 0.8;
                let amp = u * 0.1;
                let points: Vec<(f32, f32)> = (0..=24)
                    .map(|i| {
                        let q = i as f32 / 24.0;
                        let a = (q - 0.5) * length;
                        let o = amp * (TAU * q).sin();
                        (p.0 + t.0 * a + nx * o, p.1 + t.1 * a + ny * o)
                    })
                    .collect();
                layers.line(&points, v * 0.6);
                let end = points[points.len() - 1];
                layers.flower(&flower, end.0, end.1, f, 0.0);
            }
        }
        // Romance: flowers in pairs and threes, each hanging from a curl like a
        // question mark.
        7 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 0.9) {
                let (p, t) = path.at(s + rng.range(-0.2, 0.2) * u);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.2, 0.2) * u;
                let base = (p.0 + nx * off, p.1 + ny * off);
                let up = (nx, ny);
                let reach = u * rng.range(0.25, 0.4);
                let top = (base.0 + up.0 * reach, base.1 + up.1 * reach);
                let mut stem = vec![base, top];
                stem.extend(
                    curl(
                        top.0,
                        top.1,
                        up.1.atan2(up.0),
                        u * 0.08,
                        0.8,
                        if rng.next() < 0.5 { 1.0 } else { -1.0 },
                    )
                    .into_iter()
                    .skip(1),
                );
                layers.line(&stem, v * 0.5);
                layers.flower(
                    &flower,
                    base.0,
                    base.1,
                    f * rng.range(0.9, 1.3),
                    rng.range(0.0, TAU),
                );
                if rng.next() < 0.6 {
                    let a = rng.range(0.0, TAU);
                    layers.flower(
                        &flower,
                        base.0 + a.cos() * f * 2.0,
                        base.1 + a.sin() * f * 2.0,
                        f * 0.6,
                        0.0,
                    );
                }
            }
        }
        // Curly Dance: a vine of arches, looping at the foot of each, a flower
        // on every arch.
        8 => {
            let path = track(u * 0.6);
            let arch = u * 1.1;
            let n = (path.length() / arch).round().max(4.0);
            let arch = path.length() / n;
            let height = u * 0.25;
            let steps = (path.length() / 1.5) as usize;
            let points: Vec<(f32, f32)> = (0..=steps)
                .map(|i| {
                    let s = i as f32 / steps as f32 * path.length();
                    let (p, t) = path.at(s);
                    let (nx, ny) = inward(t);
                    let off = -height * (PI * s / arch).sin().abs() + height * 0.5;
                    (p.0 + nx * off, p.1 + ny * off)
                })
                .collect();
            layers.line(&points, v * 0.6);
            for k in 0..n as usize {
                let (p, t) = path.at(k as f32 * arch);
                let (nx, ny) = inward(t);
                let foot = (p.0 + nx * height * 0.5, p.1 + ny * height * 0.5);
                layers.line(
                    &curl(foot.0, foot.1, t.1.atan2(t.0), u * 0.08, 1.0, 1.0),
                    v * 0.5,
                );
                let (p, t) = path.at((k as f32 + 0.5) * arch);
                let (nx, ny) = inward(t);
                layers.flower(
                    &flower,
                    p.0 - nx * height * 0.5,
                    p.1 - ny * height * 0.5,
                    f * rng.range(1.0, 1.4),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Wisps: two fine strands twisting round each other, small flowers
        // where they cross.
        9 => {
            let path = track(u * 0.6);
            let (wave, amp) = (u * 1.4, u * 0.12);
            layers.line(&wavy(&path, wave, amp, 0.0), v * 0.4);
            layers.line(&wavy(&path, wave, amp, PI), v * 0.4);
            for (k, s) in path.spread(wave * 0.5).into_iter().enumerate() {
                let (p, _) = path.at(s - wave * 0.25);
                layers.flower(
                    &flower,
                    p.0,
                    p.1,
                    f * if k % 2 == 0 { 0.9 } else { 0.55 },
                    0.0,
                );
            }
        }
        // Spring Weed: tufts of fine blades springing up off the edge, some
        // tipped with a flower.
        10 => {
            let path = track(u * 0.4);
            for s in path.spread(u * 0.6) {
                let (p, t) = path.at(s + rng.range(-0.15, 0.15) * u);
                let (nx, ny) = inward(t);
                let blades = 3 + (rng.next() * 3.0) as usize;
                for b in 0..blades {
                    let lean = (b as f32 / (blades - 1) as f32 - 0.5) * 1.4 + rng.range(-0.2, 0.2);
                    let (c, sn) = (lean.cos(), lean.sin());
                    let dir = (nx * c - ny * sn, nx * sn + ny * c);
                    let reach = u * rng.range(0.2, 0.42);
                    let tip = (p.0 + dir.0 * reach, p.1 + dir.1 * reach);
                    let bend = (
                        p.0 + dir.0 * reach * 0.5 + t.0 * reach * 0.2 * lean.signum(),
                        p.1 + dir.1 * reach * 0.5 + t.1 * reach * 0.2 * lean.signum(),
                    );
                    layers.line(&bezier(p, bend, tip, 10), v * 0.45);
                    if rng.next() < 0.45 {
                        layers.flower(
                            &flower,
                            tip.0,
                            tip.1,
                            f * rng.range(0.5, 0.9),
                            rng.range(0.0, TAU),
                        );
                    }
                }
            }
        }
        // Eyelash: curved sprays of leaves, each rooted in a knot of flowers,
        // curving one way and then the other.
        11 => {
            let path = track(u * 0.5);
            for (k, s) in path.spread(u * 1.3).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                let reach = u * 0.55;
                let tip = (
                    p.0 + t.0 * reach * sign + nx * reach * 0.2,
                    p.1 + t.1 * reach * sign + ny * reach * 0.2,
                );
                let ctrl = (
                    p.0 + t.0 * reach * 0.5 * sign + nx * reach * 0.55,
                    p.1 + t.1 * reach * 0.5 * sign + ny * reach * 0.55,
                );
                let stem = bezier(p, ctrl, tip, 16);
                layers.line(&stem, v * 0.5);
                for i in (3..stem.len()).step_by(3) {
                    let (a, b) = (stem[i - 1], stem[i]);
                    let along = (b.1 - a.1).atan2(b.0 - a.0);
                    layers.leaf(&leaf, b.0, b.1, l * 0.8, along - sign * 0.8);
                }
                for _ in 0..3 {
                    let (ox, oy) = (rng.range(-0.12, 0.12) * u, rng.range(-0.1, 0.1) * u);
                    layers.flower(
                        &flower,
                        p.0 + ox,
                        p.1 + oy,
                        f * rng.range(0.7, 1.1),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Magic Smoke: short wavering stems, each with a pair of leaves and a
        // few flowers at its top.
        12 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.1) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let reach = u * 0.4;
                let lean = rng.range(-0.5, 0.5);
                let top = (
                    p.0 + nx * reach + t.0 * reach * lean,
                    p.1 + ny * reach + t.1 * reach * lean,
                );
                let ctrl = (
                    p.0 + nx * reach * 0.5 - t.0 * reach * 0.3,
                    p.1 + ny * reach * 0.5 - t.1 * reach * 0.3,
                );
                let stem = bezier(p, ctrl, top, 12);
                layers.line(&stem, v * 0.5);
                let mid = stem[stem.len() / 2];
                let up = ny.atan2(nx);
                layers.leaf(&leaf, mid.0, mid.1, l, up + 0.9);
                layers.leaf(&leaf, mid.0, mid.1, l, up - 0.9);
                for _ in 0..(2 + (rng.next() * 2.0) as usize) {
                    let (ox, oy) = (rng.range(-0.1, 0.1) * u, rng.range(-0.1, 0.1) * u);
                    layers.flower(
                        &flower,
                        top.0 + ox,
                        top.1 + oy,
                        f * rng.range(0.7, 1.1),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Curly Vine: a leafy vine, thick with leaves both sides and curling
        // tendrils, a flower here and there.
        13 => {
            let path = track(u * 0.6);
            let (wave, amp) = (u * 1.5, u * 0.1);
            layers.line(&wavy(&path, wave, amp, 0.0), v);
            for (k, s) in path.spread(l * 0.4).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = amp * (TAU * s / wave).sin();
                let (x, y) = (p.0 + nx * off, p.1 + ny * off);
                let along = t.1.atan2(t.0);
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                layers.leaf(
                    &leaf,
                    x,
                    y,
                    l * rng.range(0.7, 1.2),
                    along + side * rng.range(0.5, 1.3),
                );
                if k % 5 == 2 {
                    layers.line(
                        &curl(x, y, along - side * 1.0, u * 0.08, 1.2, side),
                        v * 0.5,
                    );
                }
            }
            for s in path.spread(u * 1.2) {
                let (p, _) = path.at(s + rng.range(-0.3, 0.3) * u);
                layers.flower(
                    &flower,
                    p.0,
                    p.1,
                    f * rng.range(0.7, 1.0),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Chainmail: two rows of flowers along the band's edges, laced together
        // by parallel slanting lines.
        14 => {
            let path = track(u * 0.3);
            let half = u * 0.22;
            let slant = 0.5 + lean;
            for s in path.spread(u * 0.2 / density) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let shift = half * slant.tan().clamp(-3.0, 3.0);
                let a = (p.0 - nx * half - t.0 * shift, p.1 - ny * half - t.1 * shift);
                let b = (p.0 + nx * half + t.0 * shift, p.1 + ny * half + t.1 * shift);
                layers.line(&[a, b], v * 0.35);
            }
            for side in [-1.0f32, 1.0] {
                for s in path.spread(u * 0.3) {
                    let (p, t) = path.at(s + rng.range(-0.08, 0.08) * u);
                    let (nx, ny) = inward(t);
                    let off = side * half * 1.1;
                    layers.flower(
                        &flower,
                        p.0 + nx * off,
                        p.1 + ny * off,
                        f * rng.range(0.55, 1.0),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Fun Event: bunches of flowers on curling stems, with a pair of small
        // ones between.
        15 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.2) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                for k in 0..3 {
                    let a = ny.atan2(nx) + (k as f32 - 1.0) * 0.8 + rng.range(-0.2, 0.2);
                    let reach = u * rng.range(0.2, 0.35);
                    let tip = (p.0 + reach * a.cos(), p.1 + reach * a.sin());
                    let mut stem = vec![p, tip];
                    stem.extend(
                        curl(
                            tip.0,
                            tip.1,
                            a,
                            u * 0.05,
                            0.9,
                            if k % 2 == 0 { 1.0 } else { -1.0 },
                        )
                        .into_iter()
                        .skip(1),
                    );
                    layers.line(&stem, v * 0.45);
                    layers.flower(
                        &flower,
                        tip.0,
                        tip.1,
                        f * rng.range(0.7, 1.0),
                        rng.range(0.0, TAU),
                    );
                }
                layers.flower(&flower, p.0, p.1, f * 1.2, 0.0);
                let (q, _) = path.at(s + u * 0.6);
                layers.flower(&flower, q.0 - f, q.1, f * 0.5, 0.0);
                layers.flower(&flower, q.0 + f, q.1 + f * 0.5, f * 0.5, 0.0);
            }
        }
        // Bush: clumps of leaves bursting up off a grassy line, flowers in
        // among them, big clumps and small by turns.
        16 => {
            let path = track(u * 0.5);
            let steps = (path.length() / 2.0) as usize;
            let ground: Vec<(f32, f32)> = (0..=steps)
                .map(|i| path.at(i as f32 / steps as f32 * path.length()).0)
                .collect();
            layers.line(&ground, v * 0.5);
            for (k, s) in path.spread(u * 1.0).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let big = k % 2 == 0;
                let up = ny.atan2(nx);
                // Big enough to read as a bush and not a scatter of leaves:
                // CS6's clumps stand taller than its flowers by far.
                let count = if big { 18 } else { 9 };
                for _ in 0..count {
                    let a = up + rng.range(-1.4, 1.4);
                    let ox = rng.range(-0.3, 0.3) * u;
                    let length = l * rng.range(1.0, if big { 2.2 } else { 1.5 });
                    layers.leaf(&leaf, p.0 + t.0 * ox, p.1 + t.1 * ox, length, a);
                }
                for _ in 0..if big { 3 } else { 1 } {
                    let (ox, oy) = (rng.range(-0.2, 0.2) * u, rng.range(0.05, 0.3) * u);
                    layers.flower(
                        &flower,
                        p.0 + t.0 * ox + nx * oy,
                        p.1 + t.1 * ox + ny * oy,
                        f * rng.range(0.6, 1.0),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Simple Lace: a fine waving line, flowers on the crests, large and
        // small by turns.
        17 => {
            let path = track(u * 0.6);
            let (wave, amp) = (u * 1.4, u * 0.1);
            layers.line(&wavy(&path, wave, amp, 0.0), v * 0.45);
            for (k, (s, out)) in crests(&path, wave, 0.0).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let sign = if out { 1.0 } else { -1.0 };
                layers.flower(
                    &flower,
                    p.0 + nx * amp * sign,
                    p.1 + ny * amp * sign,
                    f * if k % 2 == 0 { 1.2 } else { 0.8 },
                    0.0,
                );
            }
        }
        // Pulse: a vine waving in and out, a flower at every outer swing and a
        // pair of leaves where it crosses the line.
        18 => {
            let path = track(u * 0.6);
            let wave = u * 1.8;
            let n = (path.length() / wave).round().max(4.0);
            let wave = path.length() / n;
            let amplitude = u * 0.28;
            let steps = (path.length() / 2.0) as usize;
            let points: Vec<(f32, f32)> = (0..=steps)
                .map(|i| {
                    let s = i as f32 / steps as f32 * path.length();
                    let (p, t) = path.at(s);
                    let (nx, ny) = inward(t);
                    let off = amplitude * (TAU * s / wave).sin();
                    (p.0 + nx * off, p.1 + ny * off)
                })
                .collect();
            layers.line(&points, v);
            for k in 0..n as usize {
                let s = (k as f32 + 0.75) * wave;
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                layers.flower(
                    &flower,
                    p.0 - nx * amplitude,
                    p.1 - ny * amplitude,
                    f,
                    rng.range(0.0, TAU),
                );
                let s = (k as f32 + 0.5) * wave;
                let (p, t) = path.at(s);
                let along = t.1.atan2(t.0);
                layers.leaf(&leaf, p.0, p.1, l, along - 0.9);
                layers.leaf(&leaf, p.0, p.1, l, along + PI + 0.9);
            }
        }
        // Root: a garland — a gently swaying vine thick with leaves on both
        // sides, and flowers tucked in among them.
        19 => {
            let path = track(u * 0.8);
            let steps = (path.length() / 2.0) as usize;
            let sway = |s: f32| u * 0.12 * (s / (u * 1.4)).sin();
            let points: Vec<(f32, f32)> = (0..=steps)
                .map(|i| {
                    let s = i as f32 / steps as f32 * path.length();
                    let (p, t) = path.at(s);
                    let (nx, ny) = inward(t);
                    (p.0 + nx * sway(s), p.1 + ny * sway(s))
                })
                .collect();
            layers.line(&points, v * 1.3);
            for (k, s) in path.spread(l * 0.45).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let (x, y) = (p.0 + nx * sway(s), p.1 + ny * sway(s));
                let along = t.1.atan2(t.0);
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                let spread = rng.range(0.6, 1.1);
                layers.leaf(&leaf, x, y, l * rng.range(0.8, 1.15), along + side * spread);
            }
            for s in path.spread(u * 1.1) {
                let (p, t) = path.at(s + rng.range(-0.3, 0.3) * u);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.25, 0.25) * u;
                layers.flower(
                    &flower,
                    p.0 + nx * off,
                    p.1 + ny * off,
                    f * rng.range(0.7, 1.1),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Snakes: curling S-shaped stems, each with a flower at its head.
        20 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.5) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let reach = u * rng.range(0.45, 0.7);
                let a = t.1.atan2(t.0) + rng.range(-0.8, 0.8);
                let (dx, dy) = (a.cos() * reach, a.sin() * reach);
                let head = (p.0 + dx, p.1 + dy);
                let tail = (p.0 - dx, p.1 - dy);
                let bend = reach * 0.9;
                let first = bezier(
                    tail,
                    (p.0 - dx * 0.5 + nx * bend, p.1 - dy * 0.5 + ny * bend),
                    p,
                    12,
                );
                let second = bezier(
                    p,
                    (p.0 + dx * 0.5 - nx * bend, p.1 + dy * 0.5 - ny * bend),
                    head,
                    12,
                );
                let mut curve = first;
                curve.extend(second.into_iter().skip(1));
                layers.line(&curve, v);
                layers.flower(
                    &flower,
                    head.0,
                    head.1,
                    f * rng.range(0.9, 1.6),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Mustache: flowers joined dot to dot by fine lines, zigzagging across
        // the line.
        21 => {
            let path = track(u * 0.5);
            let mut chain: Vec<(f32, f32)> = Vec::new();
            for (k, s) in path.spread(u * 0.7).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = if k % 2 == 0 { 1.0 } else { -1.0 } * u * rng.range(0.15, 0.4);
                chain.push((p.0 + nx * off, p.1 + ny * off));
            }
            if let Some(&first) = chain.first() {
                chain.push(first);
            }
            layers.line(&chain, (v * 0.6).max(0.8));
            for &(x, y) in &chain[..chain.len().saturating_sub(1)] {
                layers.flower(&flower, x, y, f * rng.range(0.5, 1.1), rng.range(0.0, TAU));
            }
        }
        // Circle Sprinkle: flowers of every size scattered along the band.
        22 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 0.45) {
                let (p, t) = path.at(s + rng.range(-0.2, 0.2) * u);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.45, 0.45) * u;
                layers.flower(
                    &flower,
                    p.0 + nx * off,
                    p.1 + ny * off,
                    f * rng.range(0.4, 1.7),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Aligned Flowers: flowers in a row, large and small by turns, with a
        // small one set off to the side now and then.
        23 => {
            let path = track(u * 0.3);
            for (k, s) in path.spread(u * 0.9).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let big = k % 2 == 0;
                layers.flower(&flower, p.0, p.1, f * if big { 1.3 } else { 0.8 }, 0.0);
                if k % 3 == 0 {
                    let (nx, ny) = inward(t);
                    let off = u * 0.35 * if rng.next() < 0.5 { 1.0 } else { -1.0 };
                    layers.flower(&flower, p.0 + nx * off, p.1 + ny * off, f * 0.45, 0.0);
                }
            }
        }
        // Little Flowers: single flowers each with a pair of leaves, soft.
        24 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.3) {
                let (p, t) = path.at(s);
                let along = t.1.atan2(t.0);
                // Long enough to reach out from under the flower.
                layers.leaf(&leaf, p.0, p.1, l * 1.8, along + PI / 2.0 + 0.7);
                layers.leaf(&leaf, p.0, p.1, l * 1.8, along + PI / 2.0 - 0.7);
                layers.flower(
                    &flower,
                    p.0,
                    p.1,
                    f * rng.range(1.0, 1.5),
                    rng.range(0.0, TAU),
                );
            }
        }
        // Snowflakes: a drift of small flowers and leaves along the band.
        25 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 0.22) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.5, 0.5) * u;
                let (x, y) = (p.0 + nx * off, p.1 + ny * off);
                if rng.next() < 0.3 {
                    layers.leaf(&leaf, x, y, l * rng.range(0.5, 0.8), rng.range(0.0, TAU));
                } else {
                    layers.flower(&flower, x, y, f * rng.range(0.35, 0.8), rng.range(0.0, TAU));
                }
            }
        }
        // Flurry: tight clusters of flowers with a leaf or two.
        26 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.0) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                for _ in 0..2 {
                    let a = rng.range(0.0, TAU);
                    layers.leaf(&leaf, p.0, p.1, l * rng.range(0.6, 0.9), a);
                }
                for _ in 0..(3 + (rng.next() * 3.0) as usize) {
                    let (ox, oy) = (rng.range(-0.3, 0.3) * u, rng.range(-0.25, 0.25) * u);
                    layers.flower(
                        &flower,
                        p.0 + t.0 * ox + nx * oy,
                        p.1 + t.1 * ox + ny * oy,
                        f * rng.range(0.6, 1.1),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Check Marks: ticks drawn in the vine colour, a flower on the end of
        // every other one.
        27 => {
            let path = track(u * 0.5);
            for (k, s) in path.spread(u * 0.8).into_iter().enumerate() {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let arm = u * 0.35;
                let a = (
                    p.0 - t.0 * arm * 0.3 - nx * arm * 0.6,
                    p.1 - t.1 * arm * 0.3 - ny * arm * 0.6,
                );
                let b = (p.0 + nx * arm * 0.4, p.1 + ny * arm * 0.4);
                let c = (
                    p.0 + t.0 * arm - nx * arm * 0.9,
                    p.1 + t.1 * arm - ny * arm * 0.9,
                );
                layers.line(&[a, b, c], (v * 0.7).max(0.8));
                if k % 2 == 1 {
                    layers.flower(&flower, c.0, c.1, f * 1.2, 0.0);
                }
            }
        }
        // Focused Lines: short lines all aimed at the middle of the picture,
        // each ending in a flower on the inside.
        28 => {
            let path = track(u * 0.5);
            for (k, s) in path.spread(u * 0.7 / density).into_iter().enumerate() {
                let (p, _) = path.at(s);
                let (dx, dy) = (centre.0 - p.0, centre.1 - p.1);
                let (dx, dy) = (dx * flip, dy * flip);
                let d = (dx * dx + dy * dy).sqrt().max(1.0);
                let length = u * if k % 2 == 0 { 0.55 } else { 0.3 } * rng.range(0.8, 1.2);
                let end = (p.0 + dx / d * length, p.1 + dy / d * length);
                let start = (p.0 - dx / d * length * 0.3, p.1 - dy / d * length * 0.3);
                layers.line(&[start, end], (v * 0.6).max(0.8));
                layers.flower(
                    &flower,
                    end.0,
                    end.1,
                    f * if k % 2 == 0 { 1.2 } else { 0.8 },
                    0.0,
                );
            }
        }
        // Focused Parallel Lines: the same, but every line on one slant.
        29 => {
            let path = track(u * 0.5);
            // Angle leans the lines on from CS6's 45°.
            let slant = PI / 4.0 + lean;
            let (dx, dy) = (slant.cos() * flip, slant.sin() * flip);
            for (k, s) in path.spread(u * 0.75 / density).into_iter().enumerate() {
                let (p, _) = path.at(s);
                let length = u * if k % 2 == 0 { 0.5 } else { 0.3 };
                let start = (p.0 - dx * length * 0.5, p.1 - dy * length * 0.5);
                let end = (p.0 + dx * length * 0.5, p.1 + dy * length * 0.5);
                layers.line(&[start, end], (v * 0.6).max(0.8));
                layers.flower(
                    &flower,
                    start.0,
                    start.1,
                    f * if k % 2 == 0 { 1.4 } else { 0.8 },
                    0.0,
                );
            }
        }
        // Focused Vibration: lines standing straight off the edge, their
        // lengths jumping about like a trace, each tipped with a flower.
        30 => {
            let path = track(0.0);
            for s in path.spread(u * 0.45 / density) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let length = u * rng.range(0.1, 0.6);
                let end = (p.0 + nx * length, p.1 + ny * length);
                layers.line(&[p, end], (v * 0.5).max(0.8));
                layers.flower(&flower, end.0, end.1, f * 0.8, 0.0);
            }
        }
        // Zen Garden: each flower set in rings, like a stone in raked sand.
        31 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.1) {
                let (p, t) = path.at(s);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.2, 0.2) * u;
                let (x, y) = (p.0 + nx * off, p.1 + ny * off);
                for ring in 1..=3 {
                    let r = u * 0.16 * ring as f32 * rng.range(0.9, 1.1);
                    let n = ((r * 0.8) as usize).clamp(16, 96);
                    let points: Vec<(f32, f32)> = (0..=n)
                        .map(|i| {
                            let a = i as f32 / n as f32 * TAU;
                            (x + r * a.cos(), y + r * a.sin())
                        })
                        .collect();
                    layers.line(&points, (v * 0.4).max(0.7));
                }
                layers.flower(&flower, x, y, f, rng.range(0.0, TAU));
            }
        }
        // Spikes: small flowers along the line, each with a burst of fine
        // spikes round it.
        32 => {
            let path = track(u * 0.5);
            for s in path.spread(u * 1.0) {
                let (p, _) = path.at(s);
                let turn = rng.range(0.0, TAU);
                for i in 0..6 {
                    let a = turn + i as f32 / 6.0 * TAU;
                    let reach = u * if i % 2 == 0 { 0.4 } else { 0.25 };
                    layers.line(
                        &[p, (p.0 + reach * a.cos(), p.1 + reach * a.sin())],
                        (v * 0.35).max(0.7),
                    );
                }
                layers.flower(&flower, p.0, p.1, f * 0.8, 0.0);
            }
        }
        // Anemone: flowers crowded all the way round, jostling in and out of
        // line, on a well-rounded track.
        33 => {
            // In clumps of two or three, not one by one.
            let path = track(u * 1.2);
            for s in path.spread(f * 3.4) {
                let (p, t) = path.at(s + rng.range(-0.4, 0.4) * f);
                let (nx, ny) = inward(t);
                let off = rng.range(-0.9, 0.9) * f;
                for _ in 0..(2 + (rng.next() * 2.0) as usize) {
                    let (ax, ay) = (rng.range(-0.8, 0.8) * f, rng.range(-0.6, 0.6) * f);
                    layers.flower(
                        &flower,
                        p.0 + nx * (off + ay) + t.0 * ax,
                        p.1 + ny * (off + ay) + t.1 * ax,
                        f * rng.range(1.1, 1.45),
                        rng.range(0.0, TAU),
                    );
                }
            }
        }
        // Pinwheel: sprays of curved stems, each tipped with a flower, swirling
        // about a centre; set symmetrically, with a tight bunch at each corner
        // and a double spray in the middle of the long sides.
        34 => {
            let path = track(0.0);
            let spray = |layers: &mut Layers,
                         x: f32,
                         y: f32,
                         reach: f32,
                         arms: usize,
                         turn: f32,
                         swirl: f32| {
                for i in 0..arms {
                    let a = turn + i as f32 / arms as f32 * TAU;
                    let tip = (x + reach * a.cos(), y + reach * a.sin());
                    let bend = a + swirl;
                    let ctrl = (x + reach * 0.55 * bend.cos(), y + reach * 0.55 * bend.sin());
                    layers.line(&bezier((x, y), ctrl, tip, 12), v * 0.8);
                    layers.flower(&flower, tip.0, tip.1, f, 0.0);
                }
            };
            // The corners.
            for (x, y) in [
                (path.x0, path.y0),
                (path.x1, path.y0),
                (path.x0, path.y1),
                (path.x1, path.y1),
            ] {
                for k in 0..9 {
                    let a = k as f32 / 9.0 * TAU + rng.range(-0.2, 0.2);
                    let r = u * 0.45 * rng.range(0.4, 1.0);
                    let tip = (x + r * a.cos(), y + r * a.sin());
                    layers.line(&[(x, y), tip], v * 0.8);
                    layers.flower(&flower, tip.0, tip.1, f * 1.2, 0.0);
                }
            }
            // Along each side, mirrored about its middle. CS6's long side
            // carries seven: a corner, a large spray, a small one, the double in
            // the middle, and back again. The short sides have half as many
            // gaps, whatever their length.
            let long_is_width = path.x1 - path.x0 >= path.y1 - path.y0;
            let long_length = (path.x1 - path.x0).max(path.y1 - path.y0);
            let gaps_long = ((long_length / (u * 3.0)).round() as usize).max(2);
            let side =
                |layers: &mut Layers, rng: &mut Rng, a: (f32, f32), b: (f32, f32), long: bool| {
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    let n = if long {
                        gaps_long
                    } else {
                        (gaps_long / 2).max(1)
                    };
                    for i in 1..n {
                        let t = i as f32 / n as f32;
                        let (x, y) = (a.0 + dx * t, a.1 + dy * t);
                        let middle = long && n % 2 == 0 && i == n / 2;
                        let small = !middle && long && i % 2 == 0;
                        let reach = u * if small { 0.55 } else { 1.0 };
                        let turn = rng.range(0.0, TAU);
                        // Swirling one way on the left half and the other on the
                        // right, so the side reads as a mirror image.
                        let swirl = if t < 0.5 { 1.1 } else { -1.1 };
                        spray(layers, x, y, reach, 6, turn, swirl);
                        if middle {
                            spray(layers, x, y, reach, 6, turn + PI / 6.0, -swirl);
                        }
                    }
                };
            side(
                &mut layers,
                &mut rng,
                (path.x0, path.y0),
                (path.x1, path.y0),
                long_is_width,
            );
            side(
                &mut layers,
                &mut rng,
                (path.x0, path.y1),
                (path.x1, path.y1),
                long_is_width,
            );
            side(
                &mut layers,
                &mut rng,
                (path.x0, path.y0),
                (path.x0, path.y1),
                !long_is_width,
            );
            side(
                &mut layers,
                &mut rng,
                (path.x1, path.y0),
                (path.x1, path.y1),
                !long_is_width,
            );
        }
        // Spacing: flowers in a repeating run of large, medium and small.
        _ => {
            let path = track(u * 0.3);
            for (k, s) in path.spread(u * 0.6).into_iter().enumerate() {
                let (p, _) = path.at(s);
                let size = [1.3, 0.85, 0.45][k % 3];
                layers.flower(&flower, p.0, p.1, f * size, 0.0);
            }
        }
    }

    let used = uses(frame);
    if used.vine {
        fill(pixmap, &layers.vine, Rgba8::rgb(options.vine), 1.0);
    }
    if used.leaf && options.leaf != 0 {
        // Little Flowers and Snowflakes are soft, as CS6 draws them.
        let alpha = if matches!(frame, 24 | 25) { 0.7 } else { 1.0 };
        fill(
            pixmap,
            &layers.leaves,
            Rgba8::rgb(options.leaf_colour),
            alpha,
        );
    }
    if used.flower && options.flower != 0 {
        fill(
            pixmap,
            &layers.soft,
            Rgba8::rgb(options.flower_colour),
            0.45,
        );
        let alpha = if matches!(frame, 24 | 25) { 0.8 } else { 1.0 };
        fill(
            pixmap,
            &layers.flowers,
            Rgba8::rgb(options.flower_colour),
            alpha,
        );
    }
}
