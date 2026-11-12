//! Bristle tips: a bundle of tapered tracks drawn as one dab.
//!
//! Adobe's bristle physics is closed. This is a behavioural model: each track
//! has a root on a shape profile, runs along the stroke with a fan angle, and
//! bends inward as Stiffness falls. Track randomness is seeded from the stroke
//! seed and the dab index, so a replayed stroke paints the same pixels.

/// The documented round/flat × plain/fan/curve families. The exact CS6
/// enumeration is unpublished; the order is this project's choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BristleShape {
    #[default]
    Round,
    RoundFan,
    RoundCurve,
    Flat,
    FlatFan,
    FlatCurve,
}

impl BristleShape {
    pub const ALL: [BristleShape; 6] = [
        BristleShape::Round,
        BristleShape::RoundFan,
        BristleShape::RoundCurve,
        BristleShape::Flat,
        BristleShape::FlatFan,
        BristleShape::FlatCurve,
    ];

    pub fn from_index(index: i32) -> Self {
        match index {
            1 => BristleShape::RoundFan,
            2 => BristleShape::RoundCurve,
            3 => BristleShape::Flat,
            4 => BristleShape::FlatFan,
            5 => BristleShape::FlatCurve,
            _ => BristleShape::Round,
        }
    }

    pub fn to_index(self) -> i32 {
        match self {
            BristleShape::Round => 0,
            BristleShape::RoundFan => 1,
            BristleShape::RoundCurve => 2,
            BristleShape::Flat => 3,
            BristleShape::FlatFan => 4,
            BristleShape::FlatCurve => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BristleShape::Round => "Round",
            BristleShape::RoundFan => "Round Fan",
            BristleShape::RoundCurve => "Round Curve",
            BristleShape::Flat => "Flat",
            BristleShape::FlatFan => "Flat Fan",
            BristleShape::FlatCurve => "Flat Curve",
        }
    }

    fn is_flat(self) -> bool {
        matches!(
            self,
            BristleShape::Flat | BristleShape::FlatFan | BristleShape::FlatCurve
        )
    }

    fn is_fan(self) -> bool {
        matches!(self, BristleShape::RoundFan | BristleShape::FlatFan)
    }

    fn is_curve(self) -> bool {
        matches!(self, BristleShape::RoundCurve | BristleShape::FlatCurve)
    }

    /// `(lateral, axial, fan angle in radians)` for profile position
    /// `t` in `-1..=1`.
    fn profile(self, t: f32) -> (f32, f32, f32) {
        let lateral = if self.is_flat() {
            t
        } else {
            (t * std::f32::consts::FRAC_PI_2).sin()
        };
        let axial = if self.is_curve() {
            (t * t - 0.5) * 0.5
        } else if self.is_flat() {
            0.0
        } else {
            (1.0 - t * t) * 0.35 - 0.35
        };
        let fan = if self.is_fan() { t * 0.6 } else { 0.0 };
        (lateral, axial, fan)
    }
}

/// A bristle tip's parameters, all percentages in `0..=100`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BristleTip {
    pub shape: BristleShape,
    pub bristles: u8,
    pub length: u8,
    pub thickness: u8,
    pub stiffness: u8,
}

impl Default for BristleTip {
    fn default() -> Self {
        BristleTip {
            shape: BristleShape::Round,
            bristles: 50,
            length: 50,
            thickness: 50,
            stiffness: 50,
        }
    }
}

impl BristleTip {
    pub fn sanitized(self) -> Self {
        BristleTip {
            shape: self.shape,
            bristles: self.bristles.min(100),
            length: self.length.min(100),
            thickness: self.thickness.min(100),
            stiffness: self.stiffness.min(100),
        }
    }

    /// One bristle at minimum, 64 at 100 %.
    pub fn count(self) -> usize {
        1 + (self.bristles.min(100) as usize * 63) / 100
    }

    pub fn track_length(self, radius: f32) -> f32 {
        radius * (0.6 + self.length.min(100) as f32 / 100.0 * 1.4)
    }

    pub fn track_half_width(self, radius: f32) -> f32 {
        (radius * (0.02 + self.thickness.min(100) as f32 / 100.0 * 0.08)).max(0.35)
    }

