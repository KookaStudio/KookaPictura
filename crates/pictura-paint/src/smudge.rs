//! The Smudge tool's engine (`docs/03-tools/smudge-blur-sharpen.md`): a finger
//! dragged through wet paint.
//!
//! Blur and Sharpen work on a pixel and its neighbours; Smudge **carries the
//! image with it**. The finger holds the patch it picked up at the last dab,
//! lays that patch down where it is now, and picks the result up again for the
//! next one. Structure is dragged along the stroke, which is why a smudged
//! edge streaks in the direction of travel instead of merely going soft.
//!
//! **Strength** is how much of the carried patch one dab lays down: at 0 the
//! finger never touches the canvas, at 100 it replaces what it passes over and
//! drags its starting pixels the whole way. **Finger Painting** loads the
//! finger with the foreground colour first, so the stroke drags paint in.
//!
//! Ported from photorust's `core/src/smudge.rs`
//! (<https://github.com/perfecto25/photorust>). Behavioural parity only.

use crate::focus::{lerp, restrict, RetouchMode};
use crate::healing::RgbaImage;
use crate::replace::{dab_bounds, grow};
use crate::{tip_coverage, StrokeConfig};
use pictura_core::PsdRect;

/// The Smudge options bar: Strength (0.0–1.0), Mode, and Finger Painting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmudgeOptions {
    pub strength: f32,
    pub mode: RetouchMode,
    pub finger_painting: bool,
}

impl Default for SmudgeOptions {
    /// Strength 50 %, Normal, no Finger Painting.
    fn default() -> Self {
        SmudgeOptions {
            strength: 0.5,
            mode: RetouchMode::Normal,
            finger_painting: false,
        }
    }
}

/// The patch on the finger: pixels picked up around the last dab.
struct Carried {
    /// Where the patch sits, in layer coordinates.
    rect: PsdRect,
    pixels: RgbaImage,
    centre: (f32, f32),
}

/// One Smudge stroke's state.
pub(crate) struct SmudgeBrush {
    options: SmudgeOptions,
    /// Sample All Layers: the composite, layer-local, the finger picks up from.
    // ponytail: a snapshot taken at the press, so with Sample All Layers the
    // smear reaches one dab past what the composite showed then.
    sampled: Option<RgbaImage>,
    /// A transparency lock: colour may move, coverage may not.
    preserve_alpha: bool,
    /// The colour a Finger Painting stroke starts loaded with.
    paint: [u8; 4],
    carried: Option<Carried>,
}

impl SmudgeBrush {
    pub(crate) fn new(
        options: SmudgeOptions,
        sampled: Option<RgbaImage>,
        preserve_alpha: bool,
        paint: [u8; 4],
    ) -> Self {
        SmudgeBrush {
            options,
            sampled,
            preserve_alpha,
            paint,
            carried: None,
        }
    }

