//! The Art History Brush (`docs/03-tools/art-history-brush.md`): paints
//! stylized strokes coloured from the History Brush's source state.
//!
//! Every dab the drag places scatters a few short strokes over the **Area**
//! around it. Each takes its colour from the source at its start and runs
//! along the source's edges (across its luminance gradient), so the strokes
//! follow the picture's shapes; the **Style** sets how long they run, how much
//! they wander (Tight / Loose), and whether they curl. **Tolerance** keeps a
//! stroke off pixels that already look like the source: at 0 it paints
//! anywhere, higher only where the image has drifted from the source.
//!
//! Original to Kooka — photorust has no Art History Brush. Behavioural parity
//! only: Adobe's stroke kernels are closed, and the style table below is a
//! design choice, not Adobe's values. Every stroke is random, but from a fixed
//! seed, so a stroke replays exactly.

use crate::healing::RgbaImage;
use crate::replace::dab_bounds;
use crate::tip::TipParams;
use crate::StrokeConfig;
use pictura_core::PsdRect;
use rayon::prelude::*;
use std::f32::consts::{PI, SQRT_2, TAU};

/// The Style menu, in CS6's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArtStyle {
    #[default]
    TightShort,
    TightMedium,
    TightLong,
    LooseMedium,
    LooseLong,
    Dab,
    TightCurl,
    TightCurlLong,
    LooseCurl,
    LooseCurlLong,
}

/// The Style menu's labels, indexed as [`ArtStyle::from_i32`].
pub const STYLE_NAMES: [&str; 10] = [
    "Tight Short",
    "Tight Medium",
    "Tight Long",
    "Loose Medium",
    "Loose Long",
    "Dab",
    "Tight Curl",
    "Tight Curl Long",
    "Loose Curl",
    "Loose Curl Long",
];

/// A style's stroke: `length` in brush diameters, `curl` the turn per step
/// (radians), `jitter` the random wander per step (radians).
struct Shape {
    length: f32,
    curl: f32,
    jitter: f32,
}

const TIGHT: f32 = 0.05;
const LOOSE: f32 = 0.35;

impl ArtStyle {
    /// The Style menu index (0 Tight Short … 9 Loose Curl Long); `None`
    /// otherwise.
    pub fn from_i32(v: i32) -> Option<ArtStyle> {
        use ArtStyle::*;
        let all = [
            TightShort,
            TightMedium,
            TightLong,
            LooseMedium,
            LooseLong,
            Dab,
            TightCurl,
            TightCurlLong,
            LooseCurl,
            LooseCurlLong,
        ];
        usize::try_from(v).ok().and_then(|i| all.get(i).copied())
    }

    fn shape(self) -> Shape {
        let (length, curl, jitter) = match self {
            ArtStyle::TightShort => (1.5, 0.0, TIGHT),
            ArtStyle::TightMedium => (3.0, 0.0, TIGHT),
            ArtStyle::TightLong => (6.0, 0.0, TIGHT),
            ArtStyle::LooseMedium => (3.0, 0.0, LOOSE),
            ArtStyle::LooseLong => (6.0, 0.0, LOOSE),
            ArtStyle::Dab => (0.0, 0.0, 0.0),
            ArtStyle::TightCurl => (3.0, 0.4, TIGHT),
            ArtStyle::TightCurlLong => (6.0, 0.25, TIGHT),
            ArtStyle::LooseCurl => (3.0, 0.4, LOOSE),
            ArtStyle::LooseCurlLong => (6.0, 0.25, LOOSE),
        };
        Shape {
            length,
            curl,
            jitter,
        }
    }
}

/// The options bar beyond the tip and Opacity: Style, Area (the diameter
/// strokes scatter over, in pixels), Tolerance (0–100 %), and the stroke's
/// random seed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArtHistoryOptions {
    pub style: ArtStyle,
    pub area: u32,
    pub tolerance: u8,
    pub seed: u64,
}

impl Default for ArtHistoryOptions {
    /// Tight Short over 50 px. Tolerance 0 paints anywhere; the spec's
    /// unverified default of 100 % would paint almost nowhere.
    fn default() -> Self {
        ArtHistoryOptions {
            style: ArtStyle::TightShort,
            area: 50,
            tolerance: 0,
            seed: 0,
        }
    }
}