    /// The farthest any track reaches from the dab centre.
    pub fn reach(self, diameter: f32) -> f32 {
        let radius = diameter.max(1.0) * 0.5;
        radius + self.track_length(radius)
    }

    pub fn tracks(
        self,
        diameter: f32,
        tangent_deg: f32,
        pressure: f32,
        seed: u64,
    ) -> Vec<BristleTrack> {
        let radius = diameter.max(1.0) * 0.5;
        let count = self.count();
        let length = self.track_length(radius);
        let half_width = self.track_half_width(radius);
        let stiffness = self.stiffness.min(100) as f32 / 100.0;
        let bend = (1.0 - stiffness) * (0.35 + 0.65 * pressure.clamp(0.0, 1.0));
        let direction = tangent_deg.to_radians();
        let (sin, cos) = direction.sin_cos();
        let mut tracks = Vec::with_capacity(count);
        for i in 0..count {
            let t = if count <= 1 {
                0.0
            } else {
                2.0 * i as f32 / (count - 1) as f32 - 1.0
            };
            let (lateral, axial, fan) = self.shape.profile(t);
            let h = hash(seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let wobble = (unit(h) - 0.5) * 0.16;
            let length_jitter = 0.85 + unit(hash(h)) * 0.3;
            let root_lateral = (lateral + wobble * 0.12) * radius * 0.92;
            let root_axial = axial * radius;
            let root = [
                -sin * root_lateral + cos * root_axial,
                cos * root_lateral + sin * root_axial,
            ];
            let track_angle = direction + fan + wobble * 0.2;
            let (track_sin, track_cos) = track_angle.sin_cos();
            let tip = [
                root[0] + track_cos * length * length_jitter,
                root[1] + track_sin * length * length_jitter,
            ];
            // Low stiffness bends each track inward and back along the stroke.
            let bend_lateral = bend * lateral * radius * 0.45;
            let tip = [
                tip[0] - track_sin * bend_lateral,
                tip[1] + track_cos * bend_lateral,
            ];
            let width_jitter = 0.8 + unit(hash(h ^ 0xA5A5_A5A5_A5A5_A5A5)) * 0.4;
            tracks.push(BristleTrack {
                root,
                tip,
                half_width: half_width * width_jitter,
            });
        }
        tracks
    }
}

/// One tapered bristle track in dab-local coordinates.
#[derive(Clone, Copy, Debug)]
pub struct BristleTrack {
    pub root: [f32; 2],
    pub tip: [f32; 2],
    pub half_width: f32,
}

impl BristleTrack {
    fn coverage(&self, dx: f32, dy: f32) -> f32 {
        let [ax, ay] = self.root;
        let [bx, by] = self.tip;
        let (vx, vy) = (bx - ax, by - ay);
        let len2 = vx * vx + vy * vy;
        let (px, py) = (dx - ax, dy - ay);
        let t = if len2 <= f32::EPSILON {
            0.0
        } else {
            ((px * vx + py * vy) / len2).clamp(0.0, 1.0)
        };
        let cx = ax + vx * t;
        let cy = ay + vy * t;
        let distance = ((dx - cx) * (dx - cx) + (dy - cy) * (dy - cy)).sqrt();
        let half_width = self.half_width * (1.0 - 0.65 * t);
        smoothstep((half_width - distance) / 0.75 + 0.5)
    }
}

/// One bristle dab's tracks, rebuilt from the tip and tangent.
pub struct BristleDab {
    tracks: Vec<BristleTrack>,
    reach: f32,
}

impl BristleDab {
    pub fn new(tip: BristleTip, diameter: f32, tangent_deg: f32, pressure: f32, seed: u64) -> Self {
        let reach = tip.reach(diameter);
        BristleDab {
            tracks: tip.tracks(diameter, tangent_deg, pressure, seed),
            reach,
        }
    }

