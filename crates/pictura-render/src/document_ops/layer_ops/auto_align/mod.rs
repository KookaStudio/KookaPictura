//! Edit > Auto-Align Layers: match overlapping content across selected layers
//! and transform the non-reference layers so it lines up.
//!
//! Approximated, behavioural parity only: Adobe's feature detector, matcher,
//! thresholds, and "better composite" metric are unpublished. The matcher here
//! is deterministic and dependency-free; the projections use standard
//! panoramic formulas. See `docs/05-layers/align-and-distribute.md`.
//!
//! [`plan_auto_align`] only reads, so it runs on a worker over a document
//! clone; [`apply_auto_align`] lands the plan inside one edit.

mod estimate;
mod features;
mod image;
mod lens;
mod projection;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use pictura_core::{Document, LockFlags, PsdRect};
use pictura_filters::LensCorrectionParams;

use super::align::layer_content_bounds;
use super::paths::{parse_path, resolve_path, unique};
use super::transform::{
    transform_layer_quad, transform_layer_with, transform_layer_with_map, LayerTransform, PlaneMap,
};
use super::Interpolation;
use estimate::{
    apply_h, invert_h, ransac_homography, ransac_similarity, ransac_translation, score_overlap,
    Model, Pair,
};
use features::{detect, match_corners};
use image::WorkImage;

/// `progress` reaches this when a plan is complete.
pub const AUTO_ALIGN_PROGRESS_MAX: u32 = 1000;

/// One of the seven CS6 projection choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoAlignProjection {
    Auto,
    Perspective,
    Cylindrical,
    Spherical,
    SceneCollage,
    RepositionOnly,
    LensCorrection,
}

impl AutoAlignProjection {
    /// The dialog order, 0 Auto … 6 Lens Correction.
    pub fn from_index(index: i32) -> Option<Self> {
        Some(match index {
            0 => Self::Auto,
            1 => Self::Perspective,
            2 => Self::Cylindrical,
            3 => Self::Spherical,
            4 => Self::SceneCollage,
            5 => Self::RepositionOnly,
            6 => Self::LensCorrection,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoAlignProjectionKind {
    Cylindrical,
    Spherical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutoAlignOptions {
    pub projection: AutoAlignProjection,
    pub vignette_removal: bool,
    pub geometric_distortion: bool,
    /// Resolved by the app from EXIF and installed profiles; `None` skips it.
    pub lens: Option<LensCorrectionParams>,
}

/// One layer's transform, in document coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerWarp {
    Identity {
        path: String,
    },
    Translate {
        path: String,
        dx: f64,
        dy: f64,
    },
    Similarity {
        path: String,
        xx: f64,
        xy: f64,
        yx: f64,
        yy: f64,
        tx: f64,
        ty: f64,
    },
    Homography {
        path: String,
        h: [f64; 9],
    },
    Projected {
        path: String,
        kind: AutoAlignProjectionKind,
        h: [f64; 9],
        focal: f64,
        centre: (f64, f64),
    },
}

impl LayerWarp {
    pub fn path(&self) -> &str {
        match self {
            Self::Identity { path }
            | Self::Translate { path, .. }
            | Self::Similarity { path, .. }
            | Self::Homography { path, .. }
            | Self::Projected { path, .. } => path,
        }
    }
}

/// The computed alignment, ready for [`apply_auto_align`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoAlignPlan {
    pub reference: String,
    pub warps: Vec<LayerWarp>,
    /// The correction to bake before the warps, when Lens Correction applies.
    pub lens: Option<LensCorrectionParams>,
    pub notice: Option<String>,
}

/// The run was cancelled; nothing was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoAlignCancelled;

struct Control<'a> {
    cancel: &'a AtomicBool,
    progress: &'a AtomicU32,
    done: AtomicU32,
    total: u32,
}

impl Control<'_> {
    fn check(&self) -> Result<(), AutoAlignCancelled> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(AutoAlignCancelled)
        } else {
            Ok(())
        }
    }

    fn step(&self) -> Result<(), AutoAlignCancelled> {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        let share = done.min(self.total) * AUTO_ALIGN_PROGRESS_MAX / self.total.max(1);
        self.progress.store(share, Ordering::Relaxed);
        self.check()
    }
}

/// Whether `path` and every group above it are visible.
fn visible_in_tree(doc: &Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let mut layers: &[pictura_core::Layer] = &doc.layers;
    segments.iter().all(|&index| {
        let Some(layer) = layers.get(index) else {
            return false;
        };
        layers = &layer.children;
        layer.visible
    })
}

