//! Colour replacement, the engine behind the Color Replacement tool
//! (`docs/03-tools/color-replacement.md`). A dab paints the foreground only
//! onto pixels that resemble a sampled reference colour, and only the part of
//! them the Mode affects: under Color the pixel keeps its luminosity, so
//! recoloured material keeps its shading.
//!
//! What gets replaced depends on what is already there, so unlike a Brush
//! stroke this applies per dab straight into the layer, with a record of the
//! pixels already finished so a slow drag does not build up.
//!
//! Behavioural parity only: the match test is a per-channel distance (the spec
//! leaves the metric open) and Find Edges stops at a luminance step.
//!
//! Ported from photorust's `core/src/replace.rs` and `core/src/sample.rs`
//! (<https://github.com/perfecto25/photorust>).

use crate::healing::RgbaImage;
use crate::{tip_coverage, Rgba, StrokeConfig};
use pictura_core::nonseparable::{lum, sat, set_lum, set_sat};
use pictura_core::PsdRect;
use std::collections::VecDeque;

/// Which part of the pixel the replacement affects. CS6's Mode menu.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReplaceMode {
    Hue,
    Saturation,
    /// Hue and saturation, keeping the pixel's brightness; the spec's advised
    /// setting.
    #[default]
    Color,
    Luminosity,
}

impl ReplaceMode {
    /// 0 Hue, 1 Saturation, 2 Color, 3 Luminosity (the menu order).
    pub fn from_i32(v: i32) -> Option<ReplaceMode> {
        [
            ReplaceMode::Hue,
            ReplaceMode::Saturation,
            ReplaceMode::Color,
            ReplaceMode::Luminosity,
        ]
        .get(usize::try_from(v).ok()?)
        .copied()
    }

    fn blend(self, base: [f32; 3], paint: [f32; 3]) -> [f32; 3] {
        match self {
            ReplaceMode::Hue => set_lum(set_sat(paint, sat(base)), lum(base)),
            ReplaceMode::Saturation => set_lum(set_sat(base, sat(paint)), lum(base)),
            ReplaceMode::Color => set_lum(paint, lum(base)),
            ReplaceMode::Luminosity => set_lum(base, lum(paint)),
        }
    }
}

/// Where the reference colour comes from. CS6's Sampling buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sampling {
    /// Re-read under the brush centre at every dab.
    #[default]
    Continuous,
    /// Read once, where the stroke began.
    Once,
    /// The background colour; nothing is sampled from the image.
    BackgroundSwatch,
}

impl Sampling {
    /// 0 Continuous, 1 Once, 2 Background Swatch (the button order).
    pub fn from_i32(v: i32) -> Option<Sampling> {
        [
            Sampling::Continuous,
            Sampling::Once,
            Sampling::BackgroundSwatch,
        ]
        .get(usize::try_from(v).ok()?)
        .copied()
    }
}

/// How far a match may spread within a dab. CS6's Limits menu.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Limits {
    /// Every matching pixel under the brush, connected or not.
    Discontiguous,
    /// Only matching pixels joined to the one under the brush centre.
    #[default]
    Contiguous,
    /// As Contiguous, but not across a strong edge.
    FindEdges,
}

impl Limits {
    /// 0 Discontiguous, 1 Contiguous, 2 Find Edges (the menu order).
    pub fn from_i32(v: i32) -> Option<Limits> {
        [Limits::Discontiguous, Limits::Contiguous, Limits::FindEdges]
            .get(usize::try_from(v).ok()?)
            .copied()
    }
}

/// The Color Replacement options bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReplaceOptions {
    pub mode: ReplaceMode,
    pub sampling: Sampling,
    pub limits: Limits,
    /// 0–255: how far a pixel may differ per channel and still match.
    pub tolerance: u8,
    /// Taper the match toward the tolerance instead of cutting it hard.
    pub antialias: bool,
}

impl Default for ReplaceOptions {
    fn default() -> Self {
        // The bar shows Tolerance 30%; the rest are the spec's inferred defaults.
        ReplaceOptions {
            mode: ReplaceMode::Color,
            sampling: Sampling::Continuous,
            limits: Limits::Contiguous,
            tolerance: 77,
            antialias: true,
        }
    }
}

