//! Character styling for the bundled renderer: the OpenType features a run
//! selects, its caps transform, super/subscript size and rise, Faux Bold and
//! Faux Italic, and the three kerning modes. [`shape_styled`] is the one place
//! a character attribute set becomes glyphs, so the caret, the wrap, the layer
//! rect, and the paint measure the same run.
//!
//! The proportions below are CS6's documented Type-preference defaults where
//! they exist (super/subscript 58.3% size, 33.3% position) and approximations
//! otherwise (small caps 70%, the 12° slant, the bold strength, optical
//! spacing); none is oracle-verified.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use pictura_core::{CharacterAttrs, KerningMode, ShapedGlyph};
use rustybuzz::ttf_parser::{self, GlyphId, OutlineBuilder, Tag};
use rustybuzz::Feature;

use crate::text_render::BundledText;

const SCRIPT_SIZE: f32 = 0.583;
const SCRIPT_POSITION: f32 = 0.333;
const SMALL_CAPS: f32 = 0.7;
/// tan 12°.
pub(crate) const ITALIC_SKEW: f32 = 0.2126;
const BOLD_STRENGTH: f32 = 1.0 / 24.0;
const OPTICAL_CLAMP: f32 = 0.12;
/// The tightest band gap optical spacing may leave, in em.
const OPTICAL_MIN_GAP: f32 = 0.02;
const BANDS: usize = 16;

/// The Character panel's OpenType buttons in panel order, each with the tags
/// that make its feature available: Standard Ligatures, Contextual
/// Alternates, Discretionary Ligatures, Swash, Stylistic Alternates, Titling
/// Alternates, Ordinals, Fractions.
const OPENTYPE_BUTTONS: [&[&[u8; 4]]; 8] = [
    &[b"liga", b"clig"],
    &[b"calt"],
    &[b"dlig"],
    &[b"swsh"],
    &[b"salt"],
    &[b"titl"],
    &[b"ordn"],
    &[b"frac"],
];

/// Whether `face`'s GSUB or GPOS feature list names `tag`.
pub(crate) fn has_feature(face: &BundledText, tag: &[u8; 4]) -> bool {
    let tag = Tag::from_bytes(tag);
    let tables = face.ttf().tables();
    [tables.gsub, tables.gpos]
        .into_iter()
        .flatten()
        .any(|table| table.features.into_iter().any(|f| f.tag == tag))
}

/// One bit per OpenType button (bit 0 Standard Ligatures … bit 7 Fractions)
/// whose feature `face` carries.
pub(crate) fn opentype_support(face: &BundledText) -> u32 {
    OPENTYPE_BUTTONS
        .iter()
        .enumerate()
        .filter(|(_, tags)| tags.iter().any(|tag| has_feature(face, tag)))
        .fold(0, |mask, (bit, _)| mask | 1 << bit)
}

/// The feature settings `c` selects. Every toggle is explicit, so a feature
/// the shaper enables by default (`liga`, `clig`, `calt`, `kern`) turns off.
fn features(face: &BundledText, c: &CharacterAttrs) -> Vec<Feature> {
    let set = |tag: &[u8; 4], on: bool| Feature::new(Tag::from_bytes(tag), u32::from(on), ..);
    let mut out = vec![
        set(b"liga", c.standard_ligatures),
        set(b"clig", c.standard_ligatures),
        set(b"calt", c.contextual_alternates),
        set(b"dlig", c.discretionary_ligatures),
        set(b"swsh", c.swash),
        set(b"onum", c.oldstyle),
        set(b"salt", c.stylistic_alternates),
        set(b"titl", c.titling_alternates),
        set(b"ornm", c.ornaments),
        set(b"ordn", c.ordinals),
        set(b"frac", c.fractions),
        set(b"kern", c.kerning == KerningMode::Metrics),
    ];
    if native_small_caps(face, c) {
        out.push(set(b"smcp", true));
    }
    out
}

/// Small Caps from the face's own `smcp` (All Caps wins over Small Caps).
fn native_small_caps(face: &BundledText, c: &CharacterAttrs) -> bool {
    c.small_caps && !c.all_caps && has_feature(face, b"smcp")
}

/// The run's text as it shapes, in pieces with their glyph scale: upper case
/// for All Caps; for Small Caps without `smcp`, lower-case runs as capitals at
/// [`SMALL_CAPS`].
fn case_pieces(face: &BundledText, text: &str, c: &CharacterAttrs) -> Vec<(String, f32)> {
    if c.all_caps {
        return vec![(text.to_uppercase(), 1.0)];
    }
    if !c.small_caps || native_small_caps(face, c) {
        return vec![(text.to_owned(), 1.0)];
    }
    let mut pieces: Vec<(String, f32)> = Vec::new();
    for ch in text.chars() {
        let (piece, scale) = match ch.is_lowercase() {
            true => (ch.to_uppercase().collect::<String>(), SMALL_CAPS),
            false => (ch.to_string(), 1.0),
        };
        match pieces.last_mut() {
            Some((last, s)) if *s == scale => last.push_str(&piece),
            _ => pieces.push((piece, scale)),
        }
    }
    pieces
}