/// Most strokes one dab scatters, so a huge Area stays interactive.
const MAX_STROKES_PER_DAB: f32 = 16.0;

/// One Art History Brush stroke's state.
pub(crate) struct ArtHistoryBrush {
    options: ArtHistoryOptions,
    /// The source state's layer, layer-local like the pixels painted.
    source: RgbaImage,
    rng: u64,
    /// A transparency lock: colour may change, coverage may not.
    preserve_alpha: bool,
}

impl ArtHistoryBrush {
    pub(crate) fn new(options: ArtHistoryOptions, source: RgbaImage, preserve_alpha: bool) -> Self {
        ArtHistoryBrush {
            options,
            source,
            rng: options.seed,
            preserve_alpha,
        }
    }

    /// Pad the source snapshot when the layer grows (#1208).
    pub(crate) fn grow(&mut self, new_w: i32, new_h: i32, dx: i32, dy: i32) {
        self.source.grow(new_w, new_h, dx, dy);
    }

    /// Scatter the strokes of one dab centred at layer-local `(cx, cy)`.
    /// Returns the layer-local rectangle changed.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
    ) -> Option<PsdRect> {
        let diameter = cfg.diameter.max(1) as f32;
        let area = self.options.area.max(1) as f32;
        let count = (area / diameter).ceil().clamp(1.0, MAX_STROKES_PER_DAB) as usize;
        let shape = self.options.style.shape();
        let step = (diameter * 0.25).max(1.0);
        let steps = (shape.length * diameter / step).round() as usize;
        let mut dirty = None;
        for _ in 0..count {
            let (angle, reach) = (self.next() * TAU, self.next().sqrt() * area * 0.5);
            let (mut x, mut y) = (cx + reach * angle.cos(), cy + reach * angle.sin());
            let (ix, iy) = (x.floor() as i32, y.floor() as i32);
            if ix < 0 || iy < 0 || ix >= img.width || iy >= img.height {
                continue;
            }
            let paint = self.source.get(ix, iy);
            if paint[3] == 0 || !self.differs(paint, img.get(ix, iy)) {
                continue;
            }
            let mut heading = match self.edge_heading(ix, iy) {
                Some(h) => h,
                None => self.next() * TAU,
            };
            let turn = if self.next() < 0.5 {
                -shape.curl
            } else {
                shape.curl
            };
            let mut centres = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                if i > 0 {
                    heading += turn + (self.next() - 0.5) * 2.0 * shape.jitter;
                    x += step * heading.cos();
                    y += step * heading.sin();
                }
                centres.push((x, y));
            }
            if let Some(d) = self.stamp_path(img, cfg, &centres, paint) {
                dirty = Some(dirty.map_or(d, |r: PsdRect| r.union(d)));
            }
        }
        dirty
    }

    /// The Tolerance gate: whether `dest` has drifted from `source` by at
    /// least the tolerance.
    fn differs(&self, source: [u8; 4], dest: [u8; 4]) -> bool {
        let diff = (0..4)
            .map(|c| source[c].abs_diff(dest[c]))
            .max()
            .unwrap_or(0) as u32;
        diff * 100 >= self.options.tolerance.min(100) as u32 * 255
    }

    /// The direction along the source's edge at `(x, y)`, across its luminance
    /// gradient; `None` on a flat patch.
    fn edge_heading(&self, x: i32, y: i32) -> Option<f32> {
        let luma = |x: i32, y: i32| {
            let [r, g, b, _] = self.source.get(
                x.clamp(0, self.source.width - 1),
                y.clamp(0, self.source.height - 1),
            );
            0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
        };
        let gx = luma(x + 1, y) - luma(x - 1, y);
        let gy = luma(x, y + 1) - luma(x, y - 1);
        (gx.abs() + gy.abs() > 4.0).then(|| gy.atan2(gx) + PI * 0.5)
    }

    /// One stroke: a tip of `paint` at each of `centres` in order, over the
    /// pixels at Opacity × Flow. Returns the layer-local rectangle changed.
    ///
    /// The rows run in parallel, each applying the stroke's tips in order, so
    /// every pixel sees the same sequence of blends as tip-by-tip stamping
    /// (#1238).
    fn stamp_path(
        &self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        centres: &[(f32, f32)],
        paint: [u8; 4],
    ) -> Option<PsdRect> {
        let tips: Vec<(f32, f32, PsdRect)> = centres
            .iter()
            .filter_map(|&(cx, cy)| Some((cx, cy, dab_bounds(img, cfg, cx, cy)?)))
            .collect();
        let bounds = tips.iter().map(|t| t.2).reduce(PsdRect::union)?;
        let strength = cfg.opacity.min(100) as f32 / 100.0 * cfg.flow.min(100) as f32 / 100.0;
        let tip = TipParams::new(cfg);
        // How far a tip reaches from its centre: a rotated square's corners
        // reach √2 of its radius, a round or squashed tip no further than it.
        let reach = tip.radius * if tip.square { SQRT_2 } else { 1.0 };
        let preserve_alpha = self.preserve_alpha;
        let width = img.width as usize;
        img.data[bounds.top as usize * width..bounds.bottom as usize * width]
            .par_chunks_mut(width)
            .enumerate()
            .filter_map(|(i, row)| {
                let y = bounds.top + i as i32;
                let mut span: Option<(i32, i32)> = None;
                for &(cx, cy, region) in &tips {
                    if y < region.top || y >= region.bottom {
                        continue;
                    }
                    let dy = y as f32 + 0.5 - cy;
                    // The row's chord of the tip's reach: every pixel past it
                    // has no coverage.
                    let half = (reach * reach - dy * dy).max(0.0).sqrt() + 1.0;
                    let x0 = region.left.max((cx - half).floor() as i32);
                    let x1 = region.right.min((cx + half).ceil() as i32 + 1);
                    for x in x0..x1 {
                        let a = tip.coverage(x as f32 + 0.5 - cx, dy) * strength * paint[3] as f32
                            / 255.0;
                        if a <= 0.0 {
                            continue;
                        }
                        let px = &mut row[x as usize];
                        let dest = *px;
                        if preserve_alpha && dest[3] == 0 {
                            continue;
                        }
                        let mut out = over(dest, paint, a);
                        if preserve_alpha {
                            out[3] = dest[3];
                        }
                        if out != dest {
                            *px = out;
                            span = Some(span.map_or((x, x), |(l, r)| (l.min(x), r.max(x))));
                        }
                    }
                }
                span.map(|(l, r)| PsdRect {
                    top: y,
                    left: l,
                    bottom: y + 1,
                    right: r + 1,
                })
            })
            .reduce_with(PsdRect::union)
    }

    /// SplitMix64 as a float in `0.0..1.0`.
    fn next(&mut self) -> f32 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / 16_777_216.0
    }
}

