//! The Gradient tool's engine (`docs/03-tools/gradient-and-paint-bucket.md`).
//!
//! A gradient is a **ramp** (a list of colour stops) plus a **style** that
//! turns a pixel's position into a place along that ramp. The two are
//! independent, which is why any preset can be drawn in any of the five
//! styles.
//!
//! Interpolation is done in **straight alpha**, colour and opacity separately:
//! "Foreground to Transparent" keeps the foreground *colour* all the way along
//! while only its opacity falls off. Premultiplied interpolation would drag the
//! colour toward black as it faded.
//!
//! Ported from photorust's `core/src/gradient.rs`
//! (<https://github.com/perfecto25/photorust>). Behavioural parity only: the
//! interpolation space, the dither, and the preset stops are photorust's, not
//! Adobe's.

use crate::fill::{fill_layer, pixel_noise, Alpha};
use crate::healing::RgbaImage;
use crate::{PaintMode, Rgba};
use pictura_core::{ColorMode, Document, PsdRect};
use std::borrow::Cow;

/// The five styles, in options-bar order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GradientStyle {
    /// Straight along the drag.
    #[default]
    Linear,
    /// Circles out from where the drag began.
    Radial,
    /// Sweeps counter-clockwise around the start, the drag setting the zero angle.
    Angle,
    /// Linear, mirrored either side of the start.
    Reflected,
    /// Concentric diamonds out from the start.
    Diamond,
}

impl GradientStyle {
    /// 0 Linear … 4 Diamond; `None` otherwise.
    pub fn from_i32(v: i32) -> Option<GradientStyle> {
        match v {
            0 => Some(GradientStyle::Linear),
            1 => Some(GradientStyle::Radial),
            2 => Some(GradientStyle::Angle),
            3 => Some(GradientStyle::Reflected),
            4 => Some(GradientStyle::Diamond),
            _ => None,
        }
    }
}

/// One stop on the ramp; `position` is 0.0–1.0.
#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    pub position: f32,
    pub color: Rgba,
}

impl GradientStop {
    pub const fn new(position: f32, color: Rgba) -> Self {
        Self { position, color }
    }
}

/// A colour ramp. Stops are held in ascending position order, never empty.
#[derive(Clone, Debug)]
pub struct Gradient {
    pub stops: Vec<GradientStop>,
}

impl Gradient {
    pub fn new(mut stops: Vec<GradientStop>) -> Gradient {
        // A stable sort: two stops at one position keep their order, which is
        // how a ramp steps instead of blending.
        stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        if stops.is_empty() {
            stops.push(GradientStop::new(0.0, BLACK));
        }
        Gradient { stops }
    }

    pub fn two_stop(from: Rgba, to: Rgba) -> Gradient {
        Gradient::new(vec![
            GradientStop::new(0.0, from),
            GradientStop::new(1.0, to),
        ])
    }

    /// The colour at `t` along the ramp, clamped outside 0.0–1.0.
    pub fn sample(&self, t: f32) -> Rgba {
        quantise(self.sample_raw(t))
    }

    /// As [`Gradient::sample`], unquantised: channels as floats in 0.0–255.0.
    /// The renderer dithers the *quantisation* rather than the ramp position;
    /// nudging `t` instead speckles every hard-edged preset.
    fn sample_raw(&self, t: f32) -> [f32; 4] {
        let t = t.clamp(0.0, 1.0);
        let first = self.stops[0];
        if t <= first.position {
            return channels(first.color);
        }
        let last = self.stops[self.stops.len() - 1];
        if t >= last.position {
            return channels(last.color);
        }
        let mut lower = first;
        for stop in &self.stops[1..] {
            if stop.position >= t {
                let span = stop.position - lower.position;
                let f = if span <= 1e-6 {
                    0.0
                } else {
                    (t - lower.position) / span
                };
                let (from, to) = (channels(lower.color), channels(stop.color));
                return std::array::from_fn(|i| from[i] + (to[i] - from[i]) * f);
            }
            lower = *stop;
        }
        channels(last.color)
    }

