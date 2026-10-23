//! Wet paint, the engine behind the Mixer Brush (`docs/03-tools/mixer-brush.md`).
//! Two colours meet at every dab: the **reservoir**, the paint the brush is
//! loaded with, and the **pickup**, the colour it finds under the tip.
//!
//! * **Wet** — how wet the canvas is. At 0 the paint sits on top (an ordinary
//!   brush); higher, the canvas colour joins in and is dragged along.
//! * **Load** — how much paint the brush holds. It runs down over the stroke;
//!   an empty dry brush stops, an empty wet brush keeps smearing.
//! * **Mix** — the canvas share of the deposit once the canvas is wet: 0 is
//!   pure reservoir, 100 pure pickup (a smear with no colour of its own).
//! * **Flow** — how fast each dab deposits.
//!
//! Every dab reads what the previous ones left, so like colour replacement
//! this applies per dab straight into the layer. The reservoir outlives the
//! stroke (the brush stays loaded until cleaned or reloaded); the caller
//! carries it.
//!
//! Behavioural parity only: Adobe's mixing and dry-out math is closed; the
//! consumption and pickup rates below are photorust's.
//!
//! Ported from photorust's `core/src/mixer.rs`
//! (<https://github.com/perfecto25/photorust>).

use crate::healing::RgbaImage;
use crate::replace::{dab_bounds, grow};
use crate::spacing::SpacingMode;
use crate::{tip_coverage, Rgba, StrokeConfig};
use pictura_core::PsdRect;

/// Wet, Load, Mix, and Flow, each `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerOptions {
    pub wet: f32,
    pub load: f32,
    pub mix: f32,
    pub flow: f32,
}

impl Default for MixerOptions {
    /// photorust's "Dry" preset (Wet 0, Load 50, Mix 0, Flow 100); the spec
    /// leaves CS6's defaults open.
    fn default() -> Self {
        MixerOptions {
            wet: 0.0,
            load: 0.5,
            mix: 0.0,
            flow: 1.0,
        }
    }
}

/// Load a fully wet brush spends per dab at 25% spacing.
const DAB_CONSUMPTION: f32 = 0.02;

/// The spacing [`DAB_CONSUMPTION`] is quoted at; paint is spent per distance,
/// not per dab.
const REFERENCE_SPACING: f32 = 0.25;

/// How readily the reservoir takes on the colour it passes over, scaled by Wet
/// and Mix: this is what carries colour along a smear.
const PICKUP_RATE: f32 = 0.5;

/// One Mixer Brush stroke's state.
pub(crate) struct MixerBrush {
    options: MixerOptions,
    /// The paint on the brush, straight alpha; alpha 0 is a clean brush.
    reservoir: [u8; 4],
    /// What is left of the load, `0.0..=1.0`.
    paint: f32,
    /// A transparency lock: colour may change, coverage may not.
    preserve_alpha: bool,
}

impl MixerBrush {
    pub(crate) fn new(options: MixerOptions, reservoir: Rgba, preserve_alpha: bool) -> Self {
        MixerBrush {
            options,
            reservoir: [reservoir.r, reservoir.g, reservoir.b, reservoir.a],
            paint: options.load.clamp(0.0, 1.0),
            preserve_alpha,
        }
    }

    pub(crate) fn reservoir(&self) -> Rgba {
        let [r, g, b, a] = self.reservoir;
        Rgba { r, g, b, a }
    }