/// `paint`'s colour over `dest` at coverage `a` (straight alpha).
pub(crate) fn over(dest: [u8; 4], paint: [u8; 4], a: f32) -> [u8; 4] {
    let da = dest[3] as f32 / 255.0;
    let out_a = a + da * (1.0 - a);
    if out_a <= 0.0 {
        return [0; 4];
    }
    let mix = |c: usize| {
        ((paint[c] as f32 * a + dest[c] as f32 * da * (1.0 - a)) / out_a)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [
        mix(0),
        mix(1),
        mix(2),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// The document-space `source` cut to a layer's rectangle, transparent where
/// the layer runs past the document.
pub(crate) fn layer_local(source: &RgbaImage, rect: PsdRect) -> RgbaImage {
    let (w, h) = (rect.width().max(0), rect.height().max(0));
    let mut out = RgbaImage {
        width: w,
        height: h,
        data: vec![[0; 4]; (w * h) as usize],
    };
    let (uw, uh) = (w.max(0) as usize, h.max(0) as usize);
    if uw == 0 || uh == 0 {
        return out;
    }
    let (sw, sh) = (source.width, source.height);
    out.data
        .par_chunks_mut(uw)
        .enumerate()
        .for_each(|(y, row)| {
            let sy = rect.top + y as i32;
            if sy < 0 || sy >= sh {
                return;
            }
            for (x, px) in row.iter_mut().enumerate() {
                let sx = rect.left + x as i32;
                if sx < 0 || sx >= sw {
                    continue;
                }
                *px = source.data[sy as usize * sw as usize + sx as usize];
            }
        });
    out
}

#[cfg(test)]
#[path = "art_history/tests.rs"]
mod tests;
