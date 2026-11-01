//! Edit > Auto-Blend Layers: one generated raster mask per processed layer, for
//! a panorama or an image stack, with an optional clipped tonal correction.
//!
//! Approximated, behavioural parity only: Adobe's focus metric, seam method and
//! tonal model are unpublished. See `docs/05-layers/align-and-distribute.md`.
//!
//! [`plan_auto_blend`] only reads, so it can run on a worker over a document
//! clone; [`apply_auto_blend`] installs the result inside one edit.
//!
//! ponytail: video and 3D layers are not refused, because the document model has
//! neither kind (#1110). They blend as the raster layers they load as.
//! ponytail: a layer's vector mask is ignored by the analysis.

mod apply;
mod gains;
mod pyramid;
mod seams;
mod sources;
mod stack;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use pictura_core::{AdjustmentData, ColorMode, Document, Layer, LayerMask, PsdRect};
use rayon::prelude::*;

use super::paths::{parse_path, resolve_path, unique};
use crate::fill::decode_layer_fill;

pub use apply::apply_auto_blend;

use sources::Grid;

/// The name of a correction layer, which a later run replaces.
pub const AUTO_BLEND_TONES: &str = "Auto-Blend Tones";

/// `progress` reaches this when a plan is complete.
pub const AUTO_BLEND_PROGRESS_MAX: u32 = 1000;

/// What the blend optimizes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendObjective {
    Panorama,
    StackImages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoBlendOptions {
    pub objective: BlendObjective,
    pub seamless: bool,
}

/// The run was cancelled; nothing was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

/// One processed layer's generated result.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendedLayer {
    pub path: String,
    pub mask: LayerMask,
    pub correction: Option<AdjustmentData>,
}

/// The computed blend, bottom layer first, ready for [`apply_auto_blend`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoBlendPlan {
    pub layers: Vec<BlendedLayer>,
}

/// Cancellation and progress shared with the caller's thread.
pub(super) struct Control<'a> {
    cancel: &'a AtomicBool,
    progress: &'a AtomicU32,
    done: AtomicU32,
    total: u32,
}

impl Control<'_> {
    pub(super) fn check(&self) -> Result<(), Cancelled> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    /// Finish one unit of work.
    pub(super) fn step(&self) -> Result<(), Cancelled> {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        let share = done.min(self.total) * AUTO_BLEND_PROGRESS_MAX / self.total.max(1);
        self.progress.store(share, Ordering::Relaxed);
        self.check()
    }
}

fn blendable_mode(doc: &Document) -> bool {
    matches!(
        doc.source_mode.unwrap_or(doc.mode),
        ColorMode::Rgb | ColorMode::Grayscale
    )
}

/// Whether `path` and every group above it are visible.
fn visible_in_tree(doc: &Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let mut layers: &[Layer] = &doc.layers;
    segments.iter().all(|&index| {
        let Some(layer) = layers.get(index) else {
            return false;
        };
        layers = &layer.children;
        layer.visible
    })
}

/// Whether a selected layer contributes pixels. The others are skipped without a
/// mask (inferred; the Help is silent on them).
fn eligible(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group
            && layer.adjustment.is_none()
            && decode_layer_fill(layer).is_none()
            && layer.type_tool.is_none()
            && layer.smart_object.is_none()
    }) && visible_in_tree(doc, path)
        && super::layer_content_bounds(doc, path).is_some()
}

fn refused(layer: &Layer) -> bool {
    layer.background || layer.smart_object.is_some() || layer.clipping
}

/// The eligible `paths`, bottom of the stack first.
fn targets<'a>(doc: &Document, paths: &[&'a str]) -> Vec<&'a str> {
    let mut kept: Vec<&str> = unique(paths)
        .into_iter()
        .filter(|path| eligible(doc, path))
        .collect();
    kept.sort_by_key(|path| parse_path(path));
    kept
}

/// Whether Auto-Blend applies to `paths`: an RGB or Grayscale document, no
/// Background, Smart Object or clipped layer among them, and two eligible
/// layers. A Smart Object outside `paths` does not matter.
pub fn can_auto_blend(doc: &Document, paths: &[&str]) -> bool {
    blendable_mode(doc)
        && paths
            .iter()
            .all(|path| resolve_path(doc, path).is_some_and(|layer| !refused(layer)))
        && targets(doc, paths).len() >= 2
}