    /// The ramp end to start: the options bar's Reverse.
    pub fn reversed(&self) -> Gradient {
        // Reversed before the stable sort, so a hard step stays a step the
        // other way round.
        Gradient::new(
            self.stops
                .iter()
                .rev()
                .map(|s| GradientStop::new(1.0 - s.position, s.color))
                .collect(),
        )
    }

    /// The ramp with every stop's colour reduced to its Rec. 601 luma, so a
    /// Grayscale document's ramp lies on the gray axis.
    pub fn grayed(&self) -> Gradient {
        Gradient::new(
            self.stops
                .iter()
                .map(|s| {
                    let v = pictura_core::rec601_luma(s.color.r, s.color.g, s.color.b);
                    GradientStop::new(
                        s.position,
                        Rgba {
                            r: v,
                            g: v,
                            b: v,
                            a: s.color.a,
                        },
                    )
                })
                .collect(),
        )
    }

    /// The ramp as a horizontal strip, for the options bar's sample and the
    /// preset menu.
    pub fn preview(&self, width: i32, height: i32) -> RgbaImage {
        let (w, h) = (width.max(1), height.max(1));
        let mut out = RgbaImage {
            width: w,
            height: h,
            data: vec![[0; 4]; (w * h) as usize],
        };
        for x in 0..w {
            // The pixel's centre, so neither end is half a step short.
            let c = self.sample((x as f32 + 0.5) / w as f32);
            for y in 0..h {
                out.set(x, y, [c.r, c.g, c.b, c.a]);
            }
        }
        out
    }
}

/// The Gradient tool's options bar.
#[derive(Clone, Copy, Debug)]
pub struct GradientOptions {
    pub style: GradientStyle,
    pub mode: PaintMode,
    /// 0.0–1.0.
    pub opacity: f32,
    pub reverse: bool,
    /// Break up banding with a little noise.
    pub dither: bool,
    /// Honour the ramp's own alpha; off, the gradient is drawn opaque.
    pub transparency: bool,
}

impl Default for GradientOptions {
    /// Linear, Normal, 100 %, Transparency on: the spec's defaults.
    fn default() -> Self {
        Self {
            style: GradientStyle::Linear,
            mode: PaintMode::Normal,
            opacity: 1.0,
            reverse: false,
            dither: false,
            transparency: true,
        }
    }
}

/// How far dither may move a channel, in 8-bit levels: enough that a value
/// between two levels lands on either, never enough to shift one already
/// exact, so hard-edged presets stay hard.
const DITHER_LEVELS: f32 = 0.5;

/// Draw `gradient` over the pixel layer at `path` along the drag from `start`
/// to `end` (document space), through `selection` (a document-sized coverage
/// mask) when there is one. The ends of the ramp extend past the drag, so the
/// whole layer (or selection) is covered. Returns the document rectangle
/// changed; `None` for a drag with no length, a zero Opacity, or a layer that
/// cannot be filled.
pub fn draw(
    doc: &mut Document,
    path: &str,
    gradient: &Gradient,
    options: &GradientOptions,
    start: (f32, f32),
    end: (f32, f32),
    selection: Option<&[u8]>,
) -> Option<PsdRect> {
    let axis = (end.0 - start.0, end.1 - start.1);
    let length = (axis.0 * axis.0 + axis.1 * axis.1).sqrt();
    let opacity = options.opacity.clamp(0.0, 1.0);
    // A click without a drag has no axis to run along.
    if length < 1e-3 || opacity <= 0.0 {
        return None;
    }
    let reversed = if options.reverse {
        Cow::Owned(gradient.reversed())
    } else {
        Cow::Borrowed(gradient)
    };
    // A Grayscale document's ramp lies on the gray axis: the stops' luma is
    // interpolated, not the luma of the interpolated colour.
    let ramp = if doc.mode == ColorMode::Grayscale {
        Cow::Owned(reversed.grayed())
    } else {
        reversed
    };
    // A Gradient is a fill, not a brush: with Transparency on, the ramp's own
    // alpha is the alpha of the pixels it draws, so the ramp's transparent end
    // is transparent. In the other modes the blend governs the alpha, as in
    // Photoshop.
    let alpha = if options.transparency && options.mode == PaintMode::Normal {
        Alpha::Replace
    } else {
        Alpha::Blend
    };
    fill_layer(doc, path, options.mode, selection, alpha, |x, y| {
        // The pixel's own coordinate, not its centre: a drag from one pixel to
        // another puts the exact ends of the ramp on exactly those pixels.
        let t = position_at(options.style, (x as f32, y as f32), start, axis, length);
        let mut raw = ramp.sample_raw(t);
        if options.dither {
            // One value for all four channels: per-channel noise would show as
            // colour speckle rather than as grain.
            let n = (pixel_noise(x, y) * 2.0 - 1.0) * DITHER_LEVELS;
            for v in &mut raw {
                *v += n;
            }
        }
        let mut color = quantise(raw);
        if !options.transparency {
            color.a = 255;
        }
        Some((color, opacity))
    })
}

