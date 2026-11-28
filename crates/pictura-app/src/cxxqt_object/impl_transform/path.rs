use super::super::document_session::{
    PathTransform, SessionKind, TransformMode, TransformSession, TransformTarget,
};
use super::super::edit::Edit;
use super::super::paths;
use super::super::qobject::PictureView;
use super::geometry::{quad_driven, session_quad, MapParams};
use super::group::repeatable_transform;
use super::warp::map_source_point;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{LockFlags, PsdRect};
use std::convert::TryFrom;

fn selection(
    view: &PictureView,
    components: &[i32],
    anchors: &[i32],
    repeat: bool,
) -> Option<(PathTransform, PsdRect)> {
    let rust = view.rust();
    let doc = rust.session.doc.as_ref()?;
    if rust.session.read_only()
        || !rust.slot_allows(SessionKind::Transform)
        || rust.session.live.transform().is_some()
    {
        return None;
    }
    let layer = paths::layer_target(rust);
    if let Some(target) = &layer {
        let layer = pictura_render::resolve_path(doc, target)?;
        if !layer.visible || layer.lock.contains(LockFlags::POSITION) {
            return None;
        }
    }
    let original = paths::path(view)?;
    let (anchor_pairs, remainder) = anchors.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    let components: Vec<usize> = components
        .iter()
        .map(|&i| usize::try_from(i).ok())
        .collect::<Option<_>>()?;
    let anchors: Vec<(usize, usize)> = anchor_pairs
        .iter()
        .map(|&[sp, pt]| Some((usize::try_from(sp).ok()?, usize::try_from(pt).ok()?)))
        .collect::<Option<_>>()?;
    if components.is_empty() && anchors.len() < 2 {
        return None;
    }
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut add = |b: (f64, f64, f64, f64)| {
        bounds = Some(bounds.map_or(b, |a| {
            (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
        }));
    };
    for &sp in &components {
        add(original.subpath_bounds(sp)?);
    }
    for &(sp, pt) in &anchors {
        let p = original.subpaths.get(sp)?.points.get(pt)?.anchor;
        add((p.0, p.1, p.0, p.1));
    }
    let (mut left, mut top, mut right, mut bottom) = bounds?;
    if ![left, top, right, bottom].iter().all(|v| v.is_finite()) {
        return None;
    }
    if repeat {
        if right == left {
            left -= 0.5;
            right += 0.5;
        }
        if bottom == top {
            top -= 0.5;
            bottom += 0.5;
        }
    }
    if right <= left || bottom <= top {
        return None;
    }
    let rect = PsdRect {
        left: left.floor() as i32,
        top: top.floor() as i32,
        right: right.ceil() as i32,
        bottom: bottom.ceil() as i32,
    };
    Some((
        PathTransform {
            original,
            bounds: [left, top, right, bottom],
            components,
            anchors,
            active: rust.active_path,
            layer,
            generation: rust.session.document_generation,
        },
        rect,
    ))
}

pub(super) fn can_begin(
    view: &PictureView,
    components: &[i32],
    anchors: &[i32],
    repeat: bool,
) -> bool {
    selection(view, components, anchors, repeat).is_some()
}

pub(super) fn begin(
    mut view: Pin<&mut PictureView>,
    components: &[i32],
    anchors: &[i32],
    mode: &QString,
    repeat: bool,
) -> bool {
    let Some((target, rect)) = selection(&view, components, anchors, repeat) else {
        return false;
    };
    let Some(mode) = TransformMode::parse(&mode.to_string()) else {
        return false;
    };
    let mut session = TransformSession::new(
        TransformTarget::Layer,
        String::new(),
        Vec::new(),
        rect,
        mode,
        false,
        "Transform Path",
    );
    session.path_transform = Some(target);
    let corners = session.source_corners();
    session.pivot = session.source_center();
    session.pivot_origin = session.pivot;
    session.start_pivot = session.pivot;
    session.start_quad = corners;
    session.quad = mode.is_projective().then_some(corners);
    view.as_mut().rust_mut().session.live.set_transform(session);
    true
}

pub(in crate::cxxqt_object) fn mapped(
    view: &PictureView,
) -> Option<pictura_core::path::VectorPath> {
    let rust = view.rust();
    let session = rust.session.live.transform()?;
    let target = session.path_transform.as_ref()?;
    let mut path = target.original.clone();
    for (sp, subpath) in path.subpaths.iter_mut().enumerate() {
        for (pt, point) in subpath.points.iter_mut().enumerate() {
            if !target.components.contains(&sp) && !target.anchors.contains(&(sp, pt)) {
                continue;
            }
            let map = |p: (f64, f64)| map_source_point(session, p.0, p.1).map(|p| (p[0], p[1]));
            point.anchor = map(point.anchor)?;
            if let Some(handle) = point.in_handle {
                point.in_handle = Some(map(handle)?);
            }
            if let Some(handle) = point.out_handle {
                point.out_handle = Some(map(handle)?);
            }
        }
    }
    Some(path)
}

pub(super) fn mapped_bounds(view: &PictureView) -> Option<[(f64, f64); 4]> {
    let path = mapped(view)?;
    let rust = view.rust();
    let selected = rust.session.live.transform()?.path_transform.as_ref()?;
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut add = |b: (f64, f64, f64, f64)| {
        bounds = Some(bounds.map_or(b, |a| {
            (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
        }));
    };
    for &sp in &selected.components {
        add(path.subpath_bounds(sp)?);
    }
    for &(sp, pt) in &selected.anchors {
        let p = path.subpaths.get(sp)?.points.get(pt)?.anchor;
        add((p.0, p.1, p.0, p.1));
    }
    let (l, t, r, b) = bounds?;
    Some([(l, t), (r, t), (r, b), (l, b)])
}

pub(super) fn is_identity(session: &TransformSession) -> bool {
    let unmoved = session_quad(session)
        .iter()
        .zip(session.source_corners())
        .all(|(a, b)| (a.0 - b.0).abs() <= 1e-9 && (a.1 - b.1).abs() <= 1e-9);
    match session.warp.as_ref() {
        Some(warp) => warp.is_identity() && unmoved,
        None if quad_driven(session) => unmoved,
        None => MapParams::of(session).is_identity(),
    }
}

pub(super) fn commit(mut view: Pin<&mut PictureView>) -> bool {
    let Some(updated) = mapped(&view) else {
        view.as_mut().cancel_transform();
        return false;
    };
    let Some(session) = view.as_mut().rust_mut().session.live.take_transform() else {
        return false;
    };
    let Some(target) = session.path_transform.as_ref() else {
        return false;
    };
    let valid = {
        let rust = view.rust();
        let editable = target.layer.as_ref().is_none_or(|layer| {
            rust.session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, layer))
                .is_some_and(|layer| layer.visible && !layer.lock.contains(LockFlags::POSITION))
        });
        !rust.session.read_only()
            && editable
            && rust.session.document_generation == target.generation
            && rust.active_path == target.active
            && paths::layer_target(rust) == target.layer
    };
    if !valid
        || paths::path(&view).as_ref() != Some(&target.original)
        || is_identity(&session)
        || updated == target.original
    {
        return false;
    }
    let repeat = repeatable_transform(&session);
    if paths::edit(view.as_mut(), |path| *path = updated).is_none() {
        return false;
    }
    if let Some(repeat) = repeat {
        view.as_mut().rust_mut().session.last_path_transform = Some(repeat);
    }
    view.as_mut().edit("Transform Path", |_| Edit::Meta);
    true
}