/// Plan the blend of `paths`. The plan is empty when [`can_auto_blend`] is
/// false. `progress` climbs to [`AUTO_BLEND_PROGRESS_MAX`]; setting `cancel`
/// ends the run with [`Cancelled`].
pub fn plan_auto_blend(
    doc: &Document,
    paths: &[&str],
    options: AutoBlendOptions,
    cancel: &AtomicBool,
    progress: &AtomicU32,
) -> Result<AutoBlendPlan, Cancelled> {
    if !can_auto_blend(doc, paths) {
        return Ok(AutoBlendPlan::default());
    }
    let targets = targets(doc, paths);
    let layers: Vec<&Layer> = targets
        .iter()
        .filter_map(|path| resolve_path(doc, path))
        .collect();
    let control = Control {
        cancel,
        progress,
        done: AtomicU32::new(0),
        total: 3 * layers.len() as u32 + 1,
    };
    let Some(domain) = union_of_content(doc, &targets) else {
        return Ok(AutoBlendPlan::default());
    };
    let grid = Grid::new(domain, sources::MAX_WORK_PIXELS);

    let channel_gains = if options.seamless {
        gains::solve(doc, &layers, domain, &control)?
    } else {
        vec![[1.0; 3]; layers.len()]
    };
    let (labels, conflict) = match options.objective {
        BlendObjective::StackImages => (stack::label(doc, &layers, &grid, &control)?, None),
        BlendObjective::Panorama => {
            let luma: Vec<f32> = channel_gains.iter().map(gains::luma_gain).collect();
            let seams = seams::label(doc, &layers, &grid, &luma, &control)?;
            (seams.ids, Some(seams.conflict))
        }
    };
    let gray = doc.mode == ColorMode::Grayscale;
    let masks = masks(doc, &layers, &grid, &labels, conflict.as_deref(), &control)?;
    let blended = targets
        .iter()
        .zip(masks)
        .zip(&channel_gains)
        .map(|((path, mask), gain)| BlendedLayer {
            path: path.to_string(),
            mask,
            correction: gains::curve(gain, gray),
        })
        .collect();
    progress.store(AUTO_BLEND_PROGRESS_MAX, Ordering::Relaxed);
    Ok(AutoBlendPlan { layers: blended })
}

fn union_of_content(doc: &Document, paths: &[&str]) -> Option<PsdRect> {
    paths
        .iter()
        .filter_map(|path| super::layer_content_bounds(doc, path))
        .reduce(|a, b| PsdRect {
            top: a.top.min(b.top),
            left: a.left.min(b.left),
            bottom: a.bottom.max(b.bottom),
            right: a.right.max(b.right),
        })
}

/// Turn the label map into one mask per layer. Each mask holds the stacked alpha
/// that makes the layer's share of the composite match its weight:
/// `mask = w / (alpha * sum of w up to this layer)`.
///
/// ponytail: assumes Normal mode at 100 % opacity and Fill.
fn masks(
    doc: &Document,
    layers: &[&Layer],
    grid: &Grid,
    labels: &[u16],
    conflict: Option<&[f32]>,
    control: &Control,
) -> Result<Vec<LayerMask>, Cancelled> {
    let merged = merged_luma(doc, layers, grid, labels, control)?;
    let bands = pyramid::Bands::new(&merged, conflict, grid.w, grid.h);
    drop(merged);
    let mut running = vec![0f32; grid.w * grid.h];
    let mut out = Vec::with_capacity(layers.len());
    for (index, layer) in layers.iter().enumerate() {
        let alpha = sources::scan(doc, layer, grid, false, control)?.alpha;
        let hot: Vec<f32> = labels
            .iter()
            .map(|&id| f32::from(u8::from(id as usize == index)))
            .collect();
        let weights = bands.soften(&hot);
        let stacked: Vec<f32> = running
            .par_iter_mut()
            .zip(weights.par_iter().zip(alpha.par_iter()))
            .map(|(sum, (&soft, &coverage))| {
                let weight = soft * coverage;
                *sum += weight;
                if *sum > 0.0 && coverage > 0.0 {
                    (weight / (coverage * *sum)).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            })
            .collect();
        out.push(LayerMask {
            rect: layer.rect,
            default_color: 0,
            flags: super::layer_masks::MASK_FLAG_LINKED,
            data: Some(grid.expand(&stacked, layer.rect).into()),
            ..Default::default()
        });
        control.step()?;
    }
    Ok(out)
}

/// The luma of whichever layer labels each cell.
fn merged_luma(
    doc: &Document,
    layers: &[&Layer],
    grid: &Grid,
    labels: &[u16],
    control: &Control,
) -> Result<Vec<f32>, Cancelled> {
    let mut merged = vec![0f32; grid.w * grid.h];
    for (index, layer) in layers.iter().enumerate() {
        let luma = sources::scan(doc, layer, grid, false, control)?.luma;
        for ((out, &id), value) in merged.iter_mut().zip(labels).zip(luma) {
            if id as usize == index {
                *out = value;
            }
        }
        control.step()?;
    }
    Ok(merged)
}