/// Normalised luminance step above which Find Edges refuses to spread.
const EDGE_LIMIT: f32 = 0.35;

/// One colour-replacement stroke's state over a layer's pixels.
pub(crate) struct ColorReplacer {
    options: ReplaceOptions,
    /// The colour being replaced, once known (Once, Background Swatch).
    reference: Option<[u8; 4]>,
    /// Pixels fully replaced this stroke; overlapping dabs skip them.
    done: Vec<bool>,
}

impl ColorReplacer {
    /// Start a stroke over a `len`-pixel layer; `background` is the reference
    /// under Background Swatch sampling.
    pub(crate) fn new(len: usize, options: ReplaceOptions, background: Rgba) -> Self {
        let reference = (options.sampling == Sampling::BackgroundSwatch).then_some([
            background.r,
            background.g,
            background.b,
            background.a,
        ]);
        ColorReplacer {
            options,
            reference,
            done: vec![false; len],
        }
    }

    /// Pad the "already replaced" mask when the layer grows (#1208).
    pub(crate) fn grow(
        &mut self,
        old_w: usize,
        old_h: usize,
        new_w: usize,
        new_h: usize,
        dx: usize,
        dy: usize,
    ) {
        let mut done = vec![false; new_w * new_h];
        for y in 0..old_h {
            let src = y * old_w;
            let dst = (dy + y) * new_w + dx;
            if src + old_w <= self.done.len() && dst + old_w <= done.len() {
                done[dst..dst + old_w].copy_from_slice(&self.done[src..src + old_w]);
            }
        }
        self.done = done;
    }

    /// Apply one dab centred at layer-local `(cx, cy)`, painting `paint`.
    /// Returns the layer-local rectangle changed.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
        paint: Rgba,
    ) -> Option<PsdRect> {
        let region = dab_bounds(img, cfg, cx, cy)?;
        let centre = (cx.floor() as i32, cy.floor() as i32);
        let inside = |(x, y): (i32, i32)| {
            x >= region.left && x < region.right && y >= region.top && y < region.bottom
        };
        let reference = match self.reference {
            Some(fixed) if self.options.sampling != Sampling::Continuous => fixed,
            _ => {
                if !inside(centre) {
                    return None;
                }
                let sampled = img.get(centre.0, centre.1);
                if self.options.sampling == Sampling::Once {
                    self.reference = Some(sampled);
                }
                sampled
            }
        };
        let reachable = match self.options.limits {
            Limits::Discontiguous => None,
            // The flood starts under the brush centre; off-image it has nowhere
            // to start.
            _ if !inside(centre) => return None,
            limits => Some(reachable(
                img,
                cfg,
                region,
                centre,
                (cx, cy),
                |p| self.match_strength(p, reference),
                limits,
            )),
        };
        let paint = [paint.r, paint.g, paint.b].map(|c| c as f32 / 255.0);
        let rw = region.width() as usize;
        let mut dirty: Option<PsdRect> = None;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let index = (y * img.width + x) as usize;
                if self.done[index] {
                    continue;
                }
                if let Some(mask) = &reachable {
                    if !mask[(y - region.top) as usize * rw + (x - region.left) as usize] {
                        continue;
                    }
                }
                let tip = tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let base = img.get(x, y);
                // A transparent pixel has no colour to replace.
                if tip <= 0.0 || base[3] == 0 {
                    continue;
                }
                let weight = (tip * self.match_strength(base, reference)).clamp(0.0, 1.0);
                if weight <= 0.0 {
                    continue;
                }
                let from = [base[0], base[1], base[2]].map(|c| c as f32 / 255.0);
                let blended = self.options.mode.blend(from, paint);
                let out = |c: usize| {
                    ((from[c] + (blended[c] - from[c]) * weight) * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                img.set(x, y, [out(0), out(1), out(2), base[3]]);
                // Partly covered pixels stay open for the rest of the stroke.
                if weight >= 0.995 {
                    self.done[index] = true;
                }
                dirty = Some(grow(dirty, x, y));
            }
        }
        dirty
    }

    fn match_strength(&self, pixel: [u8; 4], reference: [u8; 4]) -> f32 {
        match_strength(
            pixel,
            reference,
            self.options.tolerance,
            self.options.antialias,
        )
    }
}

