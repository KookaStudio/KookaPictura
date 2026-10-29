//! Healing: rebuilding a region from the pixels around it (Spot Healing Brush)
//! or from an offset source with the destination's own lighting (Healing Brush,
//! Patch). Unlike a brush stroke, healing needs the *original* pixels around the
//! hole, so it runs once on the finished stroke mask or selection.
//!
//! Spot Healing's three CS6 types:
//!
//! * **Proximity Match** — a search around the hole for the source whose
//!   surroundings match ([`proximity`]), transplanted with the Healing Brush's
//!   Poisson solve; a Laplace solve over the hole when no source fits.
//! * **Create Texture** — a Laplace solve with the surrounding ring as a fixed
//!   boundary, plus noise matched to the roughness of the ring (fixed seed).
//!
//! Both rebuild a margin past the brush core, so a defect's soft rim is part of
//! the hole rather than its boundary.
//! * **Content-Aware** — patch synthesis ([`synthesis`]): an inward onion-peel
//!   fill refined by PatchMatch search-and-vote.
//!
//! The Healing Brush and Patch transplant the source's *gradient* with a
//! Poisson solve, holding the destination around the region fixed.
//!
//! Behavioural parity only: Adobe's solver is closed
//! (`docs/03-tools/healing-brushes.md` records its biharmonic refinement,
//! which this does not attempt). ponytail: single-threaded (photorust uses
//! rayon); fine for brush-sized regions.
//!
//! Ported from photorust's `core/src/healing.rs`
//! (<https://github.com/perfecto25/photorust>).

mod content_move;
mod layer;
mod patch;
mod proximity;
mod red_eye;
mod stroke;
mod synthesis;
#[cfg(test)]
mod tests;

pub use content_move::{move_layer, MoveOptions};
pub use layer::{heal_layer, HealError};
pub use patch::{patch_layer, PatchOptions};
pub use red_eye::{red_eye_layer, RED_EYE_DEFAULT_DARKEN, RED_EYE_DEFAULT_PUPIL};
pub use stroke::HealStroke;

use pictura_core::PsdRect;

/// Spot Healing's Type buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HealMode {
    /// The spec's default (`docs/03-tools/healing-brushes.md`).
    #[default]
    ProximityMatch,
    CreateTexture,
    ContentAware,
}

impl HealMode {
    /// 0 Proximity Match, 1 Create Texture, 2 Content-Aware; `None` otherwise.
    pub fn from_i32(v: i32) -> Option<HealMode> {
        match v {
            0 => Some(HealMode::ProximityMatch),
            1 => Some(HealMode::CreateTexture),
            2 => Some(HealMode::ContentAware),
            _ => None,
        }
    }
}

/// CS6's Content-Aware Adaptation menu: how literally the fill copies from the
/// immediate surroundings. Stricter levels match larger patches over a smaller
/// reach; looser ones match smaller patches from farther away, so the result
/// is reshuffled more. The mapping onto patch size and reach is inferred
/// (`docs/03-tools/content-aware-move-and-patch.md`: Adobe's is undocumented).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Adaptation {
    VeryStrict,
    Strict,
    /// The spec's default.
    #[default]
    Medium,
    Loose,
    VeryLoose,
}

impl Adaptation {
    /// 0 Very Strict … 4 Very Loose (the menu order); `None` otherwise.
    pub fn from_i32(v: i32) -> Option<Adaptation> {
        [
            Adaptation::VeryStrict,
            Adaptation::Strict,
            Adaptation::Medium,
            Adaptation::Loose,
            Adaptation::VeryLoose,
        ]
        .get(usize::try_from(v).ok()?)
        .copied()
    }

    /// The synthesis patch radius and search reach, in pixels.
    fn patch_and_search(self) -> (i32, i32) {
        match self {
            Adaptation::VeryStrict => (3, 12),
            Adaptation::Strict => (3, 18),
            Adaptation::Medium => (2, 24),
            Adaptation::Loose => (1, 32),
            Adaptation::VeryLoose => (1, 48),
        }
    }
}

/// How much of a cloned source to transfer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Transfer {
    /// Texture and colour, with the destination's lighting.
    #[default]
    Full,
    /// Texture only: the destination keeps its own colour (Patch's Transparent).
    TextureOnly,
}

/// A row-major RGBA8 pixel grid; `(0, 0)` is its top-left pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: i32,
    pub height: i32,
    pub data: Vec<[u8; 4]>,
}

