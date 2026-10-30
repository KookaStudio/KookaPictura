//! The focus tools' engine, **Blur** and **Sharpen**
//! (`docs/03-tools/smudge-blur-sharpen.md`): one tool with its sign flipped.
//! Both read a pixel's 3×3 neighbourhood; Blur moves the pixel *toward* the
//! neighbourhood's average, Sharpen pushes it as far the other way.
//!
//! Not the Blur and Sharpen *filters*: those are one pass over a whole layer.
//! These are brushes that work only where the tip passes, and **the more it
//! passes the stronger the effect gets** — each dab works on what the last one
//! left, so like the Mixer Brush they apply per dab straight into the layer.
//!
//! The kernel is a fixed 3×3 Gaussian: a bigger tip works a wider area by the
//! same amount per dab, and depth comes from working the same spot. A radius
//! that grew with the tip would turn one click of a large brush into a smear.
//!
//! Ported from photorust's `core/src/focus.rs`
//! (<https://github.com/perfecto25/photorust>). Behavioural parity only:
//! Adobe's kernels are closed.

use crate::healing::RgbaImage;
use crate::replace::{dab_bounds, grow};
use crate::{tip_coverage, StrokeConfig};
use pictura_core::nonseparable::{lum, sat, set_lum, set_sat};
use pictura_core::PsdRect;

/// Which part of the pixel a retouch tool (Blur, Sharpen, Smudge) may change:
/// CS6's cut-down Mode list for a tool whose source is the destination, worked
/// on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RetouchMode {
    #[default]
    Normal,
    Darken,
    Lighten,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl RetouchMode {
    /// 0 Normal … 6 Luminosity (the menu order); `None` otherwise.
    pub fn from_i32(v: i32) -> Option<RetouchMode> {
        [
            RetouchMode::Normal,
            RetouchMode::Darken,
            RetouchMode::Lighten,
            RetouchMode::Hue,
            RetouchMode::Saturation,
            RetouchMode::Color,
            RetouchMode::Luminosity,
        ]
        .get(usize::try_from(v).ok()?)
        .copied()
    }
}

/// Which way the pair moves a pixel relative to its neighbourhood.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    /// Toward the average: softer.
    #[default]
    Blur,
    /// Away from it: crisper.
    Sharpen,
}

/// The Blur and Sharpen options bars: Strength (0.0–1.0, how much one dab
/// applies), Mode, and Sharpen's Protect Detail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusOptions {
    pub focus: Focus,
    pub strength: f32,
    pub mode: RetouchMode,
    /// Sharpen only: hold the result inside the neighbourhood's own range, so
    /// repeated passes cannot throw haloes or blown speckle.
    pub protect_detail: bool,
}

impl Default for FocusOptions {
    /// Blur, Strength 50 %, Normal, Protect Detail on.
    fn default() -> Self {
        FocusOptions {
            focus: Focus::Blur,
            strength: 0.5,
            mode: RetouchMode::Normal,
            protect_detail: true,
        }
    }
}

/// The 3×3 Gaussian, in sixteenths.
const KERNEL: [f32; 9] = [1.0, 2.0, 1.0, 2.0, 4.0, 2.0, 1.0, 2.0, 1.0];

/// One Blur or Sharpen stroke's state.
pub(crate) struct FocusBrush {
    options: FocusOptions,
    /// Sample All Layers: the composite, layer-local, the neighbourhood is
    /// read from. `None` reads the layer itself.
    sampled: Option<RgbaImage>,
    /// A transparency lock: colour may change, coverage may not.
    preserve_alpha: bool,
}

impl FocusBrush {
    pub(crate) fn new(
        options: FocusOptions,
        sampled: Option<RgbaImage>,
        preserve_alpha: bool,
    ) -> Self {
        FocusBrush {
            options,
            sampled,
            preserve_alpha,
        }
    }