    /// Apply one dab centred at layer-local `(cx, cy)`. Returns the layer-local
    /// rectangle changed.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
    ) -> Option<PsdRect> {
        let region = dab_bounds(img, cfg, cx, cy)?;
        let wet = self.options.wet.clamp(0.0, 1.0);
        let mix = self.options.mix.clamp(0.0, 1.0);
        let flow = self.options.flow.clamp(0.0, 1.0);
        // A dry brush with nothing on it has nothing to give or smear with.
        if flow <= 0.0 || (wet <= 0.0 && self.paint <= 0.0) {
            return None;
        }
        let tip = |x: i32, y: i32| tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);

        // The pickup is the tip-weighted average under the dab, so the tool
        // blends rather than clones.
        let pickup = average_under(img, region, tip);
        // Dry paints the reservoir as it is; wet paint is thinned with the
        // canvas in the Mix ratio, sliding to pure canvas as the load runs out.
        let canvas_share = if wet <= 0.0 {
            0.0
        } else {
            mix + (1.0 - mix) * (1.0 - self.paint.min(1.0))
        };
        let deposit = pickup.map_or(self.reservoir, |found| {
            lerp(self.reservoir, found, canvas_share)
        });
        // A wet canvas resists: even at Wet 100 a dab carries only so far,
        // which leaves the streaked, worked look.
        let deposit_alpha = flow * (1.0 - wet * 0.5);

        let mut dirty = None;
        let opacity = cfg.opacity as f32 / 100.0;
        let source = Rgba {
            r: deposit[0],
            g: deposit[1],
            b: deposit[2],
            a: deposit[3],
        };
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let weight = (tip(x, y) * deposit_alpha * opacity).clamp(0.0, 1.0);
                let existing = img.get(x, y);
                if weight <= 0.0 || (self.preserve_alpha && existing[3] == 0) {
                    continue;
                }
                let roll = if cfg.mode == crate::PaintMode::Dissolve {
                    let hash =
                        (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
                    hash as f32 / u32::MAX as f32
                } else {
                    0.0
                };
                let Some((r, g, b, a)) = crate::stroke::blend_pixel(
                    cfg.mode,
                    (existing[0], existing[1], existing[2], existing[3]),
                    &source,
                    weight,
                    roll,
                ) else {
                    continue;
                };
                let mut mixed = [r, g, b, a];
                if self.preserve_alpha {
                    mixed[3] = existing[3];
                }
                if mixed == existing {
                    continue;
                }
                img.set(x, y, mixed);
                dirty = Some(grow(dirty, x, y));
            }
        }

        // A dry brush neither picks up nor runs out: Load has no effect until
        // Wet is above zero.
        if wet > 0.0 {
            if let Some(found) = pickup {
                self.reservoir = lerp(self.reservoir, found, wet * mix * PICKUP_RATE);
            }
            let spacing = match cfg.spacing {
                SpacingMode::Fixed(percent) => percent as f32 / 100.0,
                SpacingMode::VelocityDriven => REFERENCE_SPACING,
            };
            let travel = (spacing.max(0.01) / REFERENCE_SPACING).clamp(0.05, 4.0);
            self.paint = (self.paint - wet * DAB_CONSUMPTION * travel).max(0.0);
        }
        dirty
    }
}

/// The tip-weighted average colour under a dab, or `None` over nothing but
/// transparency. Averaged premultiplied, so the black behind cleared pixels
/// does not drag the colour down.
fn average_under(
    img: &RgbaImage,
    region: PsdRect,
    tip: impl Fn(i32, i32) -> f32,
) -> Option<[u8; 4]> {
    let (mut sum, mut alpha, mut total) = ([0.0f32; 3], 0.0f32, 0.0f32);
    for y in region.top..region.bottom {
        for x in region.left..region.right {
            let cover = tip(x, y);
            if cover <= 0.0 {
                continue;
            }
            let px = img.get(x, y);
            let a = px[3] as f32 / 255.0;
            for c in 0..3 {
                sum[c] += px[c] as f32 * a * cover;
            }
            alpha += a * cover;
            total += cover;
        }
    }
    if total <= 0.0 || alpha <= 1e-6 {
        return None;
    }
    let to_u8 = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    Some([
        to_u8(sum[0] / alpha),
        to_u8(sum[1] / alpha),
        to_u8(sum[2] / alpha),
        to_u8(alpha / total * 255.0),
    ])
}