    pub fn reach(&self) -> f32 {
        self.reach
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn coverage(&self, dx: f32, dy: f32) -> f32 {
        self.best_track(dx, dy).0
    }

    /// The strongest covering track's `(coverage, index)`.
    pub fn best_track(&self, dx: f32, dy: f32) -> (f32, usize) {
        let mut best = (0.0f32, 0usize);
        for (i, track) in self.tracks.iter().enumerate() {
            let c = track.coverage(dx, dy);
            if c > best.0 {
                best = (c, i);
                if c >= 1.0 {
                    break;
                }
            }
        }
        best
    }

    #[cfg(test)]
    pub fn tracks(&self) -> &[BristleTrack] {
        &self.tracks
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn unit(x: u64) -> f32 {
    (hash(x) >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, PsdRect};

    fn transparent_doc(w: u32, h: u32) -> Document {
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        let n = (w * h) as usize;
        let _ = doc.layers.push(Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            channels: [0, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![0; n].into(),
                })
                .into(),
            ..Default::default()
        });
        doc
    }

    fn alpha_plane(doc: &Document) -> Vec<u8> {
        doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == -1)
            .map(|c| c.data.as_slice().to_vec())
            .unwrap_or_default()
    }

    fn tip() -> BristleTip {
        BristleTip {
            shape: BristleShape::Round,
            bristles: 40,
            length: 50,
            thickness: 30,
            stiffness: 50,
        }
    }

    #[test]
    fn shape_indices_round_trip() {
        for (index, shape) in BristleShape::ALL.iter().enumerate() {
            assert_eq!(shape.to_index(), index as i32);
            assert_eq!(BristleShape::from_index(index as i32), *shape);
            assert!(!shape.name().is_empty());
        }
        assert_eq!(BristleShape::from_index(99), BristleShape::Round);
    }

    #[test]
    fn bristle_tip_sanitizes() {
        let t = BristleTip {
            bristles: 200,
            length: 200,
            thickness: 200,
            stiffness: 200,
            ..BristleTip::default()
        }
        .sanitized();
        assert_eq!(
            (t.bristles, t.length, t.thickness, t.stiffness),
            (100, 100, 100, 100)
        );
    }

    #[test]
    fn zero_bristles_still_paint() {
        let t = BristleTip {
            bristles: 0,
            ..BristleTip::default()
        };
        assert_eq!(t.count(), 1);
        let dab = BristleDab::new(t, 20.0, 0.0, 1.0, 1);
        let painted =
            (-20..=20).any(|i| (-20..=20).any(|j| dab.coverage(i as f32, j as f32) > 0.0));
        assert!(painted);
    }

    #[test]
    fn bristles_changes_the_streak_count() {
        let low = BristleTip {
            bristles: 10,
            ..tip()
        };
        let high = BristleTip {
            bristles: 90,
            ..tip()
        };
        assert!(high.count() > low.count());
    }

    #[test]
    fn length_and_thickness_change_the_mark() {
        let short = BristleTip {
            length: 10,
            ..tip()
        };
        let long = BristleTip {
            length: 100,
            ..tip()
        };
        assert!(long.reach(40.0) > short.reach(40.0));

        let thin = BristleTip {
            thickness: 10,
            ..tip()
        };
        let thick = BristleTip {
            thickness: 100,
            ..tip()
        };
        assert!(thick.track_half_width(40.0) > thin.track_half_width(40.0));
    }

    #[test]
    fn flat_and_round_marks_differ() {
        let round = BristleDab::new(
            BristleTip {
                shape: BristleShape::Round,
                bristles: 80,
                ..tip()
            },
            60.0,
            0.0,
            1.0,
            7,
        );
        let flat = BristleDab::new(
            BristleTip {
                shape: BristleShape::Flat,
                bristles: 80,
                ..tip()
            },
            60.0,
            0.0,
            1.0,
            7,
        );
        let slice = |dab: &BristleDab| {
            let mut painted = 0;
            for i in -40..=40 {
                if dab.coverage(30.0, i as f32 * 0.5) > 0.0 {
                    painted += 1;
                }
            }
            painted
        };
        assert!(slice(&flat) > slice(&round));
    }

    #[test]
    fn low_stiffness_deforms_the_tip() {
        let low = BristleDab::new(
            BristleTip {
                stiffness: 0,
                ..tip()
            },
            60.0,
            0.0,
            1.0,
            3,
        );
        let high = BristleDab::new(
            BristleTip {
                stiffness: 100,
                ..tip()
            },
            60.0,
            0.0,
            1.0,
            3,
        );
        let displaced = low
            .tracks()
            .iter()
            .zip(high.tracks())
            .any(|(a, b)| (a.tip[1] - b.tip[1]).abs() > 1.0);
        assert!(displaced, "low stiffness did not move any track tip");
    }

