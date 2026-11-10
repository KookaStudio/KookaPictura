//! Auto-Align engine tests: synthetic textures with known transforms.

use super::estimate::{apply_h, invert_h};
use super::image::WorkImage;
use super::projection::{Kind, ProjectedMap};
use super::*;
use pictura_core::{Document, Layer, LockFlags, PsdRect, SmartObject, VectorMask};
use pictura_filters::{LensCorrectionParams, LensEdge, LensModel};
use std::sync::atomic::{AtomicBool, AtomicU32};

const W: u32 = 192;
const H: u32 = 144;

/// A deterministic high-contrast texture: a noisy checkerboard with many
/// corners and no repeating 8×8 cell.
fn texture(w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let fx = f64::from(x);
            let fy = f64::from(y);
            let wave = 128.0
                + 60.0 * (0.13 * fx + 0.07 * fy).sin()
                + 50.0 * (0.31 * fx - 0.17 * fy).sin()
                + 25.0 * (0.9 * fx + 0.4 * fy).sin();
            let n = f64::from((x * 31 + y * 17) % 11) - 5.0;
            let v = (wave + n).clamp(0.0, 255.0) as u8;
            let i = ((y * w + x) * 4) as usize;
            out[i] = v;
            out[i + 1] = v.wrapping_add(37);
            out[i + 2] = v.wrapping_add(91);
            out[i + 3] = 255;
        }
    }
    out
}

/// A document with a locked reference ("0") and a moving layer offset by
/// `offset`. Returns `(document, reference path, moving path)`.
fn pair(offset: (i32, i32)) -> (Document, String, String) {
    let tex = texture(W, H);
    let mut doc = Document::from_rgba("base", W, H, &tex);
    doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
    let moving = super::super::create::add_raster_layer_from_rgba(&mut doc, "moving", W, H, &tex);
    let layer = doc.layers.last_mut().expect("moving");
    layer.rect = PsdRect {
        left: offset.0,
        top: offset.1,
        right: offset.0 + W as i32,
        bottom: offset.1 + H as i32,
    };
    (doc, "0".to_string(), moving)
}

fn plan(
    doc: &Document,
    paths: &[&str],
    projection: AutoAlignProjection,
    lens: Option<LensCorrectionParams>,
) -> AutoAlignPlan {
    let cancel = AtomicBool::new(false);
    let progress = AtomicU32::new(0);
    plan_auto_align(
        doc,
        paths,
        AutoAlignOptions {
            projection,
            vignette_removal: true,
            geometric_distortion: true,
            lens,
        },
        &cancel,
        &progress,
    )
    .expect("not cancelled")
}

fn warp_for<'a>(plan: &'a AutoAlignPlan, path: &str) -> &'a LayerWarp {
    plan.warps
        .iter()
        .find(|w| w.path() == path)
        .expect("a warp for the layer")
}

fn rect_of(doc: &Document, name: &str) -> PsdRect {
    doc.layers
        .iter()
        .find(|l| l.name == name)
        .expect("layer")
        .rect
}

fn corner(layer: &Layer) -> u8 {
    layer
        .channels
        .iter()
        .find(|c| c.id == 0)
        .and_then(|c| c.data.first().copied())
        .unwrap_or(0)
}

#[test]
fn reposition_only_recovers_a_translation() {
    let (doc, base, moving) = pair((14, -9));
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::RepositionOnly,
        None,
    );
    assert_eq!(p.reference, base);
    match warp_for(&p, &moving) {
        LayerWarp::Translate { dx, dy, .. } => {
            assert!((dx + 14.0).abs() <= 1.0, "dx {dx}");
            assert!((dy - 9.0).abs() <= 1.0, "dy {dy}");
        }
        other => panic!("expected a translation, got {other:?}"),
    }
    let mut applied = doc.clone();
    apply_auto_align(&mut applied, p);
    let r = rect_of(&applied, "moving");
    assert!((r.left as f64).abs() <= 1.0, "left {}", r.left);
    assert!((r.top as f64).abs() <= 1.0, "top {}", r.top);
}