/// Straight-alpha interpolation from `from` toward `to`.
fn lerp(from: [u8; 4], to: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|c| {
        (from[c] as f32 + (to[c] as f32 - from[c] as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: [u8; 4] = [255, 255, 255, 255];

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

    fn brush() -> StrokeConfig {
        StrokeConfig {
            diameter: 10,
            ..StrokeConfig::default()
        }
    }

    fn opts(wet: f32, load: f32, mix: f32) -> MixerOptions {
        MixerOptions {
            wet,
            load,
            mix,
            flow: 1.0,
        }
    }

    fn rgba([r, g, b, a]: [u8; 4]) -> Rgba {
        Rgba { r, g, b, a }
    }

    fn drag(img: &mut RgbaImage, m: &mut MixerBrush, xs: std::ops::Range<i32>) {
        for x in xs {
            m.dab(img, &brush(), x as f32, 16.0);
        }
    }

    #[test]
    fn a_dry_brush_paints_its_own_paint() {
        let mut img = image(32, 32, |_, _| WHITE);
        let red = [220, 20, 20, 255];
        let mut m = MixerBrush::new(opts(0.0, 1.0, 0.0), rgba(red), false);
        assert!(m.dab(&mut img, &brush(), 16.0, 16.0).is_some());
        assert_eq!(img.get(16, 16), red);
    }

    #[test]
    fn a_full_mix_smears_without_adding_colour() {
        let mut img = image(32, 32, |_, _| WHITE);
        let mut m = MixerBrush::new(opts(1.0, 1.0, 1.0), rgba([220, 20, 20, 255]), false);
        drag(&mut img, &mut m, 8..16);
        assert_eq!(img.get(12, 16), WHITE, "the load leaked into a pure smear");
    }

    #[test]
    fn a_wet_brush_drags_colour_along_the_stroke() {
        let mut img = image(
            64,
            32,
            |x, _| if x < 20 { [20, 20, 220, 255] } else { WHITE },
        );
        let mut m = MixerBrush::new(opts(0.8, 1.0, 1.0), rgba(WHITE), false);
        drag(&mut img, &mut m, 10..40);
        let past = img.get(28, 16);
        assert!(
            past[2] > past[0] + 10,
            "no blue carried past the edge: {past:?}"
        );
        assert!(past[0] > 40, "the smear stayed pure blue: {past:?}");
    }

    #[test]
    fn an_empty_wet_brush_keeps_smearing_an_empty_dry_one_stops() {
        let mut img = image(64, 32, |x, _| if x < 20 { [0, 0, 0, 255] } else { WHITE });
        let mut wet = MixerBrush::new(opts(1.0, 0.02, 0.5), rgba(WHITE), false);
        drag(&mut img, &mut wet, 10..40);
        assert!(img.get(26, 16)[0] < 250, "the wet brush stopped once empty");

        let mut img = image(32, 32, |_, _| WHITE);
        let mut dry = MixerBrush::new(opts(0.0, 0.0, 0.0), rgba([0, 0, 0, 255]), false);
        assert_eq!(dry.dab(&mut img, &brush(), 16.0, 16.0), None);
        assert_eq!(img.get(16, 16), WHITE);
    }

    #[test]
    fn transparency_does_not_darken_the_pickup_and_a_lock_keeps_alpha() {
        let clear = [0, 0, 0, 0];
        let mut img = image(
            32,
            32,
            |_, y| if y < 16 { [240, 200, 40, 255] } else { clear },
        );
        let mut m = MixerBrush::new(opts(1.0, 1.0, 1.0), rgba(clear), true);
        m.dab(&mut img, &brush(), 16.0, 15.0);
        let px = img.get(16, 12);
        assert!(
            px[0] > 150 && px[1] > 120,
            "the empty half greyed the pickup: {px:?}"
        );
        assert_eq!(
            img.get(16, 18),
            clear,
            "a locked transparent pixel was painted"
        );
    }

    #[test]
    fn the_reservoir_takes_on_what_a_wet_brush_crosses() {
        let mut img = image(64, 32, |_, _| [20, 200, 20, 255]);
        let mut m = MixerBrush::new(opts(1.0, 1.0, 1.0), rgba([200, 20, 20, 255]), false);
        drag(&mut img, &mut m, 10..20);
        let carried = m.reservoir();
        assert!(
            carried.g > carried.r,
            "the reservoir kept its own paint: {carried:?}"
        );
    }

    #[test]
    fn mixer_applies_mode_opacity() {
        let base = [200, 100, 50, 255];
        let deposit = [100, 200, 250, 255];
        let product = |s: u8, d: u8| ((s as u32 * d as u32) / 255) as u8;

        let mut img = image(32, 32, |_, _| base);
        let mut m = MixerBrush::new(opts(0.0, 1.0, 0.0), rgba(deposit), false);
        let mut cfg = brush();
        cfg.mode = crate::PaintMode::Multiply;
        cfg.opacity = 100;
        m.dab(&mut img, &cfg, 16.0, 16.0);
        let full = img.get(16, 16);
        for c in 0..3 {
            let expected = product(deposit[c], base[c]);
            assert!(
                (full[c] as i32 - expected as i32).abs() <= 1,
                "Multiply channel {c}: {full:?} vs {expected}"
            );
        }
        assert_eq!(full[3], 255);

        let mut img = image(32, 32, |_, _| base);
        let mut m = MixerBrush::new(opts(0.0, 1.0, 0.0), rgba(deposit), false);
        cfg.opacity = 50;
        m.dab(&mut img, &cfg, 16.0, 16.0);
        let half = img.get(16, 16);
        for c in 0..3 {
            let expected = ((full[c] as u32 + base[c] as u32) / 2) as u8;
            assert!(
                (half[c] as i32 - expected as i32).abs() <= 1,
                "Opacity 50 channel {c}: {half:?} vs {expected}"
            );
        }
    }
}