    /// Work one dab centred at layer-local `(cx, cy)`. Returns the layer-local
    /// rectangle changed.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
    ) -> Option<PsdRect> {
        let strength = self.options.strength.clamp(0.0, 1.0);
        if strength <= 0.0 {
            return None;
        }
        let region = dab_bounds(img, cfg, cx, cy)?;
        // The neighbourhood is read from a copy: working in place would feed
        // each pixel its already-worked neighbour and smear the dab in
        // whichever direction the loop runs.
        let snapshot = Snapshot::of(self.sampled.as_ref().unwrap_or(img), region);
        let mut dirty = None;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let cover = tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let dst = img.get(x, y);
                if cover <= 0.0 || (self.preserve_alpha && dst[3] == 0) {
                    continue;
                }
                let average = snapshot.average(x, y);
                let worked = match self.options.focus {
                    Focus::Blur => average,
                    Focus::Sharpen => {
                        let bounds = self.options.protect_detail.then(|| snapshot.range(x, y));
                        sharpen(dst, average, bounds)
                    }
                };
                let target = restrict(dst, worked, self.options.mode);
                let mut out = lerp(dst, target, cover * strength);
                if self.preserve_alpha {
                    out[3] = dst[3];
                }
                if out != dst {
                    img.set(x, y, out);
                    dirty = Some(grow(dirty, x, y));
                }
            }
        }
        dirty
    }
}

