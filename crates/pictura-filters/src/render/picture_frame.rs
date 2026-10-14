//! Filter ▸ Render ▸ Picture Frame.
//!
//! A decorative border drawn over the picture: vines, flowers and leaves laid
//! round the edge of the canvas, or a ruled or moulded frame. The dialog lists
//! 47 frames, each with its own pattern of the same three ingredients — a
//! **vine** in one colour, **flowers** chosen from 22 shapes in another, and
//! **leaves** from 23 in a third — so the frames are recipes and the shapes a
//! shared catalogue.
//!
//! The Advanced tab's **Number of Lines**, **Thickness**, **Angle**, **Fade**
//! and **Invert** are applied to every frame they make sense for; see
//! [`FrameOptions`].
//!
//! Everything is drawn as filled outlines, anti-aliased, straight onto the
//! layer. Ported from perfecto25/photorust (`core/src/filters/frame.rs`),
//! GPL-3.0. All artwork is procedural parametric geometry; no Adobe asset is
//! traced.
//!
//! No GPU path. The work is a few hundred small shapes scan-converted one at
//! a time, each touching a patch a few dozen pixels across.

use crate::FilterError;

/// The ranges for the Basic tab's sliders.
pub const MARGIN: std::ops::RangeInclusive<u32> = 0..=100;
pub const SIZE: std::ops::RangeInclusive<u32> = 1..=100;
pub const ARRANGEMENT: std::ops::RangeInclusive<u32> = 1..=20;
pub const FLOWER_SIZE: std::ops::RangeInclusive<u32> = 1..=100;
pub const LEAF_SIZE: std::ops::RangeInclusive<u32> = 1..=100;

/// The ranges for the Advanced tab's sliders, read off where its slider thumbs
/// sit: Thickness 29 is a seventh of the way along, Number of Lines 15 half way.
pub const LINES: std::ops::RangeInclusive<u32> = 1..=30;
pub const THICKNESS: std::ops::RangeInclusive<u32> = 1..=200;
pub const ANGLE: std::ops::RangeInclusive<u32> = 0..=360;
pub const FADE: std::ops::RangeInclusive<u32> = 0..=100;

/// The frames, by number.
pub const FRAMES: std::ops::RangeInclusive<u32> = 1..=47;

/// The flower indices; 0 is None.
pub const FLOWER: std::ops::RangeInclusive<u32> = 0..=22;
/// The leaf indices; 0 is None.
pub const LEAF: std::ops::RangeInclusive<u32> = 0..=23;

/// The frame names, in CS6's order.
pub const FRAME_NAMES: [&str; 47] = [
    "Happy Vine",
    "Pretty Vine",
    "Smoke Signals",
    "Party",
    "Big Curls",
    "Tilde",
    "Romance",
    "Curly Dance",
    "Wisps",
    "Spring Weed",
    "Eyelash",
    "Magic Smoke",
    "Curly Vine",
    "Chainmail",
    "Fun Event",
    "Bush",
    "Simple Lace",
    "Pulse",
    "Root",
    "Snakes",
    "Mustache",
    "Circle Sprinkle",
    "Aligned Flowers",
    "Little Flowers",
    "Snowflakes",
    "Flurry",
    "Check Marks",
    "Focused Lines",
    "Focused Parallel Lines",
    "Focused Vibration",
    "Zen Garden",
    "Spikes",
    "Anemone",
    "Pinwheel",
    "Spacing",
    "Line Box",
    "Rounded Corners",
    "Inverse Rounded Corners 1",
    "Inverse Rounded Corners 2",
    "Dual Rounded Corners 1",
    "Dual Rounded Corners 2",
    "Art Frame",
    "Rounded Art Frame",
    "Inverse Rounded Art Frame 1",
    "Inverse Rounded Art Frame 2",
    "Dual Rounded Art Frame 1",
    "Dual Rounded Art Frame 2",
];

/// The flowers, 1 to 22. 0 is None.
pub const FLOWER_NAMES: [&str; 22] = [
    "Small Circle",
    "Circle",
    "Star Flower",
    "Small Flower",
    "Orbit",
    "Pinwheel",
    "Petals",
    "Sunflower Gate",
    "Rose",
    "Grass Circle",
    "Sun",
    "Broken Line Circle",
    "Magical Door",
    "Twinkle",
    "Shiny Star",
    "Star Cloud",
    "Round Plus",
    "Plus Mark",
    "Heart",
    "Star",
    "Snow Flake",
    "Cat's Footprint",
];

/// The leaves, 1 to 23. 0 is None.
pub const LEAF_NAMES: [&str; 23] = [
    "Circle",
    "Square",
    "Drop 1",
    "Leaf 1",
    "Drop 2",
    "Drop 3",
    "Leaf 2",
    "Puff",
    "Leaf 3",
    "Ginkgo leaf",
    "Triangle",
    "Leaf 4",
    "Star 1",
    "Oval",
    "Trapezoid",
    "Lollipop 1",
    "Leaf 5",
    "Star 2",
    "Lollipop 2",
    "Cross",
    "Heart",
    "T",
    "Star 3",
];

