use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::Document;

/// The active layer's `(density, feather)` from `read`.
pub(super) fn refinement(
    view: &PictureView,
    read: fn(&Document, &str) -> Option<(u8, f64)>,
) -> Option<(u8, f64)> {
    let rust = view.rust();
    read(rust.session.doc.as_ref()?, rust.layer_target.single()?)
}

/// Apply `set` to the active layer; recomposite on a change, and on a commit
/// record `label` when this call or an earlier preview changed the mask. A
/// preview another control left uncommitted is recorded under its own label
/// first, so each state holds one control's change.
pub(super) fn edit(
    mut view: Pin<&mut PictureView>,
    label: &'static str,
    commit: bool,
    set: impl FnOnce(&mut Document, &str) -> bool,
) -> bool {
    let pending = view.as_mut().rust_mut().pending_previews.mask_refine.take();
    if let Some(other) = pending.filter(|other| *other != label) {
        view.as_mut().edit(other, |_| Edit::Record);
    }
    let pending = pending == Some(label);
    let Some(path) = view.rust().layer_target.single().map(str::to_string) else {
        return false;
    };
    let before = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, &path));
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        rust.session.doc.as_mut().is_some_and(|doc| set(doc, &path))
    };
    let after = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, &path));
    let region = pictura_render::damage::union(before, after).map(|rect| vec![rect]);
    if !commit && (changed || pending) {
        view.as_mut().rust_mut().pending_previews.mask_refine = Some(label);
        view.as_mut().preview(|_| match region {
            Some(rects) => Edit::regions(rects, false),
            None => Edit::Full,
        });
    } else if commit && (changed || pending) {
        // An earlier preview already changed the document without a history
        // state, so the commit must not narrow history to this call's region.
        view.as_mut().rust_mut().pending_previews.mask_refine = Some(label);
        view.as_mut().edit(label, |_| match region {
            Some(rects) => Edit::regions(rects, true),
            None => Edit::Full,
        });
        view.as_mut().rust_mut().pending_previews.mask_refine = None;
    }
    changed
}

pub(super) fn layer_mask_density(view: &PictureView) -> i32 {
    refinement(view, pictura_render::layer_mask_refinement).map_or(-1, |(d, _)| i32::from(d))
}

pub(super) fn layer_mask_feather(view: &PictureView) -> f64 {
    refinement(view, pictura_render::layer_mask_refinement).map_or(0.0, |(_, f)| f)
}

pub(super) fn layer_mask_set_density(
    view: Pin<&mut PictureView>,
    density: i32,
    commit: bool,
) -> bool {
    let density = density.clamp(0, 255) as u8;
    edit(view, "Mask Density", commit, |doc, path| {
        pictura_render::set_layer_mask_density(doc, path, density)
    })
}

pub(super) fn layer_mask_set_feather(
    view: Pin<&mut PictureView>,
    radius: f64,
    commit: bool,
) -> bool {
    edit(view, "Mask Feather", commit, |doc, path| {
        pictura_render::set_layer_mask_feather(doc, path, radius)
    })
}

pub(super) fn layer_mask_invert(view: Pin<&mut PictureView>) -> bool {
    edit(view, "Invert", true, pictura_render::invert_layer_mask)
}

pub(super) fn vector_mask_density(view: &PictureView) -> i32 {
    refinement(view, pictura_render::vector_mask_refinement).map_or(-1, |(d, _)| i32::from(d))
}

pub(super) fn vector_mask_feather(view: &PictureView) -> f64 {
    refinement(view, pictura_render::vector_mask_refinement).map_or(0.0, |(_, f)| f)
}

pub(super) fn vector_mask_set_density(
    view: Pin<&mut PictureView>,
    density: i32,
    commit: bool,
) -> bool {
    let density = density.clamp(0, 255) as u8;
    edit(view, "Vector Mask Density", commit, |doc, path| {
        pictura_render::set_vector_mask_density(doc, path, density)
    })
}

pub(super) fn vector_mask_set_feather(
    view: Pin<&mut PictureView>,
    radius: f64,
    commit: bool,
) -> bool {
    edit(view, "Vector Mask Feather", commit, |doc, path| {
        pictura_render::set_vector_mask_feather(doc, path, radius)
    })
}