    /// Smudge one dab centred at layer-local `(cx, cy)`. Returns the
    /// layer-local rectangle changed; the first dab only picks up.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
    ) -> Option<PsdRect> {
        let region = dab_bounds(img, cfg, cx, cy)?;
        let strength = self.options.strength.clamp(0.0, 1.0);
        let mut dirty = None;
        if let Some(carried) = self.carried.as_ref().filter(|_| strength > 0.0) {
            // Each pixel takes the one that sat at the same place under the
            // previous dab: that is what drags the image rather than blurring it.
            let dx = (cx - carried.centre.0).round() as i32;
            let dy = (cy - carried.centre.1).round() as i32;
            let r = carried.rect;
            for y in region.top..region.bottom {
                for x in region.left..region.right {
                    let (sx, sy) = (x - dx, y - dy);
                    if sx < r.left || sy < r.top || sx >= r.right || sy >= r.bottom {
                        continue;
                    }
                    let cover = tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    let dst = img.get(x, y);
                    if cover <= 0.0 || (self.preserve_alpha && dst[3] == 0) {
                        continue;
                    }
                    let src = carried.pixels.get(sx - r.left, sy - r.top);
                    let mut out =
                        lerp(dst, restrict(dst, src, self.options.mode), cover * strength);
                    if self.preserve_alpha {
                        out[3] = dst[3];
                    }
                    if out != dst {
                        img.set(x, y, out);
                        dirty = Some(grow(dirty, x, y));
                    }
                }
            }
        }
        // Pick up after laying down, so the finger carries the smear it just
        // made: that feedback is what drags a slow stroke further. The patch
        // reaches a radius past the dab, room for the next dab's offset.
        let reach = cfg.diameter as i32 / 2 + 1;
        let rect = PsdRect {
            top: (region.top - reach).max(0),
            left: (region.left - reach).max(0),
            bottom: (region.bottom + reach).min(img.height),
            right: (region.right + reach).min(img.width),
        };
        let loaded = self.carried.is_none() && self.options.finger_painting;
        let from = self.sampled.as_ref().unwrap_or(img);
        let (w, h) = (rect.width(), rect.height());
        let data = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                if loaded {
                    self.paint
                } else {
                    from.get(x + rect.left, y + rect.top)
                }
            })
            .collect();
        self.carried = Some(Carried {
            rect,
            pixels: RgbaImage {
                width: w,
                height: h,
                data,
            },
            centre: (cx, cy),
        });
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::tests::{doc, lock, pixel};
    use crate::stroke::layer_rgba;
    use crate::{Rgba, Stroke, StrokeSample};
    use pictura_core::{Document, LockFlags};

    const WHITE: [u8; 4] = [255; 4];

    /// An 80 × 40 layer, black left of x = 20 and white right of it.
    fn bar() -> Document {
        let mut d = doc(80, 40, WHITE);
        for c in &mut d.layers[0].channels[..3] {
            for (i, v) in c.data.iter_mut().enumerate() {
                if i % 80 < 20 {
                    *v = 0;
                }
            }
        }
        d
    }

    /// A 16 px Smudge dragged along row 20 from `from` to `to`, a sample per
    /// pixel; `paint` loads Finger Painting.
    fn smudge(
        d: &Document,
        options: SmudgeOptions,
        sampled: Option<RgbaImage>,
        (from, to): (i32, i32),
    ) -> Document {
        let cfg = StrokeConfig {
            diameter: 16,
            color: Rgba {
                r: 220,
                g: 20,
                b: 20,
                a: 255,
            },
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_smudge(d, "0", cfg, options, sampled).expect("begin");
        let mut out = d.clone();
        for x in from..=to {
            stroke.sample(
                &mut out,
                StrokeSample {
                    x: x as f32,
                    y: 20.0,
                    pressure: 1.0,
                },
            );
        }
        stroke.finish();
        out
    }

    fn at(strength: f32) -> SmudgeOptions {
        SmudgeOptions {
            strength,
            ..SmudgeOptions::default()
        }
    }

    #[test]
    fn a_stroke_drags_colour_along_and_fades_with_distance() {
        let d = smudge(&bar(), at(0.9), None, (12, 50));
        assert!(
            pixel(&d, 40, 20)[0] < 240,
            "no black was carried past the bar"
        );

        let d = smudge(&bar(), at(0.5), None, (12, 60));
        let (near, far) = (pixel(&d, 30, 20)[0], pixel(&d, 50, 20)[0]);
        assert!(far > near, "the smear did not fade: {near} then {far}");
    }

    #[test]
    fn a_stronger_finger_drags_further() {
        let reach = |strength| {
            let d = smudge(&bar(), at(strength), None, (12, 70));
            (20..70).filter(|&x| pixel(&d, x, 20)[0] < 240).count()
        };
        assert!(reach(0.9) > reach(0.3));
    }

    #[test]
    fn zero_strength_and_a_single_dab_change_nothing() {
        let still = |d: &Document| layer_rgba(&d.layers[0]) == layer_rgba(&bar().layers[0]);
        assert!(still(&smudge(&bar(), at(0.0), None, (12, 50))));
        assert!(
            still(&smudge(&bar(), at(1.0), None, (30, 30))),
            "a click stamped a patch"
        );
    }

    #[test]
    fn finger_painting_drags_the_foreground_in() {
        let options = SmudgeOptions {
            finger_painting: true,
            ..at(0.9)
        };
        let d = smudge(&doc(80, 40, WHITE), options, None, (20, 40));
        let px = pixel(&d, 24, 20);
        assert!(
            px[0] > px[1] + 40,
            "the foreground was not dragged in: {px:?}"
        );
    }

    #[test]
    fn the_transparency_lock_keeps_the_smear_off_empty_pixels() {
        let mut d = doc(80, 40, [240, 200, 40, 0]);
        for (i, a) in d.layers[0].channels[3].data.iter_mut().enumerate() {
            if i % 80 < 40 {
                *a = 255;
            }
        }
        lock(&mut d, LockFlags::TRANSPARENCY);
        let out = smudge(&d, at(1.0), None, (30, 60));
        assert_eq!(
            pixel(&out, 55, 20)[3],
            0,
            "the smear gave an empty pixel coverage"
        );
    }

    #[test]
    fn sample_all_layers_picks_up_from_the_composite() {
        let bar = bar();
        let composite = crate::art_history::layer_local(
            &crate::stamp::layer_surface(&bar, "0").unwrap(),
            bar.layers[0].rect,
        );
        let d = smudge(&doc(80, 40, WHITE), at(0.9), Some(composite), (12, 40));
        assert!(
            pixel(&d, 20, 20)[0] < 240,
            "nothing was picked up from the composite"
        );
        assert_eq!(
            pixel(&d, 2, 20),
            WHITE,
            "the layer was painted outside the stroke"
        );
    }
}