    #[test]
    fn high_stiffness_keeps_the_shape() {
        let a = BristleDab::new(
            BristleTip {
                stiffness: 100,
                ..tip()
            },
            60.0,
            0.0,
            1.0,
            5,
        );
        let b = BristleDab::new(
            BristleTip {
                stiffness: 100,
                ..tip()
            },
            60.0,
            90.0,
            1.0,
            5,
        );
        // With no bend, the track length is the same at either tangent.
        let len = |t: &BristleTrack| {
            let (dx, dy) = (t.tip[0] - t.root[0], t.tip[1] - t.root[1]);
            (dx * dx + dy * dy).sqrt()
        };
        for (x, y) in a.tracks().iter().zip(b.tracks()) {
            assert!((len(x) - len(y)).abs() < 0.01);
        }
    }

    #[test]
    fn bristle_stroke_shows_streaks() {
        let mut cfg = crate::StrokeConfig {
            diameter: 80,
            ..crate::StrokeConfig::default()
        };
        cfg.bristle = Some(BristleTip {
            bristles: 20,
            thickness: 20,
            ..BristleTip::default()
        });
        let mut doc = transparent_doc(120, 80);
        crate::paint_stroke(
            &mut doc,
            "0",
            &cfg,
            &[
                crate::StrokeSample {
                    x: 20.0,
                    y: 40.0,
                    pressure: 1.0,
                },
                crate::StrokeSample {
                    x: 100.0,
                    y: 40.0,
                    pressure: 1.0,
                },
            ],
        );
        let alpha = alpha_plane(&doc);
        let column: Vec<u8> = (0..80).map(|y| alpha[y * 120 + 60]).collect();
        let runs = column
            .split(|&a| a == 0)
            .filter(|run| !run.is_empty())
            .count();
        assert!(runs > 1, "one smooth run, not bristle streaks: {runs}");
    }

    #[test]
    fn spacing_still_separates_dabs() {
        let mut cfg = crate::StrokeConfig {
            diameter: 40,
            spacing: crate::spacing::SpacingMode::Fixed(1000),
            ..crate::StrokeConfig::default()
        };
        cfg.bristle = Some(BristleTip {
            length: 10,
            ..BristleTip::default()
        });
        let mut doc = transparent_doc(160, 80);
        crate::paint_stroke(
            &mut doc,
            "0",
            &cfg,
            &[
                crate::StrokeSample {
                    x: 30.0,
                    y: 40.0,
                    pressure: 1.0,
                },
                crate::StrokeSample {
                    x: 130.0,
                    y: 40.0,
                    pressure: 1.0,
                },
            ],
        );
        let alpha = alpha_plane(&doc);
        assert_eq!(alpha[40 * 160 + 80], 0, "the midpoint was painted");
    }

    #[test]
    fn bristle_stroke_replays_exactly() {
        let mut cfg = crate::StrokeConfig {
            diameter: 50,
            ..crate::StrokeConfig::default()
        };
        cfg.bristle = Some(tip());
        let samples = [
            crate::StrokeSample {
                x: 20.0,
                y: 30.0,
                pressure: 1.0,
            },
            crate::StrokeSample {
                x: 80.0,
                y: 50.0,
                pressure: 1.0,
            },
        ];
        let mut a = transparent_doc(110, 80);
        let mut b = transparent_doc(110, 80);
        crate::paint_stroke(&mut a, "0", &cfg, &samples);
        crate::paint_stroke(&mut b, "0", &cfg, &samples);
        assert_eq!(alpha_plane(&a), alpha_plane(&b));
    }

    #[test]
    fn smooth_tip_is_unchanged() {
        let mut cfg = crate::StrokeConfig {
            diameter: 30,
            hardness: 60,
            ..crate::StrokeConfig::default()
        };
        let samples = [
            crate::StrokeSample {
                x: 10.0,
                y: 10.0,
                pressure: 1.0,
            },
            crate::StrokeSample {
                x: 40.0,
                y: 20.0,
                pressure: 1.0,
            },
        ];
        let mut a = transparent_doc(60, 40);
        crate::paint_stroke(&mut a, "0", &cfg, &samples);
        cfg.bristle = None;
        let mut b = transparent_doc(60, 40);
        crate::paint_stroke(&mut b, "0", &cfg, &samples);
        assert_eq!(alpha_plane(&a), alpha_plane(&b));
    }
}
