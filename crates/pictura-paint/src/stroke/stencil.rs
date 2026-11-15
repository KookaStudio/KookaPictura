use super::*;

/// A document-space coverage clip: the active selection. The stroke's layer
/// rect is in document coordinates, so the stencil maps its local index into
/// the clip directly.
#[derive(Clone, Debug)]
pub struct CoverageClip {
    pub data: pictura_core::Plane<u8>,
    pub width: u32,
}

/// A stroke's per-pixel inputs, resolved once per sample so a dab's loop walks
/// neither the layer path nor the channel list and re-derives no profile.
pub(super) struct Stencil<'a> {
    pub(super) base: [Option<&'a [u8]>; 4],
    pub(super) locked: bool,
    pub(super) inverse_grayscale: bool,
    pub(super) coverage_clip: Option<&'a CoverageClip>,
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
        let clip = match self.coverage_clip {
            None => 255,
            Some(c) => {
                let w = self.rect.width().max(1) as usize;
                let dx = self.rect.left + (i % w) as i32;
                let dy = self.rect.top + (i / w) as i32;
                if dx < 0 || dy < 0 {
                    0
                } else {
                    c.data
                        .sample(dy as usize * c.width as usize + dx as usize)
                        .unwrap_or(0)
                }
            }
        };
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

/// Apply [`mask_dab_output`] to every rect a per-dab sample changed.
pub(super) fn mask_dabs(
    clip: Option<&CoverageClip>,
    stroke_rect: PsdRect,
    doc: &Document,
    layer_path: &[usize],
    pixels: &mut crate::healing::RgbaImage,
    rects: &[PsdRect],
) {
    let Some(clip) = clip else {
        return;
    };
    let Some(layer) = super::layer_at(doc, layer_path) else {
        return;
    };
    for &rect in rects {
        mask_dab_output(clip, stroke_rect, layer, pixels, rect);
    }
}

/// Restore the parts of a per-dab engine's output the selection does not
/// cover, blending partial coverage between the pre-dab layer pixel and the
/// engine's result.
pub(super) fn mask_dab_output(
    clip: &CoverageClip,
    stroke_rect: PsdRect,
    layer: &Layer,
    pixels: &mut crate::healing::RgbaImage,
    rect: PsdRect,
) {
    let w = stroke_rect.width().max(1) as usize;
    for y in rect.top.max(0)..rect.bottom {
        for x in rect.left.max(0)..rect.right {
            let dx = stroke_rect.left + x;
            let dy = stroke_rect.top + y;
            let coverage = if dx < 0 || dy < 0 {
                0
            } else {
                clip.data
                    .sample(dy as usize * clip.width as usize + dx as usize)
                    .unwrap_or(0)
            };
            if coverage == 255 {
                continue;
            }
            let i = y as usize * w + x as usize;
            let Some(px) = pixels.data.get_mut(i) else {
                continue;
            };
            let orig = super::read_pixel(layer, i);
            if coverage == 0 {
                *px = [orig.0, orig.1, orig.2, orig.3];
            } else {
                let t = u32::from(coverage);
                let inv = 255 - t;
                *px = [
                    ((u32::from(px[0]) * t + u32::from(orig.0) * inv + 127) / 255) as u8,
                    ((u32::from(px[1]) * t + u32::from(orig.1) * inv + 127) / 255) as u8,
                    ((u32::from(px[2]) * t + u32::from(orig.2) * inv + 127) / 255) as u8,
                    ((u32::from(px[3]) * t + u32::from(orig.3) * inv + 127) / 255) as u8,
                ];
            }
        }
    }
}

impl Stroke {
    /// Constrain dab coverage with a document-space selection clip.
    pub fn set_coverage_clip(&mut self, clip: Option<CoverageClip>) {
        self.coverage_clip = clip;
    }
    /// Blend a grayscale coverage plane in its inverted editing convention.
    /// Only touched samples cross the boundary; saved pixels stay canonical.
    pub fn set_inverse_grayscale(&mut self, inverse: bool) {
        self.inverse_grayscale = inverse;
    }
}
