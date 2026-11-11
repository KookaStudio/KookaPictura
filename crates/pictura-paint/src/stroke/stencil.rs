use super::*;

/// A stroke's per-pixel inputs, resolved once per sample so a dab's loop walks
/// neither the layer path nor the channel list and re-derives no profile.
pub(super) struct Stencil<'a> {
    pub(super) base: [Option<&'a [u8]>; 4],
    pub(super) locked: bool,
    pub(super) inverse_grayscale: bool,
    pub(super) coverage_clip: Option<&'a pictura_core::Plane>,
    pub(super) cfg: StrokeConfig,
    /// Clone Stamp / Pattern Stamp / History Brush: the colour at each pixel
    /// comes from the source instead of `paint`.
    pub(super) source: Option<&'a StampSource>,
    pub(super) rect: PsdRect,
}

impl Stencil<'_> {
    /// Lay the paint at `coverage` over the saved pixel `i` (layer-local) and
    /// write it to `planes` at `at`, the same pixel's index in the band the
    /// planes were cut to. `rng` is the stroke's Dissolve state.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn composite(
        &self,
        planes: &mut CellPlanes<'_>,
        at: usize,
        i: usize,
        rng: Option<&mut u64>,
        coverage: u8,
        paint: Rgba,
        opacity: f32,
    ) -> bool {
        let clip = self.coverage_clip.map_or(255, |p| p.sample(i).unwrap_or(0));
        let cov = coverage as f32 / 255.0 * clip as f32 / 255.0;
        if clip == 0 {
            return false;
        }
        let a = (cov * opacity).clamp(0.0, 1.0);
        let plane = |k: usize, missing: u8| {
            self.base[k]
                .and_then(|d| d.get(i))
                .copied()
                .unwrap_or(missing)
        };
        let dr = plane(0, 0);
        let dr = if self.inverse_grayscale { 255 - dr } else { dr };
        let dg = plane(1, 0);
        let db = plane(2, 0);
        // A layer without an alpha channel (a Background) is opaque.
        let da = plane(3, 255);
        let s = match self.source {
            None => paint,
            Some(source) => {
                let w = self.rect.width() as usize;
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                match source.at(self.rect.left + x, self.rect.top + y) {
                    Some(color) => color,
                    None => return false,
                }
            }
        };
        let mode = self.cfg.mode;
        let roll = match rng {
            Some(rng) if mode == PaintMode::Dissolve => Stroke::next_f32(rng),
            _ => 0.0,
        };
        let Some((r, g, b, alpha)) = blend_pixel(mode, (dr, dg, db, da), &s, a, roll) else {
            return false;
        };
        let r = if self.inverse_grayscale { 255 - r } else { r };
        // A transparency lock preserves each pixel's alpha: fully transparent
        // pixels stay untouched, everything else keeps its pre-stroke alpha.
        // (Clear/auto-erase are refused for the whole stroke in `begin_at`.)
        if self.locked {
            if da == 0 {
                return false;
            }
            write_pixel(planes, at, mode, (r, g, b, da))
        } else {
            write_pixel(planes, at, mode, (r, g, b, alpha))
        }
    }
}

impl Stroke {
    /// Constrain dab coverage with a plane in the target layer's stored geometry.
    pub fn set_coverage_clip(&mut self, clip: Option<pictura_core::Plane>) {
        self.coverage_clip = clip;
    }
    /// Blend a grayscale coverage plane in its inverted editing convention.
    /// Only touched samples cross the boundary; saved pixels stay canonical.
    pub fn set_inverse_grayscale(&mut self, inverse: bool) {
        self.inverse_grayscale = inverse;
    }
}