/// The pixel geometry a styled run paints with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RunStyle {
    /// The em size glyphs shape and rasterize at (super/subscript shrink it).
    pub size: f32,
    /// How far the run's baseline rises (negative drops): the super/subscript
    /// position plus the baseline shift.
    pub rise: f32,
    pub h: f32,
    pub v: f32,
    /// Faux Italic's slant, as `tan` of the angle.
    pub skew: f32,
    /// Faux Bold's outline strength in pixels (also added to each advance).
    pub embolden: f32,
    /// Underline centre below the baseline and thickness, in pixels.
    pub underline: Option<(f32, f32)>,
    /// Strikethrough centre above the baseline and thickness, in pixels.
    pub strike: Option<(f32, f32)>,
}

impl RunStyle {
    /// An unstyled run at `size`.
    #[cfg(test)]
    pub(crate) fn plain(size: f32) -> Self {
        Self {
            size,
            rise: 0.0,
            h: 1.0,
            v: 1.0,
            skew: 0.0,
            embolden: 0.0,
            underline: None,
            strike: None,
        }
    }

    pub(crate) fn of(face: &BundledText, c: &CharacterAttrs) -> Self {
        let authored = c.size as f32;
        let script = c.superscript || c.subscript;
        let size = if script {
            authored * SCRIPT_SIZE
        } else {
            authored
        };
        let position = match (c.superscript, c.subscript) {
            (true, _) => authored * SCRIPT_POSITION,
            (false, true) => -authored * SCRIPT_POSITION,
            _ => 0.0,
        };
        let v = (c.vertical_scale / 100.0) as f32;
        let ttf = face.ttf();
        let upem = f32::from(ttf.units_per_em().max(1));
        let px = |units: i16| f32::from(units) * size / upem * v;
        let line = |metrics: Option<ttf_parser::LineMetrics>, centre: f32| match metrics
            .filter(|m| m.thickness > 0)
        {
            Some(m) => (px(m.position), px(m.thickness)),
            None => (centre * size * v, 0.05 * size * v),
        };
        let underline = c.underline.then(|| {
            let (position, thickness) = line(ttf.underline_metrics(), -0.05);
            (-position, thickness)
        });
        let strike = c.strikethrough.then(|| {
            let (position, thickness) = line(ttf.strikeout_metrics(), 0.3);
            // OS/2 gives the stroke's top; centre it.
            match ttf.strikeout_metrics().filter(|m| m.thickness > 0) {
                Some(_) => (position - thickness / 2.0, thickness),
                None => (position, thickness),
            }
        });
        Self {
            size,
            rise: position + c.baseline_shift as f32,
            h: (c.horizontal_scale / 100.0) as f32,
            v,
            skew: if c.faux_italic { ITALIC_SKEW } else { 0.0 },
            embolden: if c.faux_bold {
                size * BOLD_STRENGTH
            } else {
                0.0
            },
            underline,
            strike,
        }
    }

    /// Whether the run paints anything the plain glyph path does not.
    pub(crate) fn decorated(&self) -> bool {
        self.underline.is_some() || self.strike.is_some()
    }
}

/// Whether `c` renders beyond what plain positioned glyphs can express (PDF
/// live text then falls back to pixels).
pub(crate) fn needs_raster(face: &BundledText, c: &CharacterAttrs) -> bool {
    c.faux_bold
        || c.faux_italic
        || c.underline
        || c.strikethrough
        || c.superscript
        || c.subscript
        || (c.small_caps && !c.all_caps && !has_feature(face, b"smcp"))
}

/// Shape `text` as `c` styles it: glyph ids with **device-pixel** advances and
/// offsets at the run's effective size, already carrying the horizontal and
/// vertical scale, each glyph's small-caps scale, Faux Bold's widening, and the
/// optical pair spacing. Tracking and manual kerning are the layout's (see
/// [`tracking`]).
pub(crate) fn shape_styled(face: &BundledText, text: &str, c: &CharacterAttrs) -> Vec<ShapedGlyph> {
    let run = RunStyle::of(face, c);
    let features = features(face, c);
    let mut glyphs: Vec<ShapedGlyph> = Vec::new();
    for (piece, scale) in case_pieces(face, text, c) {
        glyphs.extend(
            face.shape_with(&piece, run.size * scale, &features)
                .into_iter()
                .map(|g| ShapedGlyph { scale, ..g }),
        );
    }
    if c.kerning == KerningMode::Optical {
        optical_spacing(face, &mut glyphs, run.size);
    }
    for g in &mut glyphs {
        g.advance = g.advance * run.h + run.embolden;
        g.x_offset *= run.h;
        g.y_offset *= run.v;
    }
    glyphs
}

/// Tracking plus manual kerning, in 1/1000 em of the authored size.
pub(crate) fn tracking(c: &CharacterAttrs) -> f32 {
    let manual = match c.kerning {
        KerningMode::Manual(value) => value as f32,
        KerningMode::Metrics | KerningMode::Optical => 0.0,
    };
    c.tracking as f32 + manual
}