#[test]
fn perspective_recovers_a_homography() {
    let (mut doc, base, moving) = pair((0, 0));
    let quad = [
        (0.0, 0.0),
        (W as f64, 0.0),
        (W as f64 - 18.0, H as f64),
        (18.0, H as f64),
    ];
    assert!(super::super::transform::transform_layer_quad(
        &mut doc, &moving, quad
    ));
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::Perspective,
        None,
    );
    let h = match warp_for(&p, &moving) {
        LayerWarp::Homography { h, .. } => *h,
        other => panic!("expected a homography, got {other:?}"),
    };
    for (q, want) in quad.into_iter().zip([
        (0.0, 0.0),
        (W as f64, 0.0),
        (W as f64, H as f64),
        (0.0, H as f64),
    ]) {
        let got = apply_h(&h, q.0, q.1).expect("finite");
        assert!(
            (got.0 - want.0).hypot(got.1 - want.1) <= 6.0,
            "corner {q:?} -> {got:?}, want {want:?}"
        );
    }
}

#[test]
fn scene_collage_preserves_shape() {
    let (mut doc, base, moving) = pair((0, 0));
    let (cx, cy) = (W as f64 / 2.0, H as f64 / 2.0);
    let (sin, cos) = 0.22f64.sin_cos();
    let s = 0.9;
    let corners = [
        (0.0, 0.0),
        (W as f64, 0.0),
        (W as f64, H as f64),
        (0.0, H as f64),
    ];
    let quad = corners.map(|(x, y)| {
        let (dx, dy) = (x - cx, y - cy);
        (
            cx + s * (cos * dx - sin * dy),
            cy + s * (sin * dx + cos * dy),
        )
    });
    assert!(super::super::transform::transform_layer_quad(
        &mut doc, &moving, quad
    ));
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::SceneCollage,
        None,
    );
    match warp_for(&p, &moving) {
        LayerWarp::Similarity { xx, xy, yx, yy, .. } => {
            assert!((xx - yy).abs() < 1e-6, "equal scales");
            assert!((xy + yx).abs() < 1e-6, "no projective terms");
            assert!(xx * yy - xy * yx > 0.0);
        }
        other => panic!("expected a similarity, got {other:?}"),
    }
}

fn projected_warp(
    doc: &Document,
    base: &str,
    moving: &str,
    kind: AutoAlignProjection,
) -> (LayerWarp, WorkImage, WorkImage) {
    let p = plan(doc, &[base, moving], kind, None);
    let warp = warp_for(&p, moving).clone();
    let reference = WorkImage::build(doc, base, None).expect("reference image");
    let moving_image = WorkImage::build(doc, moving, None).expect("moving image");
    (warp, reference, moving_image)
}

fn score_warp(reference: &WorkImage, moving: &WorkImage, warp: &LayerWarp) -> Option<f64> {
    match warp {
        LayerWarp::Homography { h, .. } => {
            let inv = invert_h(h)?;
            super::score_model(reference, moving, |x, y| apply_h(&inv, x, y))
        }
        LayerWarp::Projected {
            kind,
            h,
            focal,
            centre,
            ..
        } => {
            let kind = match kind {
                AutoAlignProjectionKind::Cylindrical => Kind::Cylindrical,
                AutoAlignProjectionKind::Spherical => Kind::Spherical,
            };
            let map = ProjectedMap::new(kind, *h, *focal, *centre)?;
            super::score_model(reference, moving, |x, y| map.inverse(x, y))
        }
        _ => None,
    }
}