impl RgbaImage {
    pub(crate) fn get(&self, x: i32, y: i32) -> [u8; 4] {
        self.data[(y * self.width + x) as usize]
    }

    pub(crate) fn set(&mut self, x: i32, y: i32, px: [u8; 4]) {
        self.data[(y * self.width + x) as usize] = px;
    }

    pub(crate) fn rect(&self) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: self.height,
            right: self.width,
        }
    }
}

/// Coverage at or above which a pixel is rebuilt; below it (a soft edge) the
/// pixel is only blended toward the fill, so the heal is not larger than the
/// brush.
const HOLE_THRESHOLD: f32 = 0.5;

/// How far outside the hole boundary data is read.
const BORDER: i32 = 4;

/// Gauss-Seidel sweeps; ample for the small regions a brush covers.
const RELAX_SWEEPS: usize = 96;

/// Over-relaxation factor of the Poisson solve. A source lit differently from
/// the destination starts a whole level off, and plain Gauss-Seidel leaves part
/// of that step after [`RELAX_SWEEPS`] on a brush-sized hole.
const POISSON_OMEGA: f32 = 1.85;

/// The working area: the region grown by `margin` and [`BORDER`], its covered
/// pixels and those within `margin` of the brush core marked as the hole.
struct Work {
    rect: PsdRect,
    w: usize,
    h: usize,
    /// How much of the fill each pixel takes on write-back.
    weight: Vec<f32>,
    hole: Vec<bool>,
}

impl Work {
    /// `None` when nothing is covered, or the hole fills the whole working area
    /// (no boundary to reconstruct from).
    fn new(canvas: PsdRect, region: PsdRect, coverage: &[f32], margin: i32) -> Option<Work> {
        let rw = region.width().max(0) as usize;
        if coverage.len() != rw * region.height().max(0) as usize {
            return None;
        }
        let rect = intersect(inflate(region, margin + BORDER), canvas);
        if rect.width() <= 0 || rect.height() <= 0 {
            return None;
        }
        let (w, h) = (rect.width() as usize, rect.height() as usize);
        let mut cov = vec![0.0f32; w * h];
        let mut hole = vec![false; w * h];
        for y in rect.top.max(region.top)..rect.bottom.min(region.bottom) {
            for x in rect.left.max(region.left)..rect.right.min(region.right) {
                let c = coverage[(y - region.top) as usize * rw + (x - region.left) as usize]
                    .clamp(0.0, 1.0);
                let i = (y - rect.top) as usize * w + (x - rect.left) as usize;
                cov[i] = c;
                hole[i] = c >= HOLE_THRESHOLD;
            }
        }
        if margin > 0 {
            let near = proximity::distance_to(&proximity::core(&cov), w, h);
            for (i, &d) in near.iter().enumerate() {
                let t = proximity::margin_weight(d, margin);
                if t > 0.0 {
                    hole[i] = true;
                    cov[i] = cov[i].max(t);
                }
            }
        }
        if !hole.iter().any(|&h| h) || hole.iter().all(|&h| h) {
            return None;
        }
        Some(Work {
            rect,
            w,
            h,
            weight: cov,
            hole,
        })
    }

    fn read(&self, img: &RgbaImage, offset: (i32, i32)) -> Vec<[f32; 4]> {
        let mut out = Vec::with_capacity(self.w * self.h);
        for y in self.rect.top..self.rect.bottom {
            for x in self.rect.left..self.rect.right {
                // A source off the canvas clamps to its edge rather than reading
                // transparent, which would drag the gradient toward nothing.
                let sx = (x + offset.0).clamp(0, img.width - 1);
                let sy = (y + offset.1).clamp(0, img.height - 1);
                out.push(img.get(sx, sy).map(f32::from));
            }
        }
        out
    }

    /// Blend `solved` over `original` by weight, so a soft edge fades in.
    fn write_back(&self, img: &mut RgbaImage, original: &[[f32; 4]], solved: &[[f32; 4]]) {
        for y in self.rect.top..self.rect.bottom {
            for x in self.rect.left..self.rect.right {
                let i = (y - self.rect.top) as usize * self.w + (x - self.rect.left) as usize;
                let t = self.weight[i];
                if t <= 0.0 {
                    continue;
                }
                let (a, b) = (original[i], solved[i]);
                let mix = |c: usize| (a[c] + (b[c] - a[c]) * t).round().clamp(0.0, 255.0) as u8;
                img.set(x, y, [mix(0), mix(1), mix(2), mix(3)]);
            }
        }
    }
}