/// A raster layer Auto-Align can move.
fn eligible(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group
            && layer.adjustment.is_none()
            && crate::fill::decode_layer_fill(layer).is_none()
            && layer.type_tool.is_none()
            && layer.vector_mask.is_none()
            && layer.smart_object.is_none()
            && !layer.background
            && !layer.clipping
            && visible_in_tree(doc, path)
    }) && super::layer_has_content(doc, path)
}

fn targets<'a>(doc: &Document, paths: &[&'a str]) -> Vec<&'a str> {
    let mut kept: Vec<&str> = unique(paths)
        .into_iter()
        .filter(|path| eligible(doc, path))
        .collect();
    kept.sort_by_key(|path| parse_path(path));
    kept
}

/// Whether Auto-Align applies to `paths`: two eligible layers with content.
pub fn can_auto_align(doc: &Document, paths: &[&str]) -> bool {
    targets(doc, paths).len() >= 2
}

fn union_of(doc: &Document, paths: &[&str]) -> Option<PsdRect> {
    let mut rects = paths
        .iter()
        .filter_map(|path| layer_content_bounds(doc, path));
    let first = rects.next()?;
    Some(rects.fold(first, |all, r| all.union(r)))
}

/// The reference path: a position-locked layer when one is eligible, else the
/// content centre nearest the union centre.
fn reference(doc: &Document, paths: &[&str]) -> Option<String> {
    let mut all = Vec::new();
    let mut locked = Vec::new();
    for path in paths {
        let layer = resolve_path(doc, path)?;
        let bounds = layer_content_bounds(doc, path)?;
        all.push(((*path).to_string(), bounds));
        if layer_move_locked(layer) {
            locked.push(((*path).to_string(), bounds));
        }
    }
    let pool = if locked.is_empty() { &all } else { &locked };
    let union = union_of(doc, paths)?;
    let centre = (
        f64::from(union.left + union.right) / 2.0,
        f64::from(union.top + union.bottom) / 2.0,
    );
    pool.iter()
        .min_by(|(pa, a), (pb, b)| {
            let da = ((f64::from(a.left + a.right) / 2.0) - centre.0)
                .hypot((f64::from(a.top + a.bottom) / 2.0) - centre.1);
            let db = ((f64::from(b.left + b.right) / 2.0) - centre.0)
                .hypot((f64::from(b.top + b.bottom) / 2.0) - centre.1);
            da.total_cmp(&db)
                .then_with(|| parse_path(pa).cmp(&parse_path(pb)))
        })
        .map(|(path, _)| path.clone())
}

fn layer_move_locked(layer: &pictura_core::Layer) -> bool {
    layer.lock.contains(LockFlags::POSITION)
}

fn focal_and_centre(doc: &Document, paths: &[&str]) -> Option<(f64, (f64, f64))> {
    let union = union_of(doc, paths)?;
    let width = f64::from(union.width().max(1));
    let height = f64::from(union.height().max(1));
    Some((
        1.2 * width.max(height),
        (
            f64::from(union.left + union.right) / 2.0,
            f64::from(union.top + union.bottom) / 2.0,
        ),
    ))
}

fn pairs_for(moving: &WorkImage, reference: &WorkImage) -> Vec<Pair> {
    let a = detect(moving);
    let b = detect(reference);
    match_corners(&a, &b)
        .into_iter()
        .map(|(i, j)| {
            let (ax, ay) = moving.to_document(a[i].x, a[i].y);
            let (bx, by) = reference.to_document(b[j].x, b[j].y);
            Pair {
                a: (ax, ay),
                b: (bx, by),
            }
        })
        .collect()
}

fn score_model(
    reference: &WorkImage,
    moving: &WorkImage,
    inverse: impl Fn(f64, f64) -> Option<(f64, f64)>,
) -> Option<f64> {
    score_overlap(reference, moving, |x, y| {
        let (dx, dy) = reference.to_document(x, y);
        let (mx, my) = inverse(dx, dy)?;
        Some(moving.to_work(mx, my))
    })
}