/// How strongly `pixel` matches `reference`, `0.0..=1.0`: in or out at the
/// per-channel `tolerance`, or with `antialias` solid to 70% of it and fading
/// to zero.
pub(crate) fn match_strength(
    pixel: [u8; 4],
    reference: [u8; 4],
    tolerance: u8,
    antialias: bool,
) -> f32 {
    let distance = (0..3)
        .map(|c| pixel[c].abs_diff(reference[c]))
        .max()
        .unwrap_or(0) as f32;
    let tolerance = tolerance.max(1) as f32;
    if !antialias {
        return if distance <= tolerance { 1.0 } else { 0.0 };
    }
    let solid = tolerance * 0.7;
    if distance <= solid {
        1.0
    } else if distance >= tolerance {
        0.0
    } else {
        1.0 - (distance - solid) / (tolerance - solid)
    }
}

/// The pixels of `region` inside the dab reachable from `centre` through
/// pixels `matches` accepts (and, for Find Edges, without a strong luminance
/// step). Resolved per dab, so its cost follows the brush, not the canvas.
pub(crate) fn reachable(
    img: &RgbaImage,
    cfg: &StrokeConfig,
    region: PsdRect,
    centre: (i32, i32),
    (cx, cy): (f32, f32),
    matches: impl Fn([u8; 4]) -> f32,
    limits: Limits,
) -> Vec<bool> {
    let w = region.width() as usize;
    let index = |x: i32, y: i32| (y - region.top) as usize * w + (x - region.left) as usize;
    let luma = |p: [u8; 4]| lum([p[0], p[1], p[2]].map(|c| c as f32 / 255.0));
    let mut reached = vec![false; w * region.height() as usize];
    reached[index(centre.0, centre.1)] = true;
    let mut queue = VecDeque::from([centre]);
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < region.left || nx >= region.right || ny < region.top || ny >= region.bottom {
                continue;
            }
            let next = index(nx, ny);
            if reached[next]
                || tip_coverage(cfg, nx as f32 + 0.5 - cx, ny as f32 + 0.5 - cy) <= 0.0
                || matches(img.get(nx, ny)) <= 0.0
            {
                continue;
            }
            if limits == Limits::FindEdges
                && (luma(img.get(nx, ny)) - luma(img.get(x, y))).abs() > EDGE_LIMIT
            {
                continue;
            }
            reached[next] = true;
            queue.push_back((nx, ny));
        }
    }
    reached
}

pub(crate) fn dab_bounds(img: &RgbaImage, cfg: &StrokeConfig, cx: f32, cy: f32) -> Option<PsdRect> {
    let radius = cfg.bristle.map_or(cfg.diameter as f32 * 0.5, |tip| {
        tip.reach(cfg.diameter as f32)
    });
    let region = PsdRect {
        top: ((cy - radius).floor() as i32).max(0),
        left: ((cx - radius).floor() as i32).max(0),
        bottom: ((cy + radius).ceil() as i32 + 1).min(img.height),
        right: ((cx + radius).ceil() as i32 + 1).min(img.width),
    };
    (region.width() > 0 && region.height() > 0).then_some(region)
}