/// A glyph's ink extent per horizontal band between the baseline and the cap
/// height, in font units: `(leftmost, rightmost)` x, `None` for an inkless band.
type Profile = [Option<(f32, f32)>; BANDS];

struct ProfileBuilder {
    band: f32,
    last: (f32, f32),
    profile: Profile,
}

impl ProfileBuilder {
    fn add(&mut self, x: f32, y: f32) {
        let index = (y / self.band).floor();
        if index < 0.0 || index >= BANDS as f32 {
            return;
        }
        let slot = &mut self.profile[index as usize];
        *slot = Some(match *slot {
            Some((lo, hi)) => (lo.min(x), hi.max(x)),
            None => (x, x),
        });
    }

    /// Sample a segment from the pen to `at(1)` so a long edge marks every band
    /// it crosses.
    fn sample(&mut self, at: impl Fn(f32) -> (f32, f32)) {
        const STEPS: usize = 16;
        for step in 1..=STEPS {
            let (x, y) = at(step as f32 / STEPS as f32);
            self.add(x, y);
        }
        self.last = at(1.0);
    }
}

impl OutlineBuilder for ProfileBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.add(x, y);
        self.last = (x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x0, y0) = self.last;
        self.sample(|t| (x0 + (x - x0) * t, y0 + (y - y0) * t));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x0, y0) = self.last;
        self.sample(|t| {
            let u = 1.0 - t;
            (
                u * u * x0 + 2.0 * u * t * x1 + t * t * x,
                u * u * y0 + 2.0 * u * t * y1 + t * t * y,
            )
        });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x0, y0) = self.last;
        self.sample(|t| {
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            (
                a * x0 + b * x1 + c * x2 + d * x,
                a * y0 + b * y1 + c * y2 + d * y,
            )
        });
    }

    fn close(&mut self) {}
}

type ProfileCache = Mutex<HashMap<(usize, u16), Option<Profile>>>;

/// `glyph`'s band profile, cached per face and glyph for the process.
fn profile(face: &BundledText, glyph: u16) -> Option<Profile> {
    static CACHE: OnceLock<ProfileCache> = OnceLock::new();
    let key = (face.bytes().as_ptr() as usize, glyph);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(&key) {
        return *hit;
    }
    let ttf = face.ttf();
    let height = ttf
        .capital_height()
        .filter(|h| *h > 0)
        .map_or(f32::from(ttf.ascender()) * 0.7, f32::from);
    let mut builder = ProfileBuilder {
        band: height / BANDS as f32,
        last: (0.0, 0.0),
        profile: [None; BANDS],
    };
    let found = ttf
        .outline_glyph(GlyphId(glyph), &mut builder)
        .map(|_| builder.profile);
    cache.lock().ok()?.insert(key, found);
    found
}

/// The mean and the tightest band gap between `left`'s right edge and
/// `right`'s left edge, `left` advanced by `advance`, all in pixels at the
/// given per-glyph pixel-per-unit scales; `None` when no band holds both.
fn pair_gap(left: &Profile, right: &Profile, advance: f32, kl: f32, kr: f32) -> Option<(f32, f32)> {
    let gaps: Vec<f32> = left
        .iter()
        .zip(right)
        .filter_map(|(l, r)| Some(advance - l.as_ref()?.1 * kl + r.as_ref()?.0 * kr))
        .collect();
    if gaps.is_empty() {
        return None;
    }
    let mean = gaps.iter().sum::<f32>() / gaps.len() as f32;
    Some((mean, gaps.iter().copied().fold(f32::MAX, f32::min)))
}

/// Optical kerning (approximated): move each pair's mean band gap toward the
/// face's `n n` gap, clamped to ±[`OPTICAL_CLAMP`] em and never tighter than
/// [`OPTICAL_MIN_GAP`] em at the closest band. Advances are pre-scale pixels.
fn optical_spacing(face: &BundledText, glyphs: &mut [ShapedGlyph], size: f32) {
    let ttf = face.ttf();
    let upem = f32::from(ttf.units_per_em().max(1));
    let reference = ttf.glyph_index('n').and_then(|n| {
        let p = profile(face, n.0)?;
        let advance = f32::from(ttf.glyph_hor_advance(n)?) * size / upem;
        let k = size / upem;
        pair_gap(&p, &p, advance, k, k).map(|(mean, _)| mean)
    });
    let Some(reference) = reference else {
        return;
    };
    for i in 1..glyphs.len() {
        let (a, b) = (glyphs[i - 1], glyphs[i]);
        let (Some(pa), Some(pb)) = (profile(face, a.id), profile(face, b.id)) else {
            continue;
        };
        let (ka, kb) = (size * a.scale / upem, size * b.scale / upem);
        let Some((mean, tightest)) = pair_gap(&pa, &pb, a.advance, ka, kb) else {
            continue;
        };
        let limit = OPTICAL_CLAMP * size;
        let floor = OPTICAL_MIN_GAP * size - tightest;
        let adjust = (reference - mean).clamp(-limit, limit).max(floor.min(0.0));
        glyphs[i - 1].advance += adjust;
    }
}

#[cfg(test)]
mod tests;
