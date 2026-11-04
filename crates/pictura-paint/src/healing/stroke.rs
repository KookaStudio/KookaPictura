//! The healing gesture: accumulate a brush-stroke coverage mask over a layer,
//! then run the heal once on release. The mask is what [`heal_region`] and
//! [`clone_region`] take, so a spot click and a dragged repair share one path.
//!
//! [`heal_region`]: super::heal_region
//! [`clone_region`]: super::clone_region

use super::layer::{heal_layer, HealError};
use super::{clone_region, heal_region, HealMode, Transfer};
use crate::spacing::{DabPlacer, SpacingMode};
use crate::stroke::{layer_at, parse_layer_path};
use crate::tip::tip_coverage;
use crate::{StrokeConfig, StrokeOutcome};
use pictura_core::{layer_pixel_locked, Document, PsdRect};

/// A live healing gesture over one pixel layer.
pub struct HealStroke {
    coverage: Vec<f32>,
    rect: PsdRect,
    path: String,
    cfg: StrokeConfig,
    placer: DabPlacer,
    dirty: Option<PsdRect>,
    painted: bool,
}

impl HealStroke {
    /// Start a gesture on the pixel layer at `path` (a panel path such as
    /// `"0"`). The layer must be a painted pixel layer.
    pub fn begin(
        doc: &Document,
        path: &str,
        diameter: u32,
        hardness: u8,
    ) -> Result<Self, HealError> {
        let indices = parse_layer_path(path).ok_or(HealError::NoRasterLayer)?;
        let target = layer_at(doc, &indices).ok_or(HealError::NoRasterLayer)?;
        if target.is_group || target.adjustment.is_some() {
            return Err(HealError::NoRasterLayer);
        }
        if layer_pixel_locked(target) {
            return Err(HealError::Locked);
        }
        let rect = target.rect;
        let len = (rect.width().max(0) * rect.height().max(0)) as usize;
        let cfg = StrokeConfig {
            diameter,
            hardness,
            ..StrokeConfig::default()
        }
        .sanitized();
        Ok(HealStroke {
            coverage: vec![0.0; len],
            rect,
            path: path.to_string(),
            placer: DabPlacer::new(SpacingMode::Fixed(25), cfg.diameter as f32),
            cfg,
            dirty: None,
            painted: false,
        })
    }

    pub fn layer_rect(&self) -> PsdRect {
        self.rect
    }

    pub fn is_painted(&self) -> bool {
        self.painted
    }

    /// Add a dab at document-space `(x, y)`. Returns the document rectangle the
    /// mask grew by (for a preview), or `None` when nothing changed.
    pub fn dab(&mut self, x: f32, y: f32) -> Option<PsdRect> {
        let w = self.rect.width();
        let h = self.rect.height();
        if w <= 0 || h <= 0 {
            return None;
        }
        let (wu, _hu) = (w as usize, h as usize);
        let lx = x - self.rect.left as f32;
        let ly = y - self.rect.top as f32;
        let mut dabs = Vec::new();
        self.placer.feed(lx, ly, &mut dabs);
        if dabs.is_empty() {
            return None;
        }
        let radius = self.cfg.diameter as f32 * 0.5;
        let mut changed: Option<PsdRect> = None;
        for (cx, cy) in dabs {
            let x0 = ((cx - radius).floor() as i32).max(0);
            let x1 = ((cx + radius).ceil() as i32).min(w - 1);
            let y0 = ((cy - radius).floor() as i32).max(0);
            let y1 = ((cy + radius).ceil() as i32).min(h - 1);
            for py in y0..=y1 {
                for px in x0..=x1 {
                    let tip =
                        tip_coverage(&self.cfg, (px as f32 + 0.5) - cx, (py as f32 + 0.5) - cy);
                    if tip <= 0.0 {
                        continue;
                    }
                    let i = py as usize * wu + px as usize;
                    // Peaks like the paint engine: overlapping dabs do not push
                    // a pixel past its tip coverage.
                    self.coverage[i] = self.coverage[i].max(tip);
                    let r = PsdRect {
                        top: py,
                        left: px,
                        bottom: py + 1,
                        right: px + 1,
                    };
                    changed = union(changed, Some(r));
                    self.painted = true;
                }
            }
        }
        self.dirty = union(self.dirty, changed);
        changed.map(|r| offset(r, self.rect))
    }

    /// Run the heal against `doc` on release. `source` is the clone offset for
    /// the Healing Brush (`None` for Spot Healing, which infers its source from
    /// the surroundings). Returns what changed, or `None` when the mask was
    /// empty.
    pub fn commit(
        self,
        doc: &mut Document,
        mode: HealMode,
        source: Option<(i32, i32)>,
        transfer: Transfer,
    ) -> Result<Option<StrokeOutcome>, HealError> {
        if !self.painted {
            return Ok(None);
        }
        let dirty = self.dirty.expect("painted implies dirty");
        let region = offset(dirty, self.rect);
        let coverage = slice(&self.coverage, self.rect, dirty);
        let path = self.path;
        let changed = heal_layer(
            doc,
            &path,
            region,
            &coverage,
            move |img, local, cov| match source {
                Some(offset) => clone_region(img, local, cov, offset, transfer),
                None => heal_region(img, local, cov, mode),
            },
        )?;
        Ok(changed.map(|dirty| StrokeOutcome {
            document: doc.clone(),
            dirty,
        }))
    }
}

fn slice(coverage: &[f32], layer: PsdRect, dirty: PsdRect) -> Vec<f32> {
    let lw = layer.width().max(0) as usize;
    (dirty.top..dirty.bottom)
        .flat_map(|y| (dirty.left..dirty.right).map(move |x| (x, y)))
        .map(|(x, y)| coverage[y as usize * lw + x as usize])
        .collect()
}

fn union(a: Option<PsdRect>, b: Option<PsdRect>) -> Option<PsdRect> {
    match (a, b) {
        (None, None) => None,
        (Some(r), None) | (None, Some(r)) => Some(r),
        (Some(a), Some(b)) => Some(a.union(b)),
    }
}

fn offset(r: PsdRect, by: PsdRect) -> PsdRect {
    PsdRect {
        top: r.top + by.top,
        left: r.left + by.left,
        bottom: r.bottom + by.top,
        right: r.right + by.left,
    }
}