pub(crate) fn grow(r: Option<PsdRect>, x: i32, y: i32) -> PsdRect {
    match r {
        None => PsdRect {
            top: y,
            left: x,
            bottom: y + 1,
            right: x + 1,
        },
        Some(r) => PsdRect {
            top: r.top.min(y),
            left: r.left.min(x),
            bottom: r.bottom.max(y + 1),
            right: r.right.max(x + 1),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: i32, h: i32, f: impl Fn(i32, i32) -> [u8; 4]) -> RgbaImage {
        RgbaImage {
            width: w,
            height: h,
            data: (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| f(x, y))
                .collect(),
        }
    }

    fn brush(diameter: u32) -> StrokeConfig {
        StrokeConfig {
            diameter,
            ..StrokeConfig::default()
        }
    }

    fn options(tolerance: u8) -> ReplaceOptions {
        ReplaceOptions {
            sampling: Sampling::Continuous,
            limits: Limits::Discontiguous,
            tolerance,
            antialias: false,
            ..ReplaceOptions::default()
        }
    }

    fn rgba(r: u8, g: u8, b: u8) -> Rgba {
        Rgba { r, g, b, a: 255 }
    }

    fn replace(img: &mut RgbaImage, o: ReplaceOptions, d: u32, at: (f32, f32), paint: Rgba) {
        let mut r = ColorReplacer::new(img.data.len(), o, rgba(255, 255, 255));
        r.dab(img, &brush(d), at.0, at.1, paint);
    }

    fn luma(p: [u8; 4]) -> f32 {
        0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
    }

    #[test]
    fn color_mode_keeps_the_shading() {
        let mut img = image(40, 40, |x, _| {
            let v = (60 + x * 3) as u8;
            [v, v, v, 255]
        });
        let before = img.get(14, 20);
        replace(&mut img, options(200), 24, (20.0, 20.0), rgba(200, 40, 40));
        let (dark, light) = (img.get(14, 20), img.get(24, 20));
        assert!(
            dark[0] > dark[1] + 20 && light[0] > light[1] + 20,
            "not recoloured"
        );
        assert!(luma(dark) < luma(light), "the shading was flattened");
        assert!(
            (luma(dark) - luma(before)).abs() < 30.0,
            "brightness shifted"
        );
    }

    #[test]
    fn luminosity_mode_keeps_the_colour() {
        let mut img = image(30, 30, |_, _| [200, 60, 60, 255]);
        let o = ReplaceOptions {
            mode: ReplaceMode::Luminosity,
            ..options(60)
        };
        replace(&mut img, o, 16, (15.0, 15.0), rgba(30, 30, 30));
        let px = img.get(15, 15);
        assert!(px[0] > px[1] + 10 && px[0] < 200, "{px:?}");
    }

    #[test]
    fn only_matching_pixels_are_replaced() {
        let blue = [40, 40, 200, 255];
        let yellow = [220, 220, 40, 255];
        let mut img = image(60, 30, |x, _| if x < 30 { blue } else { yellow });
        replace(&mut img, options(60), 30, (25.0, 15.0), rgba(40, 200, 40));
        assert_ne!(img.get(20, 15), blue, "the blue was not replaced");
        assert_eq!(img.get(35, 15), yellow, "the yellow was altered");
    }

    #[test]
    fn contiguous_stops_at_a_gap_and_discontiguous_jumps_it() {
        let dark = [50, 50, 50, 255];
        let stripes = image(60, 20, |x, _| {
            if !(20..40).contains(&x) {
                dark
            } else {
                [240, 240, 240, 255]
            }
        });
        let run = |limits| {
            let mut img = stripes.clone();
            let o = ReplaceOptions {
                limits,
                tolerance: 40,
                ..options(40)
            };
            replace(&mut img, o, 58, (19.0, 10.0), rgba(200, 40, 40));
            img
        };
        let contiguous = run(Limits::Contiguous);
        assert_ne!(contiguous.get(10, 10), dark, "the near stripe was skipped");
        assert_eq!(contiguous.get(45, 10), dark, "contiguous jumped the gap");
        assert_ne!(run(Limits::Discontiguous).get(45, 10), dark);
    }

    #[test]
    fn find_edges_stops_at_a_luminance_step_contiguous_crosses() {
        // Two dark greys a strong step apart, both within the tolerance.
        let img = image(40, 20, |x, _| {
            if x < 20 {
                [20, 20, 20, 255]
            } else {
                [130, 130, 130, 255]
            }
        });
        let run = |limits| {
            let mut out = img.clone();
            let o = ReplaceOptions {
                limits,
                ..options(120)
            };
            replace(&mut out, o, 38, (10.0, 10.0), rgba(200, 40, 40));
            out
        };
        assert_ne!(run(Limits::Contiguous).get(25, 10), img.get(25, 10));
        assert_eq!(run(Limits::FindEdges).get(25, 10), img.get(25, 10));
    }

    #[test]
    fn once_sampling_keeps_the_first_colour() {
        let green = [20, 200, 20, 255];
        let red = [200, 20, 20, 255];
        let mut img = image(60, 20, |x, _| if x < 30 { green } else { red });
        let o = ReplaceOptions {
            sampling: Sampling::Once,
            ..options(40)
        };
        let mut r = ColorReplacer::new(img.data.len(), o, rgba(255, 255, 255));
        r.dab(&mut img, &brush(10), 10.0, 10.0, rgba(20, 20, 200));
        r.dab(&mut img, &brush(10), 45.0, 10.0, rgba(20, 20, 200));
        assert_ne!(img.get(10, 10), green, "the first sample was not replaced");
        assert_eq!(img.get(45, 10), red, "Once re-sampled the red");
    }

    #[test]
    fn background_swatch_matches_the_background_colour() {
        let green = [20, 200, 20, 255];
        let red = [200, 20, 20, 255];
        let mut img = image(40, 20, |_, y| if y < 10 { green } else { red });
        let o = ReplaceOptions {
            sampling: Sampling::BackgroundSwatch,
            ..options(40)
        };
        let mut r = ColorReplacer::new(img.data.len(), o, rgba(20, 200, 20));
        r.dab(&mut img, &brush(30), 20.0, 10.0, rgba(20, 20, 200));
        assert_ne!(img.get(20, 5), green, "the green was not replaced");
        assert_eq!(img.get(20, 15), red, "the red was altered");
    }

    #[test]
    fn repeated_dabs_do_not_compound_and_transparency_is_kept() {
        let mut img = image(30, 30, |x, _| {
            if x < 3 {
                [0, 0, 0, 0]
            } else {
                [120, 120, 120, 255]
            }
        });
        let mut r = ColorReplacer::new(img.data.len(), options(60), rgba(255, 255, 255));
        r.dab(&mut img, &brush(16), 15.0, 15.0, rgba(220, 30, 30));
        let once = img.get(15, 15);
        for _ in 0..5 {
            r.dab(&mut img, &brush(16), 15.0, 15.0, rgba(220, 30, 30));
        }
        assert_eq!(img.get(15, 15), once, "repeated dabs compounded");
        let mut edge = ColorReplacer::new(img.data.len(), options(255), rgba(255, 255, 255));
        edge.dab(&mut img, &brush(16), 4.0, 15.0, rgba(220, 30, 30));
        assert_eq!(
            img.get(1, 15),
            [0, 0, 0, 0],
            "a transparent pixel was painted"
        );
    }

    #[test]
    fn tolerance_widens_the_match() {
        let ramp = image(40, 40, |x, _| {
            let v = (100 + x * 2) as u8;
            [v, v, v, 255]
        });
        let changed = |tolerance| {
            let mut img = ramp.clone();
            replace(
                &mut img,
                options(tolerance),
                36,
                (20.0, 20.0),
                rgba(220, 20, 20),
            );
            img.data
                .iter()
                .zip(&ramp.data)
                .filter(|(a, b)| a != b)
                .count()
        };
        assert!(changed(120) > changed(5) * 2);
    }

    #[test]
    fn menus_round_trip_through_their_integers() {
        assert_eq!(ReplaceMode::from_i32(0), Some(ReplaceMode::Hue));
        assert_eq!(ReplaceMode::from_i32(3), Some(ReplaceMode::Luminosity));
        assert_eq!(ReplaceMode::from_i32(4), None);
        assert_eq!(Sampling::from_i32(2), Some(Sampling::BackgroundSwatch));
        assert_eq!(Limits::from_i32(2), Some(Limits::FindEdges));
        assert_eq!(Limits::from_i32(-1), None);
    }
}