#[cfg(test)]
mod tests {
    use super::super::super::paths::ActivePath;
    use super::*;

    #[test]
    fn nondegenerate_subpixel_path_quad_is_accepted() {
        let rect = PsdRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        let mut session = TransformSession::new(
            TransformTarget::Layer,
            String::new(),
            Vec::new(),
            rect,
            TransformMode::Skew,
            false,
            "Transform Path",
        );
        let quad = [(0.1, 0.1), (0.6, 0.1), (0.6, 0.6), (0.1, 0.6)];
        assert!(super::super::geometry::session_quad_is_degenerate(
            &session, &quad
        ));
        session.path_transform = Some(PathTransform {
            original: Default::default(),
            bounds: [0.1, 0.1, 0.6, 0.6],
            components: Vec::new(),
            anchors: Vec::new(),
            active: ActivePath::Work,
            layer: None,
            generation: 0,
        });
        assert!(!super::super::geometry::session_quad_is_degenerate(
            &session, &quad
        ));
    }

    #[test]
    fn subpixel_scale_handle_does_not_enlarge_untouched_axis() {
        let rect = PsdRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        let mut session = TransformSession::new(
            TransformTarget::Layer,
            String::new(),
            Vec::new(),
            rect,
            TransformMode::Scale,
            false,
            "Transform Path",
        );
        session.path_transform = Some(PathTransform {
            original: Default::default(),
            bounds: [0.0, 0.0, 0.5, 0.5],
            components: Vec::new(),
            anchors: Vec::new(),
            active: ActivePath::Work,
            layer: None,
            generation: 0,
        });
        session.pivot = session.source_center();
        session.handle = 5;
        session.start = [1.0, 1.0, 0.0, 0.0, 0.0];
        assert!(super::super::geometry::gesture_scale(
            &mut session,
            0.25,
            0.25,
            false
        ));
        assert!((session.scale_x - 0.5).abs() < 1e-9);
        assert!((session.scale_y - 1.0).abs() < 1e-9);
    }