fn auto_model(
    reference: &WorkImage,
    moving: &WorkImage,
    pairs: &[Pair],
    focal: f64,
    centre: (f64, f64),
    threshold: f64,
) -> Option<(Model, Option<AutoAlignProjectionKind>, Option<[f64; 9]>)> {
    let perspective = ransac_homography(pairs, threshold).and_then(|m| {
        let h = m.homography()?;
        let inv = invert_h(&h)?;
        let score = score_model(reference, moving, |x, y| apply_h(&inv, x, y))?;
        Some((score, h))
    });
    let cylinder = projection::estimate(
        projection::Kind::Cylindrical,
        pairs,
        focal,
        centre,
        threshold,
    )
    .and_then(|h| {
        let map = projection::ProjectedMap::new(projection::Kind::Cylindrical, h, focal, centre)?;
        let score = score_model(reference, moving, |x, y| map.inverse(x, y))?;
        Some((score, h))
    });
    match (perspective, cylinder) {
        (Some((ps, ph)), Some((cs, ch))) => {
            if cs > ps {
                Some((
                    Model::Homography { h: ch },
                    Some(AutoAlignProjectionKind::Cylindrical),
                    Some(ch),
                ))
            } else {
                Some((Model::Homography { h: ph }, None, None))
            }
        }
        (Some((_, h)), None) => Some((Model::Homography { h }, None, None)),
        (None, Some((_, h))) => Some((
            Model::Homography { h },
            Some(AutoAlignProjectionKind::Cylindrical),
            Some(h),
        )),
        (None, None) => None,
    }
}

/// Plan the alignment of `paths`. The plan is empty when [`can_auto_align`] is
/// false. `progress` climbs to [`AUTO_ALIGN_PROGRESS_MAX`]; setting `cancel`
/// ends the run with [`AutoAlignCancelled`].
pub fn plan_auto_align(
    doc: &Document,
    paths: &[&str],
    options: AutoAlignOptions,
    cancel: &AtomicBool,
    progress: &AtomicU32,
) -> Result<AutoAlignPlan, AutoAlignCancelled> {
    let targets = targets(doc, paths);
    if targets.len() < 2 {
        return Ok(AutoAlignPlan::default());
    }
    let reference = reference(doc, &targets).unwrap_or_else(|| targets[0].to_string());
    let lens_active =
        options.projection == AutoAlignProjection::LensCorrection && options.lens.is_some();
    let control = Control {
        cancel,
        progress,
        done: AtomicU32::new(0),
        total: (3 * targets.len() + 4) as u32,
    };
    control.check()?;
    let lens_maps: Vec<Option<pictura_filters::LensMap>> = targets
        .iter()
        .map(|path| {
            let layer = resolve_path(doc, path)?;
            let params = options.lens.as_ref().filter(|_| lens_active)?;
            let (w, h) = (
                layer.rect.width().max(1) as u32,
                layer.rect.height().max(1) as u32,
            );
            Some(pictura_filters::LensMap::new(params, w, h))
        })
        .collect();
    let images: Vec<Option<WorkImage>> = targets
        .iter()
        .zip(&lens_maps)
        .map(|(path, lens)| {
            control.step().ok()?;
            WorkImage::build(doc, path, lens.as_ref())
        })
        .collect();
    control.step()?;
    let reference_index = targets.iter().position(|p| *p == reference).unwrap_or(0);
    let Some(reference_image) = images.get(reference_index).and_then(Option::as_ref) else {
        return Ok(AutoAlignPlan {
            notice: Some("Auto-Align could not read the reference layer.".into()),
            ..AutoAlignPlan::default()
        });
    };
    let (focal, centre) = focal_and_centre(doc, &targets).unwrap_or((1024.0, (0.0, 0.0)));
    let mut warps = Vec::new();
    let mut notice = None;
    if options.projection == AutoAlignProjection::LensCorrection && options.lens.is_none() {
        notice = Some("No matching lens profile; lens correction was skipped.".to_string());
    }
    for (i, path) in targets.iter().enumerate() {
        control.step()?;
        if i == reference_index {
            warps.push(LayerWarp::Identity {
                path: (*path).to_string(),
            });
            continue;
        }
        let Some(moving) = images.get(i).and_then(Option::as_ref) else {
            notice.get_or_insert_with(|| "An unreadable layer was left unchanged.".into());
            continue;
        };
        let threshold = 3.0 * moving.scale.max(reference_image.scale);
        let pairs = pairs_for(moving, reference_image);
        if pairs.len() < 4 {
            let name =
                resolve_path(doc, path).map_or_else(|| (*path).to_string(), |l| l.name.clone());
            notice.get_or_insert_with(|| format!("Could not match {name}; it was left unchanged."));
            continue;
        }
        let chosen = match options.projection {
            AutoAlignProjection::RepositionOnly => {
                ransac_translation(&pairs, threshold).map(|m| (m, None, None))
            }
            AutoAlignProjection::SceneCollage => {
                ransac_similarity(&pairs, threshold).map(|m| (m, None, None))
            }
            AutoAlignProjection::Perspective => {
                ransac_homography(&pairs, threshold).map(|m| (m, None, None))
            }
            AutoAlignProjection::Cylindrical | AutoAlignProjection::Spherical => {
                let kind = if options.projection == AutoAlignProjection::Cylindrical {
                    projection::Kind::Cylindrical
                } else {
                    projection::Kind::Spherical
                };
                projection::estimate(kind, &pairs, focal, centre, threshold).map(|h| {
                    (
                        Model::Homography { h },
                        Some(match kind {
                            projection::Kind::Cylindrical => AutoAlignProjectionKind::Cylindrical,
                            projection::Kind::Spherical => AutoAlignProjectionKind::Spherical,
                        }),
                        Some(h),
                    )
                })
            }
            AutoAlignProjection::Auto | AutoAlignProjection::LensCorrection => {
                auto_model(reference_image, moving, &pairs, focal, centre, threshold)
            }
        };
        let Some((model, projected, h)) = chosen else {
            notice.get_or_insert_with(|| "Could not align a layer; it was left unchanged.".into());
            continue;
        };
        warps.push(match (projected, h) {
            (Some(kind), Some(h)) => LayerWarp::Projected {
                path: (*path).to_string(),
                kind,
                h,
                focal,
                centre,
            },
            _ => match model {
                Model::Translation { dx, dy } => LayerWarp::Translate {
                    path: (*path).to_string(),
                    dx,
                    dy,
                },
                Model::Similarity { a, b, tx, ty } => LayerWarp::Similarity {
                    path: (*path).to_string(),
                    xx: a,
                    xy: -b,
                    yx: b,
                    yy: a,
                    tx,
                    ty,
                },
                Model::Homography { h } => LayerWarp::Homography {
                    path: (*path).to_string(),
                    h,
                },
            },
        });
    }
    control.step()?;
    progress.store(AUTO_ALIGN_PROGRESS_MAX, Ordering::Relaxed);
    Ok(AutoAlignPlan {
        reference,
        warps,
        lens: lens_active.then(|| options.lens.clone()).flatten(),
        notice,
    })
}