#[test]
fn cylindrical_and_spherical_improve_overlap() {
    let tex = texture(320, 120);
    let mut doc = Document::from_rgba("base", 320, 120, &tex);
    doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
    let moving =
        super::super::create::add_raster_layer_from_rgba(&mut doc, "moving", 320, 120, &tex);
    let layer = doc.layers.last_mut().expect("moving");
    layer.rect = PsdRect {
        left: 42,
        top: 5,
        right: 362,
        bottom: 125,
    };
    let reference = WorkImage::build(&doc, "0", None).expect("reference");
    let moving_image = WorkImage::build(&doc, &moving, None).expect("moving");
    let unaligned =
        super::score_model(&reference, &moving_image, |x, y| Some((x, y))).expect("overlap");
    for projection in [
        AutoAlignProjection::Cylindrical,
        AutoAlignProjection::Spherical,
    ] {
        let (warp, _, _) = projected_warp(&doc, "0", &moving, projection);
        assert!(
            matches!(warp, LayerWarp::Projected { .. }),
            "{projection:?} produced {warp:?}"
        );
        let aligned = score_warp(&reference, &moving_image, &warp).expect("score");
        assert!(
            aligned > unaligned,
            "{projection:?}: aligned {aligned} not above unaligned {unaligned}"
        );
    }
}

#[test]
fn auto_keeps_the_better_projection() {
    let (doc, base, moving) = pair((14, -9));
    let auto = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::Auto,
        None,
    );
    let perspective = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::Perspective,
        None,
    );
    let cylindrical = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::Cylindrical,
        None,
    );
    let reference = WorkImage::build(&doc, &base, None).expect("reference");
    let image = WorkImage::build(&doc, &moving, None).expect("moving");
    let auto_score = score_warp(&reference, &image, warp_for(&auto, &moving)).expect("auto score");
    let p_score =
        score_warp(&reference, &image, warp_for(&perspective, &moving)).expect("perspective score");
    let c_score =
        score_warp(&reference, &image, warp_for(&cylindrical, &moving)).expect("cylindrical score");
    assert!(
        auto_score >= p_score.max(c_score) - 1e-6,
        "auto {auto_score}, perspective {p_score}, cylindrical {c_score}"
    );
}

#[test]
fn a_locked_layer_is_the_reference() {
    let (doc, base, moving) = pair((14, -9));
    let before = doc.layers[0].clone();
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::RepositionOnly,
        None,
    );
    assert_eq!(p.reference, base);
    assert!(matches!(warp_for(&p, &base), LayerWarp::Identity { .. }));
    let mut applied = doc.clone();
    apply_auto_align(&mut applied, p);
    assert_eq!(
        applied.layers[0], before,
        "the locked reference is untouched"
    );
}

#[test]
fn the_centre_most_layer_is_the_reference() {
    let tex = texture(W, H);
    let mut doc = Document::from_rgba("left", W, H, &tex);
    for (name, x) in [("middle", 40), ("right", 80)] {
        let path = super::super::create::add_raster_layer_from_rgba(&mut doc, name, W, H, &tex);
        let layer = doc.layers.last_mut().expect("layer");
        layer.rect = PsdRect {
            left: x,
            top: 0,
            right: x + W as i32,
            bottom: H as i32,
        };
        let _ = path;
    }
    let paths = ["0", "1", "2"];
    let p = plan(&doc, &paths, AutoAlignProjection::RepositionOnly, None);
    assert_eq!(
        p.reference, "1",
        "the middle content centre is nearest the union centre"
    );
}