/// Everything the Basic tab collects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameOptions {
    /// The frame number, 1 to 47.
    pub frame: u32,
    pub vine: [u8; 3],
    pub margin: u32,
    pub size: u32,
    /// Which of CS6's random arrangements: a seed, not a quantity.
    pub arrangement: u32,
    /// 0 for none, else 1 to 22.
    pub flower: u32,
    pub flower_colour: [u8; 3],
    pub flower_size: u32,
    /// 0 for none, else 1 to 23.
    pub leaf: u32,
    pub leaf_colour: [u8; 3],
    pub leaf_size: u32,
    /// Advanced: how densely the line frames lay their lines, 15 being CS6's
    /// default.
    pub lines: u32,
    /// Advanced: how thick the vines and lines are, and how wide the ruled and
    /// moulded frames' bands; 29 is CS6's default.
    pub thickness: u32,
    /// Advanced: in degrees, how far every flower and leaf is turned, and how
    /// far off square the slanted line frames lean.
    pub angle: u32,
    /// Advanced: how far the frame fades out from the corners to the middle of
    /// each side, 0 not at all and 100 to nothing.
    pub fade: u32,
    /// Advanced: the frame turned inside out — motifs that lean in lean out,
    /// and a moulding lit from the other side.
    pub invert: bool,
}

impl Default for FrameOptions {
    /// CS6's defaults, off its own dialog.
    fn default() -> FrameOptions {
        FrameOptions {
            frame: 34,
            vine: [60, 84, 34],
            margin: 14,
            size: 20,
            arrangement: 1,
            flower: 2,
            flower_colour: [0, 88, 218],
            flower_size: 20,
            leaf: 4,
            leaf_colour: [104, 170, 60],
            leaf_size: 20,
            lines: 15,
            thickness: 29,
            angle: 10,
            fade: 0,
            invert: false,
        }
    }
}

impl FrameOptions {
    /// Reject any control outside its documented range, before the caller
    /// mutates the buffer. The engine still clamps internally as a ceiling.
    pub(crate) fn validate(&self) -> Result<(), FilterError> {
        let check = |what: &str, v: u32, range: std::ops::RangeInclusive<u32>| {
            if range.contains(&v) {
                Ok(())
            } else {
                Err(FilterError::InvalidParams(format!(
                    "{what} is out of range"
                )))
            }
        };
        check("frame", self.frame, FRAMES)?;
        check("margin", self.margin, MARGIN)?;
        check("size", self.size, SIZE)?;
        check("arrangement", self.arrangement, ARRANGEMENT)?;
        check("flower", self.flower, FLOWER)?;
        check("flower size", self.flower_size, FLOWER_SIZE)?;
        check("leaf", self.leaf, LEAF)?;
        check("leaf size", self.leaf_size, LEAF_SIZE)?;
        check("number of lines", self.lines, LINES)?;
        check("thickness", self.thickness, THICKNESS)?;
        check("angle", self.angle, ANGLE)?;
        check("fade", self.fade, FADE)
    }
}

/// Which of the three ingredients a frame draws. CS6 greys out the controls
/// for the ones it does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uses {
    pub vine: bool,
    pub flower: bool,
    pub leaf: bool,
}

/// What `frame` draws with.
pub fn uses(frame: u32) -> Uses {
    let (vine, flower, leaf) = match frame {
        5 | 11 | 12 | 13 | 16 => (true, true, true),
        1..=17 => (true, true, false),
        18 | 19 => (true, true, true),
        20 | 21 => (true, true, false),
        22 | 23 => (false, true, false),
        24..=26 => (false, true, true),
        27..=32 => (true, true, false),
        33 => (false, true, false),
        34 => (true, true, false),
        35 => (false, true, false),
        // The ruled and moulded frames are drawn in the vine colour alone.
        _ => (true, false, false),
    };
    Uses { vine, flower, leaf }
}

/// Whether `frame` is drawn in lines that Number of Lines sets the density of.
/// CS6 greys the slider out for the rest.
pub fn uses_lines(frame: u32) -> bool {
    matches!(frame, 14 | 28 | 29 | 30)
}

mod frames;
mod geometry;
mod moulding;
mod scale;
mod shapes;

use crate::photorust::pixmap::{source_over, Pixmap, Rgba8};

use frames::draw_frame;

pub fn picture_frame(pixmap: &mut Pixmap, options: &FrameOptions) {
    if pixmap.is_empty() {
        return;
    }
    let mut sheet = Pixmap::new(pixmap.width(), pixmap.height());
    draw_frame(&mut sheet, options);

    let fade = options.fade.min(*FADE.end()) as f32 / 100.0;
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    let sheet = &sheet;
    let stride = pixmap.stride();
    use rayon::prelude::*;
    pixmap
        .as_bytes_mut()
        .par_chunks_exact_mut(stride)
        .enumerate()
        .for_each(|(y, out)| {
            let py = y as f32 + 0.5;
            for (x, px) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let top = sheet.get(x as i32, y as i32);
                if top.a == 0 {
                    continue;
                }
                // How far along its side from the nearer corner to the middle a
                // pixel is, 0 at a corner and 1 half way: by the nearer edge, so
                // a pixel on the top side is measured across and one on the left
                // side down.
                let px_ = x as f32 + 0.5;
                let (ex, ey) = (px_.min(w - px_), py.min(h - py));
                let along = if ey < ex {
                    ex / (w / 2.0)
                } else {
                    ey / (h / 2.0)
                };
                let alpha = 1.0 - fade * along.clamp(0.0, 1.0);
                let dst = Rgba8::new(px[0], px[1], px[2], px[3]);
                let out = source_over(dst, top, alpha);
                px.copy_from_slice(&[out.r, out.g, out.b, out.a]);
            }
        });
}

#[cfg(test)]
mod tests;