fn quad(rect: PsdRect, map: impl Fn(f64, f64) -> Option<(f64, f64)>) -> Option<[(f64, f64); 4]> {
    let corners = [
        (f64::from(rect.left), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.bottom)),
        (f64::from(rect.left), f64::from(rect.bottom)),
    ];
    let mut out = [(0.0, 0.0); 4];
    for (i, (x, y)) in corners.into_iter().enumerate() {
        out[i] = map(x, y)?;
    }
    Some(out)
}

/// Apply a plan inside the caller's one edit. Returns every processed path,
/// the unchanged reference included, so the UI can keep the run selected.
pub fn apply_auto_align(doc: &mut Document, plan: AutoAlignPlan) -> Vec<String> {
    let mut changed = Vec::new();
    for warp in &plan.warps {
        let path = warp.path().to_string();
        let Some(rect) = resolve_path(doc, &path).map(|l| l.rect) else {
            continue;
        };
        let mut moved = false;
        if let Some(params) = plan.lens.as_ref() {
            if let Some(layer) = resolve_path_mut(doc, &path) {
                moved |= lens::apply(layer, params);
            }
        }
        moved |= match warp {
            LayerWarp::Identity { .. } => false,
            LayerWarp::Translate { dx, dy, .. } => transform_layer_with(
                doc,
                &path,
                LayerTransform {
                    scale_x: 1.0,
                    scale_y: 1.0,
                    angle_radians: 0.0,
                    dx: *dx,
                    dy: *dy,
                },
                Interpolation::Bilinear,
            ),
            LayerWarp::Similarity {
                xx,
                xy,
                yx,
                yy,
                tx,
                ty,
                ..
            } => quad(rect, |x, y| {
                Some((*xx * x + *xy * y + *tx, *yx * x + *yy * y + *ty))
            })
            .is_some_and(|q| transform_layer_quad(doc, &path, q)),
            LayerWarp::Homography { h, .. } => quad(rect, |x, y| apply_h(h, x, y))
                .is_some_and(|q| transform_layer_quad(doc, &path, q)),
            LayerWarp::Projected {
                kind,
                h,
                focal,
                centre,
                ..
            } => {
                let kind = match kind {
                    AutoAlignProjectionKind::Cylindrical => projection::Kind::Cylindrical,
                    AutoAlignProjectionKind::Spherical => projection::Kind::Spherical,
                };
                transform_layer_with_map(
                    doc,
                    &path,
                    |_| projection::ProjectedMap::new(kind, *h, *focal, *centre),
                    false,
                    Interpolation::Bilinear,
                )
            }
        };
        if moved || matches!(warp, LayerWarp::Identity { .. }) {
            changed.push(path);
        }
    }
    changed
}

fn resolve_path_mut<'a>(doc: &'a mut Document, path: &str) -> Option<&'a mut pictura_core::Layer> {
    super::paths::resolve_path_mut(doc, path)
}