#[test]
fn ineligible_layers_are_skipped() {
    let (mut doc, base, moving) = pair((14, -9));
    let tex = texture(W, H);
    let group = super::super::create::add_group_in(&mut doc, "", "Group");
    let fill = super::super::create::add_solid_fill(&mut doc, "", [255, 0, 0, 255]);
    let smart = super::super::create::add_raster_layer_from_rgba(&mut doc, "smart", W, H, &tex);
    if let Some(layer) = doc.layers.iter_mut().find(|l| l.name == "smart") {
        layer.smart_object = Some(SmartObject::default());
    }
    let shape = super::super::create::add_raster_layer_from_rgba(&mut doc, "shape", W, H, &tex);
    if let Some(layer) = doc.layers.iter_mut().find(|l| l.name == "shape") {
        layer.vector_mask = Some(VectorMask::default());
    }
    let all: [&str; 6] = [
        base.as_str(),
        moving.as_str(),
        group.as_str(),
        fill.as_str(),
        smart.as_str(),
        shape.as_str(),
    ];
    let p = plan(&doc, &all, AutoAlignProjection::RepositionOnly, None);
    assert!(p
        .warps
        .iter()
        .all(|w| w.path() == base || w.path() == moving));
    assert!(!can_auto_align(&doc, &[fill.as_str(), smart.as_str()]));
    let one = plan(
        &doc,
        &[base.as_str(), fill.as_str(), smart.as_str()],
        AutoAlignProjection::RepositionOnly,
        None,
    );
    assert!(one.warps.is_empty(), "fewer than two eligible layers");
}

#[test]
fn lens_correction_uses_the_profile() {
    let params = LensCorrectionParams {
        profile: Some(LensModel {
            distortion: LensModel::brown([0.12, 0.0, 0.0]),
            ca: LensModel::NONE.ca,
            vignette: [0.15, 0.0, 0.0],
        }),
        auto_scale: true,
        edge: LensEdge::Transparency,
        ..Default::default()
    };
    let (doc, base, moving) = pair((10, 4));
    let base_corner = corner(&doc.layers[0]);
    let moving_corner = corner(doc.layers.last().expect("moving"));
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::LensCorrection,
        Some(params.clone()),
    );
    assert_eq!(p.lens, Some(params));
    let mut applied = doc.clone();
    apply_auto_align(&mut applied, p);
    assert_ne!(
        corner(&applied.layers[0]),
        base_corner,
        "reference corrected"
    );
    let moved = applied
        .layers
        .iter()
        .find(|l| l.name == "moving")
        .expect("moving");
    assert_ne!(corner(moved), moving_corner, "moving layer corrected");
}

#[test]
fn cancelling_leaves_the_document_unchanged() {
    let (doc, base, moving) = pair((14, -9));
    let cancel = AtomicBool::new(true);
    let progress = AtomicU32::new(0);
    let result = plan_auto_align(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignOptions {
            projection: AutoAlignProjection::RepositionOnly,
            vignette_removal: false,
            geometric_distortion: false,
            lens: None,
        },
        &cancel,
        &progress,
    );
    assert_eq!(result, Err(AutoAlignCancelled));
}

#[test]
fn the_plan_is_deterministic() {
    let (doc, base, moving) = pair((14, -9));
    let paths = [base.as_str(), moving.as_str()];
    let a = plan(&doc, &paths, AutoAlignProjection::Perspective, None);
    let b = plan(&doc, &paths, AutoAlignProjection::Perspective, None);
    assert_eq!(a, b);
}

#[test]
fn applying_one_plan_changes_only_the_targets() {
    let (mut doc, base, moving) = pair((14, -9));
    let mut other_texture = texture(W, H);
    for (i, v) in other_texture.iter_mut().enumerate() {
        *v = v.wrapping_add((i % 7) as u8);
    }
    let other =
        super::super::create::add_raster_layer_from_rgba(&mut doc, "other", W, H, &other_texture);
    let before = doc
        .layers
        .iter()
        .find(|l| l.name == "other")
        .cloned()
        .expect("other");
    let p = plan(
        &doc,
        &[base.as_str(), moving.as_str()],
        AutoAlignProjection::RepositionOnly,
        None,
    );
    apply_auto_align(&mut doc, p);
    let after = doc
        .layers
        .iter()
        .find(|l| l.name == "other")
        .expect("other");
    assert_eq!(after, &before, "unselected layer {other} untouched");
}