/// Rebuild the region of `img` marked by `coverage` (one value per pixel of
/// `region`, row-major, `0..=1`) from its surroundings — the Spot Healing Brush.
/// Returns the rectangle modified, or `None` when there was nothing to do.
pub fn heal_region(
    img: &mut RgbaImage,
    region: PsdRect,
    coverage: &[f32],
    mode: HealMode,
) -> Option<PsdRect> {
    heal_region_adapted(img, region, coverage, mode, Adaptation::Medium)
}

/// As [`heal_region`], with the Content-Aware synthesis at `adaptation`.
fn heal_region_adapted(
    img: &mut RgbaImage,
    region: PsdRect,
    coverage: &[f32],
    mode: HealMode,
    adaptation: Adaptation,
) -> Option<PsdRect> {
    let margin = match mode {
        HealMode::ContentAware => 0,
        HealMode::ProximityMatch | HealMode::CreateTexture => proximity::margin(region, coverage),
    };
    let work = Work::new(img.rect(), region, coverage, margin)?;
    let rgba = work.read(img, (0, 0));
    let filled = match mode {
        HealMode::ProximityMatch => {
            let smooth = laplace_fill(&rgba, &work.hole, work.w, work.h);
            match proximity::best_source(img, &work, &rgba, &smooth) {
                Some(offset) => {
                    let src = work.read(img, offset);
                    poisson_fill(&rgba, &src, &work.hole, work.w, work.h)
                }
                None => smooth,
            }
        }
        HealMode::CreateTexture => {
            let mut smooth = laplace_fill(&rgba, &work.hole, work.w, work.h);
            add_matched_noise(&mut smooth, &rgba, &work.hole, work.w, work.h);
            smooth
        }
        HealMode::ContentAware => {
            synthesis::content_aware_fill(&rgba, &work.hole, work.w, work.h, adaptation)
        }
    };
    work.write_back(img, &rgba, &filled);
    Some(intersect(inflate(region, margin), img.rect()))
}

/// Transplant the texture at `source` (an offset added to each destination
/// pixel) into the covered region, keeping the destination's lighting — the
/// Healing Brush and the Patch tool. A Poisson solve, not a copy: the source's
/// gradient is carried over while the destination around the region stays
/// fixed as the boundary, so the repair has no visible seam. A zero offset is a
/// no-op. Returns the rectangle modified.
pub fn clone_region(
    img: &mut RgbaImage,
    region: PsdRect,
    coverage: &[f32],
    source: (i32, i32),
    transfer: Transfer,
) -> Option<PsdRect> {
    if source == (0, 0) {
        return None;
    }
    let work = Work::new(img.rect(), region, coverage, 0)?;
    let dest = work.read(img, (0, 0));
    let src = work.read(img, source);
    let mut out = poisson_fill(&dest, &src, &work.hole, work.w, work.h);
    if transfer == Transfer::TextureOnly {
        keep_destination_colour(&mut out, &dest, &work.hole);
    }
    work.write_back(img, &dest, &out);
    Some(intersect(region, img.rect()))
}

/// Successive over-relaxation on the Poisson equation: each unknown moves past
/// the mean of its neighbours plus the mean source difference to them, which
/// carries the source's detail over while `dest` around the hole stays the
/// boundary.
fn poisson_fill(
    dest: &[[f32; 4]],
    src: &[[f32; 4]],
    hole: &[bool],
    w: usize,
    h: usize,
) -> Vec<[f32; 4]> {
    let mut out = dest.to_vec();
    for i in 0..out.len() {
        if hole[i] {
            out[i] = src[i];
        }
    }
    for _ in 0..RELAX_SWEEPS {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !hole[i] {
                    continue;
                }
                let mut acc = [0.0f32; 4];
                let mut n = 0.0f32;
                for j in neighbours(i, x, y, w, h).into_iter().flatten() {
                    for c in 0..4 {
                        acc[c] += out[j][c] + (src[i][c] - src[j][c]);
                    }
                    n += 1.0;
                }
                if n > 0.0 {
                    let was = out[i];
                    out[i] = std::array::from_fn(|c| {
                        (was[c] + POISSON_OMEGA * (acc[c] / n - was[c])).clamp(0.0, 255.0)
                    });
                }
            }
        }
    }
    out
}