    #[test]
    fn fractional_warp_uses_the_path_source_frame() {
        let rect = PsdRect {
            left: 10,
            top: 10,
            right: 31,
            bottom: 21,
        };
        let bounds = [10.25, 10.75, 30.75, 20.25];
        let mut session = TransformSession::new(
            TransformTarget::Layer,
            String::new(),
            Vec::new(),
            rect,
            TransformMode::Free,
            false,
            "Transform Path",
        );
        session.path_transform = Some(PathTransform {
            original: Default::default(),
            bounds,
            components: Vec::new(),
            anchors: Vec::new(),
            active: ActivePath::Work,
            layer: None,
            generation: 0,
        });
        session.pivot = session.source_center();
        let warp =
            pictura_core::warp::Warp::preset(pictura_core::warp::WarpStyle::Arc, 35.0, bounds);
        let expected = warp.map(15.25, 13.75);
        session.warp = Some(warp);
        let actual = map_source_point(&session, 15.25, 13.75).expect("valid warp");
        assert!((actual[0] - expected.0).abs() < 1e-9);
        assert!((actual[1] - expected.1).abs() < 1e-9);
    }

    #[test]
    fn fractional_source_frame_maps_and_repeats_about_exact_centre() {
        let rect = PsdRect {
            left: 10,
            top: 10,
            right: 31,
            bottom: 21,
        };
        let mut session = TransformSession::new(
            TransformTarget::Layer,
            String::new(),
            Vec::new(),
            rect,
            TransformMode::Rotate,
            false,
            "Transform Path",
        );
        session.path_transform = Some(PathTransform {
            original: Default::default(),
            bounds: [10.25, 10.75, 30.75, 20.25],
            components: Vec::new(),
            anchors: Vec::new(),
            active: ActivePath::Work,
            layer: None,
            generation: 0,
        });
        session.pivot = session.source_center();
        session.angle = std::f64::consts::FRAC_PI_2;
        let point = map_source_point(&session, 10.25, 10.75).expect("valid map");
        assert!((point[0] - 25.25).abs() < 1e-9);
        assert!((point[1] - 5.25).abs() < 1e-9);
        let repeat = repeatable_transform(&session).expect("repeatable");
        assert_eq!(repeat.pivot_offset, (0.0, 0.0));
    }
}