/// The pixel reflected through its neighbourhood average: `dst + (dst -
/// average)`, a 3×3 unsharp mask at amount 1. `bounds` (the neighbourhood's
/// per-channel min and max) is Protect Detail. Alpha is left where it is:
/// sharpening is about detail, not coverage.
fn sharpen(dst: [u8; 4], average: [u8; 4], bounds: Option<([u8; 3], [u8; 3])>) -> [u8; 4] {
    let mut out = dst;
    for c in 0..3 {
        let mut v = 2.0 * dst[c] as f32 - average[c] as f32;
        if let Some((low, high)) = bounds {
            v = v.clamp(low[c] as f32, high[c] as f32);
        }
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// A copy of `region` grown by one pixel, clipped to the image.
struct Snapshot {
    rect: PsdRect,
    pixels: RgbaImage,
}

impl Snapshot {
    fn of(img: &RgbaImage, region: PsdRect) -> Snapshot {
        let rect = PsdRect {
            top: (region.top - 1).max(0),
            left: (region.left - 1).max(0),
            bottom: (region.bottom + 1).min(img.height),
            right: (region.right + 1).min(img.width),
        };
        let (w, h) = (rect.width().max(0), rect.height().max(0));
        let data = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                let (sx, sy) = (x + rect.left, y + rect.top);
                if sx < img.width && sy < img.height {
                    img.get(sx, sy)
                } else {
                    [0; 4]
                }
            })
            .collect();
        Snapshot {
            rect,
            pixels: RgbaImage {
                width: w,
                height: h,
                data,
            },
        }
    }

    /// The 3×3 Gaussian at image `(x, y)`, clamped at the image edge so a blur
    /// there does not fade into nothing. Weighted in premultiplied colour: a
    /// transparent neighbour adds no colour, so softening a layer's edge does
    /// not draw a dark rim inward.
    fn average(&self, x: i32, y: i32) -> [u8; 4] {
        let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for ky in -1..=1 {
            for kx in -1..=1 {
                let w = KERNEL[((ky + 1) * 3 + kx + 1) as usize] / 16.0;
                let sx = (x + kx - self.rect.left).clamp(0, self.pixels.width - 1);
                let sy = (y + ky - self.rect.top).clamp(0, self.pixels.height - 1);
                let p = self.pixels.get(sx, sy);
                let alpha = p[3] as f32 / 255.0;
                r += p[0] as f32 * alpha * w;
                g += p[1] as f32 * alpha * w;
                b += p[2] as f32 * alpha * w;
                a += alpha * w;
            }
        }
        if a <= 1e-6 {
            return [0; 4];
        }
        let c = |v: f32| (v / a).round().clamp(0.0, 255.0) as u8;
        [
            c(r),
            c(g),
            c(b),
            (a * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    /// Per-channel min and max over the 3×3 neighbourhood of image `(x, y)`.
    fn range(&self, x: i32, y: i32) -> ([u8; 3], [u8; 3]) {
        let (mut low, mut high) = ([255u8; 3], [0u8; 3]);
        for ky in -1..=1 {
            for kx in -1..=1 {
                let sx = (x + kx - self.rect.left).clamp(0, self.pixels.width - 1);
                let sy = (y + ky - self.rect.top).clamp(0, self.pixels.height - 1);
                let p = self.pixels.get(sx, sy);
                for c in 0..3 {
                    low[c] = low[c].min(p[c]);
                    high[c] = high[c].max(p[c]);
                }
            }
        }
        (low, high)
    }
}

/// The part of the worked pixel `mode` lets through. Normal takes it whole,
/// alpha included (which is what softens a layer's edge); every other mode
/// keeps the pixel's coverage.
pub(crate) fn restrict(dst: [u8; 4], worked: [u8; 4], mode: RetouchMode) -> [u8; 4] {
    if mode == RetouchMode::Normal {
        return worked;
    }
    let unit = |p: [u8; 4]| {
        [
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
        ]
    };
    let (base, worked) = (unit(dst), unit(worked));
    let out = match mode {
        RetouchMode::Darken => std::array::from_fn(|i| base[i].min(worked[i])),
        RetouchMode::Lighten => std::array::from_fn(|i| base[i].max(worked[i])),
        RetouchMode::Hue => set_lum(set_sat(worked, sat(base)), lum(base)),
        RetouchMode::Saturation => set_lum(set_sat(base, sat(worked)), lum(base)),
        RetouchMode::Color => set_lum(worked, lum(base)),
        _ => set_lum(base, lum(worked)),
    };
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [c(out[0]), c(out[1]), c(out[2]), dst[3]]
}

pub(crate) fn lerp(from: [u8; 4], to: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| {
        (from[i] as f32 + (to[i] as f32 - from[i] as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::tests::{doc, lock, pixel};
    use crate::stroke::layer_rgba;
    use crate::{PaintError, Stroke, StrokeSample};
    use pictura_core::{BitDepth, Document, LockFlags};

    const WHITE: [u8; 4] = [255; 4];
    const BLACK: [u8; 4] = [0, 0, 0, 255];

    /// A hard black/white edge down the middle of a 40 px square.
    fn edge() -> Document {
        let mut d = doc(40, 40, WHITE);
        for c in &mut d.layers[0].channels[..3] {
            for (i, v) in c.data.iter_mut().enumerate() {
                if i % 40 >= 20 {
                    *v = 0;
                }
            }
        }
        d
    }

    /// `passes` back-and-forth passes of a hard 20 px Blur over (20, 20); the
    /// document unchanged when nothing changed.
    fn blur(
        d: &Document,
        options: FocusOptions,
        sampled: Option<RgbaImage>,
        passes: usize,
    ) -> Document {
        let cfg = StrokeConfig {
            diameter: 20,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_focus(d, "0", cfg, options, sampled).expect("begin");
        let mut out = d.clone();
        for _ in 0..passes {
            for x in [20.0, 25.0, 20.0] {
                stroke.sample(
                    &mut out,
                    StrokeSample {
                        x,
                        y: 20.0,
                        pressure: 1.0,
                    },
                );
            }
        }
        stroke.finish();
        out
    }

    fn full() -> FocusOptions {
        FocusOptions {
            strength: 1.0,
            ..FocusOptions::default()
        }
    }

    /// Pixels of row 20 between x = 10 and 30 that are neither black nor white.
    fn spread(d: &Document) -> usize {
        (10..30)
            .filter(|&x| (9..247).contains(&pixel(d, x, 20)[0]))
            .count()
    }

    #[test]
    fn a_dab_softens_the_edge_and_nothing_outside_the_tip() {
        let d = blur(&edge(), full(), None, 1);
        assert!(pixel(&d, 19, 20)[0] < 255, "the light side was untouched");
        assert!(pixel(&d, 20, 20)[0] > 0, "the dark side was untouched");
        assert_eq!(pixel(&d, 2, 2), WHITE);
        assert_eq!(pixel(&d, 38, 38), BLACK);
    }

    #[test]
    fn dwelling_deepens_the_blur() {
        let once = blur(&edge(), full(), None, 1);
        let many = blur(&edge(), full(), None, 8);
        assert!(
            spread(&many) > spread(&once),
            "{} vs {}",
            spread(&many),
            spread(&once)
        );
    }

    #[test]
    fn strength_scales_one_dab() {
        let soft = FocusOptions {
            strength: 0.1,
            ..FocusOptions::default()
        };
        let moved = |d: &Document| 255 - pixel(d, 19, 20)[0] as i32;
        let (weak, strong) = (blur(&edge(), soft, None, 1), blur(&edge(), full(), None, 1));
        assert!(moved(&strong) > moved(&weak) * 2);
        let none = FocusOptions {
            strength: 0.0,
            ..FocusOptions::default()
        };
        assert_eq!(
            layer_rgba(&blur(&edge(), none, None, 1).layers[0]),
            layer_rgba(&edge().layers[0])
        );
    }

    #[test]
    fn a_flat_area_is_left_alone() {
        let mut out = doc(40, 40, [123, 45, 67, 255]);
        let cfg = StrokeConfig::default();
        let mut stroke = Stroke::begin_focus(&out, "0", cfg, full(), None).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.0,
                y: 20.0,
                pressure: 1.0,
            },
        );
        assert!(stroke.finish().is_none(), "rounding drifted a flat colour");
    }

    #[test]
    fn a_layer_edge_softens_outward_without_darkening() {
        let mut d = doc(40, 40, [240, 200, 40, 0]);
        for (i, a) in d.layers[0].channels[3].data.iter_mut().enumerate() {
            if i % 40 < 20 {
                *a = 255;
            }
        }
        let out = blur(&d, full(), None, 1);
        let inside = pixel(&out, 18, 20);
        assert!(
            inside[0] > 200 && inside[1] > 160,
            "the edge darkened: {inside:?}"
        );
        assert!(
            pixel(&out, 20, 20)[3] > 0,
            "the edge did not soften outward"
        );

        lock(&mut d, LockFlags::TRANSPARENCY);
        let locked = blur(&d, full(), None, 1);
        assert_eq!(
            pixel(&locked, 20, 20)[3],
            0,
            "the blur spread past the lock"
        );
        assert_eq!(pixel(&locked, 10, 20)[3], 255);
    }

    #[test]
    fn luminosity_blurs_the_shading_and_darken_only_darkens() {
        // Two colours of about equal luminosity: nothing to blur in Luminosity.
        let mut d = doc(40, 40, [200, 100, 100, 255]);
        for (c, v) in [(0, 100u8), (1, 160)] {
            for (i, p) in d.layers[0].channels[c].data.iter_mut().enumerate() {
                if i % 40 >= 20 {
                    *p = v;
                }
            }
        }
        let luminosity = FocusOptions {
            mode: RetouchMode::Luminosity,
            ..full()
        };
        let out = blur(&d, luminosity, None, 1);
        let moved = (pixel(&out, 19, 20)[0] as i32 - 200).abs();
        assert!(moved <= 12, "Luminosity blurred the colour: moved {moved}");

        let darken = FocusOptions {
            mode: RetouchMode::Darken,
            ..full()
        };
        let out = blur(&edge(), darken, None, 1);
        assert!(pixel(&out, 19, 20)[0] < 255);
        assert_eq!(pixel(&out, 20, 20), BLACK, "Darken lightened the dark side");
    }

    #[test]
    fn sample_all_layers_reads_the_composite_it_is_given() {
        let edge = edge();
        let composite = crate::art_history::layer_local(
            &crate::stamp::layer_surface(&edge, "0").unwrap(),
            edge.layers[0].rect,
        );
        let out = blur(&doc(40, 40, WHITE), full(), Some(composite), 1);
        assert!(
            pixel(&out, 21, 20)[0] < 200,
            "the composite's edge was not sampled"
        );
    }

    #[test]
    fn a_deep_document_is_refused_and_modes_round_trip() {
        let mut deep = edge();
        deep.depth = BitDepth::Sixteen;
        let begun = Stroke::begin_focus(&deep, "0", StrokeConfig::default(), full(), None);
        assert!(matches!(begun, Err(PaintError::UnsupportedDepth)));
        assert_eq!(RetouchMode::from_i32(6), Some(RetouchMode::Luminosity));
        assert_eq!(RetouchMode::from_i32(7), None);
    }

    fn sharpen_with(protect_detail: bool) -> FocusOptions {
        FocusOptions {
            focus: Focus::Sharpen,
            protect_detail,
            ..full()
        }
    }

    /// Row 20 of `d` from x = 14 to 25, red channel.
    fn row(d: &Document) -> Vec<i32> {
        (14..26).map(|x| pixel(d, x, 20)[0] as i32).collect()
    }

    /// A grey `w`-wide square whose value at column `x` is `f(x)`.
    fn columns(f: impl Fn(usize) -> u8) -> Document {
        let mut d = doc(40, 40, WHITE);
        for c in &mut d.layers[0].channels[..3] {
            for (i, v) in c.data.iter_mut().enumerate() {
                *v = f(i % 40);
            }
        }
        d
    }

    #[test]
    fn sharpen_steepens_a_step_and_leaves_flat_and_ramp_alone() {
        let step = columns(|x| if x < 20 { 100 } else { 160 });
        let out = blur(&step, sharpen_with(false), None, 1);
        assert!(pixel(&out, 19, 20)[0] < 100, "the dark side did not darken");
        assert!(
            pixel(&out, 20, 20)[0] > 160,
            "the light side did not lighten"
        );

        // A straight ramp has no curvature to exaggerate.
        let ramp = columns(|x| (x * 6) as u8);
        let worked = blur(&ramp, sharpen_with(false), None, 4);
        for (a, b) in row(&ramp).iter().zip(row(&worked)) {
            assert!((a - b).abs() <= 1, "the ramp moved: {a} -> {b}");
        }

        let mut out = doc(40, 40, [123, 45, 67, 255]);
        let cfg = StrokeConfig::default();
        let mut stroke = Stroke::begin_focus(&out, "0", cfg, sharpen_with(true), None).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.0,
                y: 20.0,
                pressure: 1.0,
            },
        );
        assert!(
            stroke.finish().is_none(),
            "sharpening drifted a flat colour"
        );
    }

    #[test]
    fn sharpen_walks_a_blur_back() {
        let blurred = blur(&edge(), full(), None, 1);
        let sharpened = blur(&blurred, sharpen_with(false), None, 1);
        let gap = |d: &Document| {
            row(d)
                .iter()
                .zip(row(&edge()))
                .map(|(a, b)| (a - b).abs())
                .sum::<i32>()
        };
        assert!(
            gap(&sharpened) < gap(&blurred),
            "sharpening did not undo the blur"
        );
    }

    #[test]
    fn protect_detail_holds_a_pixel_inside_its_neighbourhood() {
        // A soft step: unprotected passes overshoot past both of its values.
        let step = columns(|x| match x {
            0..=18 => 80,
            19 => 120,
            20 => 140,
            _ => 180,
        });
        let loose = blur(&step, sharpen_with(false), None, 3);
        let protected = blur(&step, sharpen_with(true), None, 3);
        let out_of_range =
            |d: &Document| row(d).iter().filter(|v| !(80..=180).contains(*v)).count();
        assert!(
            out_of_range(&loose) > 0,
            "the unprotected pass did not overshoot"
        );
        assert_eq!(
            out_of_range(&protected),
            0,
            "Protect Detail let a pixel escape"
        );
        assert_ne!(
            row(&protected),
            row(&step),
            "Protect Detail sharpened nothing"
        );
    }
}