/// The 4-neighbours of `(x, y)` inside a `w`×`h` grid; outside ones are skipped
/// (a zero-flux edge).
fn neighbours(i: usize, x: usize, y: usize, w: usize, h: usize) -> [Option<usize>; 4] {
    [
        (x > 0).then(|| i - 1),
        (x + 1 < w).then(|| i + 1),
        (y > 0).then(|| i - w),
        (y + 1 < h).then(|| i + w),
    ]
}

/// Solve Laplace's equation over the hole with the known pixels as boundary,
/// seeded with the mean of the ring around the hole so it converges quickly.
fn laplace_fill(rgba: &[[f32; 4]], hole: &[bool], w: usize, h: usize) -> Vec<[f32; 4]> {
    let mut out = rgba.to_vec();
    let mut sum = [0.0f32; 4];
    let mut ring = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if hole[i]
                || !neighbours(i, x, y, w, h)
                    .into_iter()
                    .flatten()
                    .any(|j| hole[j])
            {
                continue;
            }
            for c in 0..4 {
                sum[c] += rgba[i][c];
            }
            ring += 1;
        }
    }
    if ring == 0 {
        return out;
    }
    let mean = sum.map(|v| v / ring as f32);
    for (i, px) in out.iter_mut().enumerate() {
        if hole[i] {
            *px = mean;
        }
    }
    for _ in 0..RELAX_SWEEPS {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !hole[i] {
                    continue;
                }
                let mut acc = [0.0f32; 4];
                let mut n = 0.0f32;
                for j in neighbours(i, x, y, w, h).into_iter().flatten() {
                    for c in 0..4 {
                        acc[c] += out[j][c];
                    }
                    n += 1.0;
                }
                if n > 0.0 {
                    out[i] = acc.map(|v| v / n);
                }
            }
        }
    }
    out
}

/// Add noise to a smooth fill, its strength matched to the known pixels'
/// deviation from their local mean. A fixed-seed LCG keeps it reproducible.
fn add_matched_noise(fill: &mut [[f32; 4]], rgba: &[[f32; 4]], hole: &[bool], w: usize, h: usize) {
    let mut sum_sq = [0.0f32; 3];
    let mut n = 0.0f32;
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            if hole[i] || hole[i - 1] || hole[i + 1] || hole[i - w] || hole[i + w] {
                continue;
            }
            for c in 0..3 {
                let local =
                    (rgba[i - 1][c] + rgba[i + 1][c] + rgba[i - w][c] + rgba[i + w][c]) / 4.0;
                let d = rgba[i][c] - local;
                sum_sq[c] += d * d;
            }
            n += 1.0;
        }
    }
    if n < 1.0 {
        return;
    }
    let sigma = sum_sq.map(|v| (v / n).sqrt());
    let mut state: u32 = 0x9E37_79B9;
    let mut next = || -> f32 {
        let mut draw = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((state >> 8) & 0xFFFF) as f32 / 65535.0
        };
        // Two draws averaged: closer to a bell than a flat spread.
        draw() + draw() - 1.0
    };
    for (i, px) in fill.iter_mut().enumerate() {
        if hole[i] {
            for c in 0..3 {
                px[c] = (px[c] + next() * sigma[c]).clamp(0.0, 255.0);
            }
        }
    }
}

/// Keep only the solved luminance: rescale the destination's own channels to
/// it, so hue and saturation stay where the patch lands.
fn keep_destination_colour(solved: &mut [[f32; 4]], dest: &[[f32; 4]], hole: &[bool]) {
    let luma = |px: &[f32; 4]| 0.299 * px[0] + 0.587 * px[1] + 0.114 * px[2];
    for i in 0..solved.len() {
        let have = luma(&dest[i]);
        // Near-black has no colour to keep; leave the solve alone.
        if !hole[i] || have <= 1.0 {
            continue;
        }
        let scale = luma(&solved[i]) / have;
        for c in 0..3 {
            solved[i][c] = (dest[i][c] * scale).clamp(0.0, 255.0);
        }
        solved[i][3] = dest[i][3];
    }
}

fn inflate(r: PsdRect, by: i32) -> PsdRect {
    PsdRect {
        top: r.top - by,
        left: r.left - by,
        bottom: r.bottom + by,
        right: r.right + by,
    }
}

fn intersect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.max(b.top),
        left: a.left.max(b.left),
        bottom: a.bottom.min(b.bottom),
        right: a.right.min(b.right),
    }
}