/// Where `point` falls along the ramp, 0.0–1.0 before clamping.
fn position_at(
    style: GradientStyle,
    point: (f32, f32),
    start: (f32, f32),
    axis: (f32, f32),
    length: f32,
) -> f32 {
    let (vx, vy) = (point.0 - start.0, point.1 - start.1);
    let (ax, ay) = (axis.0 / length, axis.1 / length);
    // The pixel in the axis's own frame: `along` runs from start to end,
    // `across` is perpendicular to it.
    let along = vx * ax + vy * ay;
    let across = -vx * ay + vy * ax;
    match style {
        GradientStyle::Linear => along / length,
        GradientStyle::Radial => (vx * vx + vy * vy).sqrt() / length,
        // `across` grows downward, so the sweep is negated to turn the
        // *visible* way round.
        GradientStyle::Angle => {
            let turns = (-across).atan2(along) / std::f32::consts::TAU;
            if turns < 0.0 {
                turns + 1.0
            } else {
                turns
            }
        }
        GradientStyle::Reflected => along.abs() / length,
        GradientStyle::Diamond => (along.abs() + across.abs()) / length,
    }
}

fn channels(c: Rgba) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}

fn quantise(c: [f32; 4]) -> Rgba {
    let v = |i: usize| c[i].round().clamp(0.0, 255.0) as u8;
    Rgba {
        r: v(0),
        g: v(1),
        b: v(2),
        a: v(3),
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

const BLACK: Rgba = rgb(0, 0, 0);
const WHITE: Rgba = rgb(255, 255, 255);

/// The built-in gradients, in picker order. The first two and the last follow
/// the current foreground and background colours.
pub const PRESET_NAMES: [&str; 15] = [
    "Foreground to Background",
    "Foreground to Transparent",
    "Black, White",
    "Red, Green",
    "Violet, Orange",
    "Blue, Red, Yellow",
    "Blue, Yellow, Blue",
    "Orange, Yellow, Orange",
    "Violet, Green, Orange",
    "Yellow, Violet, Orange, Blue",
    "Copper",
    "Chrome",
    "Spectrum",
    "Transparent Rainbow",
    "Transparent Stripes",
];

/// The ramp of preset `index`, or `None` past the end of [`PRESET_NAMES`].
pub fn preset(index: usize, foreground: Rgba, background: Rgba) -> Option<Gradient> {
    let clear = |c: Rgba| Rgba { a: 0, ..c };
    let even = |colours: &[Rgba]| {
        let last = (colours.len().max(2) - 1) as f32;
        Gradient::new(
            colours
                .iter()
                .enumerate()
                .map(|(i, c)| GradientStop::new(i as f32 / last, *c))
                .collect(),
        )
    };
    let at = |stops: &[(f32, Rgba)]| {
        Gradient::new(
            stops
                .iter()
                .map(|&(p, c)| GradientStop::new(p, c))
                .collect(),
        )
    };
    Some(match index {
        0 => Gradient::two_stop(foreground, background),
        1 => Gradient::two_stop(foreground, clear(foreground)),
        2 => Gradient::two_stop(BLACK, WHITE),
        3 => Gradient::two_stop(rgb(255, 0, 0), rgb(0, 255, 0)),
        4 => Gradient::two_stop(rgb(150, 0, 200), rgb(255, 140, 0)),
        5 => even(&[rgb(0, 0, 255), rgb(255, 0, 0), rgb(255, 255, 0)]),
        6 => even(&[rgb(0, 0, 255), rgb(255, 255, 0), rgb(0, 0, 255)]),
        7 => even(&[rgb(255, 130, 0), rgb(255, 255, 0), rgb(255, 130, 0)]),
        8 => even(&[rgb(150, 0, 200), rgb(0, 200, 60), rgb(255, 140, 0)]),
        9 => even(&[
            rgb(255, 240, 0),
            rgb(150, 0, 200),
            rgb(255, 140, 0),
            rgb(0, 40, 220),
        ]),
        // The metals' stops are uneven, which gives the sharp highlight.
        10 => at(&[
            (0.0, rgb(101, 42, 20)),
            (0.25, rgb(213, 133, 83)),
            (0.45, rgb(255, 226, 196)),
            (0.62, rgb(178, 96, 52)),
            (1.0, rgb(120, 55, 28)),
        ]),
        11 => at(&[
            (0.0, rgb(60, 62, 70)),
            (0.32, rgb(220, 228, 238)),
            (0.37, rgb(120, 128, 140)),
            (0.52, rgb(30, 32, 38)),
            (0.58, rgb(150, 158, 170)),
            (1.0, rgb(240, 244, 250)),
        ]),
        // The hue wheel, once round.
        12 => even(&[
            rgb(255, 0, 0),
            rgb(255, 255, 0),
            rgb(0, 255, 0),
            rgb(0, 255, 255),
            rgb(0, 0, 255),
            rgb(255, 0, 255),
            rgb(255, 0, 0),
        ]),
        13 => at(&[
            (0.0, clear(rgb(255, 0, 0))),
            (0.1, rgb(255, 0, 0)),
            (0.3, rgb(255, 255, 0)),
            (0.5, rgb(0, 255, 0)),
            (0.7, rgb(0, 160, 255)),
            (0.9, rgb(120, 0, 255)),
            (1.0, clear(rgb(120, 0, 255))),
        ]),
        // Hard-edged stripes: pairs of stops at one position, so the ramp
        // steps instead of blending.
        14 => {
            let bands = 5;
            let mut stops = Vec::new();
            for i in 0..bands {
                let edge = |k: f32| (i as f32 + k) / bands as f32;
                stops.push(GradientStop::new(edge(0.0), foreground));
                stops.push(GradientStop::new(edge(0.5), foreground));
                stops.push(GradientStop::new(edge(0.5), clear(foreground)));
                stops.push(GradientStop::new(edge(1.0), clear(foreground)));
            }
            Gradient::new(stops)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::tests::{doc, gray_doc, lock, pixel};
    use crate::fill_native::tests::depth16_doc;
    use pictura_core::Channel;
    use pictura_core::LockFlags;

    const BLACK_PX: [u8; 4] = [0, 0, 0, 255];
    const WHITE_PX: [u8; 4] = [255; 4];

    #[test]
    fn a_gray_ramp_lies_on_the_gray_axis() {
        // A Grayscale document's ramp lies on the gray axis: red's luma is
        // 76 and blue's 29.
        let red = Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let blue = Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        };
        let gray = Gradient::two_stop(red, blue).grayed();
        assert_eq!(
            gray.stops[0].color,
            Rgba {
                r: 76,
                g: 76,
                b: 76,
                a: 255
            }
        );
        assert_eq!(
            gray.stops[1].color,
            Rgba {
                r: 29,
                g: 29,
                b: 29,
                a: 255
            }
        );
        // And the drawn ramp is gray, from the luma of the foreground to the
        // luma of the background.
        let mut doc = gray_doc(3, 1, [255, 255, 255, 255]);
        let options = GradientOptions {
            style: GradientStyle::Linear,
            ..GradientOptions::default()
        };
        let drawn = draw(
            &mut doc,
            "0",
            &Gradient::two_stop(red, blue),
            &options,
            (0.0, 0.0),
            (2.0, 0.0),
            None,
        );
        assert!(drawn.is_some(), "the gradient drew");
        let plane = doc.layers[0].channels[0].data.as_slice();
        assert_eq!(plane, &[76, 53, 29]);
    }

    fn black_white() -> Gradient {
        Gradient::two_stop(BLACK, WHITE)
    }

    fn styled(style: GradientStyle) -> GradientOptions {
        GradientOptions {
            style,
            ..GradientOptions::default()
        }
    }

    /// A black-to-white `style` gradient on a `w × h` green layer.
    fn drawn(
        (w, h): (i32, i32),
        style: GradientStyle,
        start: (f32, f32),
        end: (f32, f32),
    ) -> Document {
        let mut d = doc(w, h, [0, 128, 0, 255]);
        draw(
            &mut d,
            "0",
            &black_white(),
            &styled(style),
            start,
            end,
            None,
        );
        d
    }

    #[test]
    fn a_linear_ramp_runs_from_start_to_end_and_extends_past_both() {
        let d = drawn((64, 8), GradientStyle::Linear, (0.0, 4.0), (63.0, 4.0));
        assert_eq!(pixel(&d, 0, 4), BLACK_PX);
        assert_eq!(pixel(&d, 63, 4), WHITE_PX);
        let mid = pixel(&d, 32, 4)[0];
        assert!(mid > 100 && mid < 160, "midpoint was {mid}");

        let short = drawn((64, 8), GradientStyle::Linear, (20.0, 4.0), (40.0, 4.0));
        assert_eq!(pixel(&short, 2, 4), BLACK_PX);
        assert_eq!(pixel(&short, 61, 4), WHITE_PX);
    }

    #[test]
    fn reverse_swaps_the_ends() {
        let mut d = doc(32, 4, [0; 4]);
        let options = GradientOptions {
            reverse: true,
            ..GradientOptions::default()
        };
        draw(
            &mut d,
            "0",
            &black_white(),
            &options,
            (0.0, 2.0),
            (31.0, 2.0),
            None,
        );
        assert_eq!(pixel(&d, 0, 2), WHITE_PX);
        assert_eq!(pixel(&d, 31, 2), BLACK_PX);
    }

    #[test]
    fn a_radial_ramp_starts_at_the_centre_of_the_drag() {
        let d = drawn((64, 64), GradientStyle::Radial, (32.0, 32.0), (62.0, 32.0));
        assert_eq!(pixel(&d, 32, 32), BLACK_PX);
        // Equidistant points match whatever direction they lie in.
        assert_eq!(pixel(&d, 52, 32), pixel(&d, 32, 52));
    }

    #[test]
    fn a_reflected_ramp_mirrors_about_the_start() {
        let d = drawn((64, 8), GradientStyle::Reflected, (32.0, 4.0), (52.0, 4.0));
        assert_eq!(pixel(&d, 12, 4), pixel(&d, 52, 4));
        assert_eq!(pixel(&d, 32, 4), BLACK_PX);
    }

    #[test]
    fn an_angle_ramp_sweeps_a_full_turn_counter_clockwise() {
        let d = drawn((64, 64), GradientStyle::Angle, (32.0, 32.0), (62.0, 32.0));
        assert!(pixel(&d, 60, 32)[0] < 20, "the zero angle is not the start");
        // A quarter turn counter-clockwise, up the screen, is a quarter along.
        let quarter = pixel(&d, 32, 4)[0] as i32;
        assert!((quarter - 64).abs() < 12, "a quarter turn read {quarter}");
        let three = pixel(&d, 32, 60)[0] as i32;
        assert!((three - 191).abs() < 12, "three quarters read {three}");
    }

    #[test]
    fn a_diamond_ramp_measures_manhattan_distance() {
        let d = drawn((64, 64), GradientStyle::Diamond, (32.0, 32.0), (52.0, 32.0));
        // 20 along one axis matches 10 along each of two.
        assert_eq!(pixel(&d, 52, 32), pixel(&d, 42, 42));
    }

    #[test]
    fn a_click_without_a_drag_and_a_zero_opacity_draw_nothing() {
        let mut d = doc(16, 16, [255; 4]);
        let options = GradientOptions::default();
        let at = (8.0, 8.0);
        assert!(draw(&mut d, "0", &black_white(), &options, at, at, None).is_none());
        let clear = GradientOptions {
            opacity: 0.0,
            ..options
        };
        assert!(draw(&mut d, "0", &black_white(), &clear, at, (12.0, 8.0), None).is_none());
        assert_eq!(pixel(&d, 8, 8), WHITE_PX);
    }

    #[test]
    fn fading_to_transparent_keeps_the_colour() {
        let orange = rgb(255, 140, 0);
        let g = Gradient::two_stop(orange, Rgba { a: 0, ..orange });
        let mid = g.sample(0.5);
        assert_eq!((mid.r, mid.g, mid.b), (255, 140, 0));
        assert!((mid.a as i32 - 128).abs() <= 2, "opacity {}", mid.a);
    }

    #[test]
    fn the_ramps_alpha_reaches_the_pixels() {
        // An opaque layer: the ramp's transparent end must make its pixels
        // transparent, which a source-over blend cannot do.
        let red = rgb(255, 0, 0);
        let g = Gradient::two_stop(red, Rgba { a: 0, ..red });
        let mut d = doc(32, 4, [255; 4]);
        draw(
            &mut d,
            "0",
            &g,
            &GradientOptions::default(),
            (0.0, 2.0),
            (31.0, 2.0),
            None,
        );
        assert_eq!(pixel(&d, 0, 2)[3], 255, "the ramp's opaque end");
        assert_eq!(pixel(&d, 31, 2)[3], 0, "the ramp's transparent end");
    }

    #[test]
    fn the_ramps_alpha_reaches_the_pixels_at_depth_16() {
        // The same ramp on a 16-bit document: the retained store's
        // transparency plane takes the ramp's alpha.
        let mut d = depth16_doc(32, 4, vec![0; 128], vec![0; 128], vec![0; 128]);
        d.layers[0].channels.push(Channel {
            id: -1,
            data: vec![255; 128].into(),
        });
        let red = rgb(255, 0, 0);
        let g = Gradient::two_stop(red, Rgba { a: 0, ..red });
        draw(
            &mut d,
            "0",
            &g,
            &GradientOptions::default(),
            (0.0, 2.0),
            (31.0, 2.0),
            None,
        );
        // The plane is 32 px wide: the ramp's ends are (0, 2) and (31, 2).
        let alpha = &d.layers[0].channels[3].data;
        assert_eq!(alpha[2 * 32], 255, "the ramp's opaque end");
        assert_eq!(alpha[2 * 32 + 31], 0, "the ramp's transparent end");
    }

    #[test]
    fn transparency_off_draws_the_ramp_solid() {
        let red = rgb(255, 0, 0);
        let g = Gradient::two_stop(red, Rgba { a: 0, ..red });
        let ends = ((0.0, 2.0), (31.0, 2.0));
        let mut faded = doc(32, 4, [0; 4]);
        draw(
            &mut faded,
            "0",
            &g,
            &GradientOptions::default(),
            ends.0,
            ends.1,
            None,
        );
        assert_eq!(pixel(&faded, 31, 2)[3], 0);

        let mut solid = doc(32, 4, [0; 4]);
        let options = GradientOptions {
            transparency: false,
            ..GradientOptions::default()
        };
        draw(&mut solid, "0", &g, &options, ends.0, ends.1, None);
        assert_eq!(pixel(&solid, 31, 2), [255, 0, 0, 255]);
    }

    #[test]
    fn a_transparency_lock_confines_the_gradient_to_existing_pixels() {
        let mut d = doc(32, 32, [200, 200, 200, 255]);
        for a in &mut d.layers[0].channels[3].data[16 * 32..] {
            *a = 0;
        }
        lock(&mut d, LockFlags::TRANSPARENCY);
        let black = Gradient::two_stop(BLACK, BLACK);
        let options = GradientOptions::default();
        draw(
            &mut d,
            "0",
            &black,
            &options,
            (0.0, 16.0),
            (31.0, 16.0),
            None,
        );
        assert_eq!(pixel(&d, 16, 4), BLACK_PX);
        assert_eq!(pixel(&d, 16, 24)[3], 0, "an empty pixel gained coverage");
    }

    #[test]
    fn a_selection_confines_the_gradient() {
        let mut d = doc(32, 32, [255; 4]);
        let mask: Vec<u8> = (0..32 * 32)
            .map(|i| if i % 32 < 16 { 255 } else { 0 })
            .collect();
        let black = Gradient::two_stop(BLACK, BLACK);
        let dirty = draw(
            &mut d,
            "0",
            &black,
            &GradientOptions::default(),
            (0.0, 16.0),
            (31.0, 16.0),
            Some(&mask),
        )
        .expect("drawn");
        assert_eq!((dirty.left, dirty.right), (0, 16));
        assert_eq!(pixel(&d, 8, 16), BLACK_PX);
        assert_eq!(pixel(&d, 24, 16), WHITE_PX);
    }

    #[test]
    fn dither_breaks_a_stretched_ramp_by_at_most_one_level() {
        // 1024 pixels of a 256-level ramp: four pixels a band without dither.
        const W: i32 = 1024;
        let ends = ((0.0, 1.0), ((W - 1) as f32, 1.0));
        let mut plain = doc(W, 2, [0; 4]);
        let mut dithered = doc(W, 2, [0; 4]);
        let options = GradientOptions::default();
        draw(
            &mut plain,
            "0",
            &black_white(),
            &options,
            ends.0,
            ends.1,
            None,
        );
        let noisy = GradientOptions {
            dither: true,
            ..options
        };
        draw(
            &mut dithered,
            "0",
            &black_white(),
            &noisy,
            ends.0,
            ends.1,
            None,
        );
        let mut differed = 0;
        for x in 0..W {
            let d = (pixel(&plain, x, 1)[0] as i32 - pixel(&dithered, x, 1)[0] as i32).abs();
            assert!(d <= 1, "dither moved x={x} by {d} levels");
            differed += d;
        }
        assert!(
            differed > W / 8,
            "dither changed almost nothing: {differed}"
        );
    }

    #[test]
    fn dither_leaves_hard_edged_presets_hard() {
        let stripes = preset(14, BLACK, WHITE).unwrap();
        let mut d = doc(200, 8, [0; 4]);
        let options = GradientOptions {
            dither: true,
            ..GradientOptions::default()
        };
        draw(
            &mut d,
            "0",
            &stripes,
            &options,
            (0.0, 4.0),
            (199.0, 4.0),
            None,
        );
        for x in 0..200 {
            let a = pixel(&d, x, 2)[3];
            assert!(a == 0 || a == 255, "x={x} came out half-covered: {a}");
            assert_eq!(a, pixel(&d, x, 6)[3], "x={x} differs between rows");
        }
    }

    #[test]
    fn every_preset_resolves_and_previews() {
        for (index, name) in PRESET_NAMES.iter().enumerate() {
            let g = preset(index, BLACK, WHITE).unwrap_or_else(|| panic!("no ramp for {name}"));
            let strip = g.preview(24, 6);
            assert_eq!((strip.width, strip.height), (24, 6));
            assert_eq!(
                strip.get(0, 0),
                strip.get(0, 5),
                "{name} varies down a column"
            );
        }
        assert!(preset(PRESET_NAMES.len(), BLACK, WHITE).is_none());
    }

    #[test]
    fn the_foreground_presets_follow_the_current_colours() {
        let (fg, bg) = (rgb(10, 20, 30), rgb(200, 210, 220));
        let g = preset(0, fg, bg).unwrap();
        assert_eq!(g.sample(0.0), fg);
        assert_eq!(g.sample(1.0), bg);
        assert_eq!(preset(1, fg, bg).unwrap().sample(1.0), Rgba { a: 0, ..fg });
    }

    #[test]
    fn transparent_stripes_step_rather_than_blend_either_way_round() {
        let g = preset(14, BLACK, WHITE).unwrap();
        // Five bands: opaque for the first half of each, clear for the second.
        assert_eq!(g.sample(0.05).a, 255);
        assert_eq!(g.sample(0.15).a, 0);
        assert_eq!(g.sample(0.25).a, 255);
        let r = g.reversed();
        assert_eq!(r.sample(0.05).a, 0);
        assert_eq!(r.sample(0.15).a, 255);
        assert_eq!(r.sample(0.12).a, 255, "the reversed step blended");
    }
}
